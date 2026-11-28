//! Impostazioni personali (impostazioni.txt, stesso formato della 3.4) e tempi di viaggio:
//! porting di Read-Impostazioni, Set-Impostazione, Get-NomePosto, Get-Viaggi, Get-Parametri, Clean.

use std::collections::HashMap;
use std::path::Path;

use chrono::NaiveDateTime;
use regex::Regex;
use serde_json::{json, Map, Value};

use crate::regole::SPOT;
use crate::rete::Rete;

pub const IMP_TESTO: &str = "# Radar kite - impostazioni personali. Una per riga: nome = valore. Le righe con # sono note.\r\n#\r\n# Da dove parti per andare agli spot (indirizzo e citta'). Serve per il tempo di viaggio.\r\n# Per essere precisi aggiungi le coordinate tra parentesi, es. Via Roma 1, Roma (41.90000, 12.50000).\r\ncasa = Piramide Ostiense, Roma (41.87640, 12.48080)\r\nlavoro =\r\n# Quale delle due usare adesso: casa oppure lavoro (si cambia anche dal menu nella pagina).\r\npartenza = casa\r\n#\r\n# Aggiornamento automatico anche ad app chiusa (si sceglie dalla riga in cima alla pagina): automatico = si o no\r\n# (vuoto = non ancora deciso); dalle e alle = ore:minuti; ogni = minuti (30, 60, 120 o 180).\r\nautomatico =\r\nautomatico_dalle = 06:30\r\nautomatico_alle = 19:30\r\nautomatico_ogni = 60\r\n#\r\n# Copia PDF per il telefono (si sceglie anche dalla pagina, riga \"Copia per il telefono\"):\r\n# auto = Dropbox\\Radar kite se c'e'; no = nessuna copia; oppure il percorso di una cartella.\r\ncopia_telefono = auto\r\n#\r\n# Facoltativo: chiave Google Maps Platform (Routes API) per il tempo di viaggio col traffico. Vuoto = solo stima.\r\ngoogle_chiave =\r\n";

pub type Imp = HashMap<String, String>;

/// Partenza di chi non ha ancora scritto casa o lavoro: un posto pubblico, uguale per tutti.
pub const PARTENZA_PREDEFINITA: &str = "Piramide Ostiense, Roma (41.87640, 12.48080)";

fn righe_file(file: &Path) -> Vec<String> {
    let b = std::fs::read(file).unwrap_or_default();
    let b = b.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(&b);
    String::from_utf8_lossy(b).lines().map(String::from).collect()
}

pub fn leggi_impostazioni(file: &Path) -> Imp {
    let mut imp: Imp = [("casa", ""), ("lavoro", ""), ("partenza", "casa"), ("avvio_automatico", "si"), ("dropbox", "auto"), ("copia_telefono", ""), ("google_chiave", ""), ("automatico", ""), ("automatico_dalle", "06:30"), ("automatico_alle", "19:30"), ("automatico_ogni", "60")]
        .iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    if !file.exists() {
        let mut b = vec![0xEF, 0xBB, 0xBF];
        b.extend_from_slice(IMP_TESTO.as_bytes());
        let _ = std::fs::write(file, b);
    }
    let commento = Regex::new(r"^\s*#").unwrap();
    for r in righe_file(file) {
        if commento.is_match(&r) || !r.contains('=') {
            continue;
        }
        let (k, v) = r.split_once('=').unwrap();
        imp.insert(k.trim().to_lowercase(), v.trim().to_string());
    }
    if imp["partenza"] != "lavoro" {
        imp.insert("partenza".into(), "casa".into());
    }
    if imp["copia_telefono"].is_empty() {
        let d = imp["dropbox"].clone();
        imp.insert("copia_telefono".into(), if d.is_empty() { "auto".into() } else { d });
    }
    imp
}

pub fn salva_impostazione(file: &Path, nome: &str, valore: &str) {
    let mut righe = righe_file(file);
    let re = Regex::new(&format!(r"(?i)^\s*{}\s*=", regex::escape(nome))).unwrap();
    let mut trovata = false;
    for r in righe.iter_mut() {
        if re.is_match(r) {
            *r = format!("{nome} = {valore}");
            trovata = true;
        }
    }
    if !trovata {
        righe.push(format!("{nome} = {valore}"));
    }
    let mut b = vec![0xEF, 0xBB, 0xBF];
    b.extend_from_slice((righe.join("\r\n") + "\r\n").as_bytes());
    let _ = std::fs::write(file, b);
}

/// Indirizzo da mostrare nella pagina: senza le eventuali coordinate tra parentesi.
pub fn nome_posto(v: &str) -> String {
    let t = Regex::new(r"\(?\s*-?\d{1,2}\.\d{3,}\s*,\s*-?\d{1,3}\.\d{3,}\s*\)?").unwrap().replace_all(v, "");
    let t = t.trim_matches(|c| c == ' ' || c == ',' || c == ';');
    if !t.is_empty() { t.to_string() } else if !v.is_empty() { "posizione salvata".into() } else { String::new() }
}

/// [Uri]::EscapeDataString: tutto tranne lettere, cifre e - _ . ~ diventa %XX (UTF-8).
pub fn escape(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { o.push(b as char) } else { o.push_str(&format!("%{b:02X}")) }
    }
    o
}
fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut o = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                o.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        o.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&o).into_owned()
}

/// Get-Parametri: i valori dopo ? in un link radarkite://.
pub fn parametri(link: &str) -> HashMap<String, String> {
    let mut q = HashMap::new();
    if let Some((_, p)) = link.split_once('?') {
        for kv in p.split('&') {
            if let Some((k, v)) = kv.split_once('=') {
                q.insert(k.to_string(), unescape(&v.replace('+', " ")));
            }
        }
    }
    q
}

/// Clean: niente a capo, ; " < > e al massimo max caratteri.
pub fn pulisci(x: &str, max: usize) -> String {
    let y: String = x.chars().map(|c| if "\r\n;\"<>".contains(c) { ' ' } else { c }).collect();
    y.trim().chars().take(max).collect()
}

fn f64_txt(x: f64) -> String {
    format!("{x}")
}

/// Get-Viaggi: minuti in auto dalla partenza a ogni spot (stima OSRM; col traffico se c'e' la chiave Google).
pub fn viaggi(imp: &Imp, dir_dati: &Path, rete: &mut Rete, forza: bool, automatico: bool, adesso: NaiveDateTime) -> Result<Option<Value>, String> {
    let file_v = dir_dati.join("viaggi.json");
    let indirizzo = imp.get(&imp["partenza"]).cloned().unwrap_or_default();
    if indirizzo.is_empty() {
        return Ok(None);
    }
    let cache: Option<Value> = std::fs::read_to_string(&file_v).ok().and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok());
    let firma = SPOT.iter().map(|s| format!("{}:{},{}", s.id, f64_txt(s.lat), f64_txt(s.lon))).collect::<Vec<_>>().join(";");
    let testo_cache = |k: &str| cache.as_ref().and_then(|c| c.get(k)).and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
    let valida = cache.is_some() && testo_cache("indirizzo") == indirizzo.to_lowercase() && testo_cache("firma") == firma.to_lowercase()
        && SPOT.iter().all(|s| cache.as_ref().unwrap().get("stima").and_then(|x| x.get(s.id)).is_some_and(|v| !v.is_null()));
    let chiave = imp.get("google_chiave").cloned().unwrap_or_default();
    let traffico = !chiave.is_empty() && !automatico;
    if valida && !forza && !traffico {
        return Ok(cache);
    }
    let (lat, lon, trovato, stima);
    if valida {
        let c = cache.as_ref().unwrap();
        lat = c["lat"].as_f64().unwrap_or(0.0);
        lon = c["lon"].as_f64().unwrap_or(0.0);
        trovato = c["trovato"].clone();
        stima = c["stima"].clone();
    } else {
        if let Some(cc) = Regex::new(r"(-?\d{1,2}\.\d{3,})\s*,\s*(-?\d{1,3}\.\d{3,})").unwrap().captures(&indirizzo) {
            lat = cc[1].parse().unwrap_or(0.0);
            lon = cc[2].parse().unwrap_or(0.0);
            trovato = json!("coordinate di impostazioni.txt");
        } else {
            let base = "https://nominatim.openstreetmap.org/search?format=json&limit=1&countrycodes=it&addressdetails=1&";
            let rif = "https://www.openstreetmap.org/";
            let lista = |t: &str| -> Result<Vec<Value>, String> {
                match serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).map_err(|e| format!("risposta non valida: {e}"))? {
                    Value::Array(a) => Ok(a),
                    Value::Null => Ok(vec![]),
                    x => Ok(vec![x]),
                }
            };
            let civico = |v: &Value| v.get("address").and_then(|a| a.get("house_number")).is_some_and(|h| !h.is_null() && h.as_str() != Some(""));
            let mut geo: Vec<Value> = Vec::new();
            // Prima via + numero civico + citta' (la ricerca libera spesso ignora il civico), poi l'indirizzo com'e'.
            let pz: Vec<&str> = indirizzo.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()).collect();
            if pz.len() >= 2 {
                if let Some(m) = Regex::new(r"(?i)^(.+?)\s+(\d+\s*[A-Za-z]?)$").unwrap().captures(pz[0]) {
                    let q = format!("street={}&city={}", escape(&format!("{} {}", &m[2], &m[1])), escape(pz[1]));
                    geo = rete.testo(&format!("{base}{q}"), rif, false).and_then(|t| lista(&t)).map(|v| v.into_iter().filter(civico).collect()).unwrap_or_default();
                }
            }
            if geo.is_empty() {
                geo = lista(&rete.testo(&format!("{base}q={}", escape(&indirizzo)), rif, false)?)?;
            }
            if geo.is_empty() {
                return Err(format!("indirizzo non trovato ({indirizzo})"));
            }
            if Regex::new(r"(?i)\s\d+\s*[A-Za-z]?\s*(,|$)").unwrap().is_match(&indirizzo) && !civico(&geo[0]) {
                return Err(format!("numero civico non trovato sulla mappa ({indirizzo})"));
            }
            let nume = |k: &str| geo[0].get(k).and_then(|v| v.as_str().map(|s| s.parse::<f64>().ok()).unwrap_or_else(|| v.as_f64())).unwrap_or(0.0);
            lat = nume("lat");
            lon = nume("lon");
            trovato = geo[0].get("display_name").cloned().unwrap_or(Value::Null);
        }
        let mut coord = vec![format!("{},{}", f64_txt(lon), f64_txt(lat))];
        coord.extend(SPOT.iter().map(|s| format!("{},{}", f64_txt(s.lon), f64_txt(s.lat))));
        let url = format!("https://router.project-osrm.org/table/v1/driving/{}?sources=0&annotations=duration", coord.join(";"));
        let osrm: Value = serde_json::from_str(rete.testo(&url, "https://project-osrm.org/", false)?.trim_start_matches('\u{feff}')).map_err(|e| format!("risposta non valida: {e}"))?;
        let mut m = Map::new();
        for (i, s) in SPOT.iter().enumerate() {
            let d = osrm.get("durations").and_then(|x| x.get(0)).and_then(|x| x.get(i + 1)).and_then(|x| x.as_f64());
            m.insert(s.id.into(), d.map(|d| Value::from((d / 60.0).round_ties_even() as i64)).unwrap_or(Value::Null));
        }
        stima = Value::Object(m);
    }
    let mut live = Map::new();
    let mut live_ora = String::new();
    if traffico {
        let mut esito = Ok(());
        for s in SPOT {
            let corpo = format!(r#"{{"origin":{{"location":{{"latLng":{{"latitude":{},"longitude":{}}}}}}},"destination":{{"location":{{"latLng":{{"latitude":{},"longitude":{}}}}}}},"travelMode":"DRIVE","routingPreference":"TRAFFIC_AWARE"}}"#,
                f64_txt(lat), f64_txt(lon), f64_txt(s.lat), f64_txt(s.lon));
            let r = ureq::post("https://routes.googleapis.com/directions/v2:computeRoutes").timeout(std::time::Duration::from_secs(20))
                .set("Content-Type", "application/json").set("X-Goog-Api-Key", &chiave).set("X-Goog-FieldMask", "routes.duration")
                .send_string(&corpo).map_err(|e| e.to_string()).and_then(|r| r.into_string().map_err(|e| e.to_string()));
            match r.and_then(|t| serde_json::from_str::<Value>(&t).map_err(|e| e.to_string())) {
                Ok(j) => {
                    let sec: f64 = j["routes"][0]["duration"].as_str().unwrap_or("").replace('s', "").parse().unwrap_or(0.0);
                    live.insert(s.id.into(), Value::from((sec / 60.0).round_ties_even() as i64));
                }
                Err(e) => {
                    esito = Err(e);
                    break;
                }
            }
        }
        match esito {
            Ok(()) => live_ora = adesso.format("%H:%M").to_string(),
            Err(e) => {
                println!("  (traffico Google non letto: {e})");
                live.clear();
            }
        }
    }
    let nuovo = json!({ "indirizzo": indirizzo, "firma": firma, "partenza": imp["partenza"], "trovato": trovato, "lat": lat, "lon": lon,
        "calcolato": adesso.format("%Y-%m-%d %H:%M").to_string(), "stima": stima, "traffico": Value::Object(live), "trafficoOra": live_ora });
    let _ = std::fs::write(&file_v, serde_json::to_string_pretty(&nuovo).unwrap_or_default());
    Ok(Some(nuovo))
}
