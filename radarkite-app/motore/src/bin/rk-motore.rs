//! Radar kite 4.0 - motore da riga di comando (per le prove e per il confronto con la 3.4).
//! rk-motore --cartella DIR [--cache DIR] [--solo-cache] [--adesso AAAA-MM-GGTHH:MM:SS] [--om-prova FILE] [--modello FILE] [--automatico]

use std::path::PathBuf;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let val = |nome: &str| a.iter().position(|x| x == nome).and_then(|i| a.get(i + 1)).cloned();
    let c = PathBuf::from(val("--cartella").unwrap_or_else(|| ".".into()));
    let mut o = motore::Opzioni::nuove(c);
    o.cache = val("--cache").map(PathBuf::from);
    o.solo_cache = a.iter().any(|x| x == "--solo-cache");
    o.om_prova = val("--om-prova").map(PathBuf::from);
    o.modello = val("--modello").map(PathBuf::from);
    o.link = val("--link");
    if a.iter().any(|x| x == "--automatico") {
        o.fasi = vec![2];
        o.automatico = true;
    }
    if let Some(t) = val("--adesso") {
        match chrono::NaiveDateTime::parse_from_str(&t, "%Y-%m-%dT%H:%M:%S") {
            Ok(x) => o.adesso = Some(x),
            Err(e) => {
                eprintln!("--adesso non valido ({t}): {e}");
                std::process::exit(2);
            }
        }
    }
    match motore::esegui(&o) {
        Ok(e) => println!("Report: {}", e.report.display()),
        Err(e) => {
            eprintln!("ERRORE: {e}");
            std::process::exit(1);
        }
    }
}
