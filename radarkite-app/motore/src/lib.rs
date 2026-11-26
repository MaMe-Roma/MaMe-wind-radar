//! Radar kite 4.0 - motore. Porta in Rust radar-kite.ps1 3.4 (la fonte di verita').
//! Fase 1 del piano: regole, download in parallelo con cache, Windguru e firing, Open-Meteo, mare, oggetto D.
//! Stazioni, impostazioni e tempi di viaggio arrivano nella fase 2; storico, diario, PDF e giro ogni ora nella fase 3.

pub mod previsioni;
pub mod regole;
pub mod rete;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{Duration, Local, NaiveDate, NaiveDateTime, Timelike};
use serde_json::{json, Map, Value};

use previsioni::*;
use regole::*;

pub const VERSIONE: &str = "4.0";
pub const DATA_VERSIONE: &str = "26/11/2026";
pub const AUTORE: &str = "@MadHatterMatt";
pub const CONTATTO: &str = "arch.mattia.merusi+radarkite@gmail.com";

/// La pagina della 3.4 (report-modello.html) con le poche righe per parlare con l'app.
pub const MODELLO: &str = include_str!("../../pagina/report-modello.html");

/// Numero come lo scrive ConvertTo-Json: senza ",0" quando e' intero.
pub fn num(x: f64) -> Value {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        Value::from(x as i64)
    } else {
        serde_json::Number::from_f64(x).map(Value::Number).unwrap_or(Value::Null)
    }
}
fn num_o(x: Option<f64>) -> Value {
    x.map(num).unwrap_or(Value::Null)
}

pub struct Opzioni {
    /// Cartella dell'app: report.html, stato.js e la cartella dati.
    pub cartella: PathBuf,
    /// Cache delle risposte dei siti (di solito cartella/dati/cache).
    pub cache: Option<PathBuf>,
    /// Solo per le prove: legge le risposte dalla cache invece che dai siti.
    pub solo_cache: bool,
    /// Solo per le prove: l'istante "adesso" (per confrontare con un report della 3.4).
    pub adesso: Option<NaiveDateTime>,
    /// Solo per le prove: Open-Meteo da un file (come -OmProva della 3.4).
    pub om_prova: Option<PathBuf>,
    /// Pagina da riempire (di solito quella inclusa nel programma).
    pub modello: Option<PathBuf>,
    /// 1 = pagina subito con le previsioni, 2 = pagina completa. A mano: [1, 2]; giro automatico: [2].
    pub fasi: Vec<u8>,
}

impl Opzioni {
    pub fn nuove(cartella: PathBuf) -> Self {
        Opzioni { cartella, cache: None, solo_cache: false, adesso: None, om_prova: None, modello: None, fasi: vec![1, 2] }
    }
}

struct Log {
    righe: Mutex<Vec<String>>,
    t0: std::time::Instant,
}
impl Log {
    fn scrivi(&self, s: &str) {
        use std::io::Write;
        let _ = writeln!(std::io::stdout(), "{s}");
        self.righe.lock().unwrap().push(s.to_string());
    }
    fn tempo(&self, cosa: &str) {
        self.scrivi(&format!("  [{:5.1} s] {cosa}", self.t0.elapsed().as_secs_f64()));
    }
}

struct EsitoSpot {
    s: &'static Spot,
    wg: Option<InfoWg>,
    giorni: Vec<Giorno>,
    errore: Option<String>,
}

pub struct Esito {
    pub report: PathBuf,
    pub firing: Vec<String>,
}

fn scrivi_sostituendo(file: &Path, contenuto: &[u8]) -> std::io::Result<()> {
    // Prima un file temporaneo, poi lo sostituisco: la pagina che si ricarica non trova mai un file a meta'.
    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, contenuto)?;
    std::fs::rename(&tmp, file)
}

/// JSON sicuro dentro <script>: come ConvertTo-Json, < > & ' diventano \u003c ecc.
fn json_per_pagina(v: &Value) -> String {
    let s = serde_json::to_string(v).unwrap_or_else(|_| "null".into());
    s.replace('<', "\\u003c").replace('>', "\\u003e").replace('&', "\\u0026").replace('\'', "\\u0027").replace('\u{2028}', "\\u2028").replace('\u{2029}', "\\u2029")
}

pub fn esegui(o: &Opzioni) -> Result<Esito, String> {
    let log = Log { righe: Mutex::new(Vec::new()), t0: std::time::Instant::now() };
    let adesso = o.adesso.unwrap_or_else(|| Local::now().naive_local());
    let dir_dati = o.cartella.join("dati");
    std::fs::create_dir_all(&dir_dati).map_err(|e| format!("cartella dati non creata: {e}"))?;
    let dir_cache = o.cache.clone().unwrap_or_else(|| dir_dati.join("cache"));
    log.scrivi(&format!("Radar kite {VERSIONE} ({DATA_VERSIONE}) di {AUTORE} - {}", adesso.format("%d/%m/%Y %H:%M")));

    // Tutte le richieste partono insieme: previsioni Windguru, Open-Meteo e mare. (Stazioni: fase 2.)
    let mut rete = rete::Rete::new(dir_cache, o.solo_cache);
    for s in SPOT {
        rete.parti(&url_windguru(s), "https://micro.windguru.cz/", false, false, 20, false);
    }
    let url_om = url_open_meteo(SPOT);
    let url_mare = url_acqua(SPOT);
    if o.om_prova.is_none() {
        rete.parti(&url_om, "https://open-meteo.com/", false, true, 15, false);
    }
    rete.parti(&url_mare, "https://open-meteo.com/", false, true, 15, false);
    log.tempo("richieste partite (tutte insieme)");

    let mut spot_res = Vec::new();
    for s in SPOT {
        match rete.testo(&url_windguru(s), "https://micro.windguru.cz/", false) {
            Ok(html) => {
                let wg = leggi_windguru(&html, adesso);
                let g = giorni(s, &wg.righe, adesso);
                log.scrivi(&format!("  Windguru {}... ok ({}, init {} UTC)", s.nome, wg.spot, wg.init));
                spot_res.push(EsitoSpot { s, wg: Some(wg), giorni: g, errore: None });
            }
            Err(e) => {
                log.scrivi(&format!("  Windguru {}... ERRORE: {e}", s.nome));
                spot_res.push(EsitoSpot { s, wg: None, giorni: vec![], errore: Some(e) });
            }
        }
    }
    log.tempo("previsioni Windguru lette");

    let modello = match &o.modello {
        Some(p) => std::fs::read_to_string(p).map_err(|e| format!("pagina non letta: {e}"))?,
        None => MODELLO.to_string(),
    };
    let file_report = o.cartella.join("report.html");
    let mut om: HashMap<&str, Vec<RigaOm>> = HashMap::new();
    let mut acqua: HashMap<&str, Option<f64>> = HashMap::new();
    let mut onde: Option<HashMap<&str, Vec<RigaOnde>>> = None;
    let (mut errore_om, mut errore_mare) = (String::new(), String::new());
    let mut righe_firing = Vec::new();

    for &fase in &o.fasi {
        if fase == 2 {
            let testo_om = match &o.om_prova {
                Some(p) => std::fs::read_to_string(p).map_err(|e| e.to_string()),
                None => rete.testo(&url_om, "https://open-meteo.com/", false),
            };
            match testo_om.and_then(|t| leggi_open_meteo(&t, SPOT)) {
                Ok(v) => {
                    for (s, r) in SPOT.iter().zip(v) {
                        om.insert(s.id, r);
                    }
                    log.scrivi("  Open-Meteo (meteo e secondo modello)... ok");
                }
                Err(e) => {
                    log.scrivi(&format!("  Open-Meteo (meteo e secondo modello)... ERRORE: {e}"));
                    errore_om = e;
                }
            }
            match rete.testo(&url_mare, "https://open-meteo.com/", false).and_then(|t| leggi_acqua(&t, SPOT)) {
                Ok((a, w)) => {
                    let mut mappa = HashMap::new();
                    for ((s, a), w) in SPOT.iter().zip(a).zip(w) {
                        acqua.insert(s.id, a);
                        mappa.insert(s.id, w);
                    }
                    onde = Some(mappa);
                    log.scrivi("  Mare (acqua e onde)... ok");
                }
                Err(e) => {
                    log.scrivi(&format!("  Mare (acqua e onde)... ERRORE: {e}"));
                    errore_mare = e;
                }
            }
            log.tempo("Open-Meteo e mare");
            righe_firing = firing_testo(&spot_res, adesso);
            if righe_firing.is_empty() {
                log.scrivi("Nessuna finestra di vento nei prossimi giorni.");
            } else {
                log.scrivi("FIRING:");
                for f in &righe_firing {
                    log.scrivi(&format!("  * {f}"));
                }
            }
        }
        let d = dati_pagina(&spot_res, &om, &acqua, onde.as_ref(), &errore_om, &errore_mare, adesso, fase);
        let scrittura = d["scrittura"].as_i64().unwrap_or(0);
        let html = modello.replacen("/*DATI*/null", &json_per_pagina(&d), 1);
        let mut b = vec![0xEF, 0xBB, 0xBF];
        b.extend_from_slice(html.as_bytes());
        scrivi_sostituendo(&file_report, &b).map_err(|e| format!("report non scritto: {e}"))?;
        // stato.js dice alla pagina aperta che c'e' un report nuovo: la pagina lo controlla 3 volte al secondo e si ricarica.
        let _ = scrivi_sostituendo(&o.cartella.join("stato.js"), format!("window.RK_SCRITTO = {scrittura}; window.RK_PDF = '';").as_bytes());
        log.tempo(if fase == 1 { "pagina aperta (fase 1)" } else { "pagina completa (fase 2)" });
    }
    let _ = std::fs::write(dir_dati.join("ultimo-avvio.txt"), log.righe.lock().unwrap().join("\r\n") + "\r\n");
    Ok(Esito { report: file_report, firing: righe_firing })
}

fn firing_testo(spot_res: &[EsitoSpot], adesso: NaiveDateTime) -> Vec<String> {
    let mut righe: Vec<(i64, bool, String)> = Vec::new();
    for r in spot_res {
        for g in &r.giorni {
            let Some(f) = &g.firing else { continue };
            let raff = f.raffica.map(|x| format!(", raffiche {x}")).unwrap_or_default();
            let note = if f.note.is_empty() { String::new() } else { format!(" - {}", f.note.join(", ")) };
            let quando = if g.oggi { "OGGI".to_string() } else { g.nome.clone() };
            let ordine = (g.data - adesso.date()).num_days() + 1;
            let ideale = f.note.iter().any(|n| n.starts_with("ideale"));
            righe.push((ordine, ideale, format!("{} - {}, ore {}: {} {}-{} nodi{}{}", r.s.nome, quando, f.ore, f.dir, f.min, f.maxf, raff, note)));
        }
    }
    righe.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    righe.into_iter().map(|x| x.2).collect()
}

fn giorno_txt(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

#[allow(clippy::too_many_arguments)]
fn dati_pagina(spot_res: &[EsitoSpot], om: &HashMap<&str, Vec<RigaOm>>, acqua: &HashMap<&str, Option<f64>>, onde: Option<&HashMap<&str, Vec<RigaOnde>>>,
    errore_om: &str, errore_mare: &str, adesso: NaiveDateTime, fase: u8) -> Value {
    let ora_txt = adesso.format("%Y-%m-%dT%H:%M:%S").to_string();
    let giorni_d: Vec<Value> = (0..GIORNI).map(|k| {
        let d = adesso.date() + Duration::days(k);
        let f = fascia(d);
        json!({ "data": giorno_txt(d), "nome": nome_giorno(d), "oggi": k == 0, "fascia": format!("{}-{}", f.0, f.1) })
    }).collect();

    let mut dati_spot = Vec::new();
    for r in spot_res {
        let s = r.s;
        // Vento adesso: stazione aggiornata (fase 2), altrimenti previsione dell'ora.
        let adesso_v = match &r.wg {
            Some(wg) if !wg.righe.is_empty() => {
                let mut row = &wg.righe[0];
                let dist = |x: &RigaWg| (x.t - adesso).num_milliseconds().abs();
                for x in &wg.righe {
                    if dist(x) < dist(row) {
                        row = x;
                    }
                }
                json!({ "fonte": "previsione", "wind": num(row.wind), "gust": num_o(row.gust), "deg": num(row.deg), "dir": row.dir, "ora": row.t.format("%H:%M").to_string() })
            }
            _ => Value::Null,
        };
        let om_s = om.get(s.id).filter(|v| !v.is_empty());
        let onde_s = onde.and_then(|m| m.get(s.id)).filter(|v| !v.is_empty());
        // Righe di meteo e onde divise per giorno una volta sola.
        let mut om_g: HashMap<NaiveDate, Vec<&RigaOm>> = HashMap::new();
        if let Some(v) = om_s {
            for x in v {
                om_g.entry(x.date).or_default().push(x);
            }
        }
        let mut onde_g: HashMap<NaiveDate, Vec<&RigaOnde>> = HashMap::new();
        if let Some(v) = onde_s {
            for x in v {
                onde_g.entry(x.date).or_default().push(x);
            }
        }
        let vuoto_om: Vec<&RigaOm> = Vec::new();
        let mut tessere = Vec::new();
        for g in &r.giorni {
            let mut t = Map::new();
            t.insert("firing".into(), json!(g.firing.is_some()));
            t.insert("max".into(), num_o(g.max));
            t.insert("maxRaff".into(), num_o(g.max_raff));
            t.insert("maxDir".into(), json!(g.max_dir));
            if om_s.is_some() {
                let fa = fascia(g.data);
                let ini = inizio_fascia(g.data, g.oggi, adesso);
                let om_giorno = om_g.get(&g.data).unwrap_or(&vuoto_om);
                t.insert("meteo".into(), meteo_giorno(om_giorno, g.data, fa));
                let om_ore: Vec<OraVento> = om_giorno.iter().filter(|x| x.hour >= ini && x.hour <= fa.1 && x.wind.is_some())
                    .map(|x| OraVento { hour: x.hour, wind: x.wind, deg: x.deg }).collect();
                let om_max = om_ore.iter().filter_map(|x| x.wind).reduce(f64::max);
                let fin = finestra(s, &om_ore, ini, fa.1).map(|f| json!({ "ore": f.ore, "min": f.min, "max": f.max })).unwrap_or(Value::Null);
                t.insert("om".into(), json!({ "firing": fin, "max": om_max.map(|m| Value::from(round0(m) as i64)).unwrap_or(Value::Null) }));
            }
            if onde_s.is_some() {
                let fa = fascia(g.data);
                let (mut da, mut a) = fa;
                if let Some(f) = &g.firing {
                    if let Some(m) = regex::Regex::new(r"^(\d+)\D+(\d+)$").unwrap().captures(&f.ore) {
                        da = m[1].parse().unwrap_or(da);
                        a = m[2].parse().unwrap_or(a);
                    }
                }
                t.insert("onde".into(), onde_g.get(&g.data).map(|v| onde_giorno(v, g.data, da, a)).unwrap_or(Value::Null));
            }
            if let Some(f) = &g.firing {
                t.insert("ore".into(), json!(f.ore));
                t.insert("durata".into(), json!(f.durata));
                t.insert("min".into(), num(f.min));
                t.insert("maxf".into(), num(f.maxf));
                t.insert("raffica".into(), num_o(f.raffica));
                t.insert("dir".into(), json!(f.dir));
                t.insert("pericolose".into(), json!(f.pericolose));
                t.insert("rafficato".into(), json!(f.rafficato));
                t.insert("sideoff".into(), json!(f.sideoff));
                t.insert("ideale".into(), json!(f.ideale));
                t.insert("attenzione".into(), json!(f.attenzione));
            }
            tessere.push(Value::Object(t));
        }
        let staz: Vec<Value> = s.stazioni.iter().filter_map(|id| stazione(id)).map(|x| json!({ "id": x.id, "nome": x.nome, "etichetta": x.etichetta, "pagina": x.pagina })).collect();
        let onde_ora = onde_g.get(&adesso.date()).map(|v| onde_giorno(v, adesso.date(), adesso.hour() as i64, adesso.hour() as i64)).unwrap_or(Value::Null);
        dati_spot.push(json!({
            "viaggio": null, "acqua": num_o(acqua.get(s.id).copied().flatten()), "ondeOra": onde_ora, "id": s.id, "nome": s.nome, "breve": s.breve,
            "mappa": s.mappa.unwrap_or(s.breve), "lon": s.lon, "lat": s.lat, "lab": [s.lab.0, s.lab.1, s.lab.2], "wg": s.wg, "webcam": s.webcam, "surf": s.surf,
            "ok": [num(s.ok[0]), num(s.ok[1])], "terra": [num(s.terra[0]), num(s.terra[1])], "stazioni": staz, "errore": r.errore,
            "wgSpot": r.wg.as_ref().map(|w| w.spot.clone()).unwrap_or_default(), "init": r.wg.as_ref().map(|w| w.init.clone()).unwrap_or_default(),
            "adesso": adesso_v, "giorni": tessere, "aggiornato": ora_txt
        }));
    }
    // Stazioni: si leggono dalla fase 2 del piano. Per ora la pagina le mostra come non lette.
    let errore_st = if fase == 1 { "in lettura" } else { "lettura delle stazioni: arriva con la fase 2 di Radar kite 4.0" };
    let dati_stazioni: Vec<Value> = STAZIONI.iter().map(|st| {
        let suo = SPOT.iter().find(|s| s.stazioni.contains(&st.id)).map(|s| s.id);
        json!({ "id": st.id, "nome": st.nome, "pagina": st.pagina, "spot": suo, "errore": errore_st, "lenta": null, "aggiornato": ora_txt })
    }).collect();
    json!({
        "scrittura": Local::now().timestamp_millis(), "aggiornando": fase == 1, "passatiCalcolati": "", "omErrore": errore_om, "mareErrore": errore_mare,
        "versione": VERSIONE, "dataVersione": DATA_VERSIONE, "autore": AUTORE, "contatto": CONTATTO, "app": true, "copia": null, "creato": ora_txt,
        "soglia": num(SOGLIA), "oreMin": ORE_MIN, "giorni": giorni_d, "passati": [], "spot": dati_spot, "stazioni": dati_stazioni, "diario": [],
        "partenza": { "scelta": "casa", "casa": "", "lavoro": "", "casaImp": "", "lavoroImp": "", "errore": "", "trovato": "", "lat": null, "lon": null, "google": false }
    })
}
