# MaMe-wind-radar

Legge in automatico le stazioni del vento usate dalla dashboard **Wind Radar** e salva l'ultima lettura in [`stazioni.json`](stazioni.json).

- **Stazioni:** Free Kiter MSN (Marina di San Nicola), Lega Navale Ostia, Riva di Traiano (Civitavecchia).
- **Quando:** ogni 15 minuti dalle 6 alle 20 circa, ora italiana. Il programma si può anche lanciare a mano: scheda *Actions*, poi *Stazioni vento*, poi *Run workflow*.
- **Formato:** `{"rows": [...], "meta": [...]}`. Ogni riga contiene `id`, `stazione`, `spot`, `letto_alle` (ora italiana), `wind_kn`, `gust_kn`, `dir`, `deg`, `offline` e `fonte`. Se una stazione non risponde, resta la sua lettura precedente: controlla sempre `letto_alle`.

Il codice è in [`stazioni.py`](stazioni.py) e usa solo la libreria standard di Python. L'automazione è in [`.github/workflows/stazioni.yml`](.github/workflows/stazioni.yml).

I dati sono quelli pubblicati dalle stazioni sui loro siti, letti a scopo personale.
