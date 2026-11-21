<#
  Radar kite - versione locale per il PC di Mattia.
  Legge le previsioni di Windguru (pagine "micro", solo testo) e le stazioni del vento,
  applica le regole firing di ogni spot e apre un report nel browser.
  Avvio: doppio clic su "Avvia Radar kite.bat" nella stessa cartella.
#>
param([switch]$NonAprire, [string]$SalvaGrezzi = '', [string]$Link = '', [switch]$Automatico, [string]$OmProva = '', [switch]$Nascosto, [switch]$GiaAperta)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$VERSIONE = '3.1.1'
$DATA_VERSIONE = '21/11/2026'
$AUTORE = '@MadHatterMatt'

# ---------------- Regole (modificabili) ----------------
$SOGLIA = 15          # nodi medi minimi
$ORE_MIN = 2          # ore di fila sopra soglia
$GIORNI = 8           # oggi + 7 giorni (fino allo stesso giorno della settimana prossima)
$VIAGGIO_ORE = 1      # per oggi conta solo il vento da un'ora dopo l'avvio

# Spot da nord a sud. ok = direzioni buone (gradi da cui arriva il vento), terra = vento da terra (mai firing);
# tutto il resto e' side-off. Fonti kite (Kite and Windsurfing Guide): Sant'Agostino top con scirocco; Torvaianica come Ostia;
# Salto di Fondi termica da ovest nel pomeriggio (kite a Sant'Anastasia); Fregene come Maccarese, maestrale e scirocco danno onda.
# Lido di Latina: ponente ideale (vento da O-ONO, mare formato); col maestrale meglio spostarsi verso Capo Portiere.
$SPOT = @(
  @{ id = 'sag'; nome = "Civitavecchia Sant'Agostino"; breve = 'S. Agostino'; lon = 11.73579; lat = 42.17537; lab = @(11, -9, 'start'); wg = 49021; ok = @(130, 320); terra = @(345, 120); stazioni = @('rdt_civitavecchia'); webcam = ''
     ideali = @(@{ n = 'scirocco'; d = 135 }); attenzione = @() },
  @{ id = 'msn'; nome = 'Marina di San Nicola'; breve = 'San Nicola'; lon = 12.12663; lat = 41.92557; lab = @(11, -9, 'start'); wg = 49027; ok = @(170, 300); terra = @(330, 110); stazioni = @('msn', 'passoscuro'); webcam = ''
     surf = 'https://it.surf-forecast.com/breaks/Off-The-Wall_1/forecasts/latest'; ideali = @(@{ n = 'ponente'; d = 270 }); attenzione = @(@{ n = 'libeccio: lavatrice, onde ~2 m'; d = 225 }) },
  # Fregene: stazione del Vela Sporting Club (su Weathercloud e Weather Underground IFIUMI2), webcam sulla loro pagina.
  @{ id = 'fregene'; nome = 'Fregene'; breve = 'Fregene'; lon = 12.18148; lat = 41.86864; lab = @(11, 4, 'start'); wg = 49029; ok = @(165, 320); terra = @(340, 130); stazioni = @('fregene'); webcam = 'https://www.velasportingclub.it/pages/la-nostra-stazione-meteo'
     surf = 'https://it.surf-forecast.com/breaks/Focene/forecasts/latest'; ideali = @(); attenzione = @(@{ n = 'libeccio: lavatrice'; d = 225 }, @{ n = 'maestrale: onda formata'; d = 315 }) },
  @{ id = 'ostia'; nome = 'Ostia'; breve = 'Ostia'; lon = 12.37940; lat = 41.68148; lab = @(11, -9, 'start'); wg = 1518; ok = @(115, 325); terra = @(345, 95); stazioni = @('lni_ostia'); webcam = ''
     surf = 'https://it.surf-forecast.com/breaks/Circoletto-Rome/forecasts/latest'; ideali = @(@{ n = 'scirocco'; d = 135 }, @{ n = 'ponente'; d = 270 }); attenzione = @() },
  @{ id = 'torva'; nome = 'Torvaianica'; breve = 'Torvaianica'; lon = 12.43412; lat = 41.64355; lab = @(11, 4, 'start'); wg = 49033; ok = @(115, 325); terra = @(345, 95); stazioni = @('torvaianica'); webcam = ''
     surf = 'https://it.surf-forecast.com/breaks/Circoletto-Rome/forecasts/latest'; ideali = @(@{ n = 'scirocco'; d = 135 }, @{ n = 'ponente'; d = 270 }); attenzione = @() },
  @{ id = 'anzio'; nome = 'Anzio Levante'; breve = 'Anzio Levante'; mappa = 'Anzio'; lon = 12.63555; lat = 41.45040; lab = @(8, -12, 'start'); wg = 49035; ok = @(40, 230); terra = @(250, 20); stazioni = @('anzio_levante', 'anzio_porto'); webcam = 'https://vedetta.org/webcam/italia/lazio/roma/anzio-levante/'
     surf = 'https://it.surf-forecast.com/breaks/Marinaretti-Anzio/forecasts/latest'; ideali = @(@{ n = 'libeccio'; d = 225 }, @{ n = 'grecale'; d = 45 }); attenzione = @() },
  # Lido di Latina: tra Foce Verde e Capo Portiere (scuola Ultimate Kiteboarding). Windguru non ha uno spot qui: uso Foce Verde, stessa baia.
  @{ id = 'latina'; nome = 'Lido di Latina'; breve = 'Lido di Latina'; mappa = 'Latina'; lon = 12.87552; lat = 41.40429; lab = @(11, 4, 'start'); wg = 49036; ok = @(120, 315); terra = @(335, 100); stazioni = @('latina_lido', 'latina_sud'); webcam = 'https://vedetta.org/webcam/italia/lazio/latina/lido-di-latina/'
     ideali = @(@{ n = 'ponente'; d = 270 }); attenzione = @(@{ n = 'maestrale: vai a Capo Portiere'; d = 315 }) },
  @{ id = 'fondi'; nome = 'Salto di Fondi'; breve = 'Salto di Fondi'; lon = 13.31390; lat = 41.29504; lab = @(11, 4, 'start'); wg = 49042; ok = @(110, 280); terra = @(300, 80); stazioni = @('saltodifondi'); webcam = ''
     ideali = @(@{ n = 'ponente'; d = 270 }); attenzione = @() }
)

# tipo = come leggere la pagina; fresco = minuti oltre i quali la lettura e' vecchia.
$STAZIONI = @(
  @{ id = 'rdt_civitavecchia'; tipo = 'rdt'; nome = 'Riva di Traiano'; etichetta = 'Stazione'; url = 'https://www.teoxonline.com/teox-utils/netsense-staz-ptrt/api/summary_01.php'; ref = 'https://www.rivaditraiano.com/meteo-obs/'; pagina = 'https://www.rivaditraiano.com/meteo-obs/'; fresco = 30 },
  @{ id = 'msn'; tipo = 'msn'; nome = 'Free Kiter MSN'; etichetta = 'Stazione'; url = 'https://stazioni3.soluzionimeteo.it/freekitermsn/homepage/blocks/gauges/updater.php'; ref = 'https://stazioni3.soluzionimeteo.it/freekitermsn/'; pagina = 'https://stazioni3.soluzionimeteo.it/freekitermsn/'; fresco = 30 },
  @{ id = 'passoscuro'; tipo = 'wind24'; nome = 'Passoscuro (Wind24)'; etichetta = 'Wind24 Passoscuro'; url = 'https://www.wind24.it/passoscuro'; ref = 'https://www.wind24.it/'; pagina = 'https://www.wind24.it/passoscuro'; fresco = 30 },
  @{ id = 'fregene'; tipo = 'weathercloud'; nome = 'Vela Sporting Club Fregene'; etichetta = 'Stazione'; url = 'https://app.weathercloud.net/device/values?code=3042235486'; ref = 'https://app.weathercloud.net/device/sticker/3042235486'; pagina = 'https://www.velasportingclub.it/pages/la-nostra-stazione-meteo'; fresco = 30 },
  @{ id = 'lni_ostia'; tipo = 'lni'; nome = 'Lega Navale Ostia'; etichetta = 'Stazione'; url = 'https://anemometro.leganavaleostia.it:8080/data/clientraw.txt'; ref = 'https://www.leganavaleostia.it/situazione-attuale/'; pagina = 'https://www.leganavaleostia.it/situazione-attuale/'; fresco = 30 },
  @{ id = 'torvaianica'; tipo = 'wind24'; nome = 'Torvaianica (Wind24)'; etichetta = 'Wind24'; url = 'https://www.wind24.it/torvaianica'; ref = 'https://www.wind24.it/'; pagina = 'https://www.wind24.it/torvaianica'; fresco = 30 },
  @{ id = 'anzio_levante'; tipo = 'vedetta'; nome = 'Anzio Riviera di Levante (Vedetta)'; etichetta = 'Stazione Levante'; url = 'https://vedetta.org/image-client/marina-di-nettuno/meteo/dati_meteo.html'; ref = 'https://vedetta.org/webcam/italia/lazio/roma/anzio-levante/'; pagina = 'https://vedetta.org/webcam/italia/lazio/roma/anzio-levante/'; fresco = 30 },
  @{ id = 'anzio_porto'; tipo = 'meteolazio'; nome = 'Anzio Porto (Meteo Lazio)'; etichetta = 'Stazione porto'; url = 'https://www.meteoregionelazio.it/rete/stazione.php?id=RM-050'; ref = 'https://www.meteoregionelazio.it/rete/'; pagina = 'https://www.meteoregionelazio.it/rete/stazione.php?id=RM-050'; fresco = 60 },
  @{ id = 'latina_lido'; tipo = 'vedetta'; nome = 'Lido di Latina (Vedetta)'; etichetta = 'Stazione Lido'; url = 'https://vedetta.org/image-client/lido-di-latina/meteo/dati_meteo.html'; ref = 'https://vedetta.org/webcam/italia/lazio/latina/lido-di-latina/'; pagina = 'https://vedetta.org/webcam/italia/lazio/latina/lido-di-latina/'; fresco = 30 },
  @{ id = 'latina_sud'; tipo = 'meteotemplate'; nome = 'Latina Sud (Soluzioni Meteo)'; etichetta = 'Stazione Latina Sud'; url = 'https://stazioni3.soluzionimeteo.it/latina/homepage/blocks/modernWind/data.php'; ref = 'https://stazioni3.soluzionimeteo.it/latina/indexDesktop.php'; pagina = 'https://stazioni3.soluzionimeteo.it/latina/indexDesktop.php'; fresco = 30 },
  @{ id = 'saltodifondi'; tipo = 'wind24'; nome = 'Salto di Fondi (Wind24)'; etichetta = 'Wind24'; url = 'https://www.wind24.it/saltodifondi'; ref = 'https://www.wind24.it/'; pagina = 'https://www.wind24.it/saltodifondi'; fresco = 30 }
)

$PUNTI = @('N', 'NNE', 'NE', 'ENE', 'E', 'ESE', 'SE', 'SSE', 'S', 'SSW', 'SW', 'WSW', 'W', 'WNW', 'NW', 'NNW')
$GG = @('dom', 'lun', 'mar', 'mer', 'gio', 'ven', 'sab')
$INV = [Globalization.CultureInfo]::InvariantCulture
$UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) RadarKite/1.0'
$cartella = Split-Path -Parent $MyInvocation.MyCommand.Path
$adesso = Get-Date
$T_START = Get-Date
# Cronometro nel resoconto (dati\ultimo-avvio.txt): secondi dall'avvio a ogni passaggio.
function Write-Tempo([string]$cosa) { Write-Host ('  [{0,5:0.0} s] {1}' -f ((Get-Date) - $script:T_START).TotalSeconds, $cosa) -ForegroundColor DarkGray }

# ---------------- Impostazioni personali e cartella dei dati ----------------
$dirDati = Join-Path $cartella 'dati'
if (-not (Test-Path $dirDati)) { New-Item -ItemType Directory -Path $dirDati | Out-Null }
$dirCache = Join-Path $dirDati 'cache'
if (-not (Test-Path $dirCache)) { New-Item -ItemType Directory -Path $dirCache | Out-Null }
# Avviato senza finestra dal file .bat: quello che il programma scrive va in dati\ultimo-avvio.txt.
if ($Nascosto) { try { Start-Transcript -Path (Join-Path $dirDati 'ultimo-avvio.txt') -Force | Out-Null } catch { } }
$fileImp = Join-Path $cartella 'impostazioni.txt'
$IMP_TESTO = @'
# Radar kite - impostazioni personali. Una per riga: nome = valore. Le righe con # sono note.
#
# Da dove parti per andare agli spot (indirizzo e citta'). Serve per il tempo di viaggio.
# Per essere precisi aggiungi le coordinate tra parentesi, es. Via Roma 1, Roma (41.90000, 12.50000).
casa =
lavoro =
# Quale delle due usare adesso: casa oppure lavoro (si cambia anche dal menu nella pagina).
partenza = casa
#
# si = il programma parte da solo ogni ora dalle 6:30 alle 19:30 (se il PC e' acceso); no = solo a mano.
avvio_automatico = si
#
# Copia PDF del report per il telefono: auto = nella cartella Dropbox se c'e'; no = niente; oppure il percorso di una cartella.
dropbox = auto
#
# Facoltativo: chiave Google Maps Platform (Routes API) per il tempo di viaggio col traffico. Vuoto = solo stima.
google_chiave =
'@
function Read-Impostazioni {
  $imp = @{ casa = ''; lavoro = ''; partenza = 'casa'; avvio_automatico = 'si'; dropbox = 'auto'; google_chiave = '' }
  if (-not (Test-Path $fileImp)) { [IO.File]::WriteAllText($fileImp, $IMP_TESTO, (New-Object Text.UTF8Encoding($true))) }
  foreach ($r in [IO.File]::ReadAllLines($fileImp)) {
    if ($r -match '^\s*#' -or $r -notmatch '=') { continue }
    $kv = $r.Split('=', 2)
    $imp[$kv[0].Trim().ToLower()] = $kv[1].Trim()
  }
  if ($imp.partenza -ne 'lavoro') { $imp.partenza = 'casa' }
  return $imp
}
function Set-Impostazione([string]$nome, [string]$valore) {
  $righe = @([IO.File]::ReadAllLines($fileImp))
  $trovata = $false
  for ($i = 0; $i -lt $righe.Count; $i++) { if ($righe[$i] -match "^\s*$nome\s*=") { $righe[$i] = "$nome = $valore"; $trovata = $true } }
  if (-not $trovata) { $righe += "$nome = $valore" }
  [IO.File]::WriteAllLines($fileImp, [string[]]$righe, (New-Object Text.UTF8Encoding($true)))
}
$IMP = Read-Impostazioni
if (-not $IMP.casa -and -not $Automatico -and -not $Link -and -not $Nascosto -and [Environment]::UserInteractive -and $Host.Name -eq 'ConsoleHost') {
  Write-Host ''
  Write-Host 'Da dove parti di solito per andare agli spot? Serve per il tempo di viaggio.' -ForegroundColor Cyan
  $risposta = Read-Host 'CASA - indirizzo e citta (es. Via Roma 1, Roma), oppure Invio per saltare'
  if ($risposta) {
    $risposta = ($risposta -replace '[\r\n=;#]', ' ').Trim(); Set-Impostazione 'casa' $risposta; $IMP.casa = $risposta
    $r2 = Read-Host 'LAVORO - indirizzo e citta (facoltativo), oppure Invio per saltare'
    if ($r2) { $r2 = ($r2 -replace '[\r\n=;#]', ' ').Trim(); Set-Impostazione 'lavoro' $r2; $IMP.lavoro = $r2 }
  }
}

# Pulsante "Aggiorna" del report: registra per l'utente (non serve essere amministratore) il link radarkite://
# che riavvia questo programma. Se la cartella cambia posto, al prossimo avvio il link si aggiorna da solo.
# Per toglierlo: chiave HKEY_CURRENT_USER\Software\Classes\radarkite nel Registro di sistema.
function Register-LinkAggiorna {
  try {
    $base = 'HKCU:\Software\Classes\radarkite'
    $ps = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $cmd = '"' + $ps + '" -NoProfile -ExecutionPolicy Bypass -File "' + $PSCommandPath + '" -Link "%1"'
    $ora = (Get-ItemProperty -Path "$base\shell\open\command" -ErrorAction SilentlyContinue).'(default)'
    if ($ora -eq $cmd) { return }
    New-Item -Path $base -Value 'URL:Radar kite' -Force | Out-Null
    New-ItemProperty -Path $base -Name 'URL Protocol' -Value '' -PropertyType String -Force | Out-Null
    New-Item -Path "$base\shell\open\command" -Value $cmd -Force | Out-Null
  } catch {
    Write-Host "  (pulsante Aggiorna non attivo: $($_.Exception.Message))" -ForegroundColor Yellow
  }
}
Register-LinkAggiorna
if ($Link -or $Automatico) { $NonAprire = $true }   # avviato dal pulsante o dall'avvio automatico: niente browser

# Download in parallelo: le richieste partono tutte insieme (Start-Scarico) e Get-Testo usa la risposta
# quando arriva. Per le stazioni (-Rigido) si aspetta fino a 60 secondi (la pagina e' gia' aperta), poi "non raggiungibile".
Add-Type -AssemblyName System.Net.Http
$PRE = @{}
$HTTP = $null
$T0 = Get-Date
$TEMPI = @{}
function Start-Scarico([string]$url, [string]$referer, [switch]$Xhr, [switch]$Rigido, [int]$Secondi = 20) {
  # Aggiornamento di un solo spot o stazione: quello che non va riletto viene dalla copia dell'ultimo avvio.
  if ($script:Solo -and -not ($script:FRESCHI -contains $url)) {
    $c = Read-Cache $url
    if ($c -ne $null) { $script:PRE[$url] = @{ testo = $c; rigido = [bool]$Rigido }; return }
    if ($script:STAZIONI | Where-Object { $_.url -eq $url }) { $script:PRE[$url] = @{ errore = 'non letta in questo aggiornamento'; rigido = $true }; return }
  }
  if (-not $script:HTTP) {
    $hh = New-Object System.Net.Http.HttpClientHandler
    $hh.AutomaticDecompression = [System.Net.DecompressionMethods]::GZip
    $script:HTTP = New-Object System.Net.Http.HttpClient($hh)
    $script:HTTP.Timeout = [TimeSpan]::FromSeconds(70)
  }
  $cts = New-Object System.Threading.CancellationTokenSource
  $cts.CancelAfter([TimeSpan]::FromSeconds($Secondi))
  $req = New-Object System.Net.Http.HttpRequestMessage([System.Net.Http.HttpMethod]::Get, $url)
  [void]$req.Headers.TryAddWithoutValidation('User-Agent', $UA)
  if ($referer) { [void]$req.Headers.TryAddWithoutValidation('Referer', $referer) }
  [void]$req.Headers.TryAddWithoutValidation('Cache-Control', 'no-cache')
  if ($Xhr) { [void]$req.Headers.TryAddWithoutValidation('X-Requested-With', 'XMLHttpRequest') }
  $script:PRE[$url] = @{ task = $script:HTTP.SendAsync($req, $cts.Token); rigido = [bool]$Rigido; secondi = $Secondi; stazione = [bool]($script:STAZIONI | Where-Object { $_.url -eq $url }) }
}
function Get-Scaricato([string]$url) {
  $p = $script:PRE[$url]; $script:PRE.Remove($url)
  if ($p.testo -ne $null) { return @{ testo = $p.testo; cache = $true } }
  if ($p.errore) { throw $p.errore }
  try {
    if (-not $p.task.Wait(($p.secondi + 5) * 1000)) { throw "nessuna risposta in $($p.secondi) secondi" }
    $resp = $p.task.Result
    if (-not $resp.IsSuccessStatusCode) { throw "il sito risponde con errore $([int]$resp.StatusCode)" }
    return @{ testo = [Text.Encoding]::UTF8.GetString($resp.Content.ReadAsByteArrayAsync().Result) }
  } catch {
    $e = $_.Exception; while ($e.InnerException) { $e = $e.InnerException }
    $msg = if ($e -is [System.Threading.Tasks.TaskCanceledException] -or $e -is [System.OperationCanceledException]) { "nessuna risposta in $($p.secondi) secondi" } else { $e.Message }
    # Stazioni: l'errore resta (la pagina e' gia' aperta). Open-Meteo e mare: se non e' un'attesa scaduta riprovo una volta.
    if ($p.rigido -and ($p.stazione -or $msg -like 'nessuna risposta*')) { throw $msg }
    return $null
  }
}
# Segna quando risponde ciascuna stazione (secondi dalla partenza delle richieste): si controlla spesso
# mentre il programma lavora, poi Wait-Stazioni aspetta le ultime.
function Update-Tempi {
  $tutte = $true
  foreach ($st in $STAZIONI) {
    $p = $script:PRE[$st.url]
    if (-not $p -or -not $p.task -or $script:TEMPI.ContainsKey($st.id)) { continue }
    if ($p.task.IsCompleted) { $script:TEMPI[$st.id] = [Math]::Round(((Get-Date) - $script:T0).TotalSeconds, 1) } else { $tutte = $false }
  }
  return $tutte
}
function Wait-Stazioni {
  $fine = (Get-Date).AddSeconds(66)
  while (-not (Update-Tempi) -and (Get-Date) -lt $fine) { Start-Sleep -Milliseconds 100 }
}
function Get-Testo([string]$url, [string]$referer, [switch]$Xhr) {
  # Se il download e' gia' partito uso quello; se e' fallito (e non e' una stazione) riprovo qui sotto.
  if ($script:PRE.ContainsKey($url)) { $pr = Get-Scaricato $url; if ($pr) { if (-not $pr.cache) { Save-Cache $url $pr.testo }; return $pr.testo } }
  $intestazioni = @{ Referer = $referer; 'Cache-Control' = 'no-cache' }
  if ($Xhr) { $intestazioni['X-Requested-With'] = 'XMLHttpRequest' }
  for ($n = 1; ; $n++) {
    try {
      $r = Invoke-WebRequest -Uri $url -UseBasicParsing -UserAgent $UA -Headers $intestazioni -TimeoutSec 20
      break
    } catch {
      if ($n -ge 2) { throw }
      Start-Sleep -Seconds 3
    }
  }
  # Decodifica sempre in UTF-8, cosi' le lettere accentate non si rovinano.
  $testo = if ($r.RawContentStream) { [Text.Encoding]::UTF8.GetString($r.RawContentStream.ToArray()) } else { [string]$r.Content }
  Save-Cache $url $testo
  return $testo
}
# Copia dell'ultima risposta di ogni sito (dati\cache): serve per aggiornare un solo spot o una sola stazione.
function Get-FileCache([string]$url) {
  $sha = [Security.Cryptography.SHA1]::Create()
  $hx = -join ($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($url)) | ForEach-Object { $_.ToString('x2') })
  return (Join-Path $script:dirCache ($hx + '.txt'))
}
function Save-Cache([string]$url, [string]$testo) { try { [IO.File]::WriteAllText((Get-FileCache $url), $testo, (New-Object Text.UTF8Encoding($false))) } catch { } }
function Read-Cache([string]$url) { try { $f = Get-FileCache $url; if (Test-Path $f) { return [IO.File]::ReadAllText($f) } } catch { }; return $null }

function Get-Comp([double]$deg) { return $PUNTI[[int][Math]::Round(((($deg % 360) + 360) % 360) / 22.5) % 16] }

function Get-Classe($s, [double]$deg) {
  if ($deg -ge $s.ok[0] -and $deg -le $s.ok[1]) { return 'ok' }
  if ($deg -ge $s.terra[0] -or $deg -le $s.terra[1]) { return 'terra' }
  return 'side'
}

function Get-Vicini($lista, [double[]]$gradi, [switch]$SoloBuoni, $s) {
  $out = @()
  foreach ($v in $lista) {
    foreach ($g in $gradi) {
      $diff = [Math]::Abs($g - $v.d) % 360
      if ([Math]::Min($diff, 360 - $diff) -le 22.5 -and (-not $SoloBuoni -or (Get-Classe $s $g) -eq 'ok')) { $out += $v.n; break }
    }
  }
  return $out
}

function Get-Fascia([datetime]$d) {
  $md = $d.Month * 100 + $d.Day
  if ($md -ge 1015 -or $md -le 331) { return @(8, 16) } else { return @(8, 19) }
}

# ---------------- Windguru ----------------
function Read-Windguru($s) {
  $html = Get-Testo "https://micro.windguru.cz/?s=$($s.wg)" 'https://micro.windguru.cz/'
  $t = [Net.WebUtility]::HtmlDecode(($html -replace '<[^>]+>', ''))
  if ($SalvaGrezzi) { [IO.File]::WriteAllText((Join-Path $SalvaGrezzi "windguru_$($s.wg).txt"), $t) }
  $info = @{ spot = ''; init = ''; utc = 1; righe = @() }
  if ($t -match '(?m)^\s*\S.*? - ([^,\r\n]+),\s*lat:') { $info.spot = $Matches[1].Trim() }
  if ($t -match 'init:\s*([0-9-]+ \d{2}) UTC') { $info.init = $Matches[1] }
  if ($t -match '\(UTC([+-]\d+)\)') { $info.utc = [int]$Matches[1] }
  $oggi = $adesso.Date
  foreach ($riga in ($t -split "`r?`n")) {
    if ($riga -notmatch '^\s*(Mon|Tue|Wed|Thu|Fri|Sat|Sun)\s+(\d{1,2})\.\s+(\d{1,2})h\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)') { continue }
    $giorno = [int]$Matches[2]; $ora = [int]$Matches[3]
    $w = $Matches[4]; $g = $Matches[5]; $dir = $Matches[6]; $deg = $Matches[7]
    if ($w -notmatch '^\d+$') { continue }
    $data = $null
    for ($k = -1; $k -le 20; $k++) { $c = $oggi.AddDays($k); if ($c.Day -eq $giorno) { $data = $c; break } }
    if (-not $data) { continue }
    $utc = [datetime]::SpecifyKind($data.AddHours($ora - $info.utc), 'Utc')
    $loc = $utc.ToLocalTime()
    if ($deg -notmatch '^\d+$') { $deg = [array]::IndexOf($PUNTI, $dir) * 22.5 }
    $info.righe += [pscustomobject]@{ t = $loc; date = $loc.Date; hour = $loc.Hour; wind = [int]$w
      gust = $(if ($g -match '^\d+$') { [int]$g } else { $null }); dir = $dir; deg = [double]$deg }
  }
  return $info
}

function Get-Giorni($s, $righe) {
  $risultati = @()
  for ($k = 0; $k -lt $GIORNI; $k++) {
    $giorno = $adesso.Date.AddDays($k)
    $fascia = Get-Fascia $giorno
    $delGiorno = @($righe | Where-Object { $_.date -eq $giorno } | Sort-Object t)
    $passo = 1
    if ($delGiorno.Count -gt 1) {
      $passo = 24
      for ($i = 1; $i -lt $delGiorno.Count; $i++) { $d = $delGiorno[$i].hour - $delGiorno[$i - 1].hour; if ($d -gt 0 -and $d -lt $passo) { $passo = $d } }
    }
    $inizio = $fascia[0]
    if ($k -eq 0) { $inizio = [Math]::Max($inizio, $adesso.AddHours($VIAGGIO_ORE).Hour) }
    $utili = @($delGiorno | Where-Object { $_.hour -ge $inizio -and $_.hour -le $fascia[1] })
    $migliore = $null; $run = @()
    foreach ($r in ($utili + @($null))) {
      if ($r -and $r.wind -ge $SOGLIA -and (Get-Classe $s $r.deg) -ne 'terra') { $run += $r; continue }
      if ($run.Count -gt 0) {
        $durata = [Math]::Min($run[-1].hour + $passo, $fascia[1] + 1) - $run[0].hour
        $dm = if ($migliore) { [Math]::Min($migliore[-1].hour + $passo, $fascia[1] + 1) - $migliore[0].hour } else { -1 }
        if ($durata -ge $ORE_MIN -and $durata -gt $dm) { $migliore = $run }
      }
      $run = @()
    }
    $res = [ordered]@{ data = $giorno; nome = "$($GG[[int]$giorno.DayOfWeek]) $($giorno.Day)"; oggi = ($k -eq 0); fascia = "$($fascia[0])-$($fascia[1])"
      max = $null; maxRaff = $null; maxDir = ''; firing = $false }
    if ($utili.Count) {
      $top = $utili | Sort-Object wind -Descending | Select-Object -First 1
      $res.max = $top.wind; $res.maxDir = $top.dir
      $rr = @($utili | Where-Object { $_.gust -ne $null })
      if ($rr.Count) { $res.maxRaff = ($rr | Measure-Object gust -Maximum).Maximum }
    }
    if ($migliore) {
      $gradi = @($migliore | ForEach-Object { $_.deg })
      $raff = @($migliore | Where-Object { $_.gust -ne $null })
      $res.firing = $true
      # Con la previsione ogni 3 ore l'ultima riga copre anche le ore dopo: 13-13 diventa 13-16.
      $fineOre = [Math]::Min($migliore[-1].hour + $(if ($passo -gt 1) { $passo } else { 0 }), $fascia[1])
      $res.ore = $(if ($fineOre -gt $migliore[0].hour) { "$($migliore[0].hour)-$fineOre" } else { "$($migliore[0].hour)" })
      $res.durata = [Math]::Min($migliore[-1].hour + $passo, $fascia[1] + 1) - $migliore[0].hour
      $res.min = ($migliore | Measure-Object wind -Minimum).Minimum
      $res.maxf = ($migliore | Measure-Object wind -Maximum).Maximum
      $res.raffica = $(if ($raff.Count) { ($raff | Measure-Object gust -Maximum).Maximum } else { $null })
      $res.dir = (@($migliore | ForEach-Object { $_.dir } | Select-Object -Unique)) -join '/'
      $note = @()
      $res.pericolose = ($res.raffica -ne $null -and $res.raffica -gt 35)
      $res.rafficato = (-not $res.pericolose) -and [bool]@($raff | Where-Object { $_.gust - $_.wind -gt 10 }).Count
      $res.sideoff = [bool]@($migliore | Where-Object { (Get-Classe $s $_.deg) -eq 'side' }).Count
      if ($res.pericolose) { $note += 'raffiche pericolose' } elseif ($res.rafficato) { $note += 'rafficato' }
      if ($res.sideoff) { $note += 'side-off' }
      $id = @(Get-Vicini -lista $s.ideali -gradi $gradi -SoloBuoni -s $s)
      $res.ideale = $(if ($id.Count) { 'ideale con ' + ($id -join ' e ') } else { '' })
      if ($res.ideale) { $note += $res.ideale }
      $at = @(Get-Vicini -lista $s.attenzione -gradi $gradi -s $s)
      $res.attenzione = ($at -join ', ')
      if ($at.Count) { $note += $at }
      $res.note = $note
    }
    $risultati += [pscustomobject]$res
  }
  return $risultati
}

# ---------------- Stazioni ----------------
function Read-Stazione($st) {
  $testo = (Get-Testo $st.url $st.ref -Xhr:($st.tipo -eq 'weathercloud')).TrimStart([char]0xFEFF)
  if ($SalvaGrezzi) { [IO.File]::WriteAllText((Join-Path $SalvaGrezzi "stazione_$($st.id).txt"), $testo) }
  switch ($st.tipo) {
    'msn' {
      $j = $testo | ConvertFrom-Json
      $t = [DateTimeOffset]::FromUnixTimeSeconds([int64]$j.DateTime).LocalDateTime
      return @{ t = $t; wind = [double]$j.W; gust = [double]$j.G; deg = [double]$j.B; offline = ([string]$j.offline -eq '1') }
    }
    'weathercloud' {
      # Weathercloud: JSON con vento medio e raffica in m/s, gradi e ora (epoch).
      $j = $testo | ConvertFrom-Json
      if ($j.epoch -eq $null -or $j.wspd -eq $null) { throw 'dati non trovati' }
      $t = [DateTimeOffset]::FromUnixTimeSeconds([int64]$j.epoch).LocalDateTime
      $g = if ($j.wspdhi -ne $null) { [Math]::Round([double]$j.wspdhi * 1.943844, 1) } else { $null }
      return @{ t = $t; wind = [Math]::Round([double]$j.wspd * 1.943844, 1); gust = $g; deg = [double]$j.wdir; offline = $false }
    }
    'meteotemplate' {
      # Blocco "modernWind" di Meteotemplate: JSON con velocita', raffica, gradi, ora (epoch) e unita' di misura.
      $j = $testo | ConvertFrom-Json
      if (-not $j.ok) { throw "stazione non disponibile: $($j.error)" }
      $k = switch -Regex ([string]$j.unit) { 'km' { 1 / 1.852 } 'm/s|ms' { 1.943844 } 'mph' { 0.868976 } default { 1 } }
      $t = [DateTimeOffset]::FromUnixTimeSeconds([int64]$j.updated).LocalDateTime
      $g = if ($j.gust -ne $null) { [Math]::Round([double]$j.gust * $k, 1) } else { $null }
      return @{ t = $t; wind = [Math]::Round([double]$j.speed * $k, 1); gust = $g; deg = [double]$j.bearing; offline = $false }
    }
    'lni' {
      $f = $testo.Trim() -split '\s+'
      if ($f.Count -lt 75 -or $f[0] -ne '12345') { throw 'clientraw.txt non riconosciuto' }
      $t = [datetime]::ParseExact("$($f[74]) $($f[29]):$($f[30]):$($f[31])", 'd/M/yyyy H:m:s', $INV)
      return @{ t = $t; wind = [double]$f[1]; gust = [double]$f[2]; deg = [double]$f[3]; offline = $false }
    }
    'rdt' {
      $j = ConvertFrom-Json $testo
      $voci = @($j) | Where-Object { $_ -ne $null -and $_.PSObject.Properties['b_type'] -and $_.b_type -eq 'last_obs' }
      $v = $voci | Where-Object { $_.label -like 'Velocit*vento' } | Select-Object -First 1
      $g = $voci | Where-Object { $_.label -eq 'Raffica vento' } | Select-Object -First 1
      $d = $voci | Where-Object { $_.label -eq 'Direzione vento' } | Select-Object -First 1
      $t = [datetime]::ParseExact($v.time, 'dd/MM/yyyy HH:mm', $INV)
      return @{ t = $t; wind = [double]$v.value; gust = [double]$g.value; deg = [double]$d.value; offline = $false }
    }
    'vedetta' {
      # Widget della webcam Vedetta (stessa centralina mostrata come "Anzio Riviera di Levante"); valori gia' in nodi.
      if ($testo -notmatch '(\d{1,2}:\d{2}:\d{2})\s+(\d{1,2}/\d{1,2}/\d{4})') { throw 'ora non trovata' }
      $t = [datetime]::ParseExact("$($Matches[2]) $($Matches[1])", 'd/M/yyyy H:mm:ss', $INV)
      if ($testo -notmatch 'Vento Attuale:[\s\S]*?<big>\s*([\d.,]+)\s*kts') { throw 'vento non trovato' }
      $w = [double]($Matches[1] -replace ',', '.')
      if ($testo -notmatch 'Direzione:\s*<strong>\s*<big>\s*([A-Z]{1,3})\s*</big>') { throw 'direzione non trovata' }
      $sigla = $Matches[1] -replace 'O', 'W'
      if ([array]::IndexOf($PUNTI, $sigla) -lt 0) { throw "direzione non riconosciuta: $sigla" }
      $g = $null
      if ($testo -match 'Raffica MAX:\s*<strong>\s*<big>\s*([\d.,]+)\s*kts') { $g = [double]($Matches[1] -replace ',', '.') }
      return @{ t = $t; wind = $w; gust = $g; deg = [array]::IndexOf($PUNTI, $sigla) * 22.5; offline = $false }
    }
    'wind24' {
      # Pagina Wind24: velocita' media e raffiche in nodi, direzione in gradi nel disegno della bussola.
      if ($testo -match 'momentaneamente[\s\S]{0,120}Sospeso') { throw 'stazione sospesa dal sito (rinnovo)' }
      if ($testo -notmatch 'Meteo del\s*(\d{1,2}/\d{1,2}/\d{4})\s*ore\s*(\d{1,2}:\d{2})') { throw 'ora non trovata' }
      $t = [datetime]::ParseExact("$($Matches[1]) $($Matches[2])", 'd/M/yyyy H:mm', $INV)
      if ($testo -notmatch 'Velocit[^<]{0,14}media(?:&nbsp;|\s)*<font[^>]*>\s*([\d.,]+)') { throw 'vento non trovato' }
      $w = [double]($Matches[1] -replace ',', '.')
      $g = $null
      if ($testo -match 'Raffiche(?:&nbsp;|\s)*<font[^>]*>\s*([\d.,]+)') { $g = [double]($Matches[1] -replace ',', '.') }
      if ($testo -match "createMarker\('odoMap',\s*'(\d+(?:\.\d+)?)',\s*'http") { $deg = [double]$Matches[1] }
      elseif ($testo -match "createMarker\('odoMap',\s*'(\d+(?:\.\d+)?)'") { $deg = [double]$Matches[1] }
      else { throw 'direzione non trovata' }
      return @{ t = $t; wind = $w; gust = $g; deg = $deg; offline = $false }
    }
    'meteolazio' {
      if ($testo -notmatch 'Aggiornamento:\s*<strong>([^<]+)</strong>') { throw 'ora non trovata' }
      $t = [datetime]::ParseExact($Matches[1].Trim(), 'dd/MM/yy HH:mm', $INV)
      if ($testo -notmatch 'mrl-primary-label">Velocit[^<]*</span>\s*<strong[^>]*>([\d.,]+)\s*km/h') { throw 'vento non trovato' }
      $kmh = [double]($Matches[1] -replace ',', '.')
      if ($testo -notmatch 'Direzione<br>\s*<strong>([A-Z]{1,3})</strong>') { throw 'direzione non trovata' }
      $sigla = $Matches[1] -replace 'O', 'W'
      if ([array]::IndexOf($PUNTI, $sigla) -lt 0) { throw "direzione non riconosciuta: $sigla" }
      $raffica = $null
      if ($testo -match 'Ultima raffica</span>\s*<strong[^>]*>([\d.,]+)\s*km/h') { $raffica = [Math]::Round([double]($Matches[1] -replace ',', '.') / 1.852, 1) }
      return @{ t = $t; wind = [Math]::Round($kmh / 1.852, 1); gust = $raffica; deg = [array]::IndexOf($PUNTI, $sigla) * 22.5; offline = $false }
    }
  }
}

# ---------------- Comandi dai pulsanti del report (link radarkite://) ----------------
function Clean([string]$x, [int]$max) { $y = ($x -replace '[\r\n;"<>]', ' ').Trim(); if ($y.Length -gt $max) { $y = $y.Substring(0, $max) }; return $y }
function Get-Parametri([string]$link) {
  $q = @{}
  $parte = ($link -split '\?', 2)
  if ($parte.Count -lt 2) { return $q }
  foreach ($kv in ($parte[1] -split '&')) { $a = $kv -split '=', 2; if ($a.Count -eq 2) { $q[$a[0]] = [Uri]::UnescapeDataString($a[1].Replace('+', ' ')) } }
  return $q
}
$fileDiario = Join-Path $dirDati 'diario.csv'
$forzaViaggi = $false
if ($Link -like 'radarkite://uscita*') {
  # Diario: un'uscita scritta nel modulo del report.
  try {
    $q = Get-Parametri $Link
    $d = Clean $q['d'] 10; $o = Clean $q['o'] 5; $sp = Clean $q['s'] 20; $mi = Clean $q['m'] 4; $v = Clean $q['v'] 12; $no = Clean $q['n'] 140
    if ($d -notmatch '^\d{4}-\d{2}-\d{2}$') { throw 'data non valida' }
    if ($o -and $o -notmatch '^\d{1,2}:\d{2}$') { $o = '' }
    if (-not ($SPOT | Where-Object { $_.id -eq $sp })) { throw 'spot non valido' }
    if ($mi -notmatch '^\d{1,3}$') { $mi = '' }
    if (-not (Test-Path $fileDiario)) { [IO.File]::WriteAllText($fileDiario, "data;ora;spot;minuti;vela;note`r`n", (New-Object Text.UTF8Encoding($false))) }
    [IO.File]::AppendAllText($fileDiario, "$d;$o;$sp;$mi;$v;$no`r`n", (New-Object Text.UTF8Encoding($false)))
    Write-Host "Uscita salvata nel diario: $d $sp" -ForegroundColor Green
  } catch { Write-Host "Uscita non salvata: $($_.Exception.Message)" -ForegroundColor Red }
}
if ($Link -like 'radarkite://partenza*') {
  $q = Get-Parametri $Link
  if ($q['p'] -eq 'casa' -or $q['p'] -eq 'lavoro') { Set-Impostazione 'partenza' $q['p']; $IMP.partenza = $q['p'] }
  $forzaViaggi = $true
}
if ($Link -like 'radarkite://viaggi*') { $forzaViaggi = $true }
$forzaStorico = [bool]($Link -like 'radarkite://storico*')
$Solo = ''
if ($Link -like 'radarkite://spot*') { $q = Get-Parametri $Link; if ($SPOT | Where-Object { $_.id -eq $q['id'] }) { $Solo = 'spot:' + $q['id'] } }
if ($Link -like 'radarkite://stazione*') { $q = Get-Parametri $Link; if ($STAZIONI | Where-Object { $_.id -eq $q['id'] }) { $Solo = 'st:' + $q['id'] } }
if ($Link -like 'radarkite://indirizzi*') {
  # Casa e lavoro scritti nella pagina: li salvo in impostazioni.txt e ricalcolo i tempi.
  $q = Get-Parametri $Link
  foreach ($coppia in @(@('c', 'casa'), @('l', 'lavoro'))) {
    if ($q.ContainsKey($coppia[0])) { $val = Clean $q[$coppia[0]] 120; Set-Impostazione $coppia[1] $val; $IMP[$coppia[1]] = $val }
  }
  if ($q['p'] -eq 'casa' -or $q['p'] -eq 'lavoro') { Set-Impostazione 'partenza' $q['p']; $IMP.partenza = $q['p'] }
  if (-not $IMP[$IMP.partenza]) { $alt = $(if ($IMP.partenza -eq 'casa') { 'lavoro' } else { 'casa' }); if ($IMP[$alt]) { Set-Impostazione 'partenza' $alt; $IMP.partenza = $alt } }
  Write-Host "Partenza salvata: casa = $($IMP.casa) | lavoro = $($IMP.lavoro)" -ForegroundColor Green
  $forzaViaggi = $true
}

# ---------------- Avvio automatico ogni ora (Utilita' di pianificazione di Windows) ----------------
function Register-AvvioAutomatico {
  try {
    # Controllo al massimo una volta al giorno (caricare le funzioni dell'Utilita' di pianificazione richiede qualche secondo).
    $fileAvvio = Join-Path $script:dirCache 'avvio-automatico.txt'
    $firma = "$($IMP.avvio_automatico)|$PSCommandPath"
    if ((Test-Path $fileAvvio) -and (Get-Item $fileAvvio).LastWriteTime -gt (Get-Date).AddDays(-1) -and [IO.File]::ReadAllText($fileAvvio) -eq $firma) { return }
    $nome = 'Radar kite'
    $esistente = Get-ScheduledTask -TaskName $nome -ErrorAction SilentlyContinue
    if ($IMP.avvio_automatico -ne 'si') {
      if ($esistente) { Unregister-ScheduledTask -TaskName $nome -Confirm:$false; Write-Host '  Avvio automatico tolto.' }
      [IO.File]::WriteAllText($fileAvvio, $firma); return
    }
    # Avvio senza finestre: conhost --headless (su Windows 11 -WindowStyle Hidden da solo apre comunque il Terminale).
    $ps = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $con = Join-Path $env:SystemRoot 'System32\conhost.exe'
    $arg = '--headless "' + $ps + '" -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "' + $PSCommandPath + '" -Automatico'
    if (-not (Test-Path $con)) { $con = $ps; $arg = $arg.Substring($arg.IndexOf('-NoProfile')) }
    if ($esistente -and $esistente.Actions[0].Arguments -eq $arg) { [IO.File]::WriteAllText($fileAvvio, $firma); return }
    $azione = New-ScheduledTaskAction -Execute $con -Argument $arg -WorkingDirectory $cartella
    $trigger = New-ScheduledTaskTrigger -Daily -At '06:30'
    $ripeti = New-ScheduledTaskTrigger -Once -At '06:30' -RepetitionInterval (New-TimeSpan -Hours 1) -RepetitionDuration (New-TimeSpan -Hours 13)
    $trigger.Repetition = $ripeti.Repetition
    $opz = New-ScheduledTaskSettingsSet -StartWhenAvailable -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit (New-TimeSpan -Minutes 10) -MultipleInstances IgnoreNew
    Register-ScheduledTask -TaskName $nome -Action $azione -Trigger $trigger -Settings $opz -Description 'Radar kite: previsioni e stazioni ogni ora dalle 6:30 alle 19:30' -Force | Out-Null
    Write-Host '  Avvio automatico attivo: ogni ora dalle 6:30 alle 19:30.' -ForegroundColor Green
    [IO.File]::WriteAllText($fileAvvio, $firma)
  } catch {
    Write-Host "  (avvio automatico non attivato: $($_.Exception.Message))" -ForegroundColor Yellow
  }
}

# ---------------- Open-Meteo: meteo, secondo modello del vento, acqua ----------------
function Get-UrlOpenMeteo {
  $lat = ($SPOT | ForEach-Object { ([double]$_.lat).ToString($INV) }) -join ','
  $lon = ($SPOT | ForEach-Object { ([double]$_.lon).ToString($INV) }) -join ','
  return "https://api.open-meteo.com/v1/forecast?latitude=$lat&longitude=$lon&hourly=temperature_2m,weather_code,precipitation,wind_speed_10m,wind_gusts_10m,wind_direction_10m&wind_speed_unit=kn&timezone=Europe%2FRome&forecast_days=9"
}
function Read-OpenMeteo {
  $url = Get-UrlOpenMeteo
  # -OmProva: legge un file al posto del servizio (solo per le prove).
  $testo = if ($OmProva) { [IO.File]::ReadAllText($OmProva) } else { Get-Testo $url 'https://open-meteo.com/' }
  $j = ConvertFrom-Json $testo
  $lista = @($j)
  $out = @{}
  for ($i = 0; $i -lt $SPOT.Count; $i++) {
    $h = $lista[$i].hourly
    $righe = New-Object System.Collections.ArrayList
    for ($n = 0; $n -lt $h.time.Count; $n++) {
      $t = [datetime]::ParseExact($h.time[$n], "yyyy-MM-dd'T'HH:mm", $INV)
      [void]$righe.Add([pscustomobject]@{ t = $t; date = $t.Date; hour = $t.Hour; temp = $h.temperature_2m[$n]; code = $h.weather_code[$n]; rain = $h.precipitation[$n]
        wind = $h.wind_speed_10m[$n]; gust = $h.wind_gusts_10m[$n]; deg = $h.wind_direction_10m[$n] })
    }
    $out[$SPOT[$i].id] = $righe
  }
  return $out
}
function Get-UrlAcqua {
  # Un punto un po' al largo di ogni spot, verso il mare (centro delle direzioni buone).
  $lat = @(); $lon = @()
  foreach ($s in $SPOT) {
    $c = if ($s.ok[0] -le $s.ok[1]) { ($s.ok[0] + $s.ok[1]) / 2 } else { (($s.ok[0] + $s.ok[1] + 360) / 2) % 360 }
    $lat += ([double]$s.lat + 0.06 * [Math]::Cos($c * [Math]::PI / 180)).ToString('0.000', $INV)
    $lon += ([double]$s.lon + 0.06 * [Math]::Sin($c * [Math]::PI / 180) / 0.745).ToString('0.000', $INV)
  }
  return "https://marine-api.open-meteo.com/v1/marine?latitude=$($lat -join ',')&longitude=$($lon -join ',')&hourly=sea_surface_temperature,wave_height,wave_period,wave_direction,swell_wave_height,swell_wave_period,wind_wave_height,wind_wave_period&timezone=Europe%2FRome&forecast_days=9"
}
function Read-Acqua {
  $j = ConvertFrom-Json (Get-Testo (Get-UrlAcqua) 'https://open-meteo.com/')
  $lista = @($j)
  $out = @{}
  $script:ONDE = @{}
  for ($i = 0; $i -lt $SPOT.Count; $i++) {
    $hh = $lista[$i].hourly
    # Temperatura dell'acqua: media di oggi.
    $v = @($hh.sea_surface_temperature | Select-Object -First 24 | Where-Object { $_ -ne $null })
    $out[$SPOT[$i].id] = $(if ($v.Count) { [Math]::Round(($v | Measure-Object -Average).Average) } else { $null })
    # Onde ora per ora: totale, onda lunga (swell) e onde da vento.
    $righe = New-Object System.Collections.ArrayList
    for ($k = 0; $hh.wave_height -and $k -lt @($hh.time).Count; $k++) {
      if ($hh.wave_height[$k] -eq $null) { continue }
      $t = [datetime]::ParseExact($hh.time[$k], "yyyy-MM-dd'T'HH:mm", $INV)
      [void]$righe.Add([pscustomobject]@{ date = $t.Date; hour = $t.Hour; h = $hh.wave_height[$k]; p = $hh.wave_period[$k]; d = $hh.wave_direction[$k]
        sh = $hh.swell_wave_height[$k]; sp = $hh.swell_wave_period[$k]; wh = $hh.wind_wave_height[$k]; wp = $hh.wind_wave_period[$k] })
    }
    $script:ONDE[$SPOT[$i].id] = $righe
  }
  return $out
}
# Onde di un giorno: l'ora con l'onda piu' alta tra $da e $a (la finestra del vento, o la fascia utile).
function Get-Onde($righe, [datetime]$giorno, [int]$da, [int]$a) {
  # L'ora con l'onda piu' alta nella fascia (a parita' di altezza, la prima).
  $x = $null
  foreach ($y in $righe) { if ($y.date -eq $giorno -and $y.hour -ge $da -and $y.hour -le $a -and ($x -eq $null -or [double]$y.h -gt [double]$x.h)) { $x = $y } }
  if ($x -eq $null) { return $null }
  function Arr($v, $c) { if ($v -eq $null) { $null } else { [Math]::Round([double]$v, $c) } }
  return [ordered]@{ h = (Arr $x.h 1); p = (Arr $x.p 0); d = (Arr $x.d 0); sh = (Arr $x.sh 1); sp = (Arr $x.sp 0); wh = (Arr $x.wh 1); wp = (Arr $x.wp 0); ora = $x.hour }
}
function Get-Meteo($righeOm, [datetime]$giorno, $fascia) {
  # Cicli semplici al posto di Where-Object/Measure-Object: stesso risultato, molto piu' veloce in PowerShell 5.1.
  $periodi = @(@(8, 11), @(12, 16), @(17, 20))
  $res = @()
  foreach ($p in $periodi) {
    $n = 0; $maxC = $null; $sumT = 0.0; $nP = 0; $sumP = 0.0
    foreach ($x in $righeOm) {
      if ($x.date -ne $giorno -or $x.hour -lt $p[0] -or $x.hour -gt $p[1] -or $x.code -eq $null -or $x.temp -eq $null) { continue }
      $n++; $sumT += $x.temp
      if ($maxC -eq $null -or [double]$x.code -gt $maxC) { $maxC = [double]$x.code }
      if ($x.rain -ne $null) { $nP++; $sumP += $x.rain }
    }
    if (-not $n) { $res += $null; continue }
    $res += [ordered]@{ codice = [int]$maxC; temp = [int][Math]::Round($sumT / $n)
      pioggia = $(if ($nP) { [Math]::Round($sumP, 1) } else { 0 }) }
  }
  $temporale = $false
  foreach ($x in $righeOm) { if ($x.date -eq $giorno -and $x.hour -ge $fascia[0] -and $x.hour -le $fascia[1] -and $x.code -ge 95) { $temporale = $true; break } }
  return [ordered]@{ periodi = $res; temporale = $temporale }
}

# Finestra firing su righe orarie (previsione Open-Meteo, previsione salvata o letture): 15 kts per 2 ore di fila, direzione non da terra.
function Get-Finestra($s, $righe, [int]$inizio, [int]$fine) {
  $ord = @(foreach ($x in $righe) { if ($x -ne $null -and $x.hour -ge $inizio -and $x.hour -le $fine -and $x.wind -ne $null -and $x.deg -ne $null) { $x } })
  $inOrdine = $true; for ($i = 1; $i -lt $ord.Count; $i++) { if ($ord[$i].hour -le $ord[$i - 1].hour) { $inOrdine = $false; break } }
  if (-not $inOrdine) { $ord = @($ord | Sort-Object hour) }
  $migliore = $null; $run = @()
  foreach ($r in ($ord + @($null))) {
    $ok = ($r -ne $null) -and ([double]$r.wind -ge $SOGLIA) -and ((Get-Classe $s ([double]$r.deg)) -ne 'terra')
    if ($ok -and ($run.Count -eq 0 -or $r.hour -eq $run[-1].hour + 1)) { $run += $r; continue }
    if ($run.Count) {
      $dur = $run[-1].hour - $run[0].hour + 1
      if ($dur -ge $ORE_MIN -and (-not $migliore -or $dur -gt ($migliore[-1].hour - $migliore[0].hour + 1))) { $migliore = $run }
    }
    $run = @(); if ($ok) { $run = @($r) }
  }
  if (-not $migliore) { return $null }
  $mn = [double]::MaxValue; $mx = [double]::MinValue
  foreach ($x in $migliore) { $w = [double]$x.wind; if ($w -lt $mn) { $mn = $w }; if ($w -gt $mx) { $mx = $w } }
  return [ordered]@{ ore = "$($migliore[0].hour)-$($migliore[-1].hour)"; min = [int][Math]::Round($mn); max = [int][Math]::Round($mx) }
}

# ---------------- Tempo di viaggio dalla partenza scelta ----------------
# Indirizzo da mostrare nella pagina: senza le eventuali coordinate tra parentesi.
function Get-NomePosto([string]$v) {
  $t = ($v -replace '\(?\s*-?\d{1,2}\.\d{3,}\s*,\s*-?\d{1,3}\.\d{3,}\s*\)?', '').Trim(' ,;')
  if ($t) { return $t } elseif ($v) { return 'posizione salvata' } else { return '' }
}
function Get-Viaggi([bool]$forza) {
  $fileV = Join-Path $dirDati 'viaggi.json'
  $indirizzo = $IMP[$IMP.partenza]
  if (-not $indirizzo) { return $null }
  $cache = $null
  if (Test-Path $fileV) { try { $cache = ConvertFrom-Json ([IO.File]::ReadAllText($fileV)) } catch { $cache = $null } }
  $ids = @($SPOT | ForEach-Object { $_.id })
  # Se cambiano le coordinate degli spot i tempi si ricalcolano.
  $firma = (@($SPOT | ForEach-Object { $_.id + ':' + ([double]$_.lat).ToString($INV) + ',' + ([double]$_.lon).ToString($INV) }) -join ';')
  $valida = $cache -and $cache.indirizzo -eq $indirizzo -and $cache.firma -eq $firma -and -not @($ids | Where-Object { $cache.stima.$_ -eq $null }).Count
  # Il traffico vero (chiave Google) si ricalcola solo quando lo chiedi o avvii a mano, non nelle corse automatiche.
  $traffico = $IMP.google_chiave -and -not $Automatico
  if ($valida -and -not $forza -and -not $traffico) { return $cache }
  # I servizi dei percorsi chiedono TLS 1.3 (dove Windows lo supporta): lo accendo solo qui.
  $tlsPrima = [Net.ServicePointManager]::SecurityProtocol
  try { [Net.ServicePointManager]::SecurityProtocol = $tlsPrima -bor [Net.SecurityProtocolType]'Tls13' } catch {}
  try {
  if ($valida) { $lat = $cache.lat; $lon = $cache.lon; $trovato = $cache.trovato; $stima = $cache.stima }
  else {
    # Coordinate scritte in impostazioni.txt (es. "Via Roma 1, Roma (41.90000, 12.50000)"): si usano quelle.
    $cc = [regex]::Match($indirizzo, '(-?\d{1,2}\.\d{3,})\s*,\s*(-?\d{1,3}\.\d{3,})')
    if ($cc.Success) {
      $lat = [double]::Parse($cc.Groups[1].Value, $INV); $lon = [double]::Parse($cc.Groups[2].Value, $INV); $trovato = 'coordinate di impostazioni.txt'
    } else {
      $base = 'https://nominatim.openstreetmap.org/search?format=json&limit=1&countrycodes=it&addressdetails=1&'
      $geo = @()
      # Prima via + numero civico + citta' (la ricerca libera spesso ignora il civico), poi l'indirizzo com'e'.
      $pz = @($indirizzo -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ })
      if ($pz.Count -ge 2 -and $pz[0] -match '^(.+?)\s+(\d+\s*[A-Za-z]?)$') {
        $q = 'street=' + [Uri]::EscapeDataString($Matches[2] + ' ' + $Matches[1]) + '&city=' + [Uri]::EscapeDataString($pz[1])
        try { $j = ConvertFrom-Json (Get-Testo ($base + $q) 'https://www.openstreetmap.org/'); $geo = @($j | Where-Object { $_.address.house_number }) } catch { $geo = @() }
      }
      if (-not $geo.Count) { $j = ConvertFrom-Json (Get-Testo ($base + 'q=' + [Uri]::EscapeDataString($indirizzo)) 'https://www.openstreetmap.org/'); $geo = @($j) }
      if (-not $geo.Count) { throw "indirizzo non trovato ($indirizzo)" }
      # Se hai scritto il numero civico e la mappa non lo conosce, meglio un errore che un punto qualsiasi della via.
      if ($indirizzo -match '\s\d+\s*[A-Za-z]?\s*(,|$)' -and -not $geo[0].address.house_number) { throw "numero civico non trovato sulla mappa ($indirizzo)" }
      $lat = [double]::Parse($geo[0].lat, $INV); $lon = [double]::Parse($geo[0].lon, $INV); $trovato = $geo[0].display_name
    }
    $coord = @($lon.ToString($INV) + ',' + $lat.ToString($INV)) + @($SPOT | ForEach-Object { ([double]$_.lon).ToString($INV) + ',' + ([double]$_.lat).ToString($INV) })
    $osrm = ConvertFrom-Json (Get-Testo ('https://router.project-osrm.org/table/v1/driving/' + ($coord -join ';') + '?sources=0&annotations=duration') 'https://project-osrm.org/')
    $stima = [ordered]@{}
    for ($i = 0; $i -lt $SPOT.Count; $i++) { $d = $osrm.durations[0][$i + 1]; $stima[$SPOT[$i].id] = $(if ($d -ne $null) { [int][Math]::Round([double]$d / 60) } else { $null }) }
  }
  $live = [ordered]@{}; $liveOra = ''
  if ($traffico) {
    try {
      foreach ($s in $SPOT) {
        $corpo = '{"origin":{"location":{"latLng":{"latitude":' + ([double]$lat).ToString($INV) + ',"longitude":' + ([double]$lon).ToString($INV) + '}}},"destination":{"location":{"latLng":{"latitude":' + ([double]$s.lat).ToString($INV) + ',"longitude":' + ([double]$s.lon).ToString($INV) + '}}},"travelMode":"DRIVE","routingPreference":"TRAFFIC_AWARE"}'
        $r = Invoke-RestMethod -Method Post -Uri 'https://routes.googleapis.com/directions/v2:computeRoutes' -Body $corpo -ContentType 'application/json' -Headers @{ 'X-Goog-Api-Key' = $IMP.google_chiave; 'X-Goog-FieldMask' = 'routes.duration' } -TimeoutSec 20
        $sec = [double](([string]$r.routes[0].duration) -replace 's', '')
        $live[$s.id] = [int][Math]::Round($sec / 60)
      }
      $liveOra = $adesso.ToString('HH:mm', $INV)
    } catch { Write-Host "  (traffico Google non letto: $($_.Exception.Message))" -ForegroundColor Yellow; $live = [ordered]@{} }
  }
  } finally { [Net.ServicePointManager]::SecurityProtocol = $tlsPrima }
  $nuovo = [ordered]@{ indirizzo = $indirizzo; firma = $firma; partenza = $IMP.partenza; trovato = $trovato; lat = $lat; lon = $lon; calcolato = $adesso.ToString('yyyy-MM-dd HH:mm', $INV); stima = $stima; traffico = $live; trafficoOra = $liveOra }
  [IO.File]::WriteAllText($fileV, (ConvertTo-Json $nuovo -Depth 4), (New-Object Text.UTF8Encoding($false)))
  return (ConvertFrom-Json (ConvertTo-Json $nuovo -Depth 4))
}

# ---------------- PDF del report nella cartella Dropbox ----------------
function Save-Pdf([string]$fileHtml) {
  $dest = $IMP.dropbox
  if (-not $dest -or $dest -eq 'no') { return }
  if ($dest -eq 'auto') {
    $dest = $null
    $info = Join-Path $env:LOCALAPPDATA 'Dropbox\info.json'
    if (Test-Path $info) { try { $j = ConvertFrom-Json ([IO.File]::ReadAllText($info)); if ($j.personal.path) { $dest = $j.personal.path } elseif ($j.business.path) { $dest = $j.business.path } } catch {} }
    if (-not $dest -and (Test-Path (Join-Path $env:USERPROFILE 'Dropbox'))) { $dest = Join-Path $env:USERPROFILE 'Dropbox' }
    if (-not $dest) { return }
    $dest = Join-Path $dest 'Radar kite'
  }
  try {
    if (-not (Test-Path $dest)) { New-Item -ItemType Directory -Path $dest -Force | Out-Null }
    $browser = @("${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe", "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe", "$env:ProgramFiles\Google\Chrome\Application\chrome.exe", "${env:ProgramFiles(x86)}\Google\Chrome\Application\chrome.exe", "$env:LOCALAPPDATA\Google\Chrome\Application\chrome.exe", "$env:ProgramFiles\BraveSoftware\Brave-Browser\Application\brave.exe", "$env:LOCALAPPDATA\BraveSoftware\Brave-Browser\Application\brave.exe") | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
    if (-not $browser) { Write-Host '  (PDF non creato: non trovo Edge, Chrome o Brave)' -ForegroundColor Yellow; return }
    $pdf = Join-Path $dest 'Radar kite.pdf'
    $tmp = Join-Path $env:TEMP ('radarkite-' + [guid]::NewGuid().ToString('N'))
    $argomenti = @('--headless', '--disable-gpu', '--no-first-run', '--no-pdf-header-footer', '--print-to-pdf-no-header', "--user-data-dir=`"$tmp`"", '--window-size=760,1400', '--virtual-time-budget=4000', "--print-to-pdf=`"$pdf`"", ('"' + ([Uri]$fileHtml).AbsoluteUri + '#pdf"'))
    $p = Start-Process -FilePath $browser -ArgumentList $argomenti -WindowStyle Hidden -PassThru
    if (-not $p.WaitForExit(60000)) { try { $p.Kill() } catch {} }
    Remove-Item -Path $tmp -Recurse -Force -ErrorAction SilentlyContinue
    if (Test-Path $pdf) { Write-Host "  PDF per il telefono: $pdf" -ForegroundColor Green }
  } catch { Write-Host "  (PDF non creato: $($_.Exception.Message))" -ForegroundColor Yellow }
}

# ---------------- Esecuzione ----------------
Write-Host "Radar kite $VERSIONE ($DATA_VERSIONE) di $AUTORE - $($adesso.ToString('dd/MM/yyyy HH:mm', $INV))" -ForegroundColor Cyan
# Tutte le richieste partono insieme: previsioni Windguru, Open-Meteo, acqua e stazioni.
$FRESCHI = @()
if ($Solo -like 'spot:*') {
  $sx = $SPOT | Where-Object { $_.id -eq $Solo.Substring(5) } | Select-Object -First 1
  $FRESCHI = @("https://micro.windguru.cz/?s=$($sx.wg)", (Get-UrlOpenMeteo), (Get-UrlAcqua)) + @($STAZIONI | Where-Object { $sx.stazioni -contains $_.id } | ForEach-Object { $_.url })
  Write-Host "Aggiorno solo lo spot $($sx.nome)" -ForegroundColor Cyan
} elseif ($Solo -like 'st:*') {
  $FRESCHI = @($STAZIONI | Where-Object { $_.id -eq $Solo.Substring(3) } | ForEach-Object { $_.url })
  Write-Host "Aggiorno solo la stazione $($Solo.Substring(3))" -ForegroundColor Cyan
}
foreach ($s in $SPOT) { Start-Scarico "https://micro.windguru.cz/?s=$($s.wg)" 'https://micro.windguru.cz/' -Secondi 20 }
if (-not $OmProva) { Start-Scarico (Get-UrlOpenMeteo) 'https://open-meteo.com/' -Rigido -Secondi 15 }
Start-Scarico (Get-UrlAcqua) 'https://open-meteo.com/' -Rigido -Secondi 15
$T0 = Get-Date
foreach ($st in $STAZIONI) { Start-Scarico $st.url $st.ref -Xhr:($st.tipo -eq 'weathercloud') -Rigido -Secondi 60 }
Write-Tempo 'richieste partite (tutte insieme)'
$spotRes = @()
foreach ($s in $SPOT) {
  Write-Host "  Windguru $($s.nome)... " -NoNewline
  try {
    $wg = Read-Windguru $s
    $esito = Get-Giorni $s $wg.righe
    $spotRes += [pscustomobject]@{ s = $s; wg = $wg; giorni = $esito; errore = $null }; [void](Update-Tempi)
    Write-Host "ok ($($wg.spot), init $($wg.init) UTC)" -ForegroundColor Green
  } catch {
    $spotRes += [pscustomobject]@{ s = $s; wg = $null; giorni = @(); errore = $_.Exception.Message }
    Write-Host "ERRORE: $($_.Exception.Message) (riga $($_.InvocationInfo.ScriptLineNumber))" -ForegroundColor Red
  }
}
# Open-Meteo (meteo, secondo modello) e mare (acqua, onde) arrivano nella seconda fase, con le stazioni.
$OM = @{}; $ACQUA = @{}; $ONDE = $null; $erroreOm = ''; $erroreMare = ''
Write-Tempo 'previsioni Windguru lette'
[void](Update-Tempi)
$VIAGGI = $null; $erroreViaggi = ''
if ($IMP[$IMP.partenza]) {
  Write-Host "  Tempi di viaggio da $($IMP.partenza)... " -NoNewline
  try { $VIAGGI = Get-Viaggi $forzaViaggi; Write-Host 'ok' -ForegroundColor Green } catch { $erroreViaggi = $_.Exception.Message; Write-Host "ERRORE: $erroreViaggi" -ForegroundColor Red }
}

Write-Tempo 'tempi di viaggio'
# Due fasi: 1) la pagina si apre subito con le previsioni e le stazioni "in lettura";
# 2) arrivano le stazioni, la pagina si aggiorna da sola e si fa il PDF. Gli avvii automatici fanno solo la 2.
$fasi = if ($Automatico -or $Solo) { @(2) } else { @(1, 2) }
foreach ($FASE in $fasi) {
$stRes = @{}
if ($FASE -eq 1) {
  foreach ($st in $STAZIONI) { $stRes[$st.id] = @{ errore = 'in lettura' } }
} else {
Write-Host '  Open-Meteo (meteo e secondo modello)... ' -NoNewline
try { $OM = Read-OpenMeteo; Write-Host 'ok' -ForegroundColor Green } catch { $erroreOm = $_.Exception.Message; Write-Host "ERRORE: $erroreOm" -ForegroundColor Red }
Write-Host '  Mare (acqua e onde)... ' -NoNewline
try { $ACQUA = Read-Acqua; Write-Host 'ok' -ForegroundColor Green } catch { $erroreMare = $_.Exception.Message; Write-Host "ERRORE: $erroreMare" -ForegroundColor Red }
Write-Tempo 'Open-Meteo e mare'
Wait-Stazioni
Write-Tempo 'stazioni arrivate'
foreach ($st in $STAZIONI) {
  Write-Host "  Stazione $($st.nome)... " -NoNewline
  try {
    $l = Read-Stazione $st
    $eta = ($adesso - $l.t).TotalMinutes
    $l.fresca = (-not $l.offline) -and $eta -le $st.fresco -and $eta -ge -15
    $stRes[$st.id] = $l
    Write-Host ("{0} nodi da {1} alle {2}{3} ({4} s)" -f $l.wind, (Get-Comp $l.deg), $l.t.ToString('HH:mm', $INV), $(if ($l.fresca) { '' } else { ' (vecchia)' }), $TEMPI[$st.id]) -ForegroundColor Green
  } catch {
    $stRes[$st.id] = @{ errore = $_.Exception.Message }
    Write-Host "ERRORE: $($_.Exception.Message) (riga $($_.InvocationInfo.ScriptLineNumber))" -ForegroundColor Red
  }
}
}

[void](Update-Tempi)
Write-Tempo 'stazioni lette'
# Appunti sulle stazioni: quanto ci mettono a rispondere e se rispondono (dati\tempi-stazioni.csv).
$LENTE = @{}
try {
  $fileT = Join-Path $dirDati 'tempi-stazioni.csv'
  $righeT = @(if (Test-Path $fileT) { [IO.File]::ReadAllLines($fileT) | Select-Object -Skip 1 | Where-Object { $_ } })
  if ($FASE -eq 2) {
    $nuoveT = @(foreach ($st in $STAZIONI) {
      $l = $stRes[$st.id]; if (-not $l -or $l.errore -match 'sospes') { continue }
      if ($Solo -and -not $TEMPI.ContainsKey($st.id)) { continue }
      $sec = if ($TEMPI.ContainsKey($st.id)) { ([double]$TEMPI[$st.id]).ToString('0.0', $INV) } else { '' }
      $adesso.ToString('yyyy-MM-dd HH:mm', $INV) + ';' + $st.id + ';' + $sec + ';' + $(if ($l.errore) { 'errore' } else { 'ok' })
    })
    $righeT = @($righeT + $nuoveT | Select-Object -Last 3000)
    [IO.File]::WriteAllText($fileT, "data;stazione;secondi;esito`r`n" + (($righeT -join "`r`n") + "`r`n"), (New-Object Text.UTF8Encoding($false)))
  }
  foreach ($st in $STAZIONI) {
    $sue = @($righeT | Where-Object { $_.Split(';')[1] -eq $st.id } | Select-Object -Last 30)
    if ($sue.Count -lt 3) { continue }
    $ok = @($sue | Where-Object { $_.Split(';')[3] -eq 'ok' -and $_.Split(';')[2] })
    $media = if ($ok.Count) { [Math]::Round((@($ok | ForEach-Object { [double]::Parse($_.Split(';')[2], $INV) }) | Measure-Object -Average).Average, 1) } else { $null }
    $LENTE[$st.id] = [ordered]@{ media = $media; fallite = $sue.Count - $ok.Count; prove = $sue.Count }
  }
} catch { Write-Host "  (appunti stazioni non aggiornati: $($_.Exception.Message))" -ForegroundColor Yellow }

# Righe firing (previsione + stazione che segna gia' firing adesso)
$righeFiring = @()
foreach ($r in $spotRes) {
  $s = $r.s
  foreach ($g in $r.giorni) {
    if (-not $g.firing) { continue }
    $raff = if ($g.raffica -ne $null) { ", raffiche $($g.raffica)" } else { '' }
    $note = if ($g.note.Count) { ' - ' + ($g.note -join ', ') } else { '' }
    $quando = if ($g.oggi) { 'OGGI' } else { $g.nome }
    $righeFiring += [pscustomobject]@{ ordine = ($g.data - $adesso.Date).Days + 1; testo = ("{0} - {1}, ore {2}: {3} {4}-{5} nodi{6}{7}" -f $s.nome, $quando, $g.ore, $g.dir, $g.min, $g.maxf, $raff, $note)
      ideale = [bool]($g.note -match '^ideale'); pericolo = [bool]($g.note -contains 'raffiche pericolose') }
  }
}
$righeFiring = @($righeFiring | Sort-Object ordine, @{ Expression = 'ideale'; Descending = $true })

if ($FASE -eq 2) {
Write-Host ''
if ($righeFiring.Count) {
  Write-Host 'FIRING:' -ForegroundColor Green
  foreach ($f in $righeFiring) { Write-Host ("  * " + $f.testo) }
} else {
  Write-Host 'Nessuna finestra di vento nei prossimi giorni.' -ForegroundColor Yellow
}
}

Write-Tempo 'righe firing e appunti'
# ---------------- Storico (2 anni): letture e previsioni in dati\, un file per mese ----------------
# letture-AAAA-MM.csv: le letture aggiornate delle stazioni a ogni avvio.
# previsioni-AAAA-MM.csv: la prima previsione salvata ogni giorno (per oggi e per domani), Windguru e Open-Meteo.
# Si aprono con Excel. Oltre i 2 anni i file vengono tolti.
$UTF8 = New-Object Text.UTF8Encoding($false)
function Get-FileMese([string]$tipo, [datetime]$d) { return (Join-Path $dirDati ("$tipo-" + $d.ToString('yyyy-MM', $INV) + '.csv')) }
function Read-Righe([string]$file) {
  if (-not (Test-Path $file)) { return @() }
  $a = [IO.File]::ReadAllLines($file)
  $out = New-Object System.Collections.Generic.List[string]
  for ($i = 1; $i -lt $a.Length; $i++) { if ($a[$i]) { $out.Add($a[$i]) } }
  return $out.ToArray()
}
function Add-Righe([string]$file, [string]$intestazione, $righe) {
  if (-not @($righe).Count) { return }
  if (-not (Test-Path $file)) { [IO.File]::WriteAllText($file, $intestazione + "`r`n", $UTF8) }
  [IO.File]::AppendAllText($file, ((@($righe) -join "`r`n") + "`r`n"), $UTF8)
}
$INT_LETTURE = 'data;stazione;spot;vento;raffica;gradi'
$INT_PREV = 'giorno;spot;anticipo;fonte;ora;vento;raffica;gradi;salvata'
try {
  # Lo storico.csv delle versioni 1.x passa nei file mensili (una volta sola).
  $vecchio = Join-Path $cartella 'storico.csv'
  if (Test-Path $vecchio) {
    $gruppi = @{}
    foreach ($r in @([IO.File]::ReadAllLines($vecchio) | Where-Object { $_ -match '^\d{4}-\d{2}-\d{2} ' })) { $m = $r.Substring(0, 7); if (-not $gruppi[$m]) { $gruppi[$m] = New-Object System.Collections.ArrayList }; [void]$gruppi[$m].Add($r) }
    foreach ($m in $gruppi.Keys) { $f = Join-Path $dirDati "letture-$m.csv"; $gia = Read-Righe $f; Add-Righe $f $INT_LETTURE @($gruppi[$m] | Where-Object { $gia -notcontains $_ }) }
    Move-Item -Path $vecchio -Destination (Join-Path $dirDati 'storico-versione-1.csv') -Force
  }
  # Letture di adesso.
  $fileL = Get-FileMese 'letture' $adesso
  $tutte = @(Read-Righe $fileL)
  $visti = New-Object 'System.Collections.Generic.HashSet[string]'
  for ($i = [Math]::Max(0, $tutte.Count - 400); $i -lt $tutte.Count; $i++) { [void]$visti.Add($tutte[$i]) }
  $nuove = @()
  foreach ($st in $STAZIONI) {
    $l = $stRes[$st.id]
    if (-not $l -or $l.errore -or -not $l.fresca) { continue }
    $suo = $SPOT | Where-Object { $_.stazioni -contains $st.id } | Select-Object -First 1
    $g = if ($l.gust -ne $null) { ([double]$l.gust).ToString($INV) } else { '' }
    $riga = $l.t.ToString('yyyy-MM-dd HH:mm', $INV) + ';' + $st.id + ';' + $suo.id + ';' + ([double]$l.wind).ToString($INV) + ';' + $g + ';' + ([double]$l.deg).ToString($INV)
    if ($visti.Add($riga)) { $nuove += $riga }
  }
  Add-Righe $fileL $INT_LETTURE $nuove
  # Previsione del giorno: la prima di oggi (anticipo 0) e quella per domani (anticipo 1), una volta sola.
  $fileSalv = Join-Path $dirCache 'previsioni-salvate.txt'
  $presenti = @{}
  if (Test-Path $fileSalv) { foreach ($r in [IO.File]::ReadAllLines($fileSalv)) { if ($r) { $presenti[$r] = 1 } } }
  else { foreach ($k in 0, 1) { foreach ($r in (Read-Righe (Get-FileMese 'previsioni' $adesso.Date.AddDays($k)))) { $p = $r.Split(';'); $presenti["$($p[0]);$($p[1]);$($p[2]);$($p[3])"] = 1 } } }
  $salvate = @()
  for ($k = 0; $k -le 1; $k++) {
    $giorno = $adesso.Date.AddDays($k); $chiave = $giorno.ToString('yyyy-MM-dd', $INV)
    $fileP = Get-FileMese 'previsioni' $giorno
    $nuove = @()
    foreach ($r in $spotRes) {
      $fonti = @(@{ f = 'wg'; righe = $(if ($r.wg) { $r.wg.righe } else { @() }) }, @{ f = 'om'; righe = $(if ($OM[$r.s.id]) { $OM[$r.s.id] } else { @() }) })
      foreach ($fo in $fonti) {
        $kk = "$chiave;$($r.s.id);$k;$($fo.f)"
        if ($presenti[$kk]) { continue }
        $prima = $nuove.Count
        foreach ($x in @($fo.righe | Where-Object { $_.date -eq $giorno -and $_.hour -ge 6 -and $_.hour -le 20 -and $_.wind -ne $null })) {
          $g = if ($x.gust -ne $null) { ([double]$x.gust).ToString($INV) } else { '' }
          $nuove += "$chiave;$($r.s.id);$k;$($fo.f);$($x.hour);$(([double]$x.wind).ToString('0.#', $INV));$g;$(([double]$x.deg).ToString('0', $INV));$($adesso.ToString('HH:mm', $INV))"
        }
        if ($nuove.Count -gt $prima) { $presenti[$kk] = 1 }
      }
    }
    Add-Righe $fileP $INT_PREV $nuove
  }
  # Il segno tiene solo oggi e domani.
  $ieri = $adesso.Date.AddDays(-1).ToString('yyyy-MM-dd', $INV)
  [IO.File]::WriteAllText($fileSalv, ((@($presenti.Keys | Where-Object { $_.Substring(0, 10) -gt $ieri } | Sort-Object) -join "`r`n") + "`r`n"), $UTF8)
  # Oltre i 2 anni si toglie.
  $limite = $adesso.AddMonths(-24).ToString('yyyy-MM', $INV)
  Get-ChildItem -Path $dirDati -Filter '*.csv' | Where-Object { $_.Name -match '^(letture|previsioni)-(\d{4}-\d{2})\.csv$' -and $Matches[2] -lt $limite } | Remove-Item -Force
} catch {
  Write-Host "  (storico non aggiornato: $($_.Exception.Message) riga $($_.InvocationInfo.ScriptLineNumber))" -ForegroundColor Yellow
}

# ---- Giorni passati e diario: non cambiano durante la giornata, si calcolano una volta al giorno ----
# (dati\cache\calcoli.json). Si rifanno il giorno dopo, quando cambia il diario o col pulsante "ricalcola".
$fileCalc = Join-Path $dirCache 'calcoli.json'
$oggiTxt = $adesso.ToString('yyyy-MM-dd', $INV)
$firmaDiario = if (Test-Path $fileDiario) { [string](Get-Item $fileDiario).LastWriteTimeUtc.Ticks } else { '' }
$calc = $null
if (-not $forzaStorico -and (Test-Path $fileCalc)) { try { $calc = ConvertFrom-Json ([IO.File]::ReadAllText($fileCalc)) } catch { $calc = $null } }
$usaPassati = [bool]($calc -and $calc.giorno -eq $oggiTxt -and $calc.versione -eq $VERSIONE)
$usaDiario = [bool]($usaPassati -and $calc.firmaDiario -eq $firmaDiario)
if ($usaPassati) { $datiPassati = @($calc.passati | Where-Object { $_ }); $passatiCalcolati = [string]$calc.calcolati } else {
# Giorni passati della settimana: letture orarie, previsione salvata quel giorno, verdetto.
$lunedi = $adesso.Date.AddDays(-(([int]$adesso.DayOfWeek + 6) % 7))
$mesi = @($lunedi, $adesso.Date) | ForEach-Object { $_.ToString('yyyy-MM', $INV) } | Select-Object -Unique
$letture = @(foreach ($m in $mesi) { foreach ($r in (Read-Righe (Join-Path $dirDati "letture-$m.csv"))) {
  if ($r.Substring(0, 10) -lt $lunedi.ToString('yyyy-MM-dd', $INV)) { continue }
  $p = $r.Split(';')
  [pscustomobject]@{ data = $p[0].Substring(0, 10); ora = $p[0].Substring(11, 5); h = [int]$p[0].Substring(11, 2); hour = [int]$p[0].Substring(11, 2); stazione = $p[1]; spot = $p[2]
    wind = [double]::Parse($p[3], $INV); gust = $(if ($p[4]) { [double]::Parse($p[4], $INV) } else { $null }); deg = [double]::Parse($p[5], $INV) }
} })
$previsioni = @(foreach ($m in $mesi) { foreach ($r in (Read-Righe (Join-Path $dirDati "previsioni-$m.csv"))) {
  if ($r.Substring(0, 10) -lt $lunedi.ToString('yyyy-MM-dd', $INV)) { continue }
  $p = $r.Split(';')
  [pscustomobject]@{ giorno = $p[0]; spot = $p[1]; anticipo = [int]$p[2]; fonte = $p[3]; hour = [int]$p[4]; wind = [double]::Parse($p[5], $INV)
    gust = $(if ($p[6]) { [double]::Parse($p[6], $INV) } else { $null }); deg = [double]::Parse($p[7], $INV); salvata = $p[8] }
} })
# Indici per giorno e spot: ogni ricerca guarda solo le righe di quel giorno.
$idxL = @{}; foreach ($x in $letture) { $kx = "$($x.data)|$($x.spot)"; if (-not $idxL[$kx]) { $idxL[$kx] = New-Object System.Collections.ArrayList }; [void]$idxL[$kx].Add($x) }
$idxP = @{}; foreach ($x in $previsioni) { $kx = "$($x.giorno)|$($x.spot)"; if (-not $idxP[$kx]) { $idxP[$kx] = New-Object System.Collections.ArrayList }; [void]$idxP[$kx].Add($x) }
function Get-OreRegistrate($s, $lett) {
  $out = @()
  foreach ($gr in @($lett | Group-Object h)) {
    $sel = $null
    foreach ($sid in $s.stazioni) { $x = @($gr.Group | Where-Object { $_.stazione -eq $sid }); if ($x.Count) { $sel = $x; break } }
    if (-not $sel) { $sel = @($gr.Group) }
    $gx = @($sel | Where-Object { $_.gust -ne $null })
    $out += [pscustomobject]@{ hour = [int]$gr.Name; wind = [Math]::Round(($sel | Measure-Object wind -Average).Average, 1)
      gust = $(if ($gx.Count) { ($gx | Measure-Object gust -Maximum).Maximum } else { $null }); deg = ($sel | Sort-Object wind -Descending | Select-Object -First 1).deg; n = $sel.Count }
  }
  return @($out | Sort-Object hour)
}
$datiPassati = @()
for ($giorno = $lunedi; $giorno -lt $adesso.Date; $giorno = $giorno.AddDays(1)) {
  $chiave = $giorno.ToString('yyyy-MM-dd', $INV); $f = Get-Fascia $giorno
  $perSpot = @(); $dettaglio = @()
  foreach ($s in $SPOT) {
    $lett = @($idxL["$chiave|$($s.id)"] | Where-Object { $_ -and $_.h -ge $f[0] -and $_.h -le $f[1] })
    if ($lett.Count) {
      $buone = @($lett | Where-Object { (Get-Classe $s $_.deg) -ne 'terra' })
      $top = if ($buone.Count) { $buone | Sort-Object wind -Descending | Select-Object -First 1 } else { $lett | Sort-Object wind -Descending | Select-Object -First 1 }
      $perSpot += [ordered]@{ id = $s.id; breve = $s.breve; letture = $lett.Count; max = $top.wind; gust = $top.gust; dir = (Get-Comp $top.deg); ora = $top.ora
        buono = ($top.wind -ge $SOGLIA -and (Get-Classe $s $top.deg) -ne 'terra'); oltre = @($buone | Where-Object { $_.wind -ge $SOGLIA }).Count }
    }
    # Confronto: previsione salvata quel giorno (se no quella del giorno prima) contro letture ora per ora.
    $pv = @($idxP["$chiave|$($s.id)"] | Where-Object { $_ })
    $ant = if (@($pv | Where-Object { $_.anticipo -eq 0 -and $_.fonte -eq 'wg' }).Count) { 0 } else { 1 }
    $wgPrev = @($pv | Where-Object { $_.anticipo -eq $ant -and $_.fonte -eq 'wg' -and $_.hour -ge $f[0] -and $_.hour -le $f[1] } | Sort-Object hour)
    $omPrev = @($pv | Where-Object { $_.anticipo -eq $ant -and $_.fonte -eq 'om' -and $_.hour -ge $f[0] -and $_.hour -le $f[1] } | Sort-Object hour)
    $reg = Get-OreRegistrate $s $lett
    if (-not $wgPrev.Count -and -not $reg.Count) { continue }
    $diff = @(); foreach ($x in $reg) { $y = $wgPrev | Where-Object { $_.hour -eq $x.hour } | Select-Object -First 1; if ($y) { $diff += ($y.wind - $x.wind) } }
    $dettaglio += [ordered]@{ id = $s.id; nome = $s.nome; salvata = $(if ($wgPrev.Count) { $wgPrev[0].salvata } else { '' }); anticipo = $ant
      prev = @($wgPrev | ForEach-Object { [ordered]@{ h = $_.hour; w = $_.wind; g = $_.gust; d = (Get-Comp $_.deg) } })
      om = @($omPrev | ForEach-Object { [ordered]@{ h = $_.hour; w = $_.wind } })
      reg = @($reg | ForEach-Object { [ordered]@{ h = $_.hour; w = $_.wind; g = $_.gust; d = (Get-Comp $_.deg); n = $_.n } })
      prevF = (Get-Finestra $s $wgPrev $f[0] $f[1]); omF = (Get-Finestra $s $omPrev $f[0] $f[1]); regF = (Get-Finestra $s $reg $f[0] $f[1])
      scarto = $(if ($diff.Count) { [Math]::Round(($diff | Measure-Object -Average).Average, 1) } else { $null }); oreConfronto = $diff.Count }
  }
  $datiPassati += [ordered]@{ data = $chiave; spot = $perSpot; dettaglio = $dettaglio }
}

$passatiCalcolati = $adesso.ToString("yyyy-MM-dd'T'HH:mm:ss", $INV)
}
if ($usaDiario) { $datiDiario = @($calc.diario | Where-Object { $_ }) } else {
# Diario delle uscite (ultime 30), con il vento letto durante l'uscita se c'e'.
$datiDiario = @()
if (Test-Path $fileDiario) {
  $righeD = @(Read-Righe $fileDiario | Select-Object -Last 30)
  $mesiD = @($righeD | ForEach-Object { $_.Substring(0, 7) } | Select-Object -Unique)
  $lettD = @(foreach ($m in $mesiD) { foreach ($r in (Read-Righe (Join-Path $dirDati "letture-$m.csv"))) { $p = $r.Split(';'); [pscustomobject]@{ t = [datetime]::ParseExact($p[0], 'yyyy-MM-dd HH:mm', $INV); spot = $p[2]; wind = [double]::Parse($p[3], $INV); gust = $(if ($p[4]) { [double]::Parse($p[4], $INV) } else { $null }); deg = [double]::Parse($p[5], $INV) } } })
  foreach ($r in $righeD) {
    $p = $r.Split(';')
    $vento = $null
    if ($p[1] -and $p[3]) {
      $da = [datetime]::ParseExact("$($p[0]) $($p[1])", 'yyyy-MM-dd H:mm', $INV); $a = $da.AddMinutes([int]$p[3])
      $x = @($lettD | Where-Object { $_.spot -eq $p[2] -and $_.t -ge $da.AddMinutes(-20) -and $_.t -le $a.AddMinutes(20) })
      if ($x.Count) { $gx = @($x | Where-Object { $_.gust -ne $null }); $vento = [ordered]@{ w = [Math]::Round(($x | Measure-Object wind -Average).Average, 1); g = $(if ($gx.Count) { ($gx | Measure-Object gust -Maximum).Maximum } else { $null }); d = (Get-Comp ($x | Sort-Object wind -Descending | Select-Object -First 1).deg); n = $x.Count } }
    }
    $datiDiario += [ordered]@{ data = $p[0]; ora = $p[1]; spot = $p[2]; minuti = $(if ($p[3]) { [int]$p[3] } else { $null }); vela = $p[4]; note = $(if ($p.Count -gt 5) { $p[5] } else { '' }); vento = $vento }
  }
}

}
if (-not $usaPassati -or -not $usaDiario) {
  $forzaStorico = $false
  try { [IO.File]::WriteAllText($fileCalc, (ConvertTo-Json -InputObject ([ordered]@{ giorno = $oggiTxt; versione = $VERSIONE; firmaDiario = $firmaDiario; calcolati = $passatiCalcolati; passati = @($datiPassati); diario = @($datiDiario) }) -Depth 9 -Compress), $UTF8) } catch { }
}
Write-Tempo ('giorni passati e diario ' + $(if ($usaPassati -and $usaDiario) { 'gia'' pronti' } else { 'calcolati' }))
# ---------------- Report HTML ----------------
# I dati vanno dentro report-modello.html (grafica e calcoli di pagina), che diventa report.html.
function Num($x) { if ($x -eq $null) { return $null } return [Math]::Round([double]$x, 1) }
$datiGiorni = @()
for ($k = 0; $k -lt $GIORNI; $k++) {
  $d = $adesso.Date.AddDays($k)
  $f = Get-Fascia $d
  $datiGiorni += [ordered]@{ data = $d.ToString('yyyy-MM-dd', $INV); nome = "$($GG[[int]$d.DayOfWeek]) $($d.Day)"; oggi = ($k -eq 0); fascia = "$($f[0])-$($f[1])" }
}
$datiSpot = @()
foreach ($r in $spotRes) {
  $s = $r.s
  # Vento adesso: stazione aggiornata, altrimenti previsione dell'ora, altrimenti stazione vecchia.
  $letture = @(foreach ($sid in $s.stazioni) { $x = $stRes[$sid]; if ($x -and -not $x.errore) { $x.id = $sid; $x } })
  $fresche = @($letture | Where-Object { $_.fresca })   # in ordine: prima la stazione sulla spiaggia
  $l = $null; $ad = $null
  if ($fresche.Count) { $l = $fresche[0] } else { $l = @($letture | Where-Object { -not $_.offline } | Sort-Object { $_.t } -Descending) | Select-Object -First 1 }
  $nomeSt = if ($l) { ($STAZIONI | Where-Object { $_.id -eq $l.id } | Select-Object -First 1).etichetta } else { '' }
  if ($l -and $l.fresca) {
    $ad = [ordered]@{ fonte = 'stazione'; wind = (Num $l.wind); gust = (Num $l.gust); deg = $l.deg; dir = (Get-Comp $l.deg); ora = $l.t.ToString('HH:mm', $INV); nome = $nomeSt }
  } elseif ($r.wg -and @($r.wg.righe).Count) {
    $row = $r.wg.righe | Sort-Object { [Math]::Abs(($_.t - $adesso).TotalMinutes) } | Select-Object -First 1
    $ad = [ordered]@{ fonte = 'previsione'; wind = $row.wind; gust = $row.gust; deg = $row.deg; dir = $row.dir; ora = $row.t.ToString('HH:mm', $INV) }
  } elseif ($l) {
    $ad = [ordered]@{ fonte = 'vecchia'; wind = (Num $l.wind); gust = (Num $l.gust); deg = $l.deg; dir = (Get-Comp $l.deg); ora = $l.t.ToString('HH:mm', $INV); nome = $nomeSt }
  }
  $tessere = @()
  $omS = $OM[$s.id]
  # Righe di meteo e onde divise per giorno una volta sola: ogni riquadro guarda solo le 24 ore del suo giorno
  # invece di scorrere ogni volta tutti i 9 giorni (era la parte lenta della seconda fase).
  $omG = @{}; $ondeG = @{}
  if ($omS) { foreach ($x in $omS) { $kx = $x.date.ToString('yyyy-MM-dd', $INV); if (-not $omG[$kx]) { $omG[$kx] = New-Object System.Collections.ArrayList }; [void]$omG[$kx].Add($x) } }
  if ($ONDE -and $ONDE[$s.id]) { foreach ($x in $ONDE[$s.id]) { $kx = $x.date.ToString('yyyy-MM-dd', $INV); if (-not $ondeG[$kx]) { $ondeG[$kx] = New-Object System.Collections.ArrayList }; [void]$ondeG[$kx].Add($x) } }
  foreach ($g in $r.giorni) {
    $kg = ([datetime]$g.data).ToString('yyyy-MM-dd', $INV)
    $omGiorno = if ($omG[$kg]) { $omG[$kg] } else { @() }
    $o = [ordered]@{ firing = $g.firing; max = $g.max; maxRaff = $g.maxRaff; maxDir = $g.maxDir }
    if ($omS) {
      $fa = Get-Fascia $g.data
      $ini = if ($g.oggi) { [Math]::Max($fa[0], $adesso.AddHours($VIAGGIO_ORE).Hour) } else { $fa[0] }
      $o.meteo = Get-Meteo $omGiorno $g.data $fa
      $omOre = @(foreach ($x in $omGiorno) { if ($x.hour -ge $ini -and $x.hour -le $fa[1] -and $x.wind -ne $null) { $x } })
      $omMax = $null; foreach ($x in $omOre) { if ($omMax -eq $null -or [double]$x.wind -gt $omMax) { $omMax = [double]$x.wind } }
      $o.om = [ordered]@{ firing = (Get-Finestra $s $omOre $ini $fa[1]); max = $(if ($omOre.Count) { [int][Math]::Round($omMax) } else { $null }) }
    }
    if ($ONDE -and $ONDE[$s.id]) {
      $faO = Get-Fascia $g.data
      $oraDa = $faO[0]; $oraA = $faO[1]
      if ($g.firing -and $g.ore -match '^(\d+)\D+(\d+)$') { $oraDa = [int]$Matches[1]; $oraA = [int]$Matches[2] }
      $o.onde = $(if ($ondeG[$kg]) { Get-Onde $ondeG[$kg] $g.data $oraDa $oraA } else { $null })
    }
    if ($g.firing) {
      $o.ore = $g.ore; $o.durata = $g.durata; $o.min = $g.min; $o.maxf = $g.maxf; $o.raffica = $g.raffica; $o.dir = $g.dir
      $o.pericolose = $g.pericolose; $o.rafficato = $g.rafficato; $o.sideoff = $g.sideoff; $o.ideale = $g.ideale; $o.attenzione = $g.attenzione
    }
    $tessere += $o
  }
  $staz = @(foreach ($sid in $s.stazioni) { $x = $STAZIONI | Where-Object { $_.id -eq $sid } | Select-Object -First 1; [ordered]@{ id = $x.id; nome = $x.nome; etichetta = $x.etichetta; pagina = $x.pagina } })
  $viaggio = $null
  if ($VIAGGI) { $viaggio = [ordered]@{ stima = $VIAGGI.stima.($s.id); traffico = $(if ($VIAGGI.traffico) { $VIAGGI.traffico.($s.id) } else { $null }); ora = $VIAGGI.trafficoOra } }
  $kOggi = $adesso.ToString('yyyy-MM-dd', $INV)
  $ondeOra = $(if ($ondeG[$kOggi]) { Get-Onde $ondeG[$kOggi] $adesso.Date $adesso.Hour $adesso.Hour } else { $null })
  $datiSpot += [ordered]@{ viaggio = $viaggio; acqua = $ACQUA[$s.id]; ondeOra = $ondeOra; id = $s.id; nome = $s.nome; breve = $s.breve; mappa = $(if ($s.mappa) { $s.mappa } else { $s.breve }); lon = $s.lon; lat = $s.lat; lab = $s.lab; wg = $s.wg; webcam = $s.webcam; surf = $s.surf
    ok = $s.ok; terra = $s.terra; stazioni = $staz; errore = $r.errore
    wgSpot = $(if ($r.wg) { $r.wg.spot } else { '' }); init = $(if ($r.wg) { $r.wg.init } else { '' }); adesso = $ad; giorni = $tessere }
}
$datiStazioni = @()
foreach ($st in $STAZIONI) {
  $l = $stRes[$st.id]
  $suo = $SPOT | Where-Object { $_.stazioni -contains $st.id } | Select-Object -First 1
  if ($l.errore) { $datiStazioni += [ordered]@{ id = $st.id; nome = $st.nome; pagina = $st.pagina; spot = $suo.id; errore = $l.errore; lenta = $LENTE[$st.id] }; continue }
  $stato = if ($l.offline) { 'offline' } elseif ($l.fresca) { 'aggiornata' } else { 'vecchia' }
  $datiStazioni += [ordered]@{ id = $st.id; nome = $st.nome; pagina = $st.pagina; spot = $suo.id; data = $l.t.ToString('dd/MM', $INV); giorni = [int][Math]::Floor(($adesso - $l.t).TotalDays); wind = (Num $l.wind); gust = (Num $l.gust); deg = $l.deg; dir = (Get-Comp $l.deg); ora = $l.t.ToString('HH:mm', $INV); stato = $stato; lenta = $LENTE[$st.id] }
}
# Quando e' stato aggiornato ogni spot e ogni stazione (dati\cache\meta.json): con "Aggiorna spot" cambia solo il suo.
$fileMeta = Join-Path $dirCache 'meta.json'
$META = @{ creato = ''; spot = @{}; stazioni = @{} }
try {
  if (Test-Path $fileMeta) {
    $mj = ConvertFrom-Json ([IO.File]::ReadAllText($fileMeta))
    $META.creato = [string]$mj.creato
    if ($mj.spot) { foreach ($pp in $mj.spot.PSObject.Properties) { $META.spot[$pp.Name] = [string]$pp.Value } }
    if ($mj.stazioni) { foreach ($pp in $mj.stazioni.PSObject.Properties) { $META.stazioni[$pp.Name] = [string]$pp.Value } }
  }
} catch { }
$oraTxt = $adesso.ToString("yyyy-MM-dd'T'HH:mm:ss", $INV)
if (-not $Solo -or -not $META.creato) {
  $META.creato = $oraTxt; foreach ($s in $SPOT) { $META.spot[$s.id] = $oraTxt }; foreach ($st in $STAZIONI) { $META.stazioni[$st.id] = $oraTxt }
} elseif ($Solo -like 'spot:*') {
  $META.spot[$sx.id] = $oraTxt; foreach ($sid in $sx.stazioni) { $META.stazioni[$sid] = $oraTxt }
} else { $META.stazioni[$Solo.Substring(3)] = $oraTxt }
if ($FASE -eq 2) { try { [IO.File]::WriteAllText($fileMeta, (ConvertTo-Json -InputObject $META -Depth 4 -Compress), (New-Object Text.UTF8Encoding($false))) } catch { } }
foreach ($ds in $datiSpot) { $ds.aggiornato = $META.spot[$ds.id] }
foreach ($dz in $datiStazioni) { $dz.aggiornato = $META.stazioni[$dz.id] }

$dati = [ordered]@{ aggiornando = ($FASE -eq 1); passatiCalcolati = $passatiCalcolati; omErrore = $erroreOm; mareErrore = $erroreMare; versione = $VERSIONE; dataVersione = $DATA_VERSIONE; autore = $AUTORE; creato = $META.creato; soglia = $SOGLIA; oreMin = $ORE_MIN; giorni = $datiGiorni; passati = $datiPassati; spot = $datiSpot; stazioni = $datiStazioni; diario = $datiDiario
  partenza = [ordered]@{ scelta = $IMP.partenza; casa = (Get-NomePosto $IMP.casa); lavoro = (Get-NomePosto $IMP.lavoro); casaImp = $IMP.casa; lavoroImp = $IMP.lavoro; errore = $erroreViaggi; trovato = $(if ($VIAGGI) { $VIAGGI.trovato } else { '' }); lat = $(if ($VIAGGI) { $VIAGGI.lat } else { $null }); lon = $(if ($VIAGGI) { $VIAGGI.lon } else { $null }); google = [bool]$IMP.google_chiave } }
Write-Tempo 'dati della pagina pronti'
[void](Update-Tempi)
$json = ConvertTo-Json -InputObject $dati -Depth 8 -Compress
$modello = [IO.File]::ReadAllText((Join-Path $cartella 'report-modello.html'), [Text.Encoding]::UTF8)
$html = $modello.Replace('/*DATI*/null', $json)

$file = Join-Path $cartella 'report.html'
# Scrivo prima un file temporaneo e poi lo sostituisco: la pagina che si ricarica non trova mai un file a meta'.
$tmpRep = Join-Path $cartella 'report.tmp'
[IO.File]::WriteAllText($tmpRep, $html, (New-Object Text.UTF8Encoding($true)))
Move-Item -Path $tmpRep -Destination $file -Force
if ($FASE -eq 1) {
  Write-Tempo 'pagina aperta (fase 1)'
  Write-Host '  Pagina aperta, leggo le stazioni...' -ForegroundColor Cyan
  # Con -GiaAperta il file .bat ha gia' aperto l'ultimo report, che si ricarica da solo.
  if (-not $NonAprire -and -not $GiaAperta) { Start-Process $file }
} else {
  Write-Tempo 'pagina completa (fase 2)'
  Write-Host ''
  Write-Host "Report: $file"
  # A pagina completa: controllo dell'avvio automatico (solo avvii a mano, una volta al giorno), poi il PDF.
  if (-not $Automatico -and -not $Link) { Register-AvvioAutomatico; Write-Tempo 'avvio automatico controllato' }
  if (-not $Solo) { Save-Pdf $file; Write-Tempo 'PDF' }
  if (-not $NonAprire -and $fasi.Count -eq 1) { Start-Process $file }
}
}
if ($Link) { Start-Sleep -Seconds 2 }
