# Bouwt alles met één opdracht: de app (Rust, release), de tests, Bestandsbrug.html, het installatiebestand
# (Inno Setup) en versie.txt. Alles komt in de map "oplevering".
# Gebruik (in PowerShell, in deze map):  powershell -ExecutionPolicy Bypass -File .\bouw-alles.ps1
# Ontbreekt Rust of Inno Setup, dan installeert het script die met winget (gratis) en zegt het dat.
# Het versienummer komt uit src\instellingen.rs (VERSIE). Dat is de enige plek.
param([switch]$GeenTests)
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
$utf8bom = New-Object System.Text.UTF8Encoding($true)

function Schrijf-Tekst([string]$pad, [string]$tekst) {
    $tekst = ($tekst -replace "`r`n", "`n") -replace "`n", "`r`n"
    [IO.File]::WriteAllText($pad, $tekst, $utf8bom)
}

# 1. Versie uit src\instellingen.rs, en Cargo.toml gelijk zetten.
$inst = Get-Content -LiteralPath 'src\instellingen.rs' -Raw
if ($inst -notmatch 'pub const VERSIE: &str = "([0-9.]+)";') { throw 'Geen VERSIE gevonden in src\instellingen.rs' }
$versie = $Matches[1]
Write-Host "Bestandsbrug versie $versie"
$cargo = Get-Content -LiteralPath 'Cargo.toml' -Raw
$nieuw = [regex]::Replace($cargo, '(?m)^version = "[^"]*"', "version = `"$versie`"", 1)
if ($nieuw -ne $cargo) { [IO.File]::WriteAllText("$PSScriptRoot\Cargo.toml", $nieuw); Write-Host 'Cargo.toml: versie gelijk gezet.' }

# 2. Gereedschap: Rust en Inno Setup.
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host 'Rust ontbreekt. Ik installeer Rust met winget (Rustlang.Rustup).'
    winget install --id Rustlang.Rustup -e --silent --accept-package-agreements --accept-source-agreements
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
}
$iscc = @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe", "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) {
    Write-Host 'Inno Setup ontbreekt. Ik installeer Inno Setup met winget (JRSoftware.InnoSetup).'
    winget install --id JRSoftware.InnoSetup -e --silent --accept-package-agreements --accept-source-agreements
    $iscc = @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe", "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $iscc) { throw 'Inno Setup niet gevonden na installeren.' }
}

# 3. Tests en de app zelf (release: snel en klein).
if (-not $GeenTests) {
    cargo test --release
    if ($LASTEXITCODE) { throw 'De tests zijn niet geslaagd.' }
}
cargo build --release --bins
if ($LASTEXITCODE) { throw 'Bouwen mislukt.' }

# 4. Mappen klaarzetten.
$oplevering = Join-Path $PSScriptRoot 'oplevering'
$installatiemap = Join-Path $PSScriptRoot 'installatiemap'
foreach ($m in @($oplevering, $installatiemap)) {
    if (Test-Path $m) { Remove-Item -LiteralPath $m -Recurse -Force }
    New-Item -ItemType Directory -Path $m | Out-Null
}
Copy-Item -LiteralPath 'target\release\bestandsbrug.exe' -Destination (Join-Path $installatiemap 'Bestandsbrug.exe')
# De privacyverklaring: de enige bron is talen\privacy-nl.txt en talen\privacy-en.txt.
$privacyNl = [IO.File]::ReadAllText("$PSScriptRoot\talen\privacy-nl.txt")
$privacyEn = [IO.File]::ReadAllText("$PSScriptRoot\talen\privacy-en.txt")
foreach ($m in @($oplevering, $installatiemap)) {
    Schrijf-Tekst (Join-Path $m 'Privacyverklaring.txt') $privacyNl
    Schrijf-Tekst (Join-Path $m 'Privacy-statement.txt') $privacyEn
    Schrijf-Tekst (Join-Path $m 'LEESMIJ.txt') ([IO.File]::ReadAllText("$PSScriptRoot\LEESMIJ.txt"))
    Schrijf-Tekst (Join-Path $m 'Wijzigingen.txt') ([IO.File]::ReadAllText("$PSScriptRoot\Wijzigingen.txt"))
}

# 5. Het losse HTML-bestand.
& 'target\release\maak-html.exe' (Join-Path $oplevering 'Bestandsbrug.html')
if ($LASTEXITCODE) { throw 'Bestandsbrug.html maken mislukt.' }

# 6. Het installatiebestand.
& $iscc "/DVersie=$versie" '/Qp' 'installatie\bestandsbrug.iss'
if ($LASTEXITCODE) { throw 'Installatiebestand maken mislukt.' }
$setup = Join-Path $oplevering 'Bestandsbrug-Setup.exe'
if (-not (Test-Path $setup)) { throw 'Bestandsbrug-Setup.exe niet gevonden.' }

# 7. versie.txt voor de updates: versie, bestandsnaam, SHA-256 en wat er nieuw is (uit Wijzigingen.txt).
$sha = (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash.ToLowerInvariant()
$nl = @(); $en = @(); $binnen = $false
foreach ($regel in Get-Content -LiteralPath 'Wijzigingen.txt' -Encoding UTF8) {
    if ($regel -match '^Versie\s+([0-9.]+)') { $binnen = ($Matches[1] -eq $versie); continue }
    if (-not $binnen) { continue }
    if ($regel -match '^nl:\s*(.+)$') { $nl += $Matches[1] }
    if ($regel -match '^en:\s*(.+)$') { $en += $Matches[1] }
}
if ($nl.Count -eq 0 -or $en.Count -eq 0) { throw "Zet in Wijzigingen.txt bij Versie $versie een regel nl: en een regel en:." }
$v = @("versie = $versie", 'bestand = Bestandsbrug-Setup.exe', "sha256 = $sha")
$v += $nl | ForEach-Object { "nieuw_nl = $_" }
$v += $en | ForEach-Object { "nieuw_en = $_" }
Schrijf-Tekst (Join-Path $oplevering 'versie.txt') (($v -join "`n") + "`n")

Remove-Item -LiteralPath $installatiemap -Recurse -Force
Write-Host ''
Write-Host "Klaar. In de map oplevering staan:"
Get-ChildItem -LiteralPath $oplevering | ForEach-Object { Write-Host ('  ' + $_.Name + ' (' + [math]::Round($_.Length / 1KB) + ' KB)') }
Write-Host "Voor een update: zet Bestandsbrug-Setup.exe en versie.txt in de Dropbox-map Bestandsbrug Updates."
