"""Legge le stazioni del vento di Wind Radar e salva l'ultima lettura in stazioni.json.

Gira su GitHub Actions (vedi .github/workflows/stazioni.yml). Solo libreria standard.
Formato di uscita uguale al dataset "stazioni" della dashboard Wind Radar:
{"rows": [{id, stazione, spot, letto_alle, wind_kn, gust_kn, dir, deg, offline, fonte}], "meta": [...]}
"""
import json
import sys
import urllib.request
from datetime import datetime, timezone
from zoneinfo import ZoneInfo

ROMA = ZoneInfo("Europe/Rome")
PUNTI = ["N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE",
         "S", "SSW", "SW", "WSW", "W", "WNW", "NW", "NNW"]
UA = "Mozilla/5.0 (compatible; wind-radar-stazioni; +https://github.com/MaMe-Roma/mame-wind-radar)"

MSN_URL = "https://stazioni3.soluzionimeteo.it/freekitermsn/homepage/blocks/gauges/updater.php"
LNI_URL = "https://anemometro.leganavaleostia.it:8080/data/clientraw.txt"
RDT_URL = "https://www.teoxonline.com/teox-utils/netsense-staz-ptrt/api/summary_01.php"


def comp(deg):
    return PUNTI[round((float(deg) % 360) / 22.5) % 16]


def get(url, referer):
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Referer": referer, "Cache-Control": "no-cache"})
    with urllib.request.urlopen(req, timeout=20) as r:
        return r.read().decode("utf-8", "replace")


def iso(dt):
    return dt.strftime("%Y-%m-%dT%H:%M:%S")


def num(v):
    return round(float(v), 1)


def msn(testo):
    d = json.loads(testo)
    letto = datetime.fromtimestamp(int(d["DateTime"]), tz=timezone.utc).astimezone(ROMA)
    deg = num(d["B"])
    return {"id": "msn", "stazione": "Free Kiter MSN", "spot": "Marina di San Nicola",
            "letto_alle": iso(letto), "wind_kn": num(d["W"]), "gust_kn": num(d["G"]),
            "dir": comp(deg), "deg": deg, "offline": 1 if str(d.get("offline", 0)) == "1" else 0,
            "fonte": "https://stazioni3.soluzionimeteo.it/freekitermsn/"}


def lni(testo):
    f = testo.split()
    if len(f) < 75 or f[0] != "12345":
        raise ValueError("clientraw.txt non riconosciuto")
    g, m, a = (int(x) for x in f[74].split("/"))
    letto = datetime(a, m, g, int(f[29]), int(f[30]), int(f[31]))
    deg = num(f[3])
    return {"id": "lni_ostia", "stazione": "Lega Navale Ostia", "spot": "Ostia",
            "letto_alle": iso(letto), "wind_kn": num(f[1]), "gust_kn": num(f[2]),
            "dir": comp(deg), "deg": deg, "offline": 0,
            "fonte": "https://www.leganavaleostia.it/situazione-attuale/"}


def rdt(testo):
    voci = [v for v in json.loads(testo) if isinstance(v, dict) and v.get("b_type") == "last_obs"]
    per = {v.get("label", ""): v for v in voci}
    vento = next(v for k, v in per.items() if k.startswith("Velocit") and "vento" in k)
    raffica = per["Raffica vento"]
    direz = per["Direzione vento"]
    letto = datetime.strptime(vento["time"], "%d/%m/%Y %H:%M")
    deg = num(direz["value"])
    return {"id": "rdt_civitavecchia", "stazione": "Riva di Traiano", "spot": "Civitavecchia Sant'Agostino",
            "letto_alle": iso(letto), "wind_kn": num(vento["value"]), "gust_kn": num(raffica["value"]),
            "dir": comp(deg), "deg": deg, "offline": 0,
            "fonte": "https://www.rivaditraiano.com/meteo-obs/"}


STAZIONI = [
    ("Free Kiter MSN", MSN_URL, "https://stazioni3.soluzionimeteo.it/freekitermsn/", msn),
    ("Lega Navale Ostia", LNI_URL, "https://www.leganavaleostia.it/situazione-attuale/", lni),
    ("Riva di Traiano", RDT_URL, "https://www.rivaditraiano.com/meteo-obs/", rdt),
]


def main(uscita="stazioni.json"):
    adesso = datetime.now(ROMA)
    try:
        with open(uscita, encoding="utf-8") as fh:
            prima = {r["id"]: r for r in json.load(fh).get("rows", [])}
    except (OSError, ValueError):
        prima = {}
    nuove, errori = {}, []
    for nome, url, referer, leggi in STAZIONI:
        try:
            r = leggi(get(url, referer))
            nuove[r["id"]] = r
        except Exception as e:  # una stazione rotta non deve fermare le altre
            errori.append(f"{nome}: {type(e).__name__}: {e}"[:200])
    if not nuove:
        print("Nessuna stazione letta, file lasciato com'era")
        for e in errori:
            print("  ", e)
        return 0
    # Una stazione che stavolta non ha risposto resta con la sua lettura vecchia:
    # chi legge il file controlla comunque che "letto_alle" sia recente.
    righe = list({**prima, **nuove}.values())
    meta = [
        {"label": "Come", "value": "Letto in automatico da GitHub Actions (MaMe-Roma/mame-wind-radar) alle "
                                    + adesso.strftime("%H:%M del %d/%m/%Y") + ": lettura salvata, non in diretta."},
        {"label": "Letto", "value": iso(adesso)},
        {"label": "Raffica", "value": "MSN e Lega Navale: raffica attuale. Riva di Traiano: raffica dell'ultima osservazione."},
    ]
    if errori:
        meta.append({"label": "Errori", "value": " · ".join(errori)})
    with open(uscita, "w", encoding="utf-8") as fh:
        json.dump({"rows": righe, "meta": meta}, fh, ensure_ascii=False, indent=1)
    print(f"{len(nuove)} stazioni lette, {len(errori)} errori")
    for e in errori:
        print("  ", e)
    return 0


if __name__ == "__main__":
    sys.exit(main(*sys.argv[1:]))
