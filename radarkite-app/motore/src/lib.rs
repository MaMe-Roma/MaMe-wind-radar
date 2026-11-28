//! Radar kite 4.0 - motore. Porta in Rust radar-kite.ps1 3.4 (la fonte di verita').
//! Fase 1 del piano: regole, download in parallelo con cache, Windguru e firing, Open-Meteo, mare, oggetto D.
//! Stazioni, impostazioni e tempi di viaggio arrivano nella fase 2; storico, diario, PDF e giro ogni ora nella fase 3.

pub mod copia;
pub mod pianifica;
pub mod previsioni;
pub mod regole;
pub mod rete;
pub mod stazioni;
pub mod storico;
pub mod viaggi;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{Duration, Local, NaiveDate, NaiveDateTime, Timelike};
use serde_json::{json, Map, Value};

use previsioni::*;
use regole::*;
use stazioni::Lettura;
use viaggi::Imp;

pub const VERSIONE: &str = "4.1.2";
pub const DATA_VERSIONE: &str = "28/11/2026";
/// Cambia quando cambia il formato di giorni passati e diario in calcoli.json (cosi' si ricalcolano).
const VERSIONE_CALCOLI: &str = "4.0-diario2";
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
pub fn num_o(x: Option<f64>) -> Value {
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
    /// Corsa automatica (niente traffico Google), come -Automatico.
    pub automatico: bool,
    /// Pulsante della pagina che ha avviato il motore (radarkite://...).
    pub link: Option<String>,
    /// Il programma da avviare per l'aggiornamento automatico (solo nell'app).
    pub programma: Option<PathBuf>,
}

impl Opzioni {
    pub fn nuove(cartella: PathBuf) -> Self {
        Opzioni { cartella, cache: None, solo_cache: false, adesso: None, om_prova: None, modello: None, fasi: vec![1, 2], automatico: false, link: None, programma: None }
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

    let dir_calcoli = dir_dati.join("cache");
    let _ = std::fs::create_dir_all(&dir_calcoli);
    let link = o.link.clone().unwrap_or_default();
    // Diario: un'uscita scritta nel modulo della pagina.
    if link.starts_with("radarkite://uscita") {
        match storico::salva_uscita(&dir_dati, &viaggi::parametri(&link)) {
            Ok(x) => log.scrivi(&format!("Uscita salvata nel diario: {x}")),
            Err(e) => log.scrivi(&format!("Uscita non salvata: {e}")),
        }
    }
    let forza_storico = link.starts_with("radarkite://storico");
    // Aggiornamento di un solo spot o di una sola stazione (pulsanti nella pagina).
    let mut solo = String::new();
    let q = viaggi::parametri(&link);
    let id = q.get("id").cloned().unwrap_or_default();
    if link.starts_with("radarkite://spot") && SPOT.iter().any(|s| s.id == id) {
        solo = format!("spot:{id}");
    }
    if link.starts_with("radarkite://stazione") && STAZIONI.iter().any(|s| s.id == id) {
        solo = format!("st:{id}");
    }
    let sx = solo.strip_prefix("spot:").and_then(|i| SPOT.iter().find(|s| s.id == i));

    // Tutte le richieste partono insieme: previsioni Windguru, Open-Meteo, mare e stazioni.
    let mut rete = rete::Rete::new(dir_cache, o.solo_cache);
    if let Some(s) = sx {
        let mut f = vec![url_windguru(s), url_open_meteo(SPOT), url_acqua(SPOT)];
        f.extend(STAZIONI.iter().filter(|st| s.stazioni.contains(&st.id)).map(|st| st.url.to_string()));
        rete.solo(f);
        log.scrivi(&format!("Aggiorno solo lo spot {}", s.nome));
    } else if let Some(i) = solo.strip_prefix("st:") {
        rete.solo(STAZIONI.iter().filter(|st| st.id == i).map(|st| st.url.to_string()).collect());
        log.scrivi(&format!("Aggiorno solo la stazione {i}"));
    }
    let fasi: Vec<u8> = if solo.is_empty() { o.fasi.clone() } else { vec![2] };
    for s in SPOT {
        rete.parti(&url_windguru(s), "https://micro.windguru.cz/", false, false, 20, false);
    }
    let url_om = url_open_meteo(SPOT);
    let url_mare = url_acqua(SPOT);
    if o.om_prova.is_none() {
        rete.parti(&url_om, "https://open-meteo.com/", false, true, 15, false);
    }
    rete.parti(&url_mare, "https://open-meteo.com/", false, true, 15, false);
    for st in STAZIONI {
        rete.parti(st.url, st.referer, st.tipo == "weathercloud", true, 60, true);
    }
    rete.segna_t0();
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

    // Impostazioni personali e pulsanti della pagina che le cambiano (partenza, indirizzi, tempi di viaggio).
    let file_imp = o.cartella.join("impostazioni.txt");
    let mut imp = viaggi::leggi_impostazioni(&file_imp);
    // Copia per il telefono: cartella scelta nella pagina.
    let cloud = copia::cartelle_cloud();
    let mut errore_copia = String::new();
    if link.starts_with("radarkite://copia") {
        match copia::scegli(q.get("c").map(|s| s.as_str()).unwrap_or(""), &cloud) {
            Ok(v) => {
                viaggi::salva_impostazione(&file_imp, "copia_telefono", &v);
                imp.insert("copia_telefono".into(), v.clone());
                log.scrivi(&format!("  Copia per il telefono: {v}"));
            }
            Err(e) => {
                log.scrivi(&format!("  Copia per il telefono: {e}"));
                errore_copia = e;
            }
        }
    }
    // Aggiornamento automatico anche ad app chiusa: risposta e orari scelti nella pagina.
    let mut errore_auto = String::new();
    if link.starts_with("radarkite://automatico") {
        let si = q.get("a").map(|s| s.as_str()) == Some("si");
        match pianifica::controlla(q.get("da").map(|s| s.as_str()).unwrap_or(""), q.get("al").map(|s| s.as_str()).unwrap_or(""), q.get("ogni").map(|s| s.as_str()).unwrap_or("")) {
            Ok((da, al, ogni)) => {
                for (k, v) in [("automatico_dalle", da), ("automatico_alle", al), ("automatico_ogni", ogni)] {
                    viaggi::salva_impostazione(&file_imp, k, &v);
                    imp.insert(k.into(), v);
                }
            }
            Err(e) if si => errore_auto = e,
            Err(_) => {}
        }
        if errore_auto.is_empty() {
            let v = if si { "si" } else { "no" };
            viaggi::salva_impostazione(&file_imp, "automatico", v);
            imp.insert("automatico".into(), v.into());
            if let Some(p) = &o.programma {
                if let Err(e) = pianifica::applica(&imp, p) {
                    errore_auto = e;
                }
            }
        }
        log.scrivi(&format!("  Aggiornamento automatico: {} {}", imp["automatico"], errore_auto));
    }
    let file_copia = dir_calcoli.join("copia-telefono.txt");
    let mut pdf_ultimo = std::fs::read_to_string(&file_copia).map(|t| t.trim().to_string()).unwrap_or_default();
    let mut forza_viaggi = false;
    if link.starts_with("radarkite://partenza") {
        let q = viaggi::parametri(&link);
        if let Some(p) = q.get("p").filter(|p| *p == "casa" || *p == "lavoro") {
            viaggi::salva_impostazione(&file_imp, "partenza", p);
            imp.insert("partenza".into(), p.clone());
        }
        forza_viaggi = true;
    }
    if link.starts_with("radarkite://viaggi") {
        forza_viaggi = true;
    }
    if link.starts_with("radarkite://indirizzi") {
        let q = viaggi::parametri(&link);
        for (k, nome) in [("c", "casa"), ("l", "lavoro")] {
            if let Some(v) = q.get(k) {
                let v = viaggi::pulisci(v, 120);
                viaggi::salva_impostazione(&file_imp, nome, &v);
                imp.insert(nome.into(), v);
            }
        }
        if let Some(p) = q.get("p").filter(|p| *p == "casa" || *p == "lavoro") {
            viaggi::salva_impostazione(&file_imp, "partenza", p);
            imp.insert("partenza".into(), p.clone());
        }
        if imp[&imp["partenza"]].is_empty() {
            let alt = if imp["partenza"] == "casa" { "lavoro" } else { "casa" };
            if !imp[alt].is_empty() {
                viaggi::salva_impostazione(&file_imp, "partenza", alt);
                imp.insert("partenza".into(), alt.into());
            }
        }
        log.scrivi(&format!("Partenza salvata: casa = {} | lavoro = {}", imp["casa"], imp["lavoro"]));
        forza_viaggi = true;
    }
    let (mut dati_viaggi, mut errore_viaggi) = (None, String::new());
    if !imp[&imp["partenza"]].is_empty() {
        match viaggi::viaggi(&imp, &dir_dati, &mut rete, forza_viaggi, o.automatico, adesso) {
            Ok(v) => {
                dati_viaggi = v;
                log.scrivi(&format!("  Tempi di viaggio da {}... ok", imp["partenza"]));
            }
            Err(e) => {
                log.scrivi(&format!("  Tempi di viaggio da {}... ERRORE: {e}", imp["partenza"]));
                errore_viaggi = e;
            }
        }
    }
    log.tempo("tempi di viaggio");

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

    for &fase in &fasi {
        let mut st_res: HashMap<&str, Result<Lettura, String>> = HashMap::new();
        if fase == 1 {
            for st in STAZIONI {
                st_res.insert(st.id, Err("in lettura".into()));
            }
        }
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
            for st in STAZIONI {
                let esito = rete.testo(st.url, st.referer, st.tipo == "weathercloud").and_then(|t| stazioni::leggi(st, &t)).map(|mut l| {
                    let eta = (adesso - l.t).num_milliseconds() as f64 / 60000.0;
                    l.fresca = !l.offline && eta <= st.fresco as f64 && eta >= -15.0;
                    l
                });
                match &esito {
                    Ok(l) => log.scrivi(&format!("  Stazione {}... ok{}", st.nome, if l.fresca { "" } else { " (vecchia)" })),
                    Err(e) => log.scrivi(&format!("  Stazione {}... ERRORE: {e}", st.nome)),
                }
                st_res.insert(st.id, esito);
            }
            log.tempo("stazioni lette");
            let wg: Vec<(&'static Spot, Option<&Vec<RigaWg>>)> = spot_res.iter().map(|r| (r.s, r.wg.as_ref().map(|w| &w.righe))).collect();
            storico::scrivi(&dir_dati, &dir_calcoli, adesso, &st_res, &wg, &om);
            log.tempo("storico");
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
        let lente = tempi_stazioni(&dir_dati, &st_res, &rete, fase, adesso);
        let (passati, passati_calcolati, diario) = giorni_passati(&dir_dati, &dir_calcoli, adesso, forza_storico, fase);
        let meta = segna_aggiornati(&dir_calcoli, &solo, sx, adesso, fase);
        let cx = Contesto { st_res: &st_res, lente: &lente, viaggi: dati_viaggi.as_ref(), imp: &imp, errore_viaggi: &errore_viaggi };
        let mut d = dati_pagina(&spot_res, &om, &acqua, onde.as_ref(), &errore_om, &errore_mare, adesso, fase, &cx);
        d["copia"] = copia::dati_pagina(&imp["copia_telefono"], &cloud, &errore_copia, &pdf_ultimo);
        d["automatico"] = json!({ "stato": imp["automatico"], "dalle": imp["automatico_dalle"], "alle": imp["automatico_alle"],
            "ogni": imp["automatico_ogni"].parse::<i64>().unwrap_or(60), "prossimo": pianifica::prossimo(&imp, adesso), "errore": errore_auto });
        d["passati"] = passati;
        d["passatiCalcolati"] = json!(passati_calcolati);
        d["diario"] = diario;
        d["creato"] = meta["creato"].clone();
        for x in d["spot"].as_array_mut().into_iter().flatten() {
            x["aggiornato"] = meta["spot"][x["id"].as_str().unwrap_or("")].clone();
        }
        for x in d["stazioni"].as_array_mut().into_iter().flatten() {
            x["aggiornato"] = meta["stazioni"][x["id"].as_str().unwrap_or("")].clone();
        }
        let scrittura = d["scrittura"].as_i64().unwrap_or(0);
        let html = modello.replacen("/*DATI*/null", &json_per_pagina(&d), 1);
        let mut b = vec![0xEF, 0xBB, 0xBF];
        b.extend_from_slice(html.as_bytes());
        scrivi_sostituendo(&file_report, &b).map_err(|e| format!("report non scritto: {e}"))?;
        // stato.js dice alla pagina aperta che c'e' un report nuovo: la pagina lo controlla 3 volte al secondo e si ricarica.
        let stato = |pdf: &str| { let _ = scrivi_sostituendo(&o.cartella.join("stato.js"), format!("window.RK_SCRITTO = {scrittura}; window.RK_PDF = '{}';", pdf.replace(['\\', '\''], "")).as_bytes()); };
        stato(&pdf_ultimo);
        log.tempo(if fase == 1 { "pagina aperta (fase 1)" } else { "pagina completa (fase 2)" });
        // A pagina completa il PDF per il telefono (non per l'aggiornamento di un solo spot).
        if fase == 2 && solo.is_empty() && !o.solo_cache {
            if let Some(esito) = copia::salva_pdf(&file_report, copia::cartella_copia(&imp["copia_telefono"], &cloud), adesso) {
                log.scrivi(&format!("  PDF per il telefono: {esito}"));
                pdf_ultimo = esito;
                let _ = std::fs::write(&file_copia, &pdf_ultimo);
                stato(&pdf_ultimo);
                log.tempo("PDF");
            }
        }
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

struct Contesto<'a> {
    st_res: &'a HashMap<&'a str, Result<Lettura, String>>,
    lente: &'a HashMap<&'static str, Value>,
    viaggi: Option<&'a Value>,
    imp: &'a Imp,
    errore_viaggi: &'a str,
}

/// Appunti sulle stazioni (dati/tempi-stazioni.csv): quanto ci mettono a rispondere e se rispondono.
fn tempi_stazioni(dir_dati: &Path, st_res: &HashMap<&str, Result<Lettura, String>>, rete: &rete::Rete, fase: u8, adesso: NaiveDateTime) -> HashMap<&'static str, Value> {
    let file = dir_dati.join("tempi-stazioni.csv");
    let mut righe: Vec<String> = std::fs::read_to_string(&file).map(|t| t.lines().skip(1).filter(|r| !r.is_empty()).map(String::from).collect()).unwrap_or_default();
    if fase == 2 {
        for st in STAZIONI {
            let Some(l) = st_res.get(st.id) else { continue };
            if matches!(l, Err(e) if e.to_lowercase().contains("sospes")) {
                continue;
            }
            let sec = rete.secondi(st.url).map(|x| format!("{x:.1}")).unwrap_or_default();
            righe.push(format!("{};{};{};{}", adesso.format("%Y-%m-%d %H:%M"), st.id, sec, if l.is_ok() { "ok" } else { "errore" }));
        }
        if righe.len() > 3000 {
            righe.drain(..righe.len() - 3000);
        }
        let _ = std::fs::write(&file, format!("data;stazione;secondi;esito\r\n{}\r\n", righe.join("\r\n")));
    }
    let mut lente = HashMap::new();
    for st in STAZIONI {
        let sue: Vec<Vec<&str>> = righe.iter().map(|r| r.split(';').collect::<Vec<_>>()).filter(|p| p.len() >= 2 && p[1] == st.id).collect();
        if sue.len() < 3 {
            continue;
        }
        let sue = &sue[sue.len().saturating_sub(30)..];
        let ok: Vec<f64> = sue.iter().filter(|p| p.len() > 3 && p[3] == "ok" && !p[2].is_empty()).filter_map(|p| p[2].parse().ok()).collect();
        let media = if ok.is_empty() { Value::Null } else { num(round_n(ok.iter().sum::<f64>() / ok.len() as f64, 1)) };
        lente.insert(st.id, json!({ "media": media, "fallite": sue.len() - ok.len(), "prove": sue.len() }));
    }
    lente
}

/// Giorni passati e diario: non cambiano durante la giornata, si calcolano una volta al giorno (dati/cache/calcoli.json).
/// Si rifanno il giorno dopo, quando cambia il diario o col pulsante "ricalcola".
fn giorni_passati(dir_dati: &Path, dir_calcoli: &Path, adesso: NaiveDateTime, forza: bool, fase: u8) -> (Value, String, Value) {
    let file_calc = dir_calcoli.join("calcoli.json");
    let oggi = adesso.format("%Y-%m-%d").to_string();
    let firma = std::fs::metadata(dir_dati.join("diario.csv")).and_then(|m| m.modified()).map(|t| format!("{:?}", t)).unwrap_or_default();
    let calc: Value = if forza { Value::Null } else { std::fs::read_to_string(&file_calc).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null) };
    let usa_passati = calc["giorno"] == json!(oggi) && calc["versione"] == json!(VERSIONE_CALCOLI);
    let usa_diario = usa_passati && calc["firmaDiario"] == json!(firma);
    let (passati, calcolati) = if usa_passati { (calc["passati"].clone(), calc["calcolati"].as_str().unwrap_or("").to_string()) }
        else { (Value::Array(storico::passati(dir_dati, adesso)), adesso.format("%Y-%m-%dT%H:%M:%S").to_string()) };
    let diario = if usa_diario { calc["diario"].clone() } else { Value::Array(storico::diario(dir_dati)) };
    if (!usa_passati || !usa_diario) && fase == 2 {
        let _ = std::fs::write(&file_calc, serde_json::to_string(&json!({ "giorno": oggi, "versione": VERSIONE_CALCOLI, "firmaDiario": firma, "calcolati": calcolati, "passati": passati, "diario": diario })).unwrap_or_default());
    }
    (passati, calcolati, diario)
}

/// Ora di aggiornamento di pagina, spot e stazioni (dati/cache/meta.json): con un solo spot cambiano solo i suoi.
fn segna_aggiornati(dir_calcoli: &Path, solo: &str, sx: Option<&Spot>, adesso: NaiveDateTime, fase: u8) -> Value {
    let file = dir_calcoli.join("meta.json");
    let ora = adesso.format("%Y-%m-%dT%H:%M:%S").to_string();
    let mut m: Value = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({}));
    for k in ["spot", "stazioni"] {
        if !m[k].is_object() {
            m[k] = json!({});
        }
    }
    if solo.is_empty() || m["creato"].as_str().unwrap_or("").is_empty() {
        m["creato"] = json!(ora);
        for s in SPOT {
            m["spot"][s.id] = json!(ora);
        }
        for st in STAZIONI {
            m["stazioni"][st.id] = json!(ora);
        }
    } else if let Some(s) = sx {
        m["spot"][s.id] = json!(ora);
        for sid in s.stazioni {
            m["stazioni"][*sid] = json!(ora);
        }
    } else if let Some(i) = solo.strip_prefix("st:") {
        m["stazioni"][i] = json!(ora);
    }
    if fase == 2 {
        let _ = std::fs::write(&file, serde_json::to_string(&m).unwrap_or_default());
    }
    m
}

fn giorno_txt(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

#[allow(clippy::too_many_arguments)]
fn dati_pagina(spot_res: &[EsitoSpot], om: &HashMap<&str, Vec<RigaOm>>, acqua: &HashMap<&str, Option<f64>>, onde: Option<&HashMap<&str, Vec<RigaOnde>>>,
    errore_om: &str, errore_mare: &str, adesso: NaiveDateTime, fase: u8, cx: &Contesto) -> Value {
    let ora_txt = adesso.format("%Y-%m-%dT%H:%M:%S").to_string();
    let giorni_d: Vec<Value> = (0..GIORNI).map(|k| {
        let d = adesso.date() + Duration::days(k);
        let f = fascia(d);
        json!({ "data": giorno_txt(d), "nome": nome_giorno(d), "oggi": k == 0, "fascia": format!("{}-{}", f.0, f.1) })
    }).collect();

    let mut dati_spot = Vec::new();
    for r in spot_res {
        let s = r.s;
        // Vento adesso: stazione aggiornata, altrimenti previsione dell'ora, altrimenti stazione vecchia.
        let letture: Vec<(&str, &Lettura)> = s.stazioni.iter().filter_map(|sid| match cx.st_res.get(sid) { Some(Ok(l)) => Some((*sid, l)), _ => None }).collect();
        let l = match letture.iter().find(|x| x.1.fresca) {
            Some(x) => Some(*x),
            None => {
                let mut v: Vec<(&str, &Lettura)> = letture.iter().copied().filter(|x| !x.1.offline).collect();
                ordina_ps(&mut v, |a, b| b.1.t.cmp(&a.1.t));
                v.first().copied()
            }
        };
        let da_stazione = |fonte: &str, (sid, l): (&str, &Lettura)| json!({ "fonte": fonte, "wind": num(round_n(l.wind, 1)), "gust": num_o(l.gust.map(|g| round_n(g, 1))),
            "deg": num(l.deg), "dir": comp(l.deg), "ora": l.t.format("%H:%M").to_string(), "nome": stazione(sid).map(|x| x.etichetta).unwrap_or("") });
        let adesso_v = match &r.wg {
            _ if l.is_some_and(|x| x.1.fresca) => da_stazione("stazione", l.unwrap()),
            Some(wg) if !wg.righe.is_empty() => {
                // Sort-Object { |t - adesso| } | Select-Object -First 1, con l'ordinamento della 3.4.
                let mut ord: Vec<&RigaWg> = wg.righe.iter().collect();
                ordina_ps(&mut ord, |a, b| (a.t - adesso).num_milliseconds().abs().cmp(&(b.t - adesso).num_milliseconds().abs()));
                let row = ord[0];
                json!({ "fonte": "previsione", "wind": num(row.wind), "gust": num_o(row.gust), "deg": num(row.deg), "dir": row.dir, "ora": row.t.format("%H:%M").to_string() })
            }
            _ => l.map(|x| da_stazione("vecchia", x)).unwrap_or(Value::Null),
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
            "viaggio": cx.viaggi.map(|v| json!({ "stima": v["stima"].get(s.id).cloned().unwrap_or(Value::Null), "traffico": v["traffico"].get(s.id).cloned().unwrap_or(Value::Null), "ora": v["trafficoOra"].clone() })).unwrap_or(Value::Null),
            "acqua": num_o(acqua.get(s.id).copied().flatten()), "ondeOra": onde_ora, "id": s.id, "nome": s.nome, "breve": s.breve,
            "mappa": s.mappa.unwrap_or(s.breve), "lon": s.lon, "lat": s.lat, "lab": [s.lab.0, s.lab.1, s.lab.2], "wg": s.wg, "webcam": s.webcam, "surf": s.surf,
            "ok": [num(s.ok[0]), num(s.ok[1])], "terra": [num(s.terra[0]), num(s.terra[1])], "stazioni": staz, "errore": r.errore,
            "wgSpot": r.wg.as_ref().map(|w| w.spot.clone()).unwrap_or_default(), "init": r.wg.as_ref().map(|w| w.init.clone()).unwrap_or_default(),
            "adesso": adesso_v, "giorni": tessere, "aggiornato": ora_txt
        }));
    }
    let dati_stazioni: Vec<Value> = STAZIONI.iter().map(|st| {
        let suo = SPOT.iter().find(|s| s.stazioni.contains(&st.id)).map(|s| s.id);
        let lenta = cx.lente.get(st.id).cloned().unwrap_or(Value::Null);
        match cx.st_res.get(st.id) {
            Some(Ok(l)) => {
                let stato = if l.offline { "offline" } else if l.fresca { "aggiornata" } else { "vecchia" };
                let giorni = ((adesso - l.t).num_milliseconds() as f64 / 86_400_000.0).floor() as i64;
                json!({ "id": st.id, "nome": st.nome, "pagina": st.pagina, "spot": suo, "data": l.t.format("%d/%m").to_string(), "giorni": giorni,
                    "wind": num(round_n(l.wind, 1)), "gust": num_o(l.gust.map(|g| round_n(g, 1))), "deg": num(l.deg), "dir": comp(l.deg), "ora": l.t.format("%H:%M").to_string(),
                    "stato": stato, "lenta": lenta, "aggiornato": ora_txt })
            }
            Some(Err(e)) => json!({ "id": st.id, "nome": st.nome, "pagina": st.pagina, "spot": suo, "errore": e, "lenta": lenta, "aggiornato": ora_txt }),
            None => json!({ "id": st.id, "nome": st.nome, "pagina": st.pagina, "spot": suo, "errore": "non letta", "lenta": lenta, "aggiornato": ora_txt }),
        }
    }).collect();
    let imp = cx.imp;
    let v = cx.viaggi;
    json!({
        "scrittura": Local::now().timestamp_millis(), "aggiornando": fase == 1, "passatiCalcolati": "", "omErrore": errore_om, "mareErrore": errore_mare,
        "versione": VERSIONE, "dataVersione": DATA_VERSIONE, "autore": AUTORE, "contatto": CONTATTO, "app": true, "copia": null, "creato": ora_txt,
        "soglia": num(SOGLIA), "oreMin": ORE_MIN, "giorni": giorni_d, "passati": [], "spot": dati_spot, "stazioni": dati_stazioni, "diario": [],
        "partenza": { "scelta": imp["partenza"], "casa": viaggi::nome_posto(&imp["casa"]), "lavoro": viaggi::nome_posto(&imp["lavoro"]), "casaImp": imp["casa"], "lavoroImp": imp["lavoro"],
            "errore": cx.errore_viaggi, "trovato": v.map(|v| v["trovato"].clone()).unwrap_or(json!("")), "lat": v.map(|v| v["lat"].clone()).unwrap_or(Value::Null),
            "lon": v.map(|v| v["lon"].clone()).unwrap_or(Value::Null), "google": !imp["google_chiave"].is_empty() }
    })
}
