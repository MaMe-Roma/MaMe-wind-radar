"""Storico e diario finti (dal lunedi' a ieri) per confrontare giorni passati e diario di 3.4 e 4.0.
Uso: python storico_finto.py AAAA-MM-GG cartella_dati [cartella_dati ...]   (dati inventati, nessuna lettura vera)
"""
import datetime
import os
import sys

oggi = datetime.date.fromisoformat(sys.argv[1])
lunedi = oggi - datetime.timedelta(days=oggi.weekday())
stazioni = [('rdt_civitavecchia', 'sag'), ('msn', 'msn'), ('passoscuro', 'msn'), ('fregene', 'fregene'), ('lni_ostia', 'ostia'), ('torvaianica', 'torva'),
            ('anzio_levante', 'anzio'), ('anzio_porto', 'anzio'), ('latina_lido', 'latina'), ('saltodifondi', 'fondi')]
spot = ['sag', 'msn', 'fregene', 'ostia', 'torva', 'anzio', 'latina', 'fondi']
lett, prev, diario = {}, {}, []
g = lunedi
k = 0
while g < oggi:
    m = g.strftime('%Y-%m')
    for i, (st, sp) in enumerate(stazioni):
        for h in range(7, 20):
            for mi in (5, 35):
                w = 6 + ((h * 7 + i * 3 + k * 5 + mi) % 19) + (0.3 if mi == 35 else 0)
                gu = '' if (h + i) % 5 == 0 else str(round(w + 4 + (h % 3), 1))
                d = (200 + h * 9 + i * 15 + k * 30) % 360
                lett.setdefault(m, []).append(f'{g} {h:02d}:{mi:02d};{st};{sp};{w:g};{gu};{d}')
    for j, sp in enumerate(spot):
        for ant in (0, 1):
            if ant == 0 and (j + k) % 3 == 0:
                continue  # a volte manca la previsione del giorno: si usa quella del giorno prima
            for f in ('wg', 'om'):
                for h in range(6, 21):
                    w = 7 + ((h * 5 + j * 4 + k * 3 + ant) % 17)
                    w = w if f == 'wg' else round(w * 0.93 + 0.4, 1)
                    gu = str(w + 5) if f == 'wg' else str(round(w * 1.4, 1))
                    prev.setdefault(m, []).append(f'{g};{sp};{ant};{f};{h};{w:g};{gu};{(190 + h * 11 + j * 20) % 360};07:{10 + k:02d}')
    diario.append(f'{g};{14 + k % 3}:30;{spot[k % len(spot)]};{60 + 15 * k};{9 + k}' + (';note di prova' if k % 2 else ';prova;4/3;SO'))
    k += 1
    g += datetime.timedelta(days=1)
for cart in sys.argv[2:]:
    os.makedirs(cart, exist_ok=True)
    for m, r in lett.items():
        open(os.path.join(cart, f'letture-{m}.csv'), 'w', newline='').write('data;stazione;spot;vento;raffica;gradi\r\n' + '\r\n'.join(r) + '\r\n')
    for m, r in prev.items():
        open(os.path.join(cart, f'previsioni-{m}.csv'), 'w', newline='').write('giorno;spot;anticipo;fonte;ora;vento;raffica;gradi;salvata\r\n' + '\r\n'.join(r) + '\r\n')
    if diario:
        open(os.path.join(cart, 'diario.csv'), 'w', newline='').write('data;ora;spot;minuti;vela;note\r\n' + '\r\n'.join(diario) + '\r\n')
print(f'storico finto: {(oggi - lunedi).days} giorni, {len(diario)} uscite')
