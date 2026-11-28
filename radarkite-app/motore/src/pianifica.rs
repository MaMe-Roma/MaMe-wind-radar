//! Aggiornamento automatico anche ad app chiusa: il pianificatore del sistema avvia "radar-kite --automatico"
//! nella fascia oraria e con la frequenza scelte nella pagina (come Register-AvvioAutomatico della 3.4).
//! Windows: Utilita' di pianificazione (attivita' "Radar kite 4.0"). Mac: launchd (~/Library/LaunchAgents).
//! Si attiva solo se il proprietario risponde "Si'" nella pagina (impostazione automatico = si).

use std::path::Path;

use chrono::{NaiveDateTime, NaiveTime, Timelike};

use crate::viaggi::Imp;

pub const NOME: &str = "Radar kite 4.0";
#[cfg(target_os = "macos")]
const ETICHETTA: &str = "com.madhattermatt.radarkite";

pub fn ora(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M").ok()
}

/// Gli orari della giornata: dalle, poi ogni N minuti fino alle (compreso).
pub fn orari(imp: &Imp) -> Vec<NaiveTime> {
    let (Some(da), Some(a)) = (ora(&imp["automatico_dalle"]), ora(&imp["automatico_alle"])) else { return vec![] };
    let ogni: i64 = imp["automatico_ogni"].parse().unwrap_or(60).max(15);
    let mut out = Vec::new();
    let mut t = da;
    while t <= a && out.len() < 100 {
        out.push(t);
        let (n, giro) = t.overflowing_add_signed(chrono::Duration::minutes(ogni));
        if giro != 0 {
            break;
        }
        t = n;
    }
    out
}

/// Testo per la pagina: "prossimo alle 14:30" o "prossimo domani alle 6:30".
pub fn prossimo(imp: &Imp, adesso: NaiveDateTime) -> String {
    if imp["automatico"] != "si" {
        return String::new();
    }
    let v = orari(imp);
    let fmt = |t: &NaiveTime| format!("{}:{:02}", t.hour(), t.minute());
    match v.iter().find(|t| **t > adesso.time()) {
        Some(t) => format!("prossimo alle {}", fmt(t)),
        None => v.first().map(|t| format!("prossimo domani alle {}", fmt(t))).unwrap_or_default(),
    }
}

/// Controlla i valori scelti nella pagina. Ok((dalle, alle, ogni)).
pub fn controlla(da: &str, a: &str, ogni: &str) -> Result<(String, String, String), String> {
    let (Some(x), Some(y)) = (ora(da), ora(a)) else { return Err("orario non valido".into()) };
    if y <= x {
        return Err("l'orario di fine deve venire dopo quello di inizio".into());
    }
    if !["30", "60", "120", "180"].contains(&ogni) {
        return Err("frequenza non valida".into());
    }
    Ok((x.format("%H:%M").to_string(), y.format("%H:%M").to_string(), ogni.to_string()))
}

#[allow(dead_code)]
fn comando(prog: &str, args: &[&str]) -> Result<String, String> {
    let mut c = std::process::Command::new(prog);
    c.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    let o = c.output().map_err(|e| format!("{prog}: {e}"))?;
    let testo = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    if o.status.success() { Ok(testo) } else { Err(testo.trim().to_string()) }
}

#[allow(dead_code)]
fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Attiva o toglie l'avvio automatico secondo le impostazioni. exe = il programma da avviare.
pub fn applica(imp: &Imp, exe: &Path) -> Result<(), String> {
    if imp["automatico"] == "si" {
        let v = orari(imp);
        if v.is_empty() {
            return Err("orari non validi".into());
        }
        registra(imp, &v, exe)
    } else {
        togli();
        Ok(())
    }
}

#[cfg(windows)]
fn registra(imp: &Imp, v: &[NaiveTime], exe: &Path) -> Result<(), String> {
    let ogni: i64 = imp["automatico_ogni"].parse().unwrap_or(60);
    let durata = (v[v.len() - 1] - v[0]).num_minutes() + 1;
    let inizio = chrono::Local::now().date_naive().and_time(v[0]).format("%Y-%m-%dT%H:%M:%S");
    let testo = format!(r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>Radar kite 4.0: previsioni e stazioni, e il PDF per il telefono, dalle {} alle {} ogni {} minuti</Description></RegistrationInfo>
  <Triggers><CalendarTrigger><StartBoundary>{inizio}</StartBoundary><Enabled>true</Enabled><ScheduleByDay><DaysInterval>1</DaysInterval></ScheduleByDay>
    <Repetition><Interval>PT{ogni}M</Interval><Duration>PT{durata}M</Duration><StopAtDurationEnd>false</StopAtDurationEnd></Repetition></CalendarTrigger></Triggers>
  <Principals><Principal id="Author"><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
  <Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><StartWhenAvailable>true</StartWhenAvailable><ExecutionTimeLimit>PT10M</ExecutionTimeLimit><Enabled>true</Enabled></Settings>
  <Actions Context="Author"><Exec><Command>{}</Command><Arguments>--automatico</Arguments></Exec></Actions>
</Task>"#, imp["automatico_dalle"], imp["automatico_alle"], ogni, xml(&exe.to_string_lossy()));
    let file = std::env::temp_dir().join("radarkite-attivita.xml");
    let mut b: Vec<u8> = vec![0xFF, 0xFE];
    for u in testo.encode_utf16() {
        b.extend_from_slice(&u.to_le_bytes());
    }
    std::fs::write(&file, b).map_err(|e| e.to_string())?;
    let r = comando("schtasks", &["/Create", "/F", "/TN", NOME, "/XML", &file.to_string_lossy()]);
    let _ = std::fs::remove_file(&file);
    r.map(|_| ()).map_err(|e| format!("Utilita' di pianificazione: {e}"))
}

#[cfg(windows)]
pub fn togli() {
    let _ = comando("schtasks", &["/Delete", "/F", "/TN", NOME]);
}

#[cfg(windows)]
pub fn presente() -> bool {
    comando("schtasks", &["/Query", "/TN", NOME]).is_ok()
}

#[cfg(target_os = "macos")]
fn plist() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join("Library/LaunchAgents").join(format!("{ETICHETTA}.plist")))
}
#[cfg(target_os = "macos")]
fn dominio() -> String {
    format!("gui/{}", comando("id", &["-u"]).unwrap_or_default().trim())
}

#[cfg(target_os = "macos")]
fn registra(_imp: &Imp, v: &[NaiveTime], exe: &Path) -> Result<(), String> {
    let p = plist().ok_or("cartella utente non trovata")?;
    let _ = std::fs::create_dir_all(p.parent().unwrap());
    let orari: String = v.iter().map(|t| format!("<dict><key>Hour</key><integer>{}</integer><key>Minute</key><integer>{}</integer></dict>", t.hour(), t.minute())).collect();
    let testo = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>{ETICHETTA}</string>
<key>ProgramArguments</key><array><string>{}</string><string>--automatico</string></array>
<key>StartCalendarInterval</key><array>{orari}</array>
<key>RunAtLoad</key><false/>
</dict></plist>"#, xml(&exe.to_string_lossy()));
    let _ = comando("launchctl", &["bootout", &dominio(), &p.to_string_lossy()]);
    std::fs::write(&p, testo).map_err(|e| e.to_string())?;
    comando("launchctl", &["bootstrap", &dominio(), &p.to_string_lossy()]).map(|_| ()).map_err(|e| format!("launchd: {e}"))
}

#[cfg(target_os = "macos")]
pub fn togli() {
    if let Some(p) = plist() {
        let _ = comando("launchctl", &["bootout", &dominio(), &p.to_string_lossy()]);
        let _ = std::fs::remove_file(p);
    }
}

#[cfg(target_os = "macos")]
pub fn presente() -> bool {
    comando("launchctl", &["print", &format!("{}/{ETICHETTA}", dominio())]).is_ok()
}

#[cfg(not(any(windows, target_os = "macos")))]
fn registra(_imp: &Imp, _v: &[NaiveTime], _exe: &Path) -> Result<(), String> {
    Err("aggiornamento automatico disponibile solo su Windows e Mac".into())
}
#[cfg(not(any(windows, target_os = "macos")))]
pub fn togli() {}
#[cfg(not(any(windows, target_os = "macos")))]
pub fn presente() -> bool {
    false
}
