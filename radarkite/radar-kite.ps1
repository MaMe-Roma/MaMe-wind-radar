<#
  Radar kite - versione locale per il PC di Mattia.
  Legge le previsioni di Windguru (pagine "micro", solo testo) e le stazioni del vento,
  applica le regole firing di ogni spot e apre un report nel browser.
  Avvio: doppio clic su "Avvia Radar kite.bat" nella stessa cartella.
#>
param([switch]$NonAprire, [string]$SalvaGrezzi = '', [string]$Link = '')

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$VERSIONE = '1.7'
$DATA_VERSIONE = '18/11/2026'
$AUTORE = '@MadHatterMatt'

# ---------------- Regole (modificabili) ----------------
$SOGLIA = 15          # nodi medi minimi
$ORE_MIN = 2          # ore di fila sopra soglia
$GIORNI = 8           # oggi + 7 giorni (fino allo stesso giorno della settimana prossima)
$VIAGGIO_ORE = 1      # per oggi conta solo il vento da un'ora dopo l'avvio

# Spot da nord a sud. ok = direzioni buone (gradi da cui arriva il vento), terra = vento da terra (mai firing);
# tutto il resto e' side-off. Passoscuro e Salto di Fondi: direzioni stimate dalla costa, da confermare.
$SPOT = @(
  @{ id = 'sag'; nome = "Civitavecchia Sant'Agostino"; breve = 'S. Agostino'; lon = 11.74; lat = 42.17; lab = @(11, -9, 'start'); wg = 49021; ok = @(180, 320); terra = @(350, 130); stazioni = @('rdt_civitavecchia'); webcam = ''
     ideali = @(); attenzione = @() },
  @{ id = 'msn'; nome = 'Marina di San Nicola'; breve = 'San Nicola'; lon = 12.12; lat = 41.93; lab = @(11, -9, 'start'); wg = 49027; ok = @(170, 300); terra = @(330, 110); stazioni = @('msn'); webcam = ''
     ideali = @(@{ n = 'ponente'; d = 270 }); attenzione = @(@{ n = 'libeccio: lavatrice, onde ~2 m'; d = 225 }) },
  @{ id = 'pass'; nome = 'Passoscuro'; breve = 'Passoscuro'; lon = 12.157; lat = 41.904; lab = @(11, 10, 'start'); wg = 49028; ok = @(170, 300); terra = @(330, 110); stazioni = @('passoscuro'); webcam = ''
     ideali = @(); attenzione = @() },
  @{ id = 'ostia'; nome = 'Ostia'; breve = 'Ostia'; lon = 12.3024; lat = 41.7187; lab = @(11, -9, 'start'); wg = 1518; ok = @(115, 325); terra = @(345, 95); stazioni = @('lni_ostia'); webcam = ''
     ideali = @(@{ n = 'scirocco'; d = 135 }, @{ n = 'ponente'; d = 270 }); attenzione = @() },
  @{ id = 'torva'; nome = 'Torvaianica'; breve = 'Torvaianica'; lon = 12.4346; lat = 41.6401; lab = @(11, 4, 'start'); wg = 49033; ok = @(115, 325); terra = @(345, 95); stazioni = @('torvaianica'); webcam = ''
     ideali = @(); attenzione = @() },
  @{ id = 'anzio'; nome = 'Anzio Levante'; breve = 'Anzio Levante'; lon = 12.63; lat = 41.45; lab = @(11, 4, 'start'); wg = 49035; ok = @(40, 230); terra = @(250, 20); stazioni = @('anzio_levante', 'anzio_porto'); webcam = 'https://vedetta.org/webcam/italia/lazio/roma/anzio-levante/'
     ideali = @(@{ n = 'libeccio'; d = 225 }, @{ n = 'grecale'; d = 45 }); attenzione = @() },
  # Lido di Latina: tra Foce Verde e Capo Portiere (scuola Ultimate Kiteboarding). Windguru non ha uno spot qui: uso Foce Verde, stessa baia.
  @{ id = 'latina'; nome = 'Lido di Latina'; breve = 'Lido di Latina'; lon = 12.8759; lat = 41.4042; lab = @(11, 4, 'start'); wg = 49036; ok = @(120, 315); terra = @(335, 100); stazioni = @('latina_lido', 'latina_sud'); webcam = 'https://vedetta.org/webcam/italia/lazio/latina/lido-di-latina/'
     ideali = @(); attenzione = @() },
  @{ id = 'fondi'; nome = 'Salto di Fondi'; breve = 'Salto di Fondi'; lon = 13.3122; lat = 41.2908; lab = @(-6, -14, 'end'); wg = 49042; ok = @(110, 280); terra = @(300, 80); stazioni = @('saltodifondi'); webcam = ''
     ideali = @(); attenzione = @() }
)

# tipo = come leggere la pagina; fresco = minuti oltre i quali la lettura e' vecchia.
$STAZIONI = @(
  @{ id = 'rdt_civitavecchia'; tipo = 'rdt'; nome = 'Riva di Traiano'; etichetta = 'Stazione'; url = 'https://www.teoxonline.com/teox-utils/netsense-staz-ptrt/api/summary_01.php'; ref = 'https://www.rivaditraiano.com/meteo-obs/'; pagina = 'https://www.rivaditraiano.com/meteo-obs/'; fresco = 30 },
  @{ id = 'msn'; tipo = 'msn'; nome = 'Free Kiter MSN'; etichetta = 'Stazione'; url = 'https://stazioni3.soluzionimeteo.it/freekitermsn/homepage/blocks/gauges/updater.php'; ref = 'https://stazioni3.soluzionimeteo.it/freekitermsn/'; pagina = 'https://stazioni3.soluzionimeteo.it/freekitermsn/'; fresco = 30 },
  @{ id = 'passoscuro'; tipo = 'wind24'; nome = 'Passoscuro (Wind24)'; etichetta = 'Wind24'; url = 'https://www.wind24.it/passoscuro'; ref = 'https://www.wind24.it/'; pagina = 'https://www.wind24.it/passoscuro'; fresco = 30 },
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
if ($Link) { $NonAprire = $true }   # avviato dal pulsante: la pagina gia' aperta si ricarica da sola

function Get-Testo([string]$url, [string]$referer) {
  for ($n = 1; ; $n++) {
    try {
      $r = Invoke-WebRequest -Uri $url -UseBasicParsing -UserAgent $UA -Headers @{ Referer = $referer; 'Cache-Control' = 'no-cache' } -TimeoutSec 20
      break
    } catch {
      if ($n -ge 2) { throw }
      Start-Sleep -Seconds 3
    }
  }
  if ($r.RawContentStream) {
    # Decodifica sempre in UTF-8, cosi' le lettere accentate non si rovinano.
    return [Text.Encoding]::UTF8.GetString($r.RawContentStream.ToArray())
  }
  return [string]$r.Content
}

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
        $durata = $run[-1].hour - $run[0].hour + $passo
        $dm = if ($migliore) { $migliore[-1].hour - $migliore[0].hour + $passo } else { -1 }
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
      $res.durata = $migliore[-1].hour - $migliore[0].hour + $passo
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
  $testo = (Get-Testo $st.url $st.ref).TrimStart([char]0xFEFF)
  if ($SalvaGrezzi) { [IO.File]::WriteAllText((Join-Path $SalvaGrezzi "stazione_$($st.id).txt"), $testo) }
  switch ($st.tipo) {
    'msn' {
      $j = $testo | ConvertFrom-Json
      $t = [DateTimeOffset]::FromUnixTimeSeconds([int64]$j.DateTime).LocalDateTime
      return @{ t = $t; wind = [double]$j.W; gust = [double]$j.G; deg = [double]$j.B; offline = ([string]$j.offline -eq '1') }
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

# ---------------- Esecuzione ----------------
Write-Host "Radar kite $VERSIONE ($DATA_VERSIONE) di $AUTORE - $($adesso.ToString('dd/MM/yyyy HH:mm', $INV))" -ForegroundColor Cyan
$spotRes = @()
foreach ($s in $SPOT) {
  Write-Host "  Windguru $($s.nome)... " -NoNewline
  try {
    $wg = Read-Windguru $s
    $esito = Get-Giorni $s $wg.righe
    $spotRes += [pscustomobject]@{ s = $s; wg = $wg; giorni = $esito; errore = $null }
    Write-Host "ok ($($wg.spot), init $($wg.init) UTC)" -ForegroundColor Green
  } catch {
    $spotRes += [pscustomobject]@{ s = $s; wg = $null; giorni = @(); errore = $_.Exception.Message }
    Write-Host "ERRORE: $($_.Exception.Message) (riga $($_.InvocationInfo.ScriptLineNumber))" -ForegroundColor Red
  }
}
$stRes = @{}
foreach ($st in $STAZIONI) {
  Write-Host "  Stazione $($st.nome)... " -NoNewline
  try {
    $l = Read-Stazione $st
    $eta = ($adesso - $l.t).TotalMinutes
    $l.fresca = (-not $l.offline) -and $eta -le $st.fresco -and $eta -ge -15
    $stRes[$st.id] = $l
    Write-Host ("{0} nodi da {1} alle {2}{3}" -f $l.wind, (Get-Comp $l.deg), $l.t.ToString('HH:mm', $INV), $(if ($l.fresca) { '' } else { ' (vecchia)' })) -ForegroundColor Green
  } catch {
    $stRes[$st.id] = @{ errore = $_.Exception.Message }
    Write-Host "ERRORE: $($_.Exception.Message) (riga $($_.InvocationInfo.ScriptLineNumber))" -ForegroundColor Red
  }
}

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

Write-Host ''
if ($righeFiring.Count) {
  Write-Host 'FIRING:' -ForegroundColor Green
  foreach ($f in $righeFiring) { Write-Host ("  * " + $f.testo) }
} else {
  Write-Host 'Nessuna finestra di vento nei prossimi giorni.' -ForegroundColor Yellow
}

# ---------------- Storico: le letture delle stazioni salvate a ogni avvio ----------------
# storico.csv (nella cartella) tiene le letture aggiornate degli ultimi 60 giorni; i giorni passati
# della settimana nel calendario mostrano cosa e' stato letto lì, non vanno a cercare dati altrove.
$fileStorico = Join-Path $cartella 'storico.csv'
$storico = @()
try {
  $righe = @()
  if (Test-Path $fileStorico) { $righe = @([IO.File]::ReadAllLines($fileStorico) | Where-Object { $_ -and $_ -match '^\d{4}-\d{2}-\d{2} ' }) }
  $limite = $adesso.Date.AddDays(-60).ToString('yyyy-MM-dd', $INV)
  $righe = @($righe | Where-Object { $_.Substring(0, 10) -ge $limite })
  foreach ($st in $STAZIONI) {
    $l = $stRes[$st.id]
    if (-not $l -or $l.errore -or -not $l.fresca) { continue }
    $suo = $SPOT | Where-Object { $_.stazioni -contains $st.id } | Select-Object -First 1
    $g = if ($l.gust -ne $null) { ([double]$l.gust).ToString($INV) } else { '' }
    $riga = $l.t.ToString('yyyy-MM-dd HH:mm', $INV) + ';' + $st.id + ';' + $suo.id + ';' + ([double]$l.wind).ToString($INV) + ';' + $g + ';' + ([double]$l.deg).ToString($INV)
    if ($righe -notcontains $riga) { $righe += $riga }
  }
  [IO.File]::WriteAllLines($fileStorico, [string[]](@('data;stazione;spot;vento;raffica;gradi') + $righe), (New-Object Text.UTF8Encoding($false)))
  $storico = $righe
} catch {
  Write-Host "  (storico non aggiornato: $($_.Exception.Message))" -ForegroundColor Yellow
}
$lunedi = $adesso.Date.AddDays(-(([int]$adesso.DayOfWeek + 6) % 7))
$daLunedi = @(foreach ($r in $storico) {
  if ($r.Substring(0, 10) -lt $lunedi.ToString('yyyy-MM-dd', $INV)) { continue }
  $p = $r.Split(';')
  [pscustomobject]@{ data = $p[0].Substring(0, 10); ora = $p[0].Substring(11, 5); h = [int]$p[0].Substring(11, 2); spot = $p[2]
    wind = [double]::Parse($p[3], $INV); gust = $(if ($p[4]) { [double]::Parse($p[4], $INV) } else { $null }); deg = [double]::Parse($p[5], $INV) }
})
$datiPassati = @()
for ($giorno = $lunedi; $giorno -lt $adesso.Date; $giorno = $giorno.AddDays(1)) {
  $chiave = $giorno.ToString('yyyy-MM-dd', $INV); $f = Get-Fascia $giorno
  $perSpot = @()
  foreach ($s in $SPOT) {
    $lett = @($daLunedi | Where-Object { $_.data -eq $chiave -and $_.spot -eq $s.id -and $_.h -ge $f[0] -and $_.h -le $f[1] })
    if (-not $lett.Count) { continue }
    $buone = @($lett | Where-Object { (Get-Classe $s $_.deg) -ne 'terra' })
    $top = if ($buone.Count) { $buone | Sort-Object wind -Descending | Select-Object -First 1 } else { $lett | Sort-Object wind -Descending | Select-Object -First 1 }
    $perSpot += [ordered]@{ id = $s.id; breve = $s.breve; letture = $lett.Count; max = $top.wind; gust = $top.gust; dir = (Get-Comp $top.deg); ora = $top.ora
      buono = ($top.wind -ge $SOGLIA -and (Get-Classe $s $top.deg) -ne 'terra'); oltre = @($buone | Where-Object { $_.wind -ge $SOGLIA }).Count }
  }
  $datiPassati += [ordered]@{ data = $chiave; spot = $perSpot }
}

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
  $gg = @()
  foreach ($g in $r.giorni) {
    $o = [ordered]@{ firing = $g.firing; max = $g.max; maxRaff = $g.maxRaff; maxDir = $g.maxDir }
    if ($g.firing) {
      $o.ore = $g.ore; $o.durata = $g.durata; $o.min = $g.min; $o.maxf = $g.maxf; $o.raffica = $g.raffica; $o.dir = $g.dir
      $o.pericolose = $g.pericolose; $o.rafficato = $g.rafficato; $o.sideoff = $g.sideoff; $o.ideale = $g.ideale; $o.attenzione = $g.attenzione
    }
    $gg += $o
  }
  $staz = @(foreach ($sid in $s.stazioni) { $x = $STAZIONI | Where-Object { $_.id -eq $sid } | Select-Object -First 1; [ordered]@{ id = $x.id; nome = $x.nome; etichetta = $x.etichetta; pagina = $x.pagina } })
  $datiSpot += [ordered]@{ id = $s.id; nome = $s.nome; breve = $s.breve; lon = $s.lon; lat = $s.lat; lab = $s.lab; wg = $s.wg; webcam = $s.webcam
    ok = $s.ok; terra = $s.terra; stazioni = $staz; errore = $r.errore
    wgSpot = $(if ($r.wg) { $r.wg.spot } else { '' }); init = $(if ($r.wg) { $r.wg.init } else { '' }); adesso = $ad; giorni = $gg }
}
$datiStazioni = @()
foreach ($st in $STAZIONI) {
  $l = $stRes[$st.id]
  $suo = $SPOT | Where-Object { $_.stazioni -contains $st.id } | Select-Object -First 1
  if ($l.errore) { $datiStazioni += [ordered]@{ id = $st.id; nome = $st.nome; pagina = $st.pagina; spot = $suo.id; errore = $l.errore }; continue }
  $stato = if ($l.offline) { 'offline' } elseif ($l.fresca) { 'aggiornata' } else { 'vecchia' }
  $datiStazioni += [ordered]@{ id = $st.id; nome = $st.nome; pagina = $st.pagina; spot = $suo.id; data = $l.t.ToString('dd/MM', $INV); giorni = [int][Math]::Floor(($adesso - $l.t).TotalDays); wind = (Num $l.wind); gust = (Num $l.gust); deg = $l.deg; dir = (Get-Comp $l.deg); ora = $l.t.ToString('HH:mm', $INV); stato = $stato }
}
$dati = [ordered]@{ versione = $VERSIONE; dataVersione = $DATA_VERSIONE; autore = $AUTORE; creato = $adesso.ToString("yyyy-MM-dd'T'HH:mm:ss", $INV); soglia = $SOGLIA; oreMin = $ORE_MIN; giorni = $datiGiorni; passati = $datiPassati; spot = $datiSpot; stazioni = $datiStazioni }
$json = ConvertTo-Json -InputObject $dati -Depth 8 -Compress
$modello = [IO.File]::ReadAllText((Join-Path $cartella 'report-modello.html'), [Text.Encoding]::UTF8)
$html = $modello.Replace('/*DATI*/null', $json)

$file = Join-Path $cartella 'report.html'
[IO.File]::WriteAllText($file, $html, (New-Object Text.UTF8Encoding($true)))
Write-Host ''
Write-Host "Report: $file"
if (-not $NonAprire) { Start-Process $file }
if ($Link) { Start-Sleep -Seconds 2 }
