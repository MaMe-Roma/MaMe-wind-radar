//! Regole di spot e stazioni: copia fedele del blocco "Regole" di radar-kite.ps1 3.4.

pub const SOGLIA: f64 = 15.0; // nodi medi minimi
pub const ORE_MIN: i64 = 2; // ore di fila sopra soglia
pub const GIORNI: i64 = 8; // oggi + 7 giorni
pub const VIAGGIO_ORE: i64 = 1; // per oggi conta solo il vento da un'ora dopo l'avvio

pub const PUNTI: [&str; 16] = ["N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW", "NW", "NNW"];
pub const GG: [&str; 7] = ["dom", "lun", "mar", "mer", "gio", "ven", "sab"];
pub const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) RadarKite/1.0";

pub struct Vicino {
    pub n: &'static str,
    pub d: f64,
}

pub struct Spot {
    pub id: &'static str,
    pub nome: &'static str,
    pub breve: &'static str,
    pub mappa: Option<&'static str>,
    pub lon: f64,
    pub lat: f64,
    pub lab: (i64, i64, &'static str),
    pub wg: i64,
    pub ok: [f64; 2],
    pub terra: [f64; 2],
    pub stazioni: &'static [&'static str],
    pub webcam: &'static str,
    pub surf: Option<&'static str>,
    pub ideali: &'static [Vicino],
    pub attenzione: &'static [Vicino],
}

pub struct Stazione {
    pub id: &'static str,
    pub tipo: &'static str,
    pub nome: &'static str,
    pub etichetta: &'static str,
    pub url: &'static str,
    pub referer: &'static str,
    pub pagina: &'static str,
    pub fresco: i64,
}

const SURF_CIRCOLETTO: Option<&str> = Some("https://it.surf-forecast.com/breaks/Circoletto-Rome/forecasts/latest");

// Spot da nord a sud. ok = direzioni buone (gradi da cui arriva il vento), terra = vento da terra (mai firing).
pub static SPOT: &[Spot] = &[
    Spot { id: "sag", nome: "Civitavecchia Sant'Agostino", breve: "S. Agostino", mappa: None, lon: 11.73579, lat: 42.17537, lab: (11, -9, "start"), wg: 49021,
        ok: [130.0, 320.0], terra: [345.0, 120.0], stazioni: &["rdt_civitavecchia"], webcam: "", surf: None,
        ideali: &[Vicino { n: "scirocco", d: 135.0 }], attenzione: &[] },
    Spot { id: "msn", nome: "Marina di San Nicola", breve: "San Nicola", mappa: None, lon: 12.12663, lat: 41.92557, lab: (11, -9, "start"), wg: 49027,
        ok: [170.0, 300.0], terra: [330.0, 110.0], stazioni: &["msn", "passoscuro"], webcam: "",
        surf: Some("https://it.surf-forecast.com/breaks/Off-The-Wall_1/forecasts/latest"),
        ideali: &[Vicino { n: "ponente", d: 270.0 }], attenzione: &[Vicino { n: "libeccio: lavatrice, onde ~2 m", d: 225.0 }] },
    Spot { id: "fregene", nome: "Fregene", breve: "Fregene", mappa: None, lon: 12.18148, lat: 41.86864, lab: (11, 4, "start"), wg: 49029,
        ok: [165.0, 320.0], terra: [340.0, 130.0], stazioni: &["fregene"], webcam: "https://www.velasportingclub.it/pages/la-nostra-stazione-meteo",
        surf: Some("https://it.surf-forecast.com/breaks/Focene/forecasts/latest"),
        ideali: &[], attenzione: &[Vicino { n: "libeccio: lavatrice", d: 225.0 }, Vicino { n: "maestrale: onda formata", d: 315.0 }] },
    Spot { id: "ostia", nome: "Ostia", breve: "Ostia", mappa: None, lon: 12.37940, lat: 41.68148, lab: (11, -9, "start"), wg: 1518,
        ok: [115.0, 325.0], terra: [345.0, 95.0], stazioni: &["lni_ostia"], webcam: "", surf: SURF_CIRCOLETTO,
        ideali: &[Vicino { n: "scirocco", d: 135.0 }, Vicino { n: "ponente", d: 270.0 }], attenzione: &[] },
    Spot { id: "torva", nome: "Torvaianica", breve: "Torvaianica", mappa: None, lon: 12.43412, lat: 41.64355, lab: (11, 4, "start"), wg: 49033,
        ok: [115.0, 325.0], terra: [345.0, 95.0], stazioni: &["torvaianica"], webcam: "", surf: SURF_CIRCOLETTO,
        ideali: &[Vicino { n: "scirocco", d: 135.0 }, Vicino { n: "ponente", d: 270.0 }], attenzione: &[] },
    Spot { id: "anzio", nome: "Anzio Levante", breve: "Anzio Levante", mappa: Some("Anzio"), lon: 12.63555, lat: 41.45040, lab: (8, -12, "start"), wg: 49035,
        ok: [40.0, 230.0], terra: [250.0, 20.0], stazioni: &["anzio_levante", "anzio_porto"], webcam: "https://vedetta.org/webcam/italia/lazio/roma/anzio-levante/",
        surf: Some("https://it.surf-forecast.com/breaks/Marinaretti-Anzio/forecasts/latest"),
        ideali: &[Vicino { n: "libeccio", d: 225.0 }, Vicino { n: "grecale", d: 45.0 }], attenzione: &[] },
    Spot { id: "latina", nome: "Lido di Latina", breve: "Lido di Latina", mappa: Some("Latina"), lon: 12.87552, lat: 41.40429, lab: (11, 4, "start"), wg: 49036,
        ok: [120.0, 315.0], terra: [335.0, 100.0], stazioni: &["latina_lido", "latina_sud"], webcam: "https://vedetta.org/webcam/italia/lazio/latina/lido-di-latina/", surf: None,
        ideali: &[Vicino { n: "ponente", d: 270.0 }], attenzione: &[Vicino { n: "maestrale: vai a Capo Portiere", d: 315.0 }] },
    Spot { id: "fondi", nome: "Salto di Fondi", breve: "Salto di Fondi", mappa: None, lon: 13.31390, lat: 41.29504, lab: (11, 4, "start"), wg: 49042,
        ok: [110.0, 280.0], terra: [300.0, 80.0], stazioni: &["saltodifondi"], webcam: "", surf: None,
        ideali: &[Vicino { n: "ponente", d: 270.0 }], attenzione: &[] },
    // Spot nuovi della 4.x: sempre in fondo (il confronto con la 3.4 usa solo i primi 8).
    // Lago di Bracciano: solo tramontana e grecale; con le altre direzioni conviene il mare. Col vento da sud spinge al largo.
    Spot { id: "vigna", nome: "Vigna di Valle (lago di Bracciano)", breve: "Vigna di Valle", mappa: None, lon: 12.22300, lat: 42.08500, lab: (8, 12, "start"), wg: WG_VIGNA,
        ok: [340.0, 60.0], terra: [120.0, 240.0], stazioni: &["bracciano"], webcam: "https://centrosurfbracciano.it/meteo/", surf: None,
        ideali: &[Vicino { n: "tramontana", d: 0.0 }, Vicino { n: "grecale", d: 45.0 }], attenzione: &[] },
];

const WG_VIGNA: i64 = 71531; // windguru.cz/71531

const SPOT_34: usize = 8;
static SOLO_34: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Spot attivi: tutti, oppure solo quelli della 3.4 nel confronto (--solo-cache).
pub fn spot() -> &'static [Spot] {
    if SOLO_34.load(std::sync::atomic::Ordering::Relaxed) { &SPOT[..SPOT_34] } else { SPOT }
}

/// Spot sui laghi: niente onde ne' temperatura del mare.
const LAGHI: &[&str] = &["vigna"];

pub fn lago(s: &Spot) -> bool {
    LAGHI.contains(&s.id)
}

/// Spot al mare, per la richiesta di acqua e onde.
pub fn spot_mare() -> Vec<&'static Spot> {
    spot().iter().filter(|s| !lago(s)).collect()
}

/// Spot sui laghi: l'acqua si stima dall'aria.
pub fn spot_laghi() -> Vec<&'static Spot> {
    spot().iter().filter(|s| lago(s)).collect()
}

const STAZIONI_34: usize = 11;

/// Stazioni attive: tutte, oppure solo quelle della 3.4 nel confronto (--solo-cache).
pub fn stazioni_attive() -> &'static [Stazione] {
    if SOLO_34.load(std::sync::atomic::Ordering::Relaxed) { &STAZIONI[..STAZIONI_34] } else { STAZIONI }
}

pub fn solo_spot_34(si: bool) {
    SOLO_34.store(si, std::sync::atomic::Ordering::Relaxed);
}

// tipo = come leggere la pagina; fresco = minuti oltre i quali la lettura e' vecchia. (Lettori: fase 2.)
pub static STAZIONI: &[Stazione] = &[
    Stazione { id: "rdt_civitavecchia", tipo: "rdt", nome: "Riva di Traiano", etichetta: "Stazione", url: "https://www.teoxonline.com/teox-utils/netsense-staz-ptrt/api/summary_01.php", referer: "https://www.rivaditraiano.com/meteo-obs/", pagina: "https://www.rivaditraiano.com/meteo-obs/", fresco: 30 },
    Stazione { id: "msn", tipo: "msn", nome: "Free Kiter MSN", etichetta: "Stazione", url: "https://stazioni3.soluzionimeteo.it/freekitermsn/homepage/blocks/gauges/updater.php", referer: "https://stazioni3.soluzionimeteo.it/freekitermsn/", pagina: "https://stazioni3.soluzionimeteo.it/freekitermsn/", fresco: 30 },
    Stazione { id: "passoscuro", tipo: "wind24", nome: "Passoscuro (Wind24)", etichetta: "Wind24 Passoscuro", url: "https://www.wind24.it/passoscuro", referer: "https://www.wind24.it/", pagina: "https://www.wind24.it/passoscuro", fresco: 30 },
    Stazione { id: "fregene", tipo: "weathercloud", nome: "Vela Sporting Club Fregene", etichetta: "Stazione", url: "https://app.weathercloud.net/device/values?code=3042235486", referer: "https://app.weathercloud.net/device/sticker/3042235486", pagina: "https://www.velasportingclub.it/pages/la-nostra-stazione-meteo", fresco: 30 },
    Stazione { id: "lni_ostia", tipo: "lni", nome: "Lega Navale Ostia", etichetta: "Stazione", url: "https://anemometro.leganavaleostia.it:8080/data/clientraw.txt", referer: "https://www.leganavaleostia.it/situazione-attuale/", pagina: "https://www.leganavaleostia.it/situazione-attuale/", fresco: 30 },
    Stazione { id: "torvaianica", tipo: "wind24", nome: "Torvaianica (Wind24)", etichetta: "Wind24", url: "https://www.wind24.it/torvaianica", referer: "https://www.wind24.it/", pagina: "https://www.wind24.it/torvaianica", fresco: 30 },
    Stazione { id: "anzio_levante", tipo: "vedetta", nome: "Anzio Riviera di Levante (Vedetta)", etichetta: "Stazione Levante", url: "https://vedetta.org/image-client/marina-di-nettuno/meteo/dati_meteo.html", referer: "https://vedetta.org/webcam/italia/lazio/roma/anzio-levante/", pagina: "https://vedetta.org/webcam/italia/lazio/roma/anzio-levante/", fresco: 30 },
    Stazione { id: "anzio_porto", tipo: "meteolazio", nome: "Anzio Porto (Meteo Lazio)", etichetta: "Stazione porto", url: "https://www.meteoregionelazio.it/rete/stazione.php?id=RM-050", referer: "https://www.meteoregionelazio.it/rete/", pagina: "https://www.meteoregionelazio.it/rete/stazione.php?id=RM-050", fresco: 60 },
    Stazione { id: "latina_lido", tipo: "vedetta", nome: "Lido di Latina (Vedetta)", etichetta: "Stazione Lido", url: "https://vedetta.org/image-client/lido-di-latina/meteo/dati_meteo.html", referer: "https://vedetta.org/webcam/italia/lazio/latina/lido-di-latina/", pagina: "https://vedetta.org/webcam/italia/lazio/latina/lido-di-latina/", fresco: 30 },
    Stazione { id: "latina_sud", tipo: "meteotemplate", nome: "Latina Sud (Soluzioni Meteo)", etichetta: "Stazione Latina Sud", url: "https://stazioni3.soluzionimeteo.it/latina/homepage/blocks/modernWind/data.php", referer: "https://stazioni3.soluzionimeteo.it/latina/indexDesktop.php", pagina: "https://stazioni3.soluzionimeteo.it/latina/indexDesktop.php", fresco: 30 },
    Stazione { id: "saltodifondi", tipo: "wind24", nome: "Salto di Fondi (Wind24)", etichetta: "Wind24", url: "https://www.wind24.it/saltodifondi", referer: "https://www.wind24.it/", pagina: "https://www.wind24.it/saltodifondi", fresco: 30 },
    // Stazioni nuove della 4.x: sempre in fondo (il confronto con la 3.4 usa solo le prime 11).
    Stazione { id: "bracciano", tipo: "meteolazio", nome: "Bracciano Lungolago (Meteo Lazio)", etichetta: "Stazione Lungolago", url: "https://www.meteoregionelazio.it/rete/stazione.php?id=RM-074", referer: "https://www.meteoregionelazio.it/rete/", pagina: "https://www.meteoregionelazio.it/rete/stazione.php?id=RM-074", fresco: 30 },
];

pub fn stazione(id: &str) -> Option<&'static Stazione> {
    STAZIONI.iter().find(|s| s.id == id)
}

pub fn url_windguru(s: &Spot) -> String {
    format!("https://micro.windguru.cz/?s={}", s.wg)
}
