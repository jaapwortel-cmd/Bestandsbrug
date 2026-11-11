# Bestandsbrug (Engels: Filebridge)

Windows-app in Rust van Jaap Wortel: een lijst met Jaaps apps uit een openbare pCloud-map downloaden, en zelf
een bestand naar Jaap uploaden (pCloud-uploadlink). Geen server, geen aanmelden. Ook als één los HTML-bestand.
De repository is OPENBAAR: nooit geheimen in code, tests, log of commits (zie "Veiligheid").

## Met Jaap praten

- Nederlands, eenvoudige taal (B1), met "je" en "jij". Jaap is blind (JAWS, soms NVDA) en werkt met het toetsenbord.
- Leg niet uit met regelnummers; zeg wat iets doet en wat Jaap kan testen.
- Jaap wil geen Microsoft Edge en geen WebView2 (afspraak in zijn andere projecten, 27 okt 2026). Daarom is het
  scherm gemaakt met echte Windows-onderdelen (Win32 via de crate `windows`), met eigen kleuren.

## Mappen

```
Cargo.toml, build.rs      bouwen; build.rs zet icoon, manifest en versie-info in de exe
bouw-alles.ps1            bouwt ALLES met één opdracht naar de map oplevering (Windows)
src/main.rs               alleen opstarten
src/lib.rs                de kern (werkt ook op Linux, voor de tests)
  instellingen.rs         ALLE vaste instellingen: versie, pCloud-codes, update_link, 6 seconden, limieten, technische namen
  taal.rs                 taal kiezen en teksten ophalen (uit talen/), app-naam per taal
  kleuren.rs              ALLE kleuren van licht en donker (app en HTML)
  log.rs                  logbestand (Engels, kort, max 500 KB)
  melding.rs              voortgangstekst en de timer van 6 seconden
  overdracht.rs           voortgang/stop (gedeeld met de achtergrond) en onderbroken downloads (overdrachten.json)
  pcloud/                 mod.rs (API, servers, foutcodes), lijst.rs (alle mappen, maak_boom), download.rs (Range/hervatten), upload.rs
  update.rs               versie.txt lezen, Dropbox (los of hele map als zip), SHA-256
  net.rs                  gedeelde internetverbinding (ureq 3), tijdslimieten, foutvertaling
  fout.rs, opmaak.rs, tijd.rs, paden.rs, voorkeuren.rs, html.rs (maakt Bestandsbrug.html)
src/bin/maak_html.rs      maakt Bestandsbrug.html
src/scherm/               het Windows-scherm (alleen Windows)
  mod.rs                  opstarten, berichtenlus, sneltoetsen, toestand (STAAT), zeg()/zeg_later()
  venster.rs              hoofdvenster en alle berichten (WM_*), menukeuzes -> opdracht()
  hoofdscherm.rs          onderdelen maken, teksten, indeling, tabbladen, taal en weergave wisselen
  menu.rs                 menubalk; radio-vinkjes bij Taal en Toegankelijkheid
  boom.rs                 de boom (TreeView) met mappen en bestanden; keuze, open/dicht, open mappen bewaren
  scherm_download.rs      lijst tonen/vernieuwen, downloaden, onderbroken download bij de start
  scherm_upload.rs        bestand kiezen en uploaden
  overdracht_ui.rs        lopende overdracht: balk, melding elke 6 s, Escape-vraag, afsluiten
  update_ui.rs            updatecontrole (automatisch en via Help), update downloaden en starten
  vragen.rs               vragen (TaskDialog): Ja/Nee, Escape-vraag, meldingen
  tekstvenster.rs         Uitleg en Privacyverklaring in een eigen venster; Over
  spraak.rs               meldingen aan de schermlezer (JAWS-API, anders UI Automation) en de balk-waarde
  thema.rs, tekenen.rs    kleuren/letters en het tekenen (knoppen, tabbladen, balk, kaarten)
  dialogen.rs, win.rs     standaardvensters van Windows (opslaan/openen, Verkenner) en kleine hulpjes
talen/                    nl.txt, en.txt (alle teksten), uitleg-nl/en.txt, privacy-nl/en.txt (de ENIGE bron)
web/bestandsbrug.html     bron van Bestandsbrug.html (HTML, CSS, JS; gegevens worden ingevuld)
installatie/bestandsbrug.iss  Inno Setup-script
assets/                   icoon en manifest
tests/                    tests per onderdeel; tests/echt.rs gebruikt de echte links (--ignored)
tools/Toegankelijkheid.ps1  FlaUI-controle (zelfde script als in Jaaps andere programma's)
.github/workflows/bouw.yml  bouwronde op Windows; oplevering op de tak exe-builds
```

## Wil je X veranderen? Kijk in bestand Y

| Wat | Waar |
|---|---|
| Versienummer | `src/instellingen.rs` (VERSIE); bouw-alles.ps1 zet Cargo.toml gelijk. Plus een blok in `Wijzigingen.txt` |
| pCloud-codes, updatelink, 6 seconden, tijdslimieten | `src/instellingen.rs` |
| Een tekst, knop of melding | `talen/nl.txt` EN `talen/en.txt` (zelfde sleutel; test controleert) |
| De naam (Bestandsbrug/Filebridge) | `AppNaam` in de taalbestanden; nooit vast in de code (test controleert) |
| Privacyverklaring | `talen/privacy-nl.txt` en `talen/privacy-en.txt` (woord voor woord van Jaap; niet zelf aanpassen) |
| Uitleg (F1) | `talen/uitleg-nl.txt` en `talen/uitleg-en.txt` |
| Kleuren licht/donker | `src/kleuren.rs` (test controleert contrast) |
| Menu | `src/scherm/menu.rs` + teksten + `opdracht()` in `src/scherm/venster.rs` |
| Sneltoetsen | `voor_toets()` in `src/scherm/mod.rs` |
| Indeling op het scherm | `indeling()` in `src/scherm/hoofdscherm.rs` |
| Uiterlijk (knoppen, tabbladen, balk) | `src/scherm/tekenen.rs` |
| Voortgangsmelding | `src/melding.rs` (tekst) en `src/scherm/overdracht_ui.rs` (wanneer) |
| Download/upload | `src/pcloud/` |
| Updates | `src/update.rs` en `src/scherm/update_ui.rs`; versie.txt maakt bouw-alles.ps1 |
| HTML-versie | `web/bestandsbrug.html` (+ `src/html.rs` voor de gegevens) |
| Installatieprogramma | `installatie/bestandsbrug.iss` |
| Logregels | overal `bestandsbrug::log::info/fout` (Engels, kort, geen paden of persoonlijke gegevens) |

## Bouwen en testen

- Alles (Windows): `powershell -ExecutionPolicy Bypass -File .\bouw-alles.ps1` -> map `oplevering`.
- Tests (ook op Linux): `cargo test`. Echte pCloud: `cargo test --test echt -- --ignored --nocapture`
  (de upload-test alleen met `BESTANDSBRUG_ECHTE_UPLOAD=1`; die zet een bestand in Jaaps map Uploads).
- Controleren of het scherm bouwt vanaf Linux: `cargo build --target x86_64-pc-windows-gnu`
  (nodig: `rustup target add x86_64-pc-windows-gnu` en `apt-get install gcc-mingw-w64-x86-64`).
  Starten kan met Wine (`wine64 Bestandsbrug.exe --geen-update`) op Xvfb; internet werkt dan niet.
- HTML maken: `cargo run --bin maak-html Bestandsbrug.html`; proef met Playwright (Chromium) vanaf file://.
- Opties van de exe (voor proeven): `--taal nl|en`, `--weergave licht|donker` (niet bewaard), `--meet bestand.txt`
  (meetcijfers, sluit na de lijst), `--sluit-na N` (sluit na N seconden zonder vraag), `--geen-update`.
- GitHub (`bouw.yml`, windows-latest): bouw-alles.ps1, echte pCloud-proef, installeren/opstarten/meten,
  update over de oude heen (instellingen blijven, app start weer), FlaUI-toegankelijkheid, schermafbeeldingen,
  verwijderen. Resultaat op de tak `exe-builds` (map oplevering, met meting.txt en scherm-*.png).
  De repo is openbaar: GitHub Actions kost dan geen minuten van Jaap.

## pCloud (gecontroleerd 11 nov 2026)

- Server: `eapi.pcloud.com` (Europa); `api.pcloud.com` geeft 7001 voor deze codes, blijft als reserve.
- `showpublink?code=` (lijst), `getpublinkdownload?code=&fileid=&forcedownload=1` (adres: hosts[0]+path;
  Range werkt, 206), `showuploadlink?code=` (maxspace/maxfiles, 0 = geen grens; geeft ook Jaaps e-mail: NIET loggen),
  `uploadtolink?code=&names=&nopartial=1` (multipart POST, Content-Length vooraf; geen hervatten mogelijk).
- CORS: de API geeft `Access-Control-Allow-Origin: *` (ook vanaf file://). De downloadservers niet: in de
  HTML-versie gaat downloaden daarom via de browser zelf (geen eigen voortgang).

## Updates

- Dropbox-map "Bestandsbrug Updates" (alleen-lezen link in instellingen.rs). Erin: Bestandsbrug-Setup.exe en versie.txt.
- Eerst `<map>/versie.txt?rlkey=..&dl=1`; lukt dat niet, dan de hele map als zip (zoals Jaaps andere apps).
- SHA-256 moet kloppen, anders niets installeren. Installeren: `/SILENT /LANG=..`; het installatieprogramma start de app weer.

## Veiligheid (openbare repo)

- Alleen de twee openbare pCloud-codes en de ALLEEN-LEZEN Dropbox-link. Nooit een wachtwoord, token, API-sleutel
  of de Dropbox-link met schrijfrechten (die heeft Jaap; niet om vragen).
- Geen licentie (Jaaps keuze, 11 nov 2026): voeg er geen toe en vraag er niet meer naar.
- Voor elke push: zoeken naar "token", "secret", "password", "rlkey", e-mailadressen (zie het eindrapport).

## Stand van zaken

- 1.0.0 (11 nov 2026): eerste versie, alles uit de opdracht. Nog door Jaap te testen met JAWS.
- Bouwronde (11 nov): alles groen. FlaUI-controle: namen en Tab-volgorde goed; de menu-controle van het script
  is gemaakt voor WinForms-menu's en vindt gewone Win32-menu's niet (met F10/Alt werken ze wel; gecontroleerd met Wine).
- Jaap zette op 11 nov de apps in submappen. 1.0.1 (12 nov, Jaaps wens): mappen zichtbaar als boom met de bestanden
  eronder (Windows: TreeView; HTML: details/summary), alle diepten. Installatiefout "Cannot focus a disabled or
  invisible window" opgelost (focus alleen op een zichtbare, ingeschakelde knop in CurPageChanged).
- Jaap: Controleren op updates werkt met zijn Dropbox-map (1.0.0 stond erin).
