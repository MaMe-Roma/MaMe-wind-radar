// Radar kite 4.0 - l'app: una finestra con la pagina di sempre e il motore Rust sotto.
// La pagina parla con il motore con i link radarkite:// (come nella 3.4): la finestra li intercetta
// e avvia il motore; il motore riscrive report.html e stato.js, la pagina se ne accorge e si ricarica.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::http::Response;
use tauri::{Url, WebviewUrl, WebviewWindowBuilder};

static IN_CORSO: AtomicBool = AtomicBool::new(false);

const ATTESA: &str = r#"<!DOCTYPE html>
<html lang="it"><head><meta charset="utf-8"><title>Radar kite</title></head>
<body style="margin:0;height:100vh;display:grid;place-items:center;background:#f6f4ef;color:#1f1f1f;font:15px system-ui,sans-serif">
<p>Radar kite: leggo le previsioni…</p>
<script>setTimeout(function () { location.reload(); }, 1000);</script>
</body></html>"#;

/// Cartella dei dati: Windows %APPDATA%\Radar kite, Mac ~/Library/Application Support/Radar kite.
fn cartella_app() -> PathBuf {
    if let Some(p) = std::env::var_os("RK_CARTELLA") {
        return PathBuf::from(p);
    }
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library").join("Application Support"));
    #[cfg(not(any(windows, target_os = "macos")))]
    let base = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")));
    base.unwrap_or_else(std::env::temp_dir).join("Radar kite")
}

/// Fa girare il motore in sottofondo (uno alla volta).
fn aggiorna(cartella: PathBuf) {
    if IN_CORSO.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let _ = motore::esegui(&motore::Opzioni::nuove(cartella));
        IN_CORSO.store(false, Ordering::SeqCst);
    });
}

/// I file della cartella dati, per la finestra (indirizzo rk://localhost/... o http://rk.localhost/... su Windows).
fn servi(cartella: &Path, percorso: &str) -> Response<Vec<u8>> {
    let nome = percorso.trim_start_matches('/');
    let nome = if nome.is_empty() { "report.html" } else { nome };
    let risposta = |stato: u16, tipo: &str, corpo: Vec<u8>| {
        Response::builder().status(stato).header("Content-Type", tipo).header("Cache-Control", "no-store").body(corpo).unwrap()
    };
    if nome.contains("..") || nome.contains('/') || nome.contains('\\') {
        return risposta(404, "text/plain", b"non trovato".to_vec());
    }
    let tipo = if nome.ends_with(".js") { "text/javascript; charset=utf-8" } else if nome.ends_with(".pdf") { "application/pdf" } else { "text/html; charset=utf-8" };
    match std::fs::read(cartella.join(nome)) {
        Ok(b) => risposta(200, tipo, b),
        Err(_) if nome == "report.html" => risposta(200, tipo, ATTESA.as_bytes().to_vec()),
        Err(_) => risposta(404, "text/plain", b"non trovato".to_vec()),
    }
}

/// Un pulsante della pagina (radarkite://...). Fase 1: tutti aggiornano; impostazioni, diario e copia arrivano dopo.
fn comando(url: &Url, cartella: &Path) {
    match url.host_str().unwrap_or("") {
        "apri" => {
            if let Some((_, u)) = url.query_pairs().find(|(k, _)| k == "u") {
                if u.starts_with("https://") || u.starts_with("http://") {
                    let _ = open::that_detached(u.as_ref());
                }
            }
        }
        _ => aggiorna(cartella.to_path_buf()),
    }
}

fn main() {
    let cartella = cartella_app();
    let _ = std::fs::create_dir_all(&cartella);
    let per_protocollo = cartella.clone();
    let per_finestra = cartella.clone();
    tauri::Builder::default()
        .register_uri_scheme_protocol("rk", move |_ctx, richiesta| servi(&per_protocollo, richiesta.uri().path()))
        .setup(move |app| {
            // Si apre l'ultimo report in modalita' "aggiornamento in corso" e intanto parte il motore.
            let ora = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
            #[cfg(windows)]
            let base = "http://rk.localhost/";
            #[cfg(not(windows))]
            let base = "rk://localhost/";
            let url: Url = format!("{base}report.html#aggiorno-{ora}").parse().expect("indirizzo della pagina");
            aggiorna(per_finestra.clone());
            let c = per_finestra.clone();
            WebviewWindowBuilder::new(app, "principale", WebviewUrl::External(url))
                .title("Radar kite")
                .inner_size(1180.0, 900.0)
                .min_inner_size(420.0, 500.0)
                .on_navigation(move |u| {
                    if u.scheme() == "radarkite" {
                        comando(u, &c);
                        return false;
                    }
                    let nostra = u.scheme() == "rk" || u.host_str() == Some("rk.localhost") || u.scheme() == "about" || u.scheme() == "data";
                    if !nostra && (u.scheme() == "http" || u.scheme() == "https") {
                        let _ = open::that_detached(u.as_str());
                    }
                    nostra
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Radar kite non si e' avviato");
}
