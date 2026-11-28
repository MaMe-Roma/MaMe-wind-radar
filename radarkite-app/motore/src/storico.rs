//! Storico, giorni passati e diario: porting del blocco "Storico" e di Get-OreRegistrate (radar-kite.ps1 3.4),
//! con le aggiunte della 4.0 al diario (muta e direzione) e l'import dei dati della 3.4.
//! letture-AAAA-MM.csv e previsioni-AAAA-MM.csv come nella 3.4 (si aprono con Excel); oltre i 2 anni si tolgono.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Timelike};
use serde_json::{json, Value};

use crate::previsioni::{classe, comp, fascia, finestra, ordina_ps, round_n, OraVento, RigaOm, RigaWg};
use crate::regole::{Spot, SOGLIA, SPOT, STAZIONI};
use crate::stazioni::Lettura;
use crate::{num, num_o};

pub const INT_LETTURE: &str = "data;stazione;spot;vento;raffica;gradi";
pub const INT_PREV: &str = "giorno;spot;anticipo;fonte;ora;vento;raffica;gradi;salvata";
pub const INT_DIARIO: &str = "data;ora;spot;minuti;vela;note;muta;direzione;rilevato";

pub fn file_mese(dir: &Path, tipo: &str, d: NaiveDate) -> PathBuf {
    dir.join(format!("{tipo}-{}.csv", d.format("%Y-%m")))
}

/// Read-Righe: le righe del file senza l'intestazione e senza righe vuote.
pub fn righe(file: &Path) -> Vec<String> {
    let Ok(b) = std::fs::read(file) else { return vec![] };
    let b = b.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(&b);
    String::from_utf8_lossy(b).lines().skip(1).filter(|r| !r.is_empty()).map(String::from).collect()
}

pub fn aggiungi(file: &Path, intestazione: &str, nuove: &[String]) {
    if nuove.is_empty() {
        return;
    }
    use std::io::Write;
    let nuovo = !file.exists();
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(file) {
        if nuovo {
            let _ = write!(f, "{intestazione}\r\n");
        }
        let _ = write!(f, "{}\r\n", nuove.join("\r\n"));
    }
}

/// ToString(InvariantCulture) di un double: come lo scrive .NET per questi valori (12.4, 15, 247.5).
fn txt(x: f64) -> String {
    format!("{x}")
}
/// ToString('0.#') e ToString('0'): arrotondamento "lontano dallo zero", come i formati di .NET.
fn txt_1(x: f64) -> String {
    let r = (x * 10.0).round() / 10.0;
    if r.fract() == 0.0 { format!("{}", r as i64) } else { format!("{r:.1}") }
}

/// Letture fresche delle stazioni e previsione del giorno (oggi e domani, una volta sola) nello storico.
pub fn scrivi(dir_dati: &Path, dir_calcoli: &Path, adesso: NaiveDateTime, st_res: &HashMap<&str, Result<Lettura, String>>,
    wg: &[(&'static Spot, Option<&Vec<RigaWg>>)], om: &HashMap<&str, Vec<RigaOm>>) {
    let file_l = file_mese(dir_dati, "letture", adesso.date());
    let tutte = righe(&file_l);
    let mut visti: HashSet<String> = tutte[tutte.len().saturating_sub(400)..].iter().cloned().collect();
    let mut nuove = Vec::new();
    for st in STAZIONI {
        let Some(Ok(l)) = st_res.get(st.id) else { continue };
        if !l.fresca {
            continue;
        }
        let suo = SPOT.iter().find(|s| s.stazioni.contains(&st.id)).map(|s| s.id).unwrap_or("");
        let g = l.gust.map(txt).unwrap_or_default();
        let riga = format!("{};{};{};{};{};{}", l.t.format("%Y-%m-%d %H:%M"), st.id, suo, txt(l.wind), g, txt(l.deg));
        if visti.insert(riga.clone()) {
            nuove.push(riga);
        }
    }
    aggiungi(&file_l, INT_LETTURE, &nuove);

    let file_salv = dir_calcoli.join("previsioni-salvate.txt");
    let mut presenti: HashSet<String> = HashSet::new();
    if file_salv.exists() {
        presenti.extend(std::fs::read_to_string(&file_salv).unwrap_or_default().lines().filter(|r| !r.is_empty()).map(String::from));
    } else {
        for k in 0..=1 {
            for r in righe(&file_mese(dir_dati, "previsioni", adesso.date() + Duration::days(k))) {
                let p: Vec<&str> = r.split(';').collect();
                if p.len() >= 4 {
                    presenti.insert(format!("{};{};{};{}", p[0], p[1], p[2], p[3]));
                }
            }
        }
    }
    for k in 0..=1i64 {
        let giorno = adesso.date() + Duration::days(k);
        let chiave = giorno.format("%Y-%m-%d").to_string();
        let mut nuove = Vec::new();
        for (s, righe_wg) in wg {
            let wg_v: Vec<(i64, f64, Option<f64>, f64)> = righe_wg.map(|v| v.iter().filter(|x| x.date == giorno).map(|x| (x.hour, x.wind, x.gust, x.deg)).collect()).unwrap_or_default();
            let om_v: Vec<(i64, f64, Option<f64>, f64)> = om.get(s.id).map(|v| v.iter().filter(|x| x.date == giorno && x.wind.is_some()).map(|x| (x.hour, x.wind.unwrap(), x.gust, x.deg.unwrap_or(0.0))).collect()).unwrap_or_default();
            for (f, v) in [("wg", wg_v), ("om", om_v)] {
                let kk = format!("{chiave};{};{k};{f}", s.id);
                if presenti.contains(&kk) {
                    continue;
                }
                let prima = nuove.len();
                for (h, w, g, d) in v.into_iter().filter(|x| x.0 >= 6 && x.0 <= 20) {
                    nuove.push(format!("{chiave};{};{k};{f};{h};{};{};{};{}", s.id, txt_1(w), g.map(txt).unwrap_or_default(), d.round() as i64, adesso.format("%H:%M")));
                }
                if nuove.len() > prima {
                    presenti.insert(kk);
                }
            }
        }
        aggiungi(&file_mese(dir_dati, "previsioni", giorno), INT_PREV, &nuove);
    }
    let ieri = (adesso.date() - Duration::days(1)).format("%Y-%m-%d").to_string();
    let mut tieni: Vec<&String> = presenti.iter().filter(|r| r.len() >= 10 && r[..10] > *ieri).collect();
    tieni.sort();
    let _ = std::fs::write(&file_salv, tieni.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\r\n") + "\r\n");
    // Oltre i 2 anni si toglie.
    let limite = {
        let (mut y, mut m) = (adesso.year(), adesso.month() as i32 - 24);
        while m < 1 {
            m += 12;
            y -= 1;
        }
        format!("{y:04}-{m:02}")
    };
    let re = regex::Regex::new(r"^(letture|previsioni)-(\d{4}-\d{2})\.csv$").unwrap();
    for e in std::fs::read_dir(dir_dati).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if let Some(c) = re.captures(&n) {
            if c[2] < *limite {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

#[derive(Clone)]
struct LetturaS {
    data: String,
    ora: String,
    h: i64,
    stazione: String,
    spot: String,
    wind: f64,
    gust: Option<f64>,
    deg: f64,
}
struct PrevS {
    anticipo: i64,
    fonte: String,
    hour: i64,
    wind: f64,
    gust: Option<f64>,
    deg: f64,
    salvata: String,
}
fn f64p(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}
fn f64o(s: &str) -> Option<f64> {
    if s.is_empty() { None } else { s.parse().ok() }
}

#[derive(Clone)]
struct Ora {
    hour: i64,
    wind: f64,
    gust: Option<f64>,
    deg: f64,
    n: usize,
}

fn ore_registrate(s: &Spot, lett: &[&LetturaS]) -> Vec<Ora> {
    let mut ordine: Vec<i64> = Vec::new();
    for x in lett {
        if !ordine.contains(&x.h) {
            ordine.push(x.h);
        }
    }
    let mut out = Vec::new();
    for h in ordine {
        let gr: Vec<&LetturaS> = lett.iter().copied().filter(|x| x.h == h).collect();
        let mut sel: Vec<&LetturaS> = Vec::new();
        for sid in s.stazioni {
            let x: Vec<&LetturaS> = gr.iter().copied().filter(|x| x.stazione == *sid).collect();
            if !x.is_empty() {
                sel = x;
                break;
            }
        }
        if sel.is_empty() {
            sel = gr.clone();
        }
        let gust = sel.iter().filter_map(|x| x.gust).reduce(f64::max);
        let mut ord = sel.clone();
        ordina_ps(&mut ord, |a, b| b.wind.total_cmp(&a.wind));
        out.push(Ora { hour: h, wind: round_n(sel.iter().map(|x| x.wind).sum::<f64>() / sel.len() as f64, 1), gust, deg: ord[0].deg, n: sel.len() });
    }
    ordina_ps(&mut out, |a, b| a.hour.cmp(&b.hour));
    out
}

fn fin(s: &Spot, v: Vec<OraVento>, f: (i64, i64)) -> Value {
    finestra(s, &v, f.0, f.1).map(|x| json!({ "ore": x.ore, "min": x.min, "max": x.max })).unwrap_or(Value::Null)
}

/// Giorni passati della settimana: letture orarie, previsione salvata quel giorno, verdetto.
pub fn passati(dir_dati: &Path, adesso: NaiveDateTime) -> Vec<Value> {
    let oggi = adesso.date();
    let lunedi = oggi - Duration::days(oggi.weekday().num_days_from_monday() as i64);
    let lun = lunedi.format("%Y-%m-%d").to_string();
    let mut mesi = vec![lunedi.format("%Y-%m").to_string()];
    if oggi.format("%Y-%m").to_string() != mesi[0] {
        mesi.push(oggi.format("%Y-%m").to_string());
    }
    let mut idx_l: HashMap<String, Vec<LetturaS>> = HashMap::new();
    let mut idx_p: HashMap<String, Vec<PrevS>> = HashMap::new();
    for m in &mesi {
        for r in righe(&dir_dati.join(format!("letture-{m}.csv"))) {
            if r.len() < 16 || r[..10] < *lun {
                continue;
            }
            let p: Vec<&str> = r.split(';').collect();
            if p.len() < 6 {
                continue;
            }
            let x = LetturaS { data: p[0][..10].into(), ora: p[0][11..16].into(), h: p[0][11..13].parse().unwrap_or(0), stazione: p[1].into(), spot: p[2].into(),
                wind: f64p(p[3]), gust: f64o(p[4]), deg: f64p(p[5]) };
            idx_l.entry(format!("{}|{}", x.data, x.spot)).or_default().push(x);
        }
        for r in righe(&dir_dati.join(format!("previsioni-{m}.csv"))) {
            if r.len() < 10 || r[..10] < *lun {
                continue;
            }
            let p: Vec<&str> = r.split(';').collect();
            if p.len() < 9 {
                continue;
            }
            idx_p.entry(format!("{}|{}", p[0], p[1])).or_default().push(PrevS { anticipo: p[2].parse().unwrap_or(0), fonte: p[3].into(), hour: p[4].parse().unwrap_or(0),
                wind: f64p(p[5]), gust: f64o(p[6]), deg: f64p(p[7]), salvata: p[8].into() });
        }
    }
    let mut out = Vec::new();
    let mut giorno = lunedi;
    while giorno < oggi {
        let chiave = giorno.format("%Y-%m-%d").to_string();
        let f = fascia(giorno);
        let (mut per_spot, mut dettaglio) = (Vec::new(), Vec::new());
        for s in SPOT {
            let vuoto = Vec::new();
            let lett: Vec<&LetturaS> = idx_l.get(&format!("{chiave}|{}", s.id)).unwrap_or(&vuoto).iter().filter(|x| x.h >= f.0 && x.h <= f.1).collect();
            if !lett.is_empty() {
                let buone: Vec<&LetturaS> = lett.iter().copied().filter(|x| classe(s, x.deg) != "terra").collect();
                let mut ord = if buone.is_empty() { lett.clone() } else { buone.clone() };
                ordina_ps(&mut ord, |a, b| b.wind.total_cmp(&a.wind));
                let top = ord[0];
                per_spot.push(json!({ "id": s.id, "breve": s.breve, "letture": lett.len(), "max": num(top.wind), "gust": num_o(top.gust), "dir": comp(top.deg), "ora": top.ora,
                    "buono": top.wind >= SOGLIA && classe(s, top.deg) != "terra", "oltre": buone.iter().filter(|x| x.wind >= SOGLIA).count() }));
            }
            let vuoto_p = Vec::new();
            let pv = idx_p.get(&format!("{chiave}|{}", s.id)).unwrap_or(&vuoto_p);
            let ant = if pv.iter().any(|x| x.anticipo == 0 && x.fonte == "wg") { 0 } else { 1 };
            let scegli = |fonte: &str| {
                let mut v: Vec<&PrevS> = pv.iter().filter(|x| x.anticipo == ant && x.fonte == fonte && x.hour >= f.0 && x.hour <= f.1).collect();
                ordina_ps(&mut v, |a, b| a.hour.cmp(&b.hour));
                v
            };
            let (wg_prev, om_prev) = (scegli("wg"), scegli("om"));
            let reg = ore_registrate(s, &lett);
            if wg_prev.is_empty() && reg.is_empty() {
                continue;
            }
            let diff: Vec<f64> = reg.iter().filter_map(|x| wg_prev.iter().find(|y| y.hour == x.hour).map(|y| y.wind - x.wind)).collect();
            let ov = |v: &[&PrevS]| v.iter().map(|x| OraVento { hour: x.hour, wind: Some(x.wind), deg: Some(x.deg) }).collect::<Vec<_>>();
            dettaglio.push(json!({ "id": s.id, "nome": s.nome, "salvata": wg_prev.first().map(|x| x.salvata.clone()).unwrap_or_default(), "anticipo": ant,
                "prev": wg_prev.iter().map(|x| json!({ "h": x.hour, "w": num(x.wind), "g": num_o(x.gust), "d": comp(x.deg) })).collect::<Vec<_>>(),
                "om": om_prev.iter().map(|x| json!({ "h": x.hour, "w": num(x.wind) })).collect::<Vec<_>>(),
                "reg": reg.iter().map(|x| json!({ "h": x.hour, "w": num(x.wind), "g": num_o(x.gust), "d": comp(x.deg), "n": x.n })).collect::<Vec<_>>(),
                "prevF": fin(s, ov(&wg_prev), f), "omF": fin(s, ov(&om_prev), f),
                "regF": fin(s, reg.iter().map(|x| OraVento { hour: x.hour, wind: Some(x.wind), deg: Some(x.deg) }).collect(), f),
                "scarto": if diff.is_empty() { Value::Null } else { num(round_n(diff.iter().sum::<f64>() / diff.len() as f64, 1)) }, "oreConfronto": diff.len() }));
        }
        out.push(json!({ "data": chiave, "spot": per_spot, "dettaglio": dettaglio }));
        giorno += Duration::days(1);
    }
    out
}

/// Diario delle uscite (ultime 30): vento letto dalle stazioni durante l'uscita (con la fonte)
/// e vento previsto da Windguru per quelle ore (la previsione salvata quel giorno).
pub fn diario(dir_dati: &Path) -> Vec<Value> {
    let tutte_d = righe(&dir_dati.join("diario.csv"));
    let primo = tutte_d.len().saturating_sub(30);
    let righe_d = &tutte_d[primo..];
    let mut mesi: Vec<String> = Vec::new();
    for r in righe_d {
        if r.len() >= 7 && !mesi.contains(&r[..7].to_string()) {
            mesi.push(r[..7].to_string());
        }
    }
    let mut lett: Vec<(NaiveDateTime, String, f64, Option<f64>, f64, String)> = Vec::new();
    let mut prev: HashMap<String, Vec<PrevS>> = HashMap::new();
    for m in &mesi {
        for r in righe(&dir_dati.join(format!("letture-{m}.csv"))) {
            let p: Vec<&str> = r.split(';').collect();
            if p.len() < 6 {
                continue;
            }
            if let Ok(t) = NaiveDateTime::parse_from_str(p[0], "%Y-%m-%d %H:%M") {
                lett.push((t, p[2].into(), f64p(p[3]), f64o(p[4]), f64p(p[5]), p[1].into()));
            }
        }
        for r in righe(&dir_dati.join(format!("previsioni-{m}.csv"))) {
            let p: Vec<&str> = r.split(';').collect();
            if p.len() >= 9 && p[3] == "wg" {
                prev.entry(format!("{}|{}", p[0], p[1])).or_default().push(PrevS { anticipo: p[2].parse().unwrap_or(0), fonte: p[3].into(), hour: p[4].parse().unwrap_or(0),
                    wind: f64p(p[5]), gust: f64o(p[6]), deg: f64p(p[7]), salvata: p[8].into() });
            }
        }
    }
    let mut out = Vec::new();
    for (k, r) in righe_d.iter().enumerate() {
        let p: Vec<&str> = r.split(';').collect();
        let campo = |i: usize| p.get(i).copied().unwrap_or("");
        let (mut vento, mut windguru) = (Value::Null, Value::Null);
        let inizio = NaiveDateTime::parse_from_str(&format!("{} {}", campo(0), campo(1)), "%Y-%m-%d %H:%M").ok();
        if let (Some(da), Ok(min)) = (inizio, campo(3).parse::<i64>()) {
            let a = da + Duration::minutes(min);
            let x: Vec<&(NaiveDateTime, String, f64, Option<f64>, f64, String)> = lett.iter().filter(|x| x.1 == campo(2) && x.0 >= da - Duration::minutes(20) && x.0 <= a + Duration::minutes(20)).collect();
            if !x.is_empty() {
                let mut ord = x.clone();
                ordina_ps(&mut ord, |a, b| b.2.total_cmp(&a.2));
                let mut nomi: Vec<&str> = Vec::new();
                for y in &x {
                    let n = crate::regole::stazione(&y.5).map(|s| s.nome).unwrap_or(&y.5);
                    if !nomi.contains(&n) {
                        nomi.push(n);
                    }
                }
                vento = json!({ "w": num(round_n(x.iter().map(|y| y.2).sum::<f64>() / x.len() as f64, 1)), "g": num_o(x.iter().filter_map(|y| y.3).reduce(f64::max)), "d": comp(ord[0].4), "n": x.len(),
                    "stazioni": nomi, "ore": x.iter().map(|y| y.0.format("%H:%M").to_string()).collect::<Vec<_>>() });
            }
        }
        // Windguru: la previsione salvata quel giorno (se manca, quella del giorno prima) per le ore dell'uscita.
        if let Some(da) = inizio {
            let vuoto = Vec::new();
            let pv = prev.get(&format!("{}|{}", campo(0), campo(2))).unwrap_or(&vuoto);
            let ant = if pv.iter().any(|x| x.anticipo == 0) { 0 } else { 1 };
            let pv: Vec<&PrevS> = pv.iter().filter(|x| x.anticipo == ant).collect();
            let (h0, h1) = (da.hour() as i64, (da + Duration::minutes(campo(3).parse::<i64>().unwrap_or(0))).hour() as i64);
            let mut sel: Vec<&PrevS> = pv.iter().copied().filter(|x| x.hour >= h0 && x.hour <= h1.max(h0)).collect();
            if sel.is_empty() {
                if let Some(v) = pv.iter().copied().min_by_key(|x| (x.hour - h0).abs()) {
                    sel.push(v);
                }
            }
            if !sel.is_empty() {
                let mut ord = sel.clone();
                ordina_ps(&mut ord, |a, b| b.wind.total_cmp(&a.wind));
                let (a, b) = (sel.iter().map(|x| x.hour).min().unwrap(), sel.iter().map(|x| x.hour).max().unwrap());
                windguru = json!({ "w": num(round_n(sel.iter().map(|x| x.wind).sum::<f64>() / sel.len() as f64, 1)), "g": num_o(sel.iter().filter_map(|x| x.gust).reduce(f64::max)),
                    "d": comp(ord[0].deg), "ore": if a == b { format!("{a}") } else { format!("{a}-{b}") }, "salvata": sel[0].salvata, "anticipo": ant });
            }
        }
        out.push(json!({ "riga": primo + k, "data": campo(0), "ora": campo(1), "spot": campo(2), "minuti": campo(3).parse::<i64>().ok(), "vela": campo(4), "note": campo(5),
            "muta": campo(6), "direzione": campo(7), "rilevato": campo(8), "vento": vento, "windguru": windguru }));
    }
    out
}

fn riscrivi_diario(file: &Path, righe_d: &[String]) {
    let mut t = format!("{INT_DIARIO}\r\n");
    for r in righe_d {
        t.push_str(r);
        t.push_str("\r\n");
    }
    let _ = std::fs::write(file, t);
}

/// Diario: un'uscita scritta, modificata (i = riga) o tolta (canc = riga) dalla pagina.
/// radarkite://uscita?d=&o=&s=&m=&v=&n=&u=&r=&w=[&i=&od=]  oppure  radarkite://uscita?canc=&od=
pub fn salva_uscita(dir_dati: &Path, q: &HashMap<String, String>) -> Result<String, String> {
    let c = |k: &str, max: usize| crate::viaggi::pulisci(q.get(k).map(|s| s.as_str()).unwrap_or(""), max);
    let file = dir_dati.join("diario.csv");
    let mut tutte = righe(&file);
    // La riga da modificare o togliere deve essere ancora quella mostrata nella pagina (data;spot).
    let trova = |k: &str, tutte: &[String]| -> Result<Option<usize>, String> {
        let Some(i) = q.get(k).and_then(|x| x.parse::<usize>().ok()) else { return Ok(None) };
        let od = q.get("od").cloned().unwrap_or_default();
        let ok = tutte.get(i).is_some_and(|r| { let p: Vec<&str> = r.split(';').collect(); p.len() >= 3 && format!("{};{}", p[0], p[2]) == od });
        if ok { Ok(Some(i)) } else { Err("il diario e' cambiato: ricarica la pagina e riprova".into()) }
    };
    if let Some(i) = trova("canc", &tutte)? {
        let tolta = tutte.remove(i);
        riscrivi_diario(&file, &tutte);
        return Ok(format!("tolta {}", tolta.split(';').take(3).collect::<Vec<_>>().join(" ")));
    }
    let (d, mut o, sp, mut mi, v, no, mu, mut di) = (c("d", 10), c("o", 5), c("s", 20), c("m", 4), c("v", 12), c("n", 140), c("u", 20), c("r", 3));
    let re = |p: &str, x: &str| regex::Regex::new(p).unwrap().is_match(x);
    if !re(r"^\d{4}-\d{2}-\d{2}$", &d) {
        return Err("data non valida".into());
    }
    if !o.is_empty() && !re(r"^\d{1,2}:\d{2}$", &o) {
        o.clear();
    }
    if !SPOT.iter().any(|s| s.id == sp) {
        return Err("spot non valido".into());
    }
    if !re(r"^\d{1,3}$", &mi) {
        mi.clear();
    }
    if !["N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSO", "SO", "OSO", "O", "ONO", "NO", "NNO"].contains(&di.as_str()) {
        di.clear();
    }
    let mut ri = c("w", 5).replace(',', ".");
    if !re(r"^\d{1,2}(\.\d)?$", &ri) {
        ri.clear();
    }
    let riga = format!("{d};{o};{sp};{mi};{v};{no};{mu};{di};{ri}");
    match trova("i", &tutte)? {
        Some(i) => {
            tutte[i] = riga;
            riscrivi_diario(&file, &tutte);
            Ok(format!("modificata {d} {sp}"))
        }
        None => {
            if tutte.is_empty() {
                riscrivi_diario(&file, &[riga]);
            } else {
                aggiungi(&file, INT_DIARIO, &[riga]);
            }
            Ok(format!("{d} {sp}"))
        }
    }
}

/// Al primo avvio della 4.0: copia uscite, storico e casa/lavoro dalla cartella della 3.4, se c'e'.
/// La 3.4 non si tocca; ognuno importa solo i dati del proprio PC.
pub fn importa_34(cartella: &Path) -> Option<String> {
    let segno = cartella.join("dati").join("importato-3.4.txt");
    if segno.exists() {
        return None;
    }
    let mut basi: Vec<PathBuf> = Vec::new();
    for v in ["USERPROFILE", "HOME", "OneDrive", "OneDriveConsumer"] {
        if let Some(h) = std::env::var_os(v) {
            basi.push(PathBuf::from(&h).join("Desktop"));
            basi.push(PathBuf::from(&h).join("OneDrive").join("Desktop"));
        }
    }
    let vecchia = basi.into_iter().map(|b| b.join("Radar kite")).find(|c| c.join("dati").is_dir() && c.join("radar-kite.ps1").exists())?;
    let (da, a) = (vecchia.join("dati"), cartella.join("dati"));
    let _ = std::fs::create_dir_all(&a);
    let (mut file_n, mut righe_n) = (0, 0);
    for e in std::fs::read_dir(&da).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        let int = if n == "diario.csv" { "data;ora;spot;minuti;vela;note" } else if n.starts_with("letture-") && n.ends_with(".csv") { INT_LETTURE } else if n.starts_with("previsioni-") && n.ends_with(".csv") { INT_PREV } else { continue };
        let vecchie = righe(&e.path());
        let dest = a.join(&n);
        let gia: HashSet<String> = righe(&dest).into_iter().collect();
        let mut tutte: Vec<String> = vecchie.into_iter().filter(|r| !gia.contains(r)).collect();
        righe_n += tutte.len();
        if dest.exists() {
            // Le righe della 3.4 vengono prima di quelle gia' scritte dalla 4.0.
            let mut nuove = tutte.clone();
            nuove.extend(righe(&dest));
            if n.starts_with("letture-") || n.starts_with("previsioni-") {
                nuove.sort();
            }
            tutte = nuove;
            let _ = std::fs::remove_file(&dest);
        }
        let intest = if n == "diario.csv" { INT_DIARIO } else { int };
        aggiungi(&dest, intest, &tutte);
        file_n += 1;
    }
    // Casa, lavoro e le altre scelte personali, se nella 4.0 non sono ancora scritte.
    let imp34 = crate::viaggi::leggi_impostazioni(&vecchia.join("impostazioni.txt"));
    let file_imp = cartella.join("impostazioni.txt");
    let imp40 = crate::viaggi::leggi_impostazioni(&file_imp);
    if (imp40["casa"].is_empty() || imp40["casa"] == crate::viaggi::PARTENZA_PREDEFINITA) && imp40["lavoro"].is_empty() {
        for k in ["casa", "lavoro", "partenza", "copia_telefono", "google_chiave"] {
            if !imp34[k].is_empty() {
                crate::viaggi::salva_impostazione(&file_imp, k, &imp34[k]);
            }
        }
    }
    let nota = format!("Dati importati dalla 3.4 ({}): {file_n} file, {righe_n} righe.", vecchia.display());
    let _ = std::fs::write(&segno, &nota);
    Some(nota)
}
