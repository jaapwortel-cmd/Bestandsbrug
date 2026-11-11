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

/// Zet de bestanden op het scherm, als boom met mappen. De keuze en open mappen blijven staan.
pub fn vul_lijst(lijst: &[Bestand]) {
    let oud = met(|s| s.lijst.clone()).unwrap_or_default();
    super::boom::vul(lijst, &oud);
}

/// Het gekozen bestand (geen map).
pub fn gekozen_index() -> Option<usize> {
    match super::boom::keuze() {
        Some(super::boom::Keuze::Bestand(i)) => Some(i),
        _ => None,
    }
}

/// Bewaar en toon een lijst.
pub fn toon_lijst(nieuw: Vec<Bestand>, van_internet: bool) {
    let anders = met(|s| s.lijst != nieuw).unwrap_or(true);
    if anders {
        vul_lijst(&nieuw);
    }
    met(|s| {
        s.lijst = nieuw;
        s.lijst_van_internet |= van_internet;
    });
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
            let melding = if handmatig || (eerste_keer && oud.is_empty()) {
                Some(match nieuw.len() {
                    0 => t("LijstLeeg"),
                    1 => t("LijstEenBestand"),
                    n => f("LijstGeladen", &[&n.to_string()]),
                })
            } else {
                let erbij = lijst::nieuwe_namen(&oud, &nieuw);
                if !erbij.is_empty() {
                    Some(f("LijstNieuw", &[&erbij.join(", ")]))
                } else if oud != nieuw {
                    Some(t("LijstAnders"))
                } else {
                    None
                }
            };
            toon_lijst(nieuw, true);
            if let Some(m) = melding {
                zeg(&m, if handmatig { Belang::Belangrijk } else { Belang::Gewoon });
            }
            if hoofdscherm::focus_hier() && unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() } == hoofdscherm::onderdelen().tab {
                // De focus niet verplaatsen; de lijst staat klaar met Tab.
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
pub fn start_download() {
    if overdracht_ui::bezig() {
        zeg(&t("AlBezig"), Belang::Belangrijk);
        return;
    }
    if matches!(super::boom::keuze(), Some(super::boom::Keuze::Map(_))) {
        zeg(&t("KiesBestandInMap"), Belang::Belangrijk);
        return;
    }
    let bestand = gekozen_index().and_then(|i| met(|s| s.lijst.get(i).cloned()).flatten());
    let Some(b) = bestand else {
        zeg(&t("GeenKeuze"), Belang::Belangrijk);
        return;
    };
    let Some(doel) = dialogen::kies_opslaan(&b.naam) else { return };
    if doel.exists() && !vragen::ja_nee(hwnd(), &f("Overschrijven", &[&doel.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()]), false) {
        return;
    }
    let naam = b.naam.clone();
    overdracht_ui::start(Taak::Download { bestand: b, doel }, naam);
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
                overdracht_ui::start(Taak::Download { bestand: b.clone(), doel: o.doel.clone() }, b.naam.clone());
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
