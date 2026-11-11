# Toegankelijkheidscontrole (Jaaps wens, 8 nov 2026), met FlaUI (github.com/FlaUI/FlaUI, UI Automation 3, de techniek
# die ook moderne schermlezers gebruiken). Start het programma en kijkt naar het hoofdvenster:
#  1. elk onderdeel dat je kunt bedienen (knop, vak, lijst, vinkje, tabblad, ...) heeft een naam;
#  2. de Tab-volgorde: met Tab kom je langs de onderdelen, elk met een naam, en je blijft in het programma;
#  3. de menu's: elk menu en elke menuregel (eerste laag) heeft een naam.
# Staat er bij de start een vraag of melding open, dan wordt die genoemd en gesloten.
# Gebruik: powershell -File tools\Toegankelijkheid.ps1 -Exe 'uit\Programma.exe' [-Argumenten '--proefstart'] [-Wacht 8]
# Geeft exitcode 1 als er iets mis is; het rapport staat in de log en in de samenvatting van de bouwronde.
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [string]$Argumenten = '',
    [int]$Wacht = 8,
    [string]$Rapport = 'toegankelijkheid.txt'
)
$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# ---------- FlaUI ophalen (NuGet), alleen de eerste keer ----------
$map = Join-Path ([IO.Path]::GetTempPath()) 'flaui-4.0.0'
$pakketten = @(
    @{ Naam = 'Interop.UIAutomationClient'; Versie = '10.19041.0'; Dll = 'Interop.UIAutomationClient.dll' },
    @{ Naam = 'FlaUI.Core'; Versie = '4.0.0'; Dll = 'FlaUI.Core.dll' },
    @{ Naam = 'FlaUI.UIA3'; Versie = '4.0.0'; Dll = 'FlaUI.UIA3.dll' }
)
New-Item -ItemType Directory -Path $map -Force | Out-Null
Add-Type -AssemblyName System.IO.Compression.FileSystem
foreach ($pk in $pakketten) {
    $dll = Join-Path $map $pk.Dll
    if (-not (Test-Path $dll)) {
        $zip = Join-Path $map ($pk.Naam + '.zip')
        Invoke-WebRequest -UseBasicParsing -Uri ('https://www.nuget.org/api/v2/package/' + $pk.Naam + '/' + $pk.Versie) -OutFile $zip
        $z = [IO.Compression.ZipFile]::OpenRead($zip)
        try {
            # De versie voor .NET Framework (net4x) als die er is, anders netstandard.
            $kandidaten = @($z.Entries | Where-Object { $_.Name -eq $pk.Dll -and $_.FullName -like 'lib/*' })
            $keuze = $kandidaten | Where-Object { $_.FullName -match '^lib/net4' } | Sort-Object FullName -Descending | Select-Object -First 1
            if (-not $keuze) { $keuze = $kandidaten | Where-Object { $_.FullName -match '^lib/netstandard' } | Sort-Object FullName -Descending | Select-Object -First 1 }
            if (-not $keuze) { throw ($pk.Dll + ' niet gevonden in het pakket.') }
            [IO.Compression.ZipFileExtensions]::ExtractToFile($keuze, $dll, $true)
        } finally { $z.Dispose() }
    }
    Add-Type -Path $dll
}

$regels = New-Object System.Collections.Generic.List[string]
$fouten = New-Object System.Collections.Generic.List[string]
function Schrijf([string]$t) { $regels.Add($t); Write-Host $t }
function Fout([string]$t) { $fouten.Add($t); $regels.Add('FOUT: ' + $t); Write-Host ('FOUT: ' + $t) }

$CT = [FlaUI.Core.Definitions.ControlType]
function Beschrijf($e) {
    try {
        $d = [string]$e.ControlType
        if ($e.Name) { $d += ' "' + $e.Name + '"' } else { $d += ' (zonder naam)' }
        $id = $e.Properties.AutomationId.ValueOrDefault
        if ($id) { $d += ' [id ' + $id + ']' }
        $k = $e.Properties.ClassName.ValueOrDefault
        if ($k) { $d += ' {' + $k + '}' }
        return $d
    } catch { return '(onbekend onderdeel)' }
}
function HeeftNaam($e) { try { return [bool]($e.Name -and $e.Name.Trim()) } catch { return $false } }
function Toets([string]$naam) { [FlaUI.Core.Input.Keyboard]::Type([FlaUI.Core.WindowsAPI.VirtualKeyShort]::$naam); Start-Sleep -Milliseconds 150 }

# Onderdelen die je bedient en dus een naam moeten hebben.
$bedienbaar = @($CT::Button, $CT::Edit, $CT::ComboBox, $CT::List, $CT::CheckBox, $CT::RadioButton, $CT::Tree,
    $CT::Slider, $CT::Spinner, $CT::Tab, $CT::TabItem, $CT::Hyperlink, $CT::DataGrid, $CT::Table, $CT::Document, $CT::SplitButton)

$psi = New-Object System.Diagnostics.ProcessStartInfo((Resolve-Path $Exe).Path)
if ($Argumenten) { $psi.Arguments = $Argumenten }
$psi.WorkingDirectory = Split-Path (Resolve-Path $Exe).Path
$app = [FlaUI.Core.Application]::Launch($psi)
$auto = New-Object FlaUI.UIA3.UIA3Automation
try {
    Start-Sleep -Seconds $Wacht
    if ($app.HasExited) { throw ('Het programma stopte meteen, code ' + $app.ExitCode) }
    $venster = $app.GetMainWindow($auto, [TimeSpan]::FromSeconds(20))
    if (-not $venster) { throw 'Het hoofdvenster is niet gevonden.' }
    Schrijf ('Programma: ' + (Split-Path $Exe -Leaf) + ', venster "' + $venster.Title + '"')
    if (-not $venster.Title) { Fout 'Het venster zelf heeft geen titel.' }

    # Een vraag of melding die bij de start openstaat: noemen en sluiten (anders kom je met Tab niet in het venster).
    # Eerst met Nee/Annuleren/Sluiten/OK, anders met Escape, en als laatste met Sluiten van het venster zelf.
    $gemeld = @{}
    for ($i = 0; $i -lt 5; $i++) {
        $open = @($venster.ModalWindows)
        if ($open.Count -eq 0) { break }
        foreach ($m in $open) {
            $titel = $m.Title
            $tekst = (@($m.FindAllDescendants($auto.ConditionFactory.ByControlType($CT::Text)) | ForEach-Object { $_.Name }) -join ' ').Trim()
            if (-not $gemeld.ContainsKey($titel + $tekst)) { $gemeld[$titel + $tekst] = 1; Schrijf ('Bij de start stond een venster open: "' + $titel + '" ' + $tekst + ' (wordt gesloten).') }
            $knop = $null
            foreach ($k in $m.FindAllDescendants($auto.ConditionFactory.ByControlType($CT::Button))) {
                if ($k.Name -match '^\s*(&?Nee|&?No|&?Annuleren|&?Cancel|&?Sluiten|&?Close|&?OK)\s*$') { $knop = $k; break }
            }
            try {
                if ($knop) { (New-Object FlaUI.Core.AutomationElements.Button($knop.FrameworkAutomationElement)).Invoke() }
                elseif ($i -lt 3) { $m.Focus(); Toets 'ESCAPE' }
                else { $m.Close() }
            } catch { Toets 'ESCAPE' }
        }
        Start-Sleep -Seconds 1
    }
    if (@($venster.ModalWindows).Count -gt 0) { Fout 'Een venster dat bij de start openstond ging niet dicht; de Tab-volgorde is dan niet te controleren.' }

    # ---------- 1. Namen van alle onderdelen ----------
    Schrijf ''
    Schrijf '1. Namen van de onderdelen'
    $bekeken = 0
    foreach ($e in $venster.FindAllDescendants()) {
        try {
            $soort = $e.ControlType
            if ($soort -eq $CT::TitleBar -or $soort -eq $CT::ScrollBar -or $soort -eq $CT::Thumb -or $soort -eq $CT::MenuBar -or $soort -eq $CT::MenuItem) { continue }
            $ouder = $e.Parent
            if ($ouder -and ($ouder.ControlType -eq $CT::TitleBar -or $ouder.ControlType -eq $CT::ScrollBar)) { continue }
            $focusbaar = $e.Properties.IsKeyboardFocusable.ValueOrDefault
            # Knopjes binnen een schuifbalk, boom of keuzelijst (WPF) waar je niet met Tab komt: die zijn van het onderdeel zelf.
            if ($soort -eq $CT::Button -and -not $focusbaar -and $ouder -and (@($CT::Slider, $CT::TreeItem, $CT::ComboBox, $CT::Spinner, $CT::ScrollBar) -contains $ouder.ControlType)) { continue }
            # Plaatjes, tekst en balken die je niet met Tab bereikt (zoals een niveaumeter) tellen niet mee.
            $versiering = @($CT::Pane, $CT::Window, $CT::Image, $CT::Text, $CT::ProgressBar, $CT::Group, $CT::StatusBar, $CT::ToolTip)
            if ($bedienbaar -contains $soort -or ($focusbaar -and -not ($versiering -contains $soort))) {
                $bekeken++
                if (-not (HeeftNaam $e)) { Fout ('Geen naam: ' + (Beschrijf $e)) }
            }
        } catch { }
    }
    Schrijf ("Bekeken: $bekeken onderdelen.")

    # ---------- 2. Tab-volgorde ----------
    Schrijf ''
    Schrijf '2. Tab-volgorde'
    try { $venster.SetForeground() } catch { }
    Start-Sleep -Milliseconds 500
    $eerste = $null; $vorige = $null; $rond = $false; $vast = 0; $foutenVoor = $fouten.Count; $menuInTab = $false
    for ($i = 1; $i -le 40; $i++) {
        Toets 'TAB'
        Start-Sleep -Milliseconds 250
        $f = $auto.FocusedElement()
        if (-not $f) { Fout "Na Tab $i heeft niets de focus."; break }
        if ($f.Properties.ProcessId.ValueOrDefault -ne $app.ProcessId) { Fout ("Na Tab $i staat de focus buiten het programma: " + (Beschrijf $f)); break }
        $sleutel = [string]::Join(',', $f.Properties.RuntimeId.ValueOrDefault)
        Schrijf ("  Tab ${i}: " + (Beschrijf $f))
        if (-not (HeeftNaam $f)) { Fout ("Na Tab $i een onderdeel zonder naam: " + (Beschrijf $f)) }
        if ($f.ControlType -eq $CT::MenuItem) { $menuInTab = $true }
        if ($null -eq $eerste) { $eerste = $sleutel }
        elseif ($sleutel -eq $eerste) { $rond = $true; Schrijf ("  Na $($i - 1) onderdelen ben je weer bij het begin: goed."); break }
        if ($sleutel -eq $vorige) { $vast++ } else { $vast = 0 }
        if ($vast -ge 2) { Fout ('De focus blijft hangen op: ' + (Beschrijf $f) + ' (Tab komt er niet meer uit).'); break }
        $vorige = $sleutel
    }
    if ($menuInTab) { Schrijf '  LET OP: met Tab kom je ook langs de menu''s. In Windows doe je dat meestal alleen met Alt of F10.' }
    if (-not $rond -and $fouten.Count -eq $foutenVoor) { Schrijf '  Na 40 keer Tab nog niet terug bij het begin (veel onderdelen; geen fout).' }

    # ---------- 3. Menu's ----------
    Schrijf ''
    Schrijf "3. Menu's"
    $cf = $auto.ConditionFactory
    $balk = $null
    foreach ($b in $venster.FindAllDescendants($cf.ByControlType($CT::MenuBar).Or($cf.ByControlType($CT::Menu)))) {
        $namen = @($b.FindAllChildren() | ForEach-Object { $_.Name })
        if ($namen.Count -gt 0 -and -not ($namen -contains 'System' -or $namen -contains 'Systeem')) { $balk = $b; break }
    }
    if (-not $balk) { Schrijf '  Geen menubalk gevonden.' }
    else {
        $menus = @($balk.FindAllChildren($cf.ByControlType($CT::MenuItem)))
        for ($m = 0; $m -lt $menus.Count; $m++) {
            $mi = New-Object FlaUI.Core.AutomationElements.MenuItem($menus[$m].FrameworkAutomationElement)
            $naam = $mi.Name
            if (-not $naam) { Fout ("Menu $($m + 1) heeft geen naam.") }
            $items = @()
            try { $items = @($mi.Items) } catch { }
            if ($items.Count -eq 0) {
                # Reserve: met het toetsenbord openen, zoals een schermlezergebruiker het doet, en de uitgeklapte lijst zoeken.
                try { $venster.SetForeground() } catch { }
                Toets 'ESCAPE'; Toets 'ESCAPE'; Toets 'F10'
                for ($r = 0; $r -lt $m; $r++) { Toets 'RIGHT' }
                Toets 'DOWN'
                Start-Sleep -Milliseconds 600
                foreach ($w in $auto.GetDesktop().FindAllChildren($cf.ByProcessId($app.ProcessId))) {
                    if ($w.Properties.NativeWindowHandle.ValueOrDefault -eq $venster.Properties.NativeWindowHandle.ValueOrDefault) { continue }
                    $items += @($w.FindAllDescendants($cf.ByControlType($CT::MenuItem)))
                }
            }
            Schrijf ('  Menu ' + ($m + 1) + ' "' + $naam + '": ' + $items.Count + ' regels')
            if ($items.Count -eq 0) { Fout ('Menu "' + $naam + '" ging niet open of heeft geen regels.') }
            foreach ($it in $items) { if (-not (HeeftNaam $it)) { Fout ('In menu "' + $naam + '" een regel zonder naam: ' + (Beschrijf $it)) } }
            try { [void]$mi.Collapse() } catch { }
            Toets 'ESCAPE'; Toets 'ESCAPE'; Toets 'ESCAPE'
            Start-Sleep -Milliseconds 200
        }
    }
}
finally {
    try { if (-not $app.HasExited) { [void]$app.Close(); Start-Sleep -Seconds 3; if (-not $app.HasExited) { $app.Kill() } } } catch { }
    try { $auto.Dispose() } catch { }
}

Schrijf ''
if ($fouten.Count -eq 0) { Schrijf 'Toegankelijkheid: alles goed.' } else { Schrijf ('Toegankelijkheid: ' + $fouten.Count + ' dingen om te verbeteren.') }
$regels | Set-Content -Path $Rapport -Encoding UTF8
if ($env:GITHUB_STEP_SUMMARY) {
    $samenvatting = @('## Toegankelijkheid', '', '```') + $regels + @('```')
    $samenvatting | Add-Content -Path $env:GITHUB_STEP_SUMMARY -Encoding UTF8
}
if ($fouten.Count -gt 0) { exit 1 }
