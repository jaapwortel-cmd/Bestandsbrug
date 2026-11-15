//! Tabblad Downloaden: de lijst tonen (eerst die van de vorige keer, dan de nieuwe), vernieuwen op de
//! achtergrond, een bestand downloaden (vragen waar, en of overschrijven mag) en een onderbroken
//! download van de vorige keer weer oppakken.

use bestandsbrug::fout::Uitkomst;
use bestandsbrug::overdracht;
use bestandsbrug::pcloud::lijst::{self, Bestand};
use bestandsbrug::taal::{f, t};
use bestandsbrug::log;
use windows::Win32::UI::WindowsAndMessaging::SetTimer;

use super::overdracht_ui::{self, Taak};
use super::win;
use super::*;

/// Zet de bestanden op het scherm (de map die open is). Je blijft in dezelfde map en op dezelfde regel.
pub fn vul_lijst(lijst: &[Bestand]) {
    let oud = met(|s| s.lijst.clone()).unwrap_or_default();
    super::verkenner::vul(lijst, &oud);
}

/// Bewaar en toon een lijst.
pub fn toon_lijst(nieuw: Vec<Bestand>, van_internet: bool) {
    let anders = met(|s| s.lijst != nieuw).unwrap_or(true);
    // Eerst bewaren, dan tonen: de knoppen (Bijwerken) kijken naar de bewaarde lijst.
    let oud = met(|s| {
        s.lijst_van_internet |= van_internet;
        std::mem::replace(&mut s.lijst, nieuw.clone())
    })
    .unwrap_or_default();
    if anders {
        super::verkenner::vul(&nieuw, &oud);
    }
}

/// De geïnstalleerde programma's opnieuw lezen. Is er iets veranderd (een programma erbij, een nieuwe versie),
/// dan de lijst opnieuw tonen, op dezelfde plek.
pub fn programmas_bijwerken() {
    let nieuw = super::programmas::lees();
    let anders = met(|s| {
        let anders = s.programmas != nieuw;
        if anders {
            s.programmas = nieuw;
        }
        anders
    })
    .unwrap_or(false);
    if anders {
        let lijst = met(|s| s.lijst.clone()).unwrap_or_default();
        super::verkenner::vul(&lijst, &lijst);
    }
}

/// Haal de lijst op de achtergrond op. `handmatig` = de gebruiker vroeg erom (F5 of de knop).
pub fn vernieuw(handmatig: bool) {
    if handmatig {
        zeg(&t("LijstLaden"), Belang::Gewoon);
    }
    let h = hwnd().0 as isize;
    std::thread::spawn(move || {
        let uitkomst: Box<Uitkomst<Vec<Bestand>>> = Box::new(lijst::haal_lijst());
        win::post(windows::Win32::Foundation::HWND(h as *mut _), WM_LIJST_KLAAR, handmatig as usize, Box::into_raw(uitkomst) as isize);
    });
}

pub fn lijst_klaar(uitkomst: Uitkomst<Vec<Bestand>>, handmatig: bool) {
    let (oud, eerste_keer, start) = met(|s| (s.lijst.clone(), !s.lijst_van_internet, s.start)).unwrap_or((Vec::new(), true, std::time::Instant::now()));
    match uitkomst {
        Ok(nieuw) => {
            log::info(&format!("List loaded: {} files", nieuw.len()));
            if eerste_keer {
                log_meting(format!("List from internet after {} ms", start.elapsed().as_millis()));
            }
            lijst::bewaar_cache(&nieuw);
            // Wat is er nieuw sinds de vorige keer? (De allereerste keer is niets "nieuw".)
            let gezien = lijst::laad_gezien();
            let erbij = gezien.as_ref().map(|g| lijst::nieuw_sinds(g, &nieuw)).unwrap_or_default();
            lijst::bewaar_gezien(&nieuw);
            met(|s| s.nieuw.extend(erbij.iter().map(|&i| lijst::sleutel(&nieuw[i]))));
            if !erbij.is_empty() {
                log::info(&format!("New since last time: {} files", erbij.len()));
            }
            let mut delen = Vec::new();
            if handmatig || (gezien.is_none() && oud.is_empty()) {
                delen.push(match nieuw.len() {
                    0 => t("LijstLeeg"),
                    1 => t("LijstEenBestand"),
                    n => f("LijstGeladen", &[&n.to_string()]),
                });
            }
            if !erbij.is_empty() {
                delen.push(nieuws_tekst(&nieuw, &erbij));
            }
            toon_lijst(nieuw, true);
            programmas_bijwerken();
            // Een korte melding; de focus blijft waar hij is.
            if !delen.is_empty() {
                zeg(&delen.join(" "), if handmatig { Belang::Belangrijk } else { Belang::Gewoon });
            }
            if eerste_keer {
                if met(|s| s.opties.meet.is_some()).unwrap_or(false) {
                    unsafe {
                        SetTimer(Some(hwnd()), TIMER_SLUIT, 1000, None);
                    }
                }
                if met(|s| std::mem::take(&mut s.onderbroken_vragen)).unwrap_or(false) {
                    vraag_onderbroken();
                }
            }
        }
        Err(e) => {
            log::fout(&format!("List failed: {}", e.log_tekst()));
            if handmatig || oud.is_empty() {
                zeg(&f("FoutLijst", &[&e.tekst()]), Belang::Belangrijk);
            }
        }
    }
}

/// Enter of de knop Downloaden.
/// "Nieuw sinds de vorige keer: X in de map Y." Bij veel nieuwe bestanden alleen het aantal.
fn nieuws_tekst(lijst: &[Bestand], nieuw: &[usize]) -> String {
    if nieuw.len() > 3 {
        return f("NieuwVeel", &[&nieuw.len().to_string()]);
    }
    let namen: Vec<String> = nieuw
        .iter()
        .map(|&i| {
            let b = &lijst[i];
            if b.map.is_empty() { b.naam.clone() } else { f("NieuwInMap", &[&b.naam, &b.map.replace('/', " > ")]) }
        })
        .collect();
    f("NieuwSinds", &[&namen.join(", ")])
}

/// Enter of dubbelklik in de lijst: een map openen (zoals in de Verkenner), een bestand downloaden.
pub fn enter_in_lijst() {
    if let Some(super::verkenner::Regel::Map { pad, .. }) = super::verkenner::keuze() {
        super::verkenner::open(&pad);
        return;
    }
    start_download();
}

/// De knop Downloaden, Ctrl+C of het menu: een bestand downloaden, of een hele map kopiëren naar de computer.
pub fn start_download() {
    let bestand = match super::verkenner::keuze() {
        Some(super::verkenner::Regel::Map { naam, pad, .. }) => {
            kopieer_map(&naam, &pad);
            return;
        }
        Some(super::verkenner::Regel::Bestand(i)) => met(|s| s.lijst.get(i).cloned()).flatten(),
        None => None,
    };
    if overdracht_ui::bezig() {
        zeg(&t("AlBezig"), Belang::Belangrijk);
        return;
    }
    let Some(b) = bestand else {
        zeg(&t("GeenKeuze"), Belang::Belangrijk);
        return;
    };
    let Some(doel) = dialogen::kies_opslaan(&b.naam) else { return };
    if doel.exists() && !vragen::ja_nee(hwnd(), &f("Overschrijven", &[&doel.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()]), false) {
        return;
    }
    let naam = b.naam.clone();
    overdracht_ui::start(Taak::Download { bestand: b, doel, starten: false }, naam);
}

/// De knop Downloaden (of het menu): heet de knop nu Bijwerken, dan de nieuwe versie installeren;
/// anders gewoon downloaden. (Ctrl+C kopieert altijd het bestand of de hele map.)
pub fn knop_download() {
    match super::verkenner::bijwerk_info() {
        Some((map, i, versie)) => bijwerken(&map, i, &versie),
        None => start_download(),
    }
}

/// Bijwerken: het nieuwste installatiebestand downloaden naar de map Downloads (zonder te vragen waar) en
/// het daarna starten.
fn bijwerken(map: &str, i: usize, versie: &str) {
    if overdracht_ui::bezig() {
        zeg(&t("AlBezig"), Belang::Belangrijk);
        return;
    }
    let Some(b) = met(|s| s.lijst.get(i).cloned()).flatten() else {
        zeg(&t("GeenKeuze"), Belang::Belangrijk);
        return;
    };
    let doel = dialogen::downloads_map().join(bestandsbrug::pcloud::download::veilige_naam(&b.naam));
    if doel.exists() && !vragen::ja_nee(hwnd(), &f("Overschrijven", &[&b.naam]), false) {
        return;
    }
    log::info(&format!("Update program: {map} to {versie}"));
    let naam = b.naam.clone();
    overdracht_ui::start(Taak::Download { bestand: b, doel, starten: true }, naam);
}

/// De knop Openen: het programma van de gekozen map starten, als het op de computer staat.
pub fn open_programma() {
    let Some(map) = super::verkenner::programma_map() else {
        zeg(&t("OpenenGeenProgramma"), Belang::Belangrijk);
        return;
    };
    let ps = met(|s| s.programmas.clone()).unwrap_or_default();
    let Some(p) = bestandsbrug::programmas::zoek_programma(&map, &ps) else {
        zeg(&t("OpenenNietGeinstalleerd"), Belang::Belangrijk);
        return;
    };
    let Some(exe) = super::programmas::start_pad(p, &map) else {
        log::fout(&format!("Program not found: {map}"));
        zeg(&f("OpenenNietGevonden", &[&map]), Belang::Belangrijk);
        return;
    };
    let werkmap = exe.parent().map(|m| m.to_string_lossy().to_string()).unwrap_or_default();
    if dialogen::open_in(&exe.to_string_lossy(), "", &werkmap) {
        log::info(&format!("Program started: {map}"));
        zeg(&f("OpenenGestart", &[&map]), Belang::Belangrijk);
    } else {
        log::fout(&format!("Program could not start: {map}"));
        zeg(&f("OpenenNietGevonden", &[&map]), Belang::Belangrijk);
    }
}

/// Een hele map kopiëren: kies waar hij komt (standaard Downloads); daar komt de map met alles erin.
fn kopieer_map(naam: &str, pad: &str) {
    if overdracht_ui::bezig() {
        zeg(&t("AlBezig"), Belang::Belangrijk);
        return;
    }
    let Some(basis) = dialogen::kies_map(&f("MapKopierenTitel", &[naam])) else { return };
    let doel = basis.join(bestandsbrug::pcloud::download::veilige_naam(naam));
    if doel.exists() && !vragen::ja_nee(hwnd(), &f("MapBestaatAl", &[naam]), false) {
        return;
    }
    let lijst = met(|s| s.lijst.clone()).unwrap_or_default();
    let bestanden = bestandsbrug::pcloud::download::bestanden_in_map(&lijst, pad, &doel);
    if bestanden.is_empty() {
        zeg(&t("MapLeeg"), Belang::Belangrijk);
        return;
    }
    overdracht_ui::start(Taak::Map { naam: naam.to_string(), bestanden, doel }, f("MapNaam", &[naam]));
}

/// Bij het opstarten: er is een download die niet af is. Mag die verder?
fn vraag_onderbroken() {
    let lijst = met(|s| s.lijst.clone()).unwrap_or_default();
    for o in overdracht::laad() {
        let Some(b) = lijst.iter().find(|b| b.fileid == o.fileid && b.hash == o.hash && b.grootte == o.grootte).cloned() else {
            overdracht::gooi_weg(&o);
            log::info(&format!("Interrupted download removed (file changed): {}", o.naam));
            zeg(&f("OnderbrokenNietMeer", &[&o.naam]), Belang::Belangrijk);
            continue;
        };
        const JA: i32 = 200;
        const LATER: i32 = 201;
        const WEG: i32 = 202;
        let knoppen = [vragen::knop(JA, t("OnderbrokenJa")), vragen::knop(LATER, t("Later")), vragen::knop(WEG, t("OnderbrokenWeg"))];
        let tekst = f("OnderbrokenVraag", &[&o.naam, &o.procent().to_string()]);
        match vragen::vraag(hwnd(), &bestandsbrug::taal::app_naam(), &tekst, "", &knoppen, JA, Some(LATER)) {
            JA => {
                log::info(&format!("Resume download: {}", o.naam));
                overdracht_ui::start(Taak::Download { bestand: b.clone(), doel: o.doel.clone(), starten: false }, b.naam.clone());
            }
            WEG => {
                overdracht::gooi_weg(&o);
                log::info(&format!("Interrupted download removed by user: {}", o.naam));
                zeg_later(&f("OnderbrokenWeg2", &[&o.naam]), Belang::Belangrijk);
            }
            _ => {}
        }
        // Eén tegelijk; een volgende komt bij de volgende start.
        break;
    }
}
