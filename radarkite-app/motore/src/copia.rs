//! Copia per il telefono: PDF della pagina in una cartella Dropbox, Google Drive, OneDrive o iCloud.
//! Porting di Get-CartelleCloud, Get-CartellaCopia, Save-Pdf (radar-kite.ps1 3.4), anche per il Mac.
//! Il PDF lo fa il browser in modalita' nascosta: Edge o Chrome su Windows; sul Mac Chrome (o Edge, Brave) se c'e'.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::{Local, NaiveDateTime};
use serde_json::{json, Value};

const LEGGIMI: &str = "RADAR KITE - copia per il telefono\r\n\r\nQuesta cartella la riempie Radar kite. Dentro c'e' \"Radar kite.pdf\": la pagina del radar con\r\nl'ultima rilevazione (vento previsto, stazioni, meteo, onde), sempre con lo stesso nome.\r\n\r\nSi aggiorna ogni volta che Radar kite si aggiorna: a mano, oppure da solo nella fascia oraria\r\nscelta nella pagina (riga in cima con l'orologio), se il PC o il Mac e' acceso.\r\n\r\nSul telefono: apri il file dall'app (Dropbox, Google Drive, OneDrive...) e mettilo tra i\r\npreferiti o \"disponibile offline\": cosi' hai sempre l'ultima rilevazione a portata di mano,\r\nanche fuori casa.\r\n\r\nPer cambiare cartella o non fare piu' la copia: nella pagina di Radar kite, riga\r\n\"Copia per il telefono\". Questa cartella si puo' cancellare quando vuoi.\r\n";

pub struct Cloud {
    pub nome: String,
    pub radice: PathBuf,
    pub cartella: PathBuf,
}

fn env(k: &str) -> Option<PathBuf> {
    std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from)
}

pub fn cartelle_cloud() -> Vec<Cloud> {
    let mut lista: Vec<Cloud> = Vec::new();
    let mut agg = |nome: &str, radice: Option<PathBuf>| {
        let Some(r) = radice else { return };
        if !r.is_dir() || lista.iter().any(|x| x.radice == r) {
            return;
        }
        lista.push(Cloud { nome: nome.into(), cartella: r.join("Radar kite"), radice: r });
    };
    let home = env("USERPROFILE").or_else(|| env("HOME"));
    let info = if cfg!(windows) { env("LOCALAPPDATA").map(|p| p.join("Dropbox").join("info.json")) } else { home.as_ref().map(|h| h.join(".dropbox").join("info.json")) };
    if let Some(j) = info.and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
        for (k, n) in [("personal", "Dropbox"), ("business", "Dropbox (lavoro)")] {
            agg(n, j[k]["path"].as_str().map(PathBuf::from));
        }
    }
    agg("Dropbox", home.as_ref().map(|h| h.join("Dropbox")));
    if cfg!(windows) {
        for l in b'C'..=b'Z' {
            for n in ["Il mio Drive", "My Drive"] {
                agg("Google Drive", Some(PathBuf::from(format!("{}:\\{n}", l as char))));
            }
        }
        for n in ["Il mio Drive", "My Drive", "Google Drive"] {
            agg("Google Drive", home.as_ref().map(|h| h.join(n)));
        }
        agg("OneDrive", env("OneDriveConsumer"));
        agg("OneDrive", env("OneDrive"));
        agg("OneDrive (lavoro)", env("OneDriveCommercial"));
        agg("iCloud Drive", home.as_ref().map(|h| h.join("iCloudDrive")));
    } else if let Some(h) = &home {
        // Mac: le app cloud di oggi stanno in ~/Library/CloudStorage; iCloud Drive in Mobile Documents.
        let cs = h.join("Library").join("CloudStorage");
        let mut voci: Vec<PathBuf> = std::fs::read_dir(&cs).into_iter().flatten().flatten().map(|e| e.path()).collect();
        voci.sort();
        for p in voci {
            let n = p.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            if n.starts_with("Dropbox") {
                agg("Dropbox", Some(p));
            } else if n.starts_with("GoogleDrive") {
                for d in ["Il mio Drive", "My Drive"] {
                    agg("Google Drive", Some(p.join(d)));
                }
            } else if n.starts_with("OneDrive") {
                agg("OneDrive", Some(p));
            }
        }
        agg("iCloud Drive", Some(h.join("Library").join("Mobile Documents").join("com~apple~CloudDocs")));
    }
    lista
}

pub fn cartella_copia(scelta: &str, cloud: &[Cloud]) -> Option<PathBuf> {
    match scelta {
        "" | "no" => None,
        "auto" => cloud.iter().find(|x| x.nome.starts_with("Dropbox")).map(|x| x.cartella.clone()),
        v => Some(PathBuf::from(v)),
    }
}

/// Pulsante "Copia per il telefono": controlla la cartella scelta. Ok(valore da salvare) o Err(messaggio per la pagina).
pub fn scegli(valore: &str, cloud: &[Cloud]) -> Result<String, String> {
    let mut v: String = valore.chars().filter(|c| !"\r\n\"<>|*?".contains(*c)).collect::<String>().trim().to_string();
    if v.len() > 3 {
        v = v.trim_end_matches(['\\', '/']).to_string();
    }
    let nuvola = cloud.iter().any(|x| x.cartella.to_string_lossy() == v);
    if v == "no" || v == "auto" || nuvola {
        return Ok(v);
    }
    if v.is_empty() {
        return Err("scrivi il percorso di una cartella".into());
    }
    let p = Path::new(&v);
    if !p.is_absolute() {
        return Err(if cfg!(windows) { format!("percorso non completo: {v} (deve iniziare con la lettera del disco, es. C:\\...)") } else { format!("percorso non completo: {v} (deve iniziare con /)") });
    }
    if !p.is_dir() {
        return Err(format!("cartella non trovata: {v}"));
    }
    Ok(v)
}

pub fn dati_pagina(scelta: &str, cloud: &[Cloud], errore: &str, ultimo: &str) -> Value {
    json!({ "scelta": scelta, "cartella": cartella_copia(scelta, cloud).map(|p| p.to_string_lossy().to_string()),
        "candidati": cloud.iter().map(|c| json!({ "nome": c.nome, "radice": c.radice.to_string_lossy(), "cartella": c.cartella.to_string_lossy() })).collect::<Vec<_>>(),
        "errore": errore, "ultimo": ultimo })
}

fn browser() -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        for (v, p) in [("ProgramFiles(x86)", r"Microsoft\Edge\Application\msedge.exe"), ("ProgramFiles", r"Microsoft\Edge\Application\msedge.exe"), ("ProgramFiles", r"Google\Chrome\Application\chrome.exe"),
            ("ProgramFiles(x86)", r"Google\Chrome\Application\chrome.exe"), ("LOCALAPPDATA", r"Google\Chrome\Application\chrome.exe"), ("ProgramFiles", r"BraveSoftware\Brave-Browser\Application\brave.exe")] {
            if let Some(b) = env(v) {
                c.push(b.join(p));
            }
        }
    } else {
        for app in ["Google Chrome.app/Contents/MacOS/Google Chrome", "Microsoft Edge.app/Contents/MacOS/Microsoft Edge", "Brave Browser.app/Contents/MacOS/Brave Browser"] {
            c.push(PathBuf::from("/Applications").join(app));
            if let Some(h) = env("HOME") {
                c.push(h.join("Applications").join(app));
            }
        }
    }
    c.into_iter().find(|p| p.is_file())
}

/// ([Uri]$file).AbsoluteUri: file:///C:/... con gli spazi e le lettere accentate in %XX.
fn url_file(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let mut o = String::from(if s.starts_with('/') { "file://" } else { "file:///" });
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~:".contains(&b) { o.push(b as char) } else { o.push_str(&format!("%{b:02X}")) }
    }
    o
}

/// Save-Pdf: "Radar kite.pdf" nella cartella scelta. Restituisce lo stato per la pagina ("ok|ora" o "errore|motivo"), se c'e' una copia da fare.
pub fn salva_pdf(report: &Path, dest: Option<PathBuf>, adesso: NaiveDateTime) -> Option<String> {
    let dest = dest?;
    if !dest.is_dir() {
        match dest.parent() {
            Some(p) if p.is_dir() => {
                let _ = std::fs::create_dir_all(&dest);
            }
            _ => return Some("errore|cartella non trovata".into()),
        }
    }
    let leggimi = dest.join("LEGGIMI - Radar kite.txt");
    if !leggimi.exists() {
        let mut b = vec![0xEF, 0xBB, 0xBF];
        b.extend_from_slice(LEGGIMI.as_bytes());
        let _ = std::fs::write(&leggimi, b);
    }
    let Some(br) = browser() else {
        return Some(if cfg!(windows) { "errore|non trovo Edge, Chrome o Brave".into() } else { "errore|non trovo Chrome: installalo per avere il PDF per il telefono".into() });
    };
    let pdf = dest.join("Radar kite.pdf");
    let tmp = std::env::temp_dir().join(format!("radarkite-{}", Local::now().timestamp_nanos_opt().unwrap_or(0)));
    let mut cmd = std::process::Command::new(&br);
    cmd.args(["--headless", "--disable-gpu", "--no-first-run", "--no-pdf-header-footer", "--print-to-pdf-no-header"])
        .arg(format!("--user-data-dir={}", tmp.display())).args(["--window-size=760,1400", "--virtual-time-budget=4000"])
        .arg(format!("--print-to-pdf={}", pdf.display())).arg(format!("{}#pdf", url_file(report)))
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // senza finestra
    }
    let esito = match cmd.spawn() {
        Ok(mut p) => {
            let t0 = Instant::now();
            loop {
                if let Ok(Some(_)) = p.try_wait() {
                    break;
                }
                if t0.elapsed() > Duration::from_secs(60) {
                    let _ = p.kill();
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            let nuovo = std::fs::metadata(&pdf).and_then(|m| m.modified()).ok()
                .map(|t| chrono::DateTime::<Local>::from(t).naive_local() > adesso - chrono::Duration::minutes(1)).unwrap_or(false);
            if nuovo { format!("ok|{}", Local::now().format("%Y-%m-%dT%H:%M:%S")) } else { "errore|il browser non ha creato il PDF".into() }
        }
        Err(e) => format!("errore|{e}"),
    };
    let _ = std::fs::remove_dir_all(&tmp);
    Some(esito)
}
