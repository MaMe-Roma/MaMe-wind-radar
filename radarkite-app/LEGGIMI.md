# Radar kite 4.1.2

Novità: sul Mac l'app non risulta più "danneggiata" (4.1.2); il PDF per il telefono si crea anche quando Edge o Chrome ci mettono qualche secondo (4.1.1).

Radar kite mostra il vento previsto (Windguru e un secondo modello) e quello delle stazioni sugli spot da kite del Lazio: giorni firing, meteo, onde, tempi di viaggio e un diario delle uscite.
Autore @MadHatterMatt · idee e suggerimenti: arch.mattia.merusi+radarkite@gmail.com

## Installare
- **Windows**: scarica il file che finisce con `x64-setup.exe` e aprilo. Se compare "Windows ha protetto il PC": *Ulteriori informazioni* → *Esegui comunque* (l'app non è firmata da Microsoft).
- **Mac**: scarica il `.dmg` del tuo Mac (`aarch64` = Apple Silicon M1–M4, `x64` = Intel) e trascina Radar kite in **Applicazioni** (serve anche per l'aggiornamento automatico). La prima volta: Impostazioni di Sistema → Privacy e sicurezza → in fondo *Apri comunque* (oppure clic destro sull'app → *Apri*). Se il Mac dice che è "danneggiata": premi *Annulla*, poi nel Terminale incolla `xattr -cr "/Applications/Radar kite.app"` e premi Invio.

## Primo avvio
- ⏰ In cima alla pagina scegli se Radar kite si aggiorna da solo anche ad app chiusa (per avere sempre aggiornato il PDF sul telefono), da che ora a che ora e ogni quanto.
- 🚗 Partenza: scrivi casa o lavoro per i tempi di viaggio (finché non lo fai si parte da Piramide Ostiense, Roma).
- 📱 Copia per il telefono: scegli la cartella Dropbox, Drive, OneDrive o iCloud dove mettere il PDF. Sul Mac il PDF lo fa Google Chrome.
- Se sul Desktop c'era Radar kite 3.4 (Windows), al primo avvio la 4.1 copia uscite, storico e casa/lavoro.

## Aggiornamenti
All'avvio l'app controlla se c'è una versione nuova e chiede se installarla.

## I tuoi dati
Restano sul tuo computer (Windows: `%APPDATA%\Radar kite`; Mac: `Libreria/Application Support/Radar kite`). Ognuno scarica i dati per sé: niente viene pubblicato.
