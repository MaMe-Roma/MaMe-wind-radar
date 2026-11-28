// Radar kite 4.0 - l'app: una finestra con la pagina di sempre e il motore Rust sotto.
// La pagina parla con il motore con i link radarkite:// (come nella 3.4): la finestra li intercetta
// e avvia il motore; il motore riscrive report.html e stato.js, la pagina se ne accorge e si ricarica.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::http::Response;
use tauri::{Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_updater::UpdaterExt;

static IN_CORSO: AtomicBool = AtomicBool::new(false);
// Pulsante premuto mentre il motore lavora: parte appena finisce (vale l'ultimo).
static IN_CODA: Mutex<Option<Option<String>>> = Mutex::new(None);

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

/// Fa girare il motore in sottofondo (uno alla volta). link = il pulsante della pagina (radarkite://...).
/// automatico = giro di ogni ora: se il motore sta gia' lavorando si salta.
fn aggiorna(cartella: PathBuf, link: Option<String>, automatico: bool) {
    if IN_CORSO.swap(true, Ordering::SeqCst) {
        if !automatico {
            *IN_CODA.lock().unwrap() = Some(link);
        }
        return;
    }
    std::thread::spawn(move || {
        let mut link = link;
        loop {
            let mut o = motore::Opzioni::nuove(cartella.clone());
            o.link = link;
            o.programma = std::env::current_exe().ok();
            if automatico && o.link.is_none() {
                o.fasi = vec![2];
                o.automatico = true;
            }
            let _ = motore::esegui(&o);
            match IN_CODA.lock().unwrap().take() {
                Some(dopo) => link = dopo,
                None => break,
            }
        }
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

/// Un pulsante della pagina (radarkite://...). Partenza, indirizzi e tempi di viaggio li gestisce il motore;
/// diario, copia per il telefono e aggiornamento di un solo spot arrivano con la fase 3 (intanto aggiornano tutto).
fn comando(url: &Url, cartella: &Path) {
    match url.host_str().unwrap_or("") {
        "apri" => {
            if let Some((_, u)) = url.query_pairs().find(|(k, _)| k == "u") {
                if u.starts_with("https://") || u.starts_with("http://") {
                    let _ = open::that_detached(u.as_ref());
                }
            }
        }
        _ => aggiorna(cartella.to_path_buf(), Some(url.as_str().to_string()), false),
    }
}

/// All'avvio: se su GitHub c'e' una versione nuova (firmata con la chiave di Radar kite) chiede se aggiornare.
fn controlla_aggiornamenti(h: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let Ok(u) = h.updater() else { return };
        let Ok(Some(nuova)) = u.check().await else { return };
        let testo = format!("È disponibile Radar kite {}.\n\nAggiorno adesso? L'app si chiude, si aggiorna e si riapre da sola. I tuoi dati restano.", nuova.version);
        let h2 = h.clone();
        h.dialog().message(testo).title("Radar kite").buttons(MessageDialogButtons::OkCancelCustom("Sì, aggiorna".into(), "Più tardi".into())).show(move |si| {
            if si {
                tauri::async_runtime::spawn(async move {
                    if nuova.download_and_install(|_, _| {}, || {}).await.is_ok() {
                        h2.restart();
                    }
                });
            }
        });
    });
}

fn main() {
    let cartella = cartella_app();
    // Avviato dal pianificatore del sistema: aggiorna senza finestra ed esce.
    if std::env::args().any(|a| a == "--automatico") {
        let _ = std::fs::create_dir_all(&cartella);
        let mut o = motore::Opzioni::nuove(cartella);
        o.fasi = vec![2];
        o.automatico = true;
        let _ = motore::esegui(&o);
        return;
    }
    let _ = std::fs::create_dir_all(&cartella);
    let per_protocollo = cartella.clone();
    let per_finestra = cartella.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("rk", move |_ctx, richiesta| servi(&per_protocollo, richiesta.uri().path()))
        .setup(move |app| {
            // Si apre l'ultimo report in modalita' "aggiornamento in corso" e intanto parte il motore.
            let ora = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
            #[cfg(windows)]
            let base = "http://rk.localhost/";
            #[cfg(not(windows))]
            let base = "rk://localhost/";
            let url: Url = format!("{base}report.html#aggiorno-{ora}").parse().expect("indirizzo della pagina");
            // Primo avvio: uscite, storico e casa/lavoro dalla 3.4 sul Desktop, poi il primo aggiornamento.
            let c0 = per_finestra.clone();
            std::thread::spawn(move || {
                let _ = motore::storico::importa_34(&c0);
                // L'aggiornamento automatico punta sempre a questo programma (anche se e' stato spostato).
                let imp = motore::viaggi::leggi_impostazioni(&c0.join("impostazioni.txt"));
                if let (true, Ok(exe)) = (imp["automatico"] == "si", std::env::current_exe()) {
                    let _ = motore::pianifica::applica(&imp, &exe);
                }
                aggiorna(c0, None, false);
            });
            let c = per_finestra.clone();
            controlla_aggiornamenti(app.handle().clone());
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
