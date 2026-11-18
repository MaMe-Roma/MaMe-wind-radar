<#
  Radar kite - versione locale per il PC di Mattia.
  Legge le previsioni di Windguru (pagine "micro", solo testo) e le stazioni del vento,
  applica le regole firing di ogni spot e apre un report nel browser.
  Avvio: doppio clic su "Avvia Radar kite.bat" nella stessa cartella.
#>
param([switch]$NonAprire, [string]$SalvaGrezzi = '')

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# ---------------- Regole (modificabili) ----------------
$SOGLIA = 15          # nodi medi minimi
$ORE_MIN = 2          # ore di fila sopra soglia
$GIORNI = 5           # oggi + 4 giorni
$VIAGGIO_ORE = 1      # per oggi conta solo il vento da un'ora dopo l'avvio

$SPOT = @(
  @{ id = 'msn'; nome = 'Marina di San Nicola'; wg = 49027; ok = @(170, 300); terra = @(330, 110); stazione = 'msn'
     ideali = @(@{ n = 'ponente'; d = 270 }); attenzione = @(@{ n = 'libeccio: effetto lavatrice, onde ~2 m'; d = 225 }) },
  @{ id = 'ostia'; nome = 'Ostia'; wg = 1518; ok = @(115, 325); terra = @(345, 95); stazione = 'lni_ostia'
     ideali = @(@{ n = 'scirocco'; d = 135 }, @{ n = 'ponente'; d = 270 }); attenzione = @() },
  @{ id = 'sag'; nome = "Civitavecchia Sant'Agostino"; wg = 49021; ok = @(180, 320); terra = @(350, 130); stazione = 'rdt_civitavecchia'
     ideali = @(); attenzione = @() },
  @{ id = 'anzio'; nome = 'Anzio Levante'; wg = 49035; ok = @(40, 230); terra = @(250, 20); stazione = 'anzio_porto'
     ideali = @(@{ n = 'libeccio'; d = 225 }, @{ n = 'grecale'; d = 45 }); attenzione = @() }
)

$STAZIONI = @(
  @{ id = 'msn'; nome = 'Free Kiter MSN'; url = 'https://stazioni3.soluzionimeteo.it/freekitermsn/homepage/blocks/gauges/updater.php'; ref = 'https://stazioni3.soluzionimeteo.it/freekitermsn/'; pagina = 'https://stazioni3.soluzionimeteo.it/freekitermsn/'; fresco = 30 },
  @{ id = 'lni_ostia'; nome = 'Lega Navale Ostia'; url = 'https://anemometro.leganavaleostia.it:8080/data/clientraw.txt'; ref = 'https://www.leganavaleostia.it/situazione-attuale/'; pagina = 'https://www.leganavaleostia.it/situazione-attuale/'; fresco = 30 },
  @{ id = 'rdt_civitavecchia'; nome = 'Riva di Traiano'; url = 'https://www.teoxonline.com/teox-utils/netsense-staz-ptrt/api/summary_01.php'; ref = 'https://www.rivaditraiano.com/meteo-obs/'; pagina = 'https://www.rivaditraiano.com/meteo-obs/'; fresco = 30 },
  @{ id = 'anzio_porto'; nome = 'Anzio Porto (Meteo Lazio)'; url = 'https://www.meteoregionelazio.it/rete/stazione.php?id=RM-050'; ref = 'https://www.meteoregionelazio.it/rete/'; pagina = 'https://www.meteoregionelazio.it/rete/stazione.php?id=RM-050'; fresco = 60 }
)

$PUNTI = @('N', 'NNE', 'NE', 'ENE', 'E', 'ESE', 'SE', 'SSE', 'S', 'SSW', 'SW', 'WSW', 'W', 'WNW', 'NW', 'NNW')
$GG = @('dom', 'lun', 'mar', 'mer', 'gio', 'ven', 'sab')
$UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) RadarKite/1.0'
$cartella = Split-Path -Parent $MyInvocation.MyCommand.Path
$adesso = Get-Date

function Get-Testo([string]$url, [string]$referer) {
  for ($n = 1; ; $n++) {
    try {
      $r = Invoke-WebRequest -Uri $url -UseBasicParsing -UserAgent $UA -Headers @{ Referer = $referer; 'Cache-Control' = 'no-cache' } -TimeoutSec 30
      break
    } catch {
      if ($n -ge 3) { throw }
      Start-Sleep -Seconds 5
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
      max = $(if ($utili.Count) { ($utili | Measure-Object wind -Maximum).Maximum } else { $null }); firing = $false }
    if ($migliore) {
      $gradi = @($migliore | ForEach-Object { $_.deg })
      $raff = @($migliore | Where-Object { $_.gust -ne $null })
      $res.firing = $true
      $res.ore = "$($migliore[0].hour)-$($migliore[-1].hour)"
      $res.min = ($migliore | Measure-Object wind -Minimum).Minimum
      $res.maxf = ($migliore | Measure-Object wind -Maximum).Maximum
      $res.raffica = $(if ($raff.Count) { ($raff | Measure-Object gust -Maximum).Maximum } else { $null })
      $res.dir = (@($migliore | ForEach-Object { $_.dir } | Select-Object -Unique)) -join '/'
      $note = @()
      if ($res.raffica -ne $null -and $res.raffica -gt 35) { $note += 'raffiche pericolose' }
      elseif (@($raff | Where-Object { $_.gust - $_.wind -gt 10 }).Count) { $note += 'rafficato' }
      if (@($migliore | Where-Object { (Get-Classe $s $_.deg) -eq 'side' }).Count) { $note += 'side-off' }
      $id = Get-Vicini -lista $s.ideali -gradi $gradi -SoloBuoni -s $s
      if ($id.Count) { $note += 'ideale con ' + ($id -join ' e ') }
      $at = Get-Vicini -lista $s.attenzione -gradi $gradi -s $s
      if ($at.Count) { $note += $at }
      $res.note = $note
    }
    $risultati += [pscustomobject]$res
  }
  return $risultati
}

# ---------------- Stazioni ----------------
function Read-Stazione($st) {
  $testo = Get-Testo $st.url $st.ref
  switch ($st.id) {
    'msn' {
      $j = $testo | ConvertFrom-Json
      $t = [DateTimeOffset]::FromUnixTimeSeconds([int64]$j.DateTime).LocalDateTime
      return @{ t = $t; wind = [double]$j.W; gust = [double]$j.G; deg = [double]$j.B; offline = ([string]$j.offline -eq '1') }
    }
    'lni_ostia' {
      $f = $testo.Trim() -split '\s+'
      if ($f.Count -lt 75 -or $f[0] -ne '12345') { throw 'clientraw.txt non riconosciuto' }
      $t = [datetime]::ParseExact("$($f[74]) $($f[29]):$($f[30]):$($f[31])", 'd/M/yyyy H:m:s', $null)
      return @{ t = $t; wind = [double]$f[1]; gust = [double]$f[2]; deg = [double]$f[3]; offline = $false }
    }
    'rdt_civitavecchia' {
      $j = ConvertFrom-Json $testo
      $voci = @($j) | Where-Object { $_ -ne $null -and $_.PSObject.Properties['b_type'] -and $_.b_type -eq 'last_obs' }
      $v = $voci | Where-Object { $_.label -like 'Velocit*vento' } | Select-Object -First 1
      $g = $voci | Where-Object { $_.label -eq 'Raffica vento' } | Select-Object -First 1
      $d = $voci | Where-Object { $_.label -eq 'Direzione vento' } | Select-Object -First 1
      $t = [datetime]::ParseExact($v.time, 'dd/MM/yyyy HH:mm', $null)
      return @{ t = $t; wind = [double]$v.value; gust = [double]$g.value; deg = [double]$d.value; offline = $false }
    }
    'anzio_porto' {
      if ($testo -notmatch 'Aggiornamento:\s*<strong>([^<]+)</strong>') { throw 'ora non trovata' }
      $t = [datetime]::ParseExact($Matches[1].Trim(), 'dd/MM/yy HH:mm', $null)
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
Write-Host "Radar kite - $($adesso.ToString('dd/MM/yyyy HH:mm'))" -ForegroundColor Cyan
$spotRes = @()
foreach ($s in $SPOT) {
  Write-Host "  Windguru $($s.nome)... " -NoNewline
  try {
    $wg = Read-Windguru $s
    $giorni = Get-Giorni $s $wg.righe
    $spotRes += [pscustomobject]@{ s = $s; wg = $wg; giorni = $giorni; errore = $null }
    Write-Host "ok ($($wg.spot), init $($wg.init) UTC)" -ForegroundColor Green
  } catch {
    $spotRes += [pscustomobject]@{ s = $s; wg = $null; giorni = @(); errore = $_.Exception.Message }
    Write-Host "ERRORE: $($_.Exception.Message)" -ForegroundColor Red
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
    Write-Host ("{0} nodi da {1} alle {2}{3}" -f $l.wind, (Get-Comp $l.deg), $l.t.ToString('HH:mm'), $(if ($l.fresca) { '' } else { ' (vecchia)' })) -ForegroundColor Green
  } catch {
    $stRes[$st.id] = @{ errore = $_.Exception.Message }
    Write-Host "ERRORE: $($_.Exception.Message)" -ForegroundColor Red
  }
}

# Righe firing (previsione + stazione che segna gia' firing adesso)
$righeFiring = @()
foreach ($r in $spotRes) {
  $s = $r.s; $l = $stRes[$s.stazione]
  if ($l -and $l.fresca -and $l.wind -ge $SOGLIA -and (Get-Classe $s $l.deg) -ne 'terra') {
    $righeFiring += [pscustomobject]@{ ordine = 0; testo = ("{0} - ADESSO dalla stazione: {1} nodi da {2} alle {3}" -f $s.nome, $l.wind, (Get-Comp $l.deg), $l.t.ToString('HH:mm')); ideale = $false; pericolo = $false }
  }
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

# ---------------- Report HTML ----------------
function H([string]$x) { return [Net.WebUtility]::HtmlEncode($x) }
$sb = New-Object Text.StringBuilder
[void]$sb.Append(@"
<!DOCTYPE html><html lang="it"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Radar kite</title><style>
body{margin:0;background:#f6f5f1;color:#1d1d1b;font:15px/1.45 system-ui,'Segoe UI',sans-serif}
.w{max-width:980px;margin:0 auto;padding:18px}h1{font-weight:500;margin:0}h2{font-weight:500;font-size:19px;margin:22px 0 8px}
.sub{color:#6b6a65;font-size:13px;margin:2px 0 14px}.box{background:#fff;border:1px solid #e2e0d8;border-radius:10px;padding:12px 14px;margin-bottom:10px}
.f{border-color:#1f8a4c;box-shadow:inset 0 0 0 2px #1f8a4c}.f li{margin:4px 0}.danger{color:#c2412d;font-weight:600}
.days{display:grid;grid-template-columns:repeat(auto-fill,minmax(130px,1fr));gap:8px;margin-top:8px}
.d{border:1px solid #e2e0d8;border-radius:8px;padding:8px}.d.ok{background:#e6f4ec;border-color:#1f8a4c}
.big{font-size:22px}.m{color:#6b6a65;font-size:12px}a{color:inherit}table{border-collapse:collapse;width:100%}td,th{text-align:left;padding:6px;border-bottom:1px solid #e2e0d8;font-size:14px}
</style></head><body><div class="w">
<h1>Radar kite</h1><p class="sub">Report del $(H $adesso.ToString('dd/MM/yyyy HH:mm')) &middot; firing = almeno $SOGLIA nodi per $ORE_MIN ore di fila, da una direzione buona, nella fascia oraria utile.</p>
"@)
if ($righeFiring.Count) {
  [void]$sb.Append('<div class="box f"><b>&#x1F7E2; Firing</b><ul>')
  foreach ($f in $righeFiring) { $cls = if ($f.pericolo) { ' class="danger"' } else { '' }; [void]$sb.Append("<li$cls>$(H $f.testo)</li>") }
  [void]$sb.Append('</ul></div>')
} else {
  [void]$sb.Append('<div class="box"><b>Nessuna finestra di vento</b> nei prossimi giorni.</div>')
}
[void]$sb.Append('<h2>Stazioni adesso</h2><div class="box"><table><tr><th>Stazione</th><th>Vento</th><th>Raffica</th><th>Lettura</th></tr>')
foreach ($st in $STAZIONI) {
  $l = $stRes[$st.id]
  if ($l.errore) { [void]$sb.Append("<tr><td><a href='$($st.pagina)'>$(H $st.nome)</a></td><td colspan='3' class='m'>non raggiungibile: $(H $l.errore)</td></tr>"); continue }
  $stato = if ($l.offline) { 'offline' } elseif ($l.fresca) { 'aggiornata' } else { 'vecchia' }
  $raff = if ($l.gust -ne $null) { "$($l.gust)" } else { '&mdash;' }
  [void]$sb.Append("<tr><td><a href='$($st.pagina)'>$(H $st.nome)</a></td><td><b>$($l.wind)</b> nodi da $(Get-Comp $l.deg) ($([int]$l.deg)&deg;)</td><td>$raff</td><td>$($l.t.ToString('HH:mm')) &middot; $stato</td></tr>")
}
[void]$sb.Append('</table></div><h2>Spot</h2>')
foreach ($r in $spotRes) {
  $s = $r.s
  [void]$sb.Append("<div class='box'><b>$(H $s.nome)</b> <span class='m'>&middot; <a href='https://www.windguru.cz/$($s.wg)'>Windguru</a>")
  if ($r.errore) { [void]$sb.Append("</span><div class='danger'>Previsione non letta: $(H $r.errore)</div></div>"); continue }
  [void]$sb.Append(" &middot; $(H $r.wg.spot), GFS init $(H $r.wg.init) UTC</span><div class='days'>")
  foreach ($g in $r.giorni) {
    $nome = if ($g.oggi) { 'oggi' } else { $g.nome }
    if ($g.firing) {
      $raff = if ($g.raffica -ne $null) { " &middot; raffiche $($g.raffica)" } else { '' }
      $note = if ($g.note.Count) { "<div class='m'>$(H ($g.note -join ', '))</div>" } else { '' }
      [void]$sb.Append("<div class='d ok'><div class='m'>$nome</div><div class='big'>$($g.min)&ndash;$($g.maxf)</div><div class='m'>ore $($g.ore) &middot; $($g.dir)$raff</div>$note</div>")
    } else {
      $mx = if ($g.max -ne $null) { $g.max } else { '&mdash;' }
      [void]$sb.Append("<div class='d'><div class='m'>$nome</div><div class='big'>$mx</div><div class='m'>max, no firing</div></div>")
    }
  }
  [void]$sb.Append('</div></div>')
}
[void]$sb.Append("<p class='m'>Previsioni: Windguru (pagine micro). Stazioni lette direttamente dai siti. Vento live: <a href='https://mame-roma.github.io/MaMe-wind-radar/'>mame-roma.github.io/MaMe-wind-radar</a></p></div></body></html>")

$file = Join-Path $cartella 'report.html'
[IO.File]::WriteAllText($file, $sb.ToString(), (New-Object Text.UTF8Encoding($true)))
Write-Host ''
Write-Host "Report: $file"
if (-not $NonAprire) { Start-Process $file }
