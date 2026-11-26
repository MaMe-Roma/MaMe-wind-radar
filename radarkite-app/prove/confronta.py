"""Confronto 3.4 / 4.0: legge l'oggetto D dai due report.html e controlla che i numeri della fase 1 siano uguali.

Uso: python confronta.py report-3.4.html report-4.0.html
Fasi 1 e 2: previsioni Windguru, firing, meteo e secondo modello, onde, acqua, stazioni, vento adesso, partenza e
tempi di viaggio. Non si confrontano (fase 3) storico, diario, copia per il telefono, ne' i secondi di risposta delle
stazioni. Degli errori conta che ci siano in tutte e due. Nel registro solo i nomi dei campi diversi, non i dati.
"""
import json
import re
import sys


def leggi(file):
    h = open(file, encoding='utf-8-sig').read()
    m = re.search(r'var D = (.*?);\r?\n\(function', h, re.S)
    if not m:
        sys.exit(f'oggetto D non trovato in {file}')
    return json.loads(m.group(1))


diff, contati = [], [0]


def uguale(a, b, via):
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                diff.append(f'{via}.{k}: c\'e\' solo nella {"3.4" if k in a else "4.0"}')
                continue
            uguale(a[k], b[k], f'{via}.{k}')
        return
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            diff.append(f'{via}: {len(a)} elementi nella 3.4, {len(b)} nella 4.0')
        for i, (x, y) in enumerate(zip(a, b)):
            uguale(x, y, f'{via}[{i}]')
        return
    contati[0] += 1
    if isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(a, bool) and not isinstance(b, bool):
        if abs(a - b) > 1e-9:
            diff.append(f'{via}: numero diverso')
        return
    if a != b:
        diff.append(f'{via}: valore diverso')


def main():
    a, b = leggi(sys.argv[1]), leggi(sys.argv[2])
    for k in ('creato', 'soglia', 'oreMin', 'giorni'):
        uguale(a.get(k), b.get(k), k)
    for k in ('omErrore', 'mareErrore'):
        if bool(a.get(k)) != bool(b.get(k)):
            diff.append(f'{k}: errore solo in una delle due')
    if len(a['spot']) != len(b['spot']):
        diff.append('numero di spot diverso')
    firing = 0
    for sa, sb in zip(a['spot'], b['spot']):
        via = f'spot[{sa["id"]}]'
        for k in ('id', 'nome', 'breve', 'mappa', 'lon', 'lat', 'lab', 'wg', 'webcam', 'surf', 'ok', 'terra', 'stazioni', 'wgSpot', 'init', 'acqua', 'ondeOra', 'giorni'):
            uguale(sa.get(k), sb.get(k), f'{via}.{k}')
        if bool(sa.get('errore')) != bool(sb.get('errore')):
            diff.append(f'{via}.errore: Windguru letto solo da una delle due')
        uguale(sa.get('adesso'), sb.get('adesso'), f'{via}.adesso')
        uguale(sa.get('viaggio'), sb.get('viaggio'), f'{via}.viaggio')
        firing += sum(1 for g in sa.get('giorni', []) if g.get('firing'))
    for xa, xb in zip(a['stazioni'], b['stazioni']):
        via = f'stazioni[{xa["id"]}]'
        if bool(xa.get('errore')) != bool(xb.get('errore')):
            diff.append(f'{via}.errore: letta solo da una delle due')
            continue
        for k in sorted((set(xa) | set(xb)) - {'lenta', 'aggiornato', 'errore'}):
            uguale(xa.get(k), xb.get(k), f'{via}.{k}')
    pa, pb = a.get('partenza', {}), b.get('partenza', {})
    for k in sorted((set(pa) | set(pb)) - {'errore'}):
        uguale(pa.get(k), pb.get(k), f'partenza.{k}')
    if bool(pa.get('errore')) != bool(pb.get('errore')):
        diff.append('partenza.errore: tempi di viaggio calcolati solo da una delle due')
    stOk = sum(1 for x in a['stazioni'] if not x.get('errore'))
    letti = sum(1 for s in a['spot'] if not s.get('errore'))
    print(f'Spot con Windguru letto: {letti} di {len(a["spot"])}; giorni firing (3.4): {firing}; '
          f'Open-Meteo: {"errore" if a.get("omErrore") else "ok"}; mare: {"errore" if a.get("mareErrore") else "ok"}; '
          f'stazioni lette: {stOk} di {len(a["stazioni"])}; tempi di viaggio: {"errore" if pa.get("errore") else "ok"}')
    print(f'Valori confrontati: {contati[0]}; differenze: {len(diff)}')
    for d in diff[:60]:
        print('  - ' + d)
    sys.exit(1 if diff or not letti else 0)


main()
