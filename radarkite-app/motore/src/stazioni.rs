//! Stazioni: porting di Read-Stazione (radar-kite.ps1 3.4), un lettore per formato.
//! Come in PowerShell, -match non distingue maiuscole e minuscole: (?i) in tutte le espressioni.

use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone};
use regex::Regex;
use serde_json::Value;

use crate::previsioni::round_n;
use crate::regole::{Stazione, PUNTI};

#[derive(Clone, Debug)]
pub struct Lettura {
    pub t: NaiveDateTime,
    pub wind: f64,
    pub gust: Option<f64>,
    pub deg: f64,
    pub offline: bool,
    pub fresca: bool,
}

fn re(p: &str) -> Regex {
    Regex::new(&format!("(?i){p}")).unwrap()
}

/// [double] di PowerShell: numero, testo con il punto (cultura invariante) o niente (= 0).
fn dbl(v: Option<&Value>) -> Result<f64, String> {
    match v {
        None | Some(Value::Null) => Ok(0.0),
        Some(Value::Number(n)) => Ok(n.as_f64().unwrap_or(0.0)),
        Some(Value::String(s)) => if s.trim().is_empty() { Ok(0.0) } else { s.trim().parse::<f64>().map_err(|_| format!("valore non numerico: {s}")) },
        Some(Value::Bool(b)) => Ok(if *b { 1.0 } else { 0.0 }),
        Some(x) => Err(format!("valore non numerico: {x}")),
    }
}
fn num_testo(s: &str) -> Result<f64, String> {
    s.replace(',', ".").parse::<f64>().map_err(|_| format!("valore non numerico: {s}"))
}
fn da_epoch(v: Option<&Value>) -> Result<NaiveDateTime, String> {
    let s = dbl(v)? as i64;
    Local.timestamp_opt(s, 0).single().map(|d| d.naive_local()).ok_or_else(|| "ora non valida".to_string())
}
fn json(testo: &str) -> Result<Value, String> {
    serde_json::from_str(testo).map_err(|e| format!("risposta non valida: {e}"))
}
/// ParseExact con giorno/mese/anno e ore/minuti/secondi gia' separati.
fn data_ora(d: &str, m: &str, y: &str, h: &str, mi: &str, s: &str) -> Result<NaiveDateTime, String> {
    let n = |x: &str| x.trim().parse::<u32>().map_err(|_| format!("data non valida: {d}/{m}/{y} {h}:{mi}:{s}"));
    let mut anno = n(y)? as i32;
    if y.trim().len() == 2 {
        anno += if anno <= 49 { 2000 } else { 1900 }; // come .NET (anno a due cifre fino al 2049)
    }
    let (hh, mm, ss) = (n(h)?, n(mi)?, n(s)?);
    NaiveDate::from_ymd_opt(anno, n(m)?, n(d)?).and_then(|g| g.and_hms_opt(hh, mm, ss)).ok_or_else(|| format!("data non valida: {d}/{m}/{y} {h}:{mi}:{s}"))
}
/// Sigla italiana (O = ovest) -> gradi, come [array]::IndexOf($PUNTI, $sigla) * 22.5.
fn gradi_sigla(sigla: &str) -> Result<f64, String> {
    let s = sigla.replace(['O', 'o'], "W");
    PUNTI.iter().position(|p| *p == s).map(|i| i as f64 * 22.5).ok_or_else(|| format!("direzione non riconosciuta: {s}"))
}

pub fn leggi(st: &Stazione, testo: &str) -> Result<Lettura, String> {
    let testo = testo.trim_start_matches('\u{feff}');
    let l = |t, wind, gust, deg, offline| Ok(Lettura { t, wind, gust, deg, offline, fresca: false });
    match st.tipo {
        "msn" => {
            let j = json(testo)?;
            let off = match j.get("offline") { Some(Value::String(s)) => s == "1", Some(Value::Number(n)) => n.to_string() == "1", _ => false };
            l(da_epoch(j.get("DateTime"))?, dbl(j.get("W"))?, Some(dbl(j.get("G"))?), dbl(j.get("B"))?, off)
        }
        "weathercloud" => {
            let j = json(testo)?;
            let nullo = |k: &str| j.get(k).map_or(true, |v| v.is_null());
            if nullo("epoch") || nullo("wspd") {
                return Err("dati non trovati".into());
            }
            let g = if nullo("wspdhi") { None } else { Some(round_n(dbl(j.get("wspdhi"))? * 1.943844, 1)) };
            l(da_epoch(j.get("epoch"))?, round_n(dbl(j.get("wspd"))? * 1.943844, 1), g, dbl(j.get("wdir"))?, false)
        }
        "meteotemplate" => {
            let j = json(testo)?;
            let ok = match j.get("ok") { None | Some(Value::Null) => false, Some(Value::Bool(b)) => *b, Some(Value::Number(n)) => n.as_f64() != Some(0.0), Some(Value::String(s)) => !s.is_empty(), _ => true };
            if !ok {
                let e = j.get("error").map(|v| v.as_str().map(String::from).unwrap_or_else(|| v.to_string())).unwrap_or_default();
                return Err(format!("stazione non disponibile: {e}"));
            }
            let unita = j.get("unit").map(|v| v.as_str().map(String::from).unwrap_or_else(|| v.to_string())).unwrap_or_default();
            let k = if re("km").is_match(&unita) { 1.0 / 1.852 } else if re("m/s|ms").is_match(&unita) { 1.943844 } else if re("mph").is_match(&unita) { 0.868976 } else { 1.0 };
            let g = if j.get("gust").map_or(true, |v| v.is_null()) { None } else { Some(round_n(dbl(j.get("gust"))? * k, 1)) };
            l(da_epoch(j.get("updated"))?, round_n(dbl(j.get("speed"))? * k, 1), g, dbl(j.get("bearing"))?, false)
        }
        "lni" => {
            let f: Vec<&str> = testo.trim().split_whitespace().collect();
            if f.len() < 75 || f[0] != "12345" {
                return Err("clientraw.txt non riconosciuto".into());
            }
            let d: Vec<&str> = f[74].split('/').collect();
            if d.len() != 3 {
                return Err(format!("data non valida: {}", f[74]));
            }
            let t = data_ora(d[0], d[1], d[2], f[29], f[30], f[31])?;
            l(t, num_testo(f[1])?, Some(num_testo(f[2])?), num_testo(f[3])?, false)
        }
        "rdt" => {
            let j = json(testo)?;
            let voci: Vec<&Value> = match &j { Value::Array(a) => a.iter().collect(), x => vec![x] };
            let voci: Vec<&Value> = voci.into_iter().filter(|v| v.get("b_type").and_then(|x| x.as_str()).is_some_and(|x| x.eq_ignore_ascii_case("last_obs"))).collect();
            let etichetta = |v: &&Value| v.get("label").and_then(|x| x.as_str()).unwrap_or("").to_lowercase();
            let v = voci.iter().find(|v| re(r"^velocit.*vento$").is_match(&etichetta(v))).ok_or("vento non trovato")?;
            let g = voci.iter().find(|v| etichetta(v) == "raffica vento");
            let d = voci.iter().find(|v| etichetta(v) == "direzione vento");
            let tempo = v.get("time").and_then(|x| x.as_str()).unwrap_or("");
            let m = Regex::new(r"^(\d{2})/(\d{2})/(\d{4}) (\d{2}):(\d{2})$").unwrap().captures(tempo).ok_or_else(|| format!("ora non valida: {tempo}"))?;
            let t = data_ora(&m[1], &m[2], &m[3], &m[4], &m[5], "0")?;
            l(t, dbl(v.get("value"))?, Some(dbl(g.and_then(|x| x.get("value")))?), dbl(d.and_then(|x| x.get("value")))?, false)
        }
        "vedetta" => {
            let m = re(r"(\d{1,2}):(\d{2}):(\d{2})\s+(\d{1,2})/(\d{1,2})/(\d{4})").captures(testo).ok_or("ora non trovata")?;
            let t = data_ora(&m[4], &m[5], &m[6], &m[1], &m[2], &m[3])?;
            let w = re(r"Vento Attuale:[\s\S]*?<big>\s*([\d.,]+)\s*kts").captures(testo).ok_or("vento non trovato")?;
            let w = num_testo(&w[1])?;
            let d = re(r"Direzione:\s*<strong>\s*<big>\s*([A-Z]{1,3})\s*</big>").captures(testo).ok_or("direzione non trovata")?;
            let deg = gradi_sigla(&d[1])?;
            let g = match re(r"Raffica MAX:\s*<strong>\s*<big>\s*([\d.,]+)\s*kts").captures(testo) { Some(c) => Some(num_testo(&c[1])?), None => None };
            l(t, w, g, deg, false)
        }
        "wind24" => {
            if re(r"momentaneamente[\s\S]{0,120}Sospeso").is_match(testo) {
                return Err("stazione sospesa dal sito (rinnovo)".into());
            }
            let m = re(r"Meteo del\s*(\d{1,2})/(\d{1,2})/(\d{4})\s*ore\s*(\d{1,2}):(\d{2})").captures(testo).ok_or("ora non trovata")?;
            let t = data_ora(&m[1], &m[2], &m[3], &m[4], &m[5], "0")?;
            let w = re(r"Velocit[^<]{0,14}media(?:&nbsp;|\s)*<font[^>]*>\s*([\d.,]+)").captures(testo).ok_or("vento non trovato")?;
            let w = num_testo(&w[1])?;
            let g = match re(r"Raffiche(?:&nbsp;|\s)*<font[^>]*>\s*([\d.,]+)").captures(testo) { Some(c) => Some(num_testo(&c[1])?), None => None };
            let deg = re(r"createMarker\('odoMap',\s*'(\d+(?:\.\d+)?)',\s*'http").captures(testo)
                .or_else(|| re(r"createMarker\('odoMap',\s*'(\d+(?:\.\d+)?)'").captures(testo)).ok_or("direzione non trovata")?;
            l(t, w, g, num_testo(&deg[1])?, false)
        }
        "meteolazio" => {
            let m = re(r"Aggiornamento:\s*<strong>([^<]+)</strong>").captures(testo).ok_or("ora non trovata")?;
            let ora = m[1].trim().to_string();
            let p = Regex::new(r"^(\d{2})/(\d{2})/(\d{2}) (\d{2}):(\d{2})$").unwrap().captures(&ora).ok_or_else(|| format!("ora non valida: {ora}"))?;
            let t = data_ora(&p[1], &p[2], &p[3], &p[4], &p[5], "0")?;
            let w = re(r#"mrl-primary-label">Velocit[^<]*</span>\s*<strong[^>]*>([\d.,]+)\s*km/h"#).captures(testo).ok_or("vento non trovato")?;
            let kmh = num_testo(&w[1])?;
            let d = re(r"Direzione<br>\s*<strong>([A-Z]{1,3})</strong>").captures(testo).ok_or("direzione non trovata")?;
            let deg = gradi_sigla(&d[1])?;
            let g = match re(r"Ultima raffica</span>\s*<strong[^>]*>([\d.,]+)\s*km/h").captures(testo) { Some(c) => Some(round_n(num_testo(&c[1])? / 1.852, 1)), None => None };
            l(t, round_n(kmh / 1.852, 1), g, deg, false)
        }
        x => Err(format!("tipo di stazione sconosciuto: {x}")),
    }
}
