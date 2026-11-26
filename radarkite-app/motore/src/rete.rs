//! Download in parallelo e cache: come Start-Scarico, Get-Scaricato, Get-Testo, Save-Cache, Read-Cache della 3.4.
//! Le richieste partono tutte insieme (un thread ciascuna); get_testo usa la risposta quando arriva.
//! La cache e' la stessa della 3.4: dati/cache/<sha1 dell'indirizzo>.txt, testo UTF-8.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::regole::UA;

struct Pre {
    rx: Option<Receiver<Result<String, String>>>,
    testo: Option<String>,
    errore: Option<String>,
    rigido: bool,
    secondi: u64,
    stazione: bool,
}

pub struct Rete {
    cache: PathBuf,
    solo_cache: bool,
    pre: HashMap<String, Pre>,
    /// Quando ha risposto ogni sito (per i tempi delle stazioni, come Update-Tempi).
    fatti: Arc<Mutex<HashMap<String, Instant>>>,
    t0: Instant,
}

pub fn file_cache(dir: &Path, url: &str) -> PathBuf {
    let hx = sha1_smol::Sha1::from(url.as_bytes()).digest().to_string();
    dir.join(format!("{hx}.txt"))
}

pub fn leggi_cache(dir: &Path, url: &str) -> Option<String> {
    let b = std::fs::read(file_cache(dir, url)).ok()?;
    let b = b.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(&b);
    Some(String::from_utf8_lossy(b).into_owned())
}

fn salva_cache(dir: &Path, url: &str, testo: &str) {
    let _ = std::fs::write(file_cache(dir, url), testo.as_bytes());
}

fn scarica(url: &str, referer: &str, xhr: bool, secondi: u64) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(secondi)).user_agent(UA).build();
    let mut req = agent.get(url).set("Cache-Control", "no-cache");
    if !referer.is_empty() {
        req = req.set("Referer", referer);
    }
    if xhr {
        req = req.set("X-Requested-With", "XMLHttpRequest");
    }
    match req.call() {
        Ok(resp) => {
            let mut b = Vec::new();
            resp.into_reader().take(64 * 1024 * 1024).read_to_end(&mut b).map_err(|e| messaggio(&e.to_string(), secondi))?;
            // Decodifica sempre in UTF-8, cosi' le lettere accentate non si rovinano.
            Ok(String::from_utf8_lossy(&b).into_owned())
        }
        Err(ureq::Error::Status(code, _)) => Err(format!("il sito risponde con errore {code}")),
        Err(e) => Err(messaggio(&e.to_string(), secondi)),
    }
}

fn messaggio(m: &str, secondi: u64) -> String {
    let l = m.to_lowercase();
    if l.contains("timed out") || l.contains("timeout") {
        format!("nessuna risposta in {secondi} secondi")
    } else {
        m.to_string()
    }
}

impl Rete {
    pub fn new(cache: PathBuf, solo_cache: bool) -> Self {
        let _ = std::fs::create_dir_all(&cache);
        Rete { cache, solo_cache, pre: HashMap::new(), fatti: Arc::new(Mutex::new(HashMap::new())), t0: Instant::now() }
    }

    /// Fa partire una richiesta (Start-Scarico). rigido = Open-Meteo, mare e stazioni.
    pub fn parti(&mut self, url: &str, referer: &str, xhr: bool, rigido: bool, secondi: u64, stazione: bool) {
        if self.solo_cache {
            let (testo, errore) = match leggi_cache(&self.cache, url) {
                Some(t) => (Some(t), None),
                None => (None, Some("non nella cache".to_string())),
            };
            self.pre.insert(url.to_string(), Pre { rx: None, testo, errore, rigido: true, secondi, stazione });
            return;
        }
        let (tx, rx) = channel();
        let (u, r) = (url.to_string(), referer.to_string());
        let fatti = self.fatti.clone();
        std::thread::spawn(move || {
            let esito = scarica(&u, &r, xhr, secondi);
            fatti.lock().unwrap().insert(u, Instant::now());
            let _ = tx.send(esito);
        });
        self.pre.insert(url.to_string(), Pre { rx: Some(rx), testo: None, errore: None, rigido, secondi, stazione });
    }

    /// Da qui si contano i secondi di risposta delle stazioni (come $T0 nella 3.4).
    pub fn segna_t0(&mut self) {
        self.t0 = Instant::now();
    }
    /// Secondi dalla partenza delle richieste alla risposta del sito (None se non e' stato scaricato ora).
    pub fn secondi(&self, url: &str) -> Option<f64> {
        self.fatti.lock().unwrap().get(url).map(|t| crate::previsioni::round_n(t.saturating_duration_since(self.t0).as_secs_f64(), 1))
    }

    /// Get-Scaricato: Ok(Some((testo, dalla_cache))), Ok(None) = da riprovare, Err = errore definitivo.
    fn scaricato(&mut self, url: &str) -> Result<Option<(String, bool)>, String> {
        let p = match self.pre.remove(url) {
            Some(p) => p,
            None => return Ok(None),
        };
        if let Some(t) = p.testo {
            return Ok(Some((t, true)));
        }
        if let Some(e) = p.errore {
            return Err(e);
        }
        let esito = match p.rx.as_ref().map(|rx| rx.recv_timeout(Duration::from_secs(p.secondi + 5))) {
            Some(Ok(r)) => r,
            Some(Err(RecvTimeoutError::Timeout)) | Some(Err(RecvTimeoutError::Disconnected)) | None => Err(format!("nessuna risposta in {} secondi", p.secondi)),
        };
        match esito {
            Ok(t) => Ok(Some((t, false))),
            Err(msg) => {
                // Stazioni: l'errore resta. Open-Meteo e mare: se non e' un'attesa scaduta riprovo una volta.
                if p.rigido && (p.stazione || msg.starts_with("nessuna risposta")) {
                    Err(msg)
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Get-Testo: usa il download gia' partito; se e' fallito (e non e' una stazione) riprova qui.
    pub fn testo(&mut self, url: &str, referer: &str, xhr: bool) -> Result<String, String> {
        if self.pre.contains_key(url) {
            if let Some((t, dalla_cache)) = self.scaricato(url)? {
                if !dalla_cache {
                    salva_cache(&self.cache, url, &t);
                }
                return Ok(t);
            }
        }
        if self.solo_cache {
            return leggi_cache(&self.cache, url).ok_or_else(|| "non nella cache".to_string());
        }
        let mut n = 1;
        loop {
            match scarica(url, referer, xhr, 20) {
                Ok(t) => {
                    salva_cache(&self.cache, url, &t);
                    return Ok(t);
                }
                Err(e) => {
                    if n >= 2 {
                        return Err(e);
                    }
                    n += 1;
                    std::thread::sleep(Duration::from_secs(3));
                }
            }
        }
    }
}
