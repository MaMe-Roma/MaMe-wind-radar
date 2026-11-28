//! Windguru, firing, Open-Meteo, mare: porting di Read-Windguru, Get-Giorni, Get-Fascia, Get-Classe, Get-Vicini,
//! Get-Finestra, Read-OpenMeteo, Get-UrlOpenMeteo, Get-UrlAcqua, Read-Acqua, Get-Onde, Get-Meteo (radar-kite.ps1 3.4).
//! Arrotondamenti come .NET: [Math]::Round e [int] arrotondano "al pari" (0,5 -> 0, 1,5 -> 2, 2,5 -> 2).

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use regex::Regex;
use serde_json::Value;

use crate::regole::{Spot, Vicino, GG, ORE_MIN, PUNTI, SOGLIA, VIAGGIO_ORE};

pub fn round0(x: f64) -> f64 {
    x.round_ties_even()
}
pub fn round_n(x: f64, cifre: i32) -> f64 {
    if x.abs() >= 1e16 {
        return x;
    }
    let p = 10f64.powi(cifre);
    (x * p).round_ties_even() / p
}

/// Sort-Object di Windows PowerShell 5.1: .NET Framework vi usa il quicksort "legacy" (ArraySortHelper.DepthLimitedQuickSort),
/// che a parita' di valore non conserva l'ordine. Lo rifaccio passo per passo, cosi' anche le parita' danno lo stesso risultato.
pub fn ordina_ps<T: Clone>(v: &mut [T], cmp: impl Fn(&T, &T) -> std::cmp::Ordering) {
    if v.len() > 1 {
        quick_ps(v, 0, v.len() as isize - 1, &cmp, 32);
    }
}
fn scambia_se_maggiore<T>(v: &mut [T], cmp: &impl Fn(&T, &T) -> std::cmp::Ordering, a: isize, b: isize) {
    if a != b && cmp(&v[a as usize], &v[b as usize]) == std::cmp::Ordering::Greater {
        v.swap(a as usize, b as usize);
    }
}
fn quick_ps<T: Clone>(v: &mut [T], mut left: isize, mut right: isize, cmp: &impl Fn(&T, &T) -> std::cmp::Ordering, mut limite: i32) {
    use std::cmp::Ordering::Less;
    loop {
        if limite == 0 {
            // Heapsort di .NET: mai raggiunto con le poche righe di Radar kite.
            v[left as usize..=right as usize].sort_by(|a, b| cmp(a, b));
            return;
        }
        let (mut i, mut j) = (left, right);
        let mezzo = i + ((j - i) >> 1);
        scambia_se_maggiore(v, cmp, i, mezzo);
        scambia_se_maggiore(v, cmp, i, j);
        scambia_se_maggiore(v, cmp, mezzo, j);
        let x = v[mezzo as usize].clone();
        loop {
            while cmp(&v[i as usize], &x) == Less {
                i += 1;
            }
            while cmp(&x, &v[j as usize]) == Less {
                j -= 1;
            }
            if i > j {
                break;
            }
            if i < j {
                v.swap(i as usize, j as usize);
            }
            i += 1;
            j -= 1;
            if i > j {
                break;
            }
        }
        limite -= 1;
        if j - left <= right - i {
            if left < j {
                quick_ps(v, left, j, cmp, limite);
            }
            left = i;
        } else {
            if i < right {
                quick_ps(v, i, right, cmp, limite);
            }
            right = j;
        }
        if left >= right {
            break;
        }
    }
}

pub fn comp(deg: f64) -> &'static str {
    let i = round0((((deg % 360.0) + 360.0) % 360.0) / 22.5) as i64;
    PUNTI[(i % 16) as usize]
}

pub fn classe(s: &Spot, deg: f64) -> &'static str {
    if deg >= s.ok[0] && deg <= s.ok[1] {
        return "ok";
    }
    if deg >= s.terra[0] || deg <= s.terra[1] {
        return "terra";
    }
    "side"
}

fn vicini(lista: &[Vicino], gradi: &[f64], solo_buoni: bool, s: &Spot) -> Vec<&'static str> {
    let mut out = Vec::new();
    for v in lista {
        for &g in gradi {
            let diff = (g - v.d).abs() % 360.0;
            if diff.min(360.0 - diff) <= 22.5 && (!solo_buoni || classe(s, g) == "ok") {
                out.push(v.n);
                break;
            }
        }
    }
    out
}

pub fn fascia(d: NaiveDate) -> (i64, i64) {
    let md = d.month() * 100 + d.day();
    if md >= 1015 || md <= 331 {
        (8, 16)
    } else {
        (8, 19)
    }
}

pub fn nome_giorno(d: NaiveDate) -> String {
    format!("{} {}", GG[d.weekday().num_days_from_sunday() as usize], d.day())
}

/// Ora di inizio della fascia utile: per oggi conta solo il vento da un'ora dopo l'avvio.
pub fn inizio_fascia(d: NaiveDate, oggi: bool, adesso: NaiveDateTime) -> i64 {
    let f = fascia(d);
    if oggi {
        f.0.max((adesso + Duration::hours(VIAGGIO_ORE)).hour() as i64)
    } else {
        f.0
    }
}

// ---------------- Windguru ----------------
#[derive(Clone, Debug)]
pub struct RigaWg {
    pub t: NaiveDateTime,
    pub date: NaiveDate,
    pub hour: i64,
    pub wind: f64,
    pub gust: Option<f64>,
    pub dir: String,
    pub deg: f64,
}

#[derive(Clone, Debug, Default)]
pub struct InfoWg {
    pub spot: String,
    pub init: String,
    pub utc: i64,
    pub righe: Vec<RigaWg>,
}

pub fn leggi_windguru(html: &str, adesso: NaiveDateTime) -> InfoWg {
    let senza_tag = Regex::new(r"<[^>]+>").unwrap().replace_all(html, "");
    let t = html_escape::decode_html_entities(&senza_tag).into_owned();
    let mut info = InfoWg { utc: 1, ..Default::default() };
    // In PowerShell -match non distingue maiuscole e minuscole: (?i) ovunque.
    if let Some(m) = Regex::new(r"(?im)^\s*\S.*? - ([^,\r\n]+),\s*lat:").unwrap().captures(&t) {
        info.spot = m[1].trim().to_string();
    }
    if let Some(m) = Regex::new(r"(?i)init:\s*([0-9-]+ \d{2}) UTC").unwrap().captures(&t) {
        info.init = m[1].to_string();
    }
    if let Some(m) = Regex::new(r"(?i)\(UTC([+-]\d+)\)").unwrap().captures(&t) {
        info.utc = m[1].parse().unwrap_or(1);
    }
    let re = Regex::new(r"(?i)^\s*(Mon|Tue|Wed|Thu|Fri|Sat|Sun)\s+(\d{1,2})\.\s+(\d{1,2})h\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)").unwrap();
    let solo_cifre = Regex::new(r"^\d+$").unwrap();
    let oggi = adesso.date();
    for riga in Regex::new(r"\r?\n").unwrap().split(&t) {
        let Some(m) = re.captures(riga) else { continue };
        let giorno: u32 = m[2].parse().unwrap_or(0);
        let ora: i64 = m[3].parse().unwrap_or(0);
        let (w, g, dir, deg) = (&m[4], &m[5], &m[6], &m[7]);
        if !solo_cifre.is_match(w) {
            continue;
        }
        let mut data = None;
        for k in -1..=20 {
            let c = oggi + Duration::days(k);
            if c.day() == giorno {
                data = Some(c);
                break;
            }
        }
        let Some(data) = data else { continue };
        let utc = data.and_hms_opt(0, 0, 0).unwrap() + Duration::hours(ora - info.utc);
        let loc = Local.from_utc_datetime(&utc).naive_local();
        let _ = Utc; // l'ora del sito e' UTC + scarto: la converto nell'ora del PC, come la 3.4
        let gradi = if solo_cifre.is_match(deg) {
            deg.parse::<f64>().unwrap_or(0.0)
        } else {
            PUNTI.iter().position(|p| *p == dir).map(|i| i as f64).unwrap_or(-1.0) * 22.5
        };
        info.righe.push(RigaWg {
            t: loc,
            date: loc.date(),
            hour: loc.hour() as i64,
            wind: w.parse::<f64>().unwrap_or(0.0),
            gust: if solo_cifre.is_match(g) { g.parse::<f64>().ok() } else { None },
            dir: dir.to_string(),
            deg: gradi,
        });
    }
    info
}

#[derive(Clone, Debug, Default)]
pub struct Firing {
    pub ore: String,
    pub durata: i64,
    pub min: f64,
    pub maxf: f64,
    pub raffica: Option<f64>,
    pub dir: String,
    pub pericolose: bool,
    pub rafficato: bool,
    pub sideoff: bool,
    pub ideale: String,
    pub attenzione: String,
    pub note: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Giorno {
    pub data: NaiveDate,
    pub nome: String,
    pub oggi: bool,
    pub fascia: String,
    pub max: Option<f64>,
    pub max_raff: Option<f64>,
    pub max_dir: String,
    pub firing: Option<Firing>,
}

pub fn giorni(s: &Spot, righe: &[RigaWg], adesso: NaiveDateTime) -> Vec<Giorno> {
    let mut out = Vec::new();
    for k in 0..crate::regole::GIORNI {
        let giorno = adesso.date() + Duration::days(k);
        let f = fascia(giorno);
        let mut del_giorno: Vec<&RigaWg> = righe.iter().filter(|r| r.date == giorno).collect();
        del_giorno.sort_by_key(|r| r.t);
        let mut passo = 1;
        if del_giorno.len() > 1 {
            passo = 24;
            for i in 1..del_giorno.len() {
                let d = del_giorno[i].hour - del_giorno[i - 1].hour;
                if d > 0 && d < passo {
                    passo = d;
                }
            }
        }
        let inizio = inizio_fascia(giorno, k == 0, adesso);
        let utili: Vec<&RigaWg> = del_giorno.iter().copied().filter(|r| r.hour >= inizio && r.hour <= f.1).collect();
        let durata_di = |run: &[&RigaWg]| (run[run.len() - 1].hour + passo).min(f.1 + 1) - run[0].hour;
        let mut migliore: Option<Vec<&RigaWg>> = None;
        let mut run: Vec<&RigaWg> = Vec::new();
        for r in utili.iter().map(Some).chain(std::iter::once(None)) {
            if let Some(r) = r {
                if r.wind >= SOGLIA && classe(s, r.deg) != "terra" {
                    run.push(r);
                    continue;
                }
            }
            if !run.is_empty() {
                let durata = durata_di(&run);
                let dm = migliore.as_ref().map(|m| durata_di(m)).unwrap_or(-1);
                if durata >= ORE_MIN && durata > dm {
                    migliore = Some(run.clone());
                }
            }
            run.clear();
        }
        let mut res = Giorno { data: giorno, nome: nome_giorno(giorno), oggi: k == 0, fascia: format!("{}-{}", f.0, f.1), max: None, max_raff: None, max_dir: String::new(), firing: None };
        if !utili.is_empty() {
            // Il piu' ventoso: Sort-Object wind -Descending | Select-Object -First 1, con lo stesso ordinamento
            // della 3.4 (a parita' di vento conta, come in PowerShell, l'ordinamento non stabile di .NET).
            let mut ord = utili.clone();
            ordina_ps(&mut ord, |a, b| b.wind.total_cmp(&a.wind));
            let top = ord[0];
            res.max = Some(top.wind);
            res.max_dir = top.dir.clone();
            res.max_raff = utili.iter().filter_map(|r| r.gust).reduce(f64::max);
        }
        if let Some(m) = migliore {
            let gradi: Vec<f64> = m.iter().map(|r| r.deg).collect();
            let raff: Vec<&&RigaWg> = m.iter().filter(|r| r.gust.is_some()).collect();
            let (primo, ultimo) = (m[0].hour, m[m.len() - 1].hour);
            // Con la previsione ogni 3 ore l'ultima riga copre anche le ore dopo: 13-13 diventa 13-16.
            let fine_ore = (ultimo + if passo > 1 { passo } else { 0 }).min(f.1);
            let mut x = Firing {
                ore: if fine_ore > primo { format!("{primo}-{fine_ore}") } else { format!("{primo}") },
                durata: durata_di(&m),
                min: m.iter().map(|r| r.wind).fold(f64::INFINITY, f64::min),
                maxf: m.iter().map(|r| r.wind).fold(f64::NEG_INFINITY, f64::max),
                raffica: raff.iter().filter_map(|r| r.gust).reduce(f64::max),
                ..Default::default()
            };
            let mut dirs: Vec<&str> = Vec::new();
            for r in &m {
                if !dirs.contains(&r.dir.as_str()) {
                    dirs.push(&r.dir);
                }
            }
            x.dir = dirs.join("/");
            x.pericolose = matches!(x.raffica, Some(g) if g > 35.0);
            x.rafficato = !x.pericolose && raff.iter().any(|r| r.gust.unwrap() - r.wind > 10.0);
            x.sideoff = m.iter().any(|r| classe(s, r.deg) == "side");
            if x.pericolose {
                x.note.push("raffiche pericolose".into());
            } else if x.rafficato {
                x.note.push("rafficato".into());
            }
            if x.sideoff {
                x.note.push("side-off".into());
            }
            let id = vicini(s.ideali, &gradi, true, s);
            x.ideale = if id.is_empty() { String::new() } else { format!("ideale con {}", id.join(" e ")) };
            if !x.ideale.is_empty() {
                x.note.push(x.ideale.clone());
            }
            let at = vicini(s.attenzione, &gradi, false, s);
            x.attenzione = at.join(", ");
            x.note.extend(at.iter().map(|a| a.to_string()));
            res.firing = Some(x);
        }
        out.push(res);
    }
    out
}

// ---------------- Finestra firing su righe orarie (Get-Finestra) ----------------
pub struct OraVento {
    pub hour: i64,
    pub wind: Option<f64>,
    pub deg: Option<f64>,
}

pub struct Finestra {
    pub ore: String,
    pub min: i64,
    pub max: i64,
}

pub fn finestra(s: &Spot, righe: &[OraVento], inizio: i64, fine: i64) -> Option<Finestra> {
    let mut ord: Vec<&OraVento> = righe.iter().filter(|x| x.hour >= inizio && x.hour <= fine && x.wind.is_some() && x.deg.is_some()).collect();
    if ord.windows(2).any(|w| w[1].hour <= w[0].hour) {
        ord.sort_by_key(|x| x.hour);
    }
    let mut migliore: Option<Vec<&OraVento>> = None;
    let mut run: Vec<&OraVento> = Vec::new();
    let dur = |r: &[&OraVento]| r[r.len() - 1].hour - r[0].hour + 1;
    for r in ord.iter().map(Some).chain(std::iter::once(None)) {
        let ok = matches!(r, Some(x) if x.wind.unwrap() >= SOGLIA && classe(s, x.deg.unwrap()) != "terra");
        if ok && (run.is_empty() || r.unwrap().hour == run[run.len() - 1].hour + 1) {
            run.push(r.unwrap());
            continue;
        }
        if !run.is_empty() {
            let d = dur(&run);
            if d >= ORE_MIN && migliore.as_ref().map_or(true, |m| d > dur(m)) {
                migliore = Some(run.clone());
            }
        }
        run.clear();
        if ok {
            run.push(r.unwrap());
        }
    }
    let m = migliore?;
    let mn = m.iter().map(|x| x.wind.unwrap()).fold(f64::MAX, f64::min);
    let mx = m.iter().map(|x| x.wind.unwrap()).fold(f64::MIN, f64::max);
    Some(Finestra { ore: format!("{}-{}", m[0].hour, m[m.len() - 1].hour), min: round0(mn) as i64, max: round0(mx) as i64 })
}

// ---------------- Open-Meteo: meteo, secondo modello del vento, acqua ----------------
fn coord(x: f64) -> String {
    format!("{x}")
}

pub fn url_open_meteo(spot: &[Spot]) -> String {
    let lat: Vec<String> = spot.iter().map(|s| coord(s.lat)).collect();
    let lon: Vec<String> = spot.iter().map(|s| coord(s.lon)).collect();
    format!("https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m,weather_code,precipitation,wind_speed_10m,wind_gusts_10m,wind_direction_10m&wind_speed_unit=kn&timezone=Europe%2FRome&forecast_days=9", lat.join(","), lon.join(","))
}

/// Un punto un po' al largo di ogni spot, verso il mare (centro delle direzioni buone).
pub fn url_acqua(spot: &[&Spot]) -> String {
    let (mut lat, mut lon) = (Vec::new(), Vec::new());
    for s in spot {
        let c = if s.ok[0] <= s.ok[1] { (s.ok[0] + s.ok[1]) / 2.0 } else { ((s.ok[0] + s.ok[1] + 360.0) / 2.0) % 360.0 };
        let r = c * std::f64::consts::PI / 180.0;
        lat.push(format!("{:.3}", s.lat + 0.06 * r.cos()));
        lon.push(format!("{:.3}", s.lon + 0.06 * r.sin() / 0.745));
    }
    format!("https://marine-api.open-meteo.com/v1/marine?latitude={}&longitude={}&hourly=sea_surface_temperature,wave_height,wave_period,wave_direction,swell_wave_height,swell_wave_period,wind_wave_height,wind_wave_period&timezone=Europe%2FRome&forecast_days=9", lat.join(","), lon.join(","))
}

#[derive(Clone, Debug)]
pub struct RigaOm {
    pub date: NaiveDate,
    pub hour: i64,
    pub temp: Option<f64>,
    pub code: Option<f64>,
    pub rain: Option<f64>,
    pub wind: Option<f64>,
    pub gust: Option<f64>,
    pub deg: Option<f64>,
}

fn ora_om(v: &Value) -> Result<NaiveDateTime, String> {
    let s = v.as_str().ok_or("ora mancante")?;
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").map_err(|e| format!("ora non valida {s}: {e}"))
}
fn val(a: &Value, campo: &str, i: usize) -> Option<f64> {
    a.get(campo).and_then(|x| x.get(i)).and_then(|x| x.as_f64())
}
fn lista(j: Value) -> Vec<Value> {
    match j {
        Value::Array(a) => a,
        x => vec![x],
    }
}

pub fn leggi_open_meteo(testo: &str, spot: &[Spot]) -> Result<Vec<Vec<RigaOm>>, String> {
    let j: Value = serde_json::from_str(testo.trim_start_matches('\u{feff}')).map_err(|e| format!("risposta non valida: {e}"))?;
    let l = lista(j);
    let mut out = Vec::new();
    for i in 0..spot.len() {
        let h = l.get(i).and_then(|x| x.get("hourly")).ok_or("dati orari mancanti")?;
        let tempi = h.get("time").and_then(|x| x.as_array()).ok_or("ore mancanti")?;
        let mut righe = Vec::new();
        for (n, tv) in tempi.iter().enumerate() {
            let t = ora_om(tv)?;
            righe.push(RigaOm { date: t.date(), hour: t.hour() as i64, temp: val(h, "temperature_2m", n), code: val(h, "weather_code", n), rain: val(h, "precipitation", n),
                wind: val(h, "wind_speed_10m", n), gust: val(h, "wind_gusts_10m", n), deg: val(h, "wind_direction_10m", n) });
        }
        out.push(righe);
    }
    Ok(out)
}

#[derive(Clone, Debug)]
pub struct RigaOnde {
    pub date: NaiveDate,
    pub hour: i64,
    pub h: Option<f64>,
    pub p: Option<f64>,
    pub d: Option<f64>,
    pub sh: Option<f64>,
    pub sp: Option<f64>,
    pub wh: Option<f64>,
    pub wp: Option<f64>,
}

/// Temperatura dell'acqua (media di oggi) e onde ora per ora, per ogni spot.
pub fn leggi_acqua(testo: &str, spot: &[&Spot]) -> Result<(Vec<Option<f64>>, Vec<Vec<RigaOnde>>), String> {
    let j: Value = serde_json::from_str(testo.trim_start_matches('\u{feff}')).map_err(|e| format!("risposta non valida: {e}"))?;
    let l = lista(j);
    let (mut acqua, mut onde) = (Vec::new(), Vec::new());
    for i in 0..spot.len() {
        let hh = l.get(i).and_then(|x| x.get("hourly")).cloned().unwrap_or(Value::Null);
        let sst: Vec<f64> = hh.get("sea_surface_temperature").and_then(|x| x.as_array()).map(|a| a.iter().take(24).filter_map(|x| x.as_f64()).collect()).unwrap_or_default();
        acqua.push(if sst.is_empty() { None } else { Some(round0(sst.iter().sum::<f64>() / sst.len() as f64)) });
        let mut righe = Vec::new();
        let ha_onde = hh.get("wave_height").and_then(|x| x.as_array()).is_some_and(|a| !a.is_empty());
        let tempi = hh.get("time").and_then(|x| x.as_array()).cloned().unwrap_or_default();
        if ha_onde {
            for (k, tv) in tempi.iter().enumerate() {
                let Some(h) = val(&hh, "wave_height", k) else { continue };
                let t = ora_om(tv)?;
                righe.push(RigaOnde { date: t.date(), hour: t.hour() as i64, h: Some(h), p: val(&hh, "wave_period", k), d: val(&hh, "wave_direction", k),
                    sh: val(&hh, "swell_wave_height", k), sp: val(&hh, "swell_wave_period", k), wh: val(&hh, "wind_wave_height", k), wp: val(&hh, "wind_wave_period", k) });
            }
        }
        onde.push(righe);
    }
    Ok((acqua, onde))
}

/// Onde di un giorno: l'ora con l'onda piu' alta tra da e a (a parita' di altezza, la prima).
pub fn onde_giorno(righe: &[&RigaOnde], giorno: NaiveDate, da: i64, a: i64) -> Value {
    let mut x: Option<&RigaOnde> = None;
    for y in righe {
        if y.date == giorno && y.hour >= da && y.hour <= a && x.map_or(true, |x| y.h.unwrap_or(0.0) > x.h.unwrap_or(0.0)) {
            x = Some(y);
        }
    }
    let Some(x) = x else { return Value::Null };
    let arr = |v: Option<f64>, c: i32| v.map(|v| crate::num(round_n(v, c))).unwrap_or(Value::Null);
    serde_json::json!({ "h": arr(x.h, 1), "p": arr(x.p, 0), "d": arr(x.d, 0), "sh": arr(x.sh, 1), "sp": arr(x.sp, 0), "wh": arr(x.wh, 1), "wp": arr(x.wp, 0), "ora": x.hour })
}

pub fn meteo_giorno(righe: &[&RigaOm], giorno: NaiveDate, fa: (i64, i64)) -> Value {
    let mut periodi = Vec::new();
    for p in [(8, 11), (12, 16), (17, 20)] {
        let (mut n, mut max_c, mut sum_t, mut n_p, mut sum_p) = (0, None::<f64>, 0.0, 0, 0.0);
        for x in righe {
            if x.date != giorno || x.hour < p.0 || x.hour > p.1 || x.code.is_none() || x.temp.is_none() {
                continue;
            }
            n += 1;
            sum_t += x.temp.unwrap();
            if max_c.map_or(true, |m| x.code.unwrap() > m) {
                max_c = x.code;
            }
            if let Some(r) = x.rain {
                n_p += 1;
                sum_p += r;
            }
        }
        if n == 0 {
            periodi.push(Value::Null);
            continue;
        }
        periodi.push(serde_json::json!({ "codice": round0(max_c.unwrap()) as i64, "temp": round0(sum_t / n as f64) as i64,
            "pioggia": if n_p > 0 { crate::num(round_n(sum_p, 1)) } else { Value::from(0) } }));
    }
    let temporale = righe.iter().any(|x| x.date == giorno && x.hour >= fa.0 && x.hour <= fa.1 && x.code.is_some_and(|c| c >= 95.0));
    serde_json::json!({ "periodi": periodi, "temporale": temporale })
}

#[cfg(test)]
mod prove {
    use super::ordina_ps;
    #[test]
    fn ordina_come_net() {
        // Risultati noti del quicksort legacy di .NET Framework (chiave, posizione originale).
        let mut v: Vec<(i32, usize)> = [3, 1, 2, 1, 3].iter().copied().zip(0..).collect();
        ordina_ps(&mut v, |a, b| a.0.cmp(&b.0));
        assert_eq!(v.iter().map(|x| x.0).collect::<Vec<_>>(), vec![1, 1, 2, 3, 3]);
        let mut w: Vec<i32> = (0..40).map(|i| (i * 7919) % 23).collect();
        let mut atteso = w.clone();
        atteso.sort();
        ordina_ps(&mut w, |a, b| a.cmp(b));
        assert_eq!(w, atteso);
    }
}
