//! Updates op het scherm: automatisch kijken kort na het opstarten (stil als er niets is), en Help,
//! Controleren op updates. De nieuwe versie downloaden met dezelfde voortgangsmeldingen, de SHA-256
//! controleren, vragen of de app mag afsluiten, en dan het installatiebestand starten.

use std::path::PathBuf;
use std::time::Duration;

use bestandsbrug::fout::{Fout, Uitkomst};
use bestandsbrug::instellingen::{LIMIET_UPDATECONTROLE_SECONDEN, UPDATE_LINK, VERSIE};
use bestandsbrug::taal::{self, f, t};
use bestandsbrug::update::{self, Bron, VersieInfo};
use bestandsbrug::log;
use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;

use super::overdracht_ui::{self, Taak};
use super::*;

pub struct Gevonden {
    uitkomst: Uitkomst<(VersieInfo, Bron)>,
}

fn zoek(handmatig: bool) {
    let h = hwnd().0 as isize;
    let limiet = Duration::from_secs(if handmatig { 60 } else { LIMIET_UPDATECONTROLE_SECONDEN });
    std::thread::spawn(move || {
        let g = Box::new(Gevonden { uitkomst: update::haal_versie_info(UPDATE_LINK, limiet) });
        win::post(windows::Win32::Foundation::HWND(h as *mut _), WM_UPDATE_INFO, handmatig as usize, Box::into_raw(g) as isize);
    });
}

/// Kort na het opstarten. Tijdens een overdracht wachten we tot die klaar is.
pub fn automatisch() {
    if overdracht_ui::bezig() {
        met(|s| s.update_uitgesteld = true);
        return;
    }
    zoek(false);
}

/// Help, Controleren op updates.
pub fn handmatig() {
    if overdracht_ui::bezig() {
        vragen::info(hwnd(), &t("UpdateWacht"));
        return;
    }
    zeg(&t("UpdateZoeken"), Belang::Gewoon);
    zoek(true);
}

pub fn gevonden(g: Gevonden, handmatig: bool) {
    let later = met(|s| s.voorkeuren.update_later.clone()).flatten();
    match g.uitkomst {
        Err(e) => {
            log::info(&format!("Update check failed: {}", e.log_tekst()));
            if handmatig {
                let tekst = if e.is_geen_internet() { t("UpdateGeenInternet") } else { f("UpdateFout", &[&e.tekst()]) };
                vragen::info(hwnd(), &tekst);
            }
        }
        Ok((info, bron)) => {
            let nieuwer = update::is_nieuwer(&info.versie, VERSIE);
            log::info(&format!("Update check: newest {} (this is {})", info.versie, VERSIE));
            if !handmatig {
                // Alleen één nette melding, geen venster dat de focus pakt.
                if update::moet_automatisch_melden(&info.versie, later.as_deref()) {
                    zeg(&t("UpdateMelding"), Belang::Gewoon);
                }
                return;
            }
            if !nieuwer {
                vragen::info(hwnd(), &f("UpdateNieuwste", &[VERSIE]));
                return;
            }
            const NU: i32 = 300;
            const LATER: i32 = 301;
            let knoppen = [vragen::knop(NU, t("UpdateNuUpdaten")), vragen::knop(LATER, t("Later"))];
            let tekst = f("UpdateNieuw", &[&info.versie, VERSIE, info.nieuw(taal::huidig())]);
            let (kop, rest) = tekst.split_once("\n\n").unwrap_or((&tekst, ""));
            let keuze = vragen::vraag(hwnd(), &t("UpdateTitel"), kop, rest, &knoppen, NU, Some(LATER));
            if keuze == NU {
                overdracht_ui::start(Taak::Update { info, bron }, t("UpdateNaam"));
            } else {
                log::info(&format!("Update {} postponed (Later)", info.versie));
                met(|s| {
                    s.voorkeuren.update_later = Some(info.versie.clone());
                    bestandsbrug::voorkeuren::bewaar(&s.voorkeuren);
                });
            }
        }
    }
}

/// De nieuwe versie is gedownload (of niet).
pub fn update_binnen(info: VersieInfo, uitkomst: Uitkomst<Option<PathBuf>>) {
    match uitkomst {
        Ok(Some(pad)) => {
            log::info(&format!("Update {} downloaded, SHA-256 ok", info.versie));
            zeg(&t("UpdateBinnen"), Belang::Belangrijk);
            if vragen::ja_nee(hwnd(), &t("UpdateAfsluitenVraag"), true) {
                installeer(&pad, &info);
            }
        }
        Ok(None) => {}
        Err(Fout::Gestopt(_)) => {
            log::info("Update download cancelled");
            zeg(&t("MeldDownloadGeannuleerd"), Belang::Belangrijk);
        }
        Err(e) => {
            log::fout(&format!("Update download failed: {}", e.log_tekst()));
            let tekst = if matches!(e, Fout::Beschadigd) { t("UpdateBeschadigd") } else { f("MeldDownloadFout", &[&e.tekst()]) };
            vragen::info(hwnd(), &tekst);
        }
    }
}

/// Start het installatiebestand (stil, in de taal van de app) en sluit de app. Het installatiebestand
/// start de app daarna vanzelf weer. Instellingen blijven bewaard (die staan in AppData).
fn installeer(pad: &std::path::Path, info: &VersieInfo) {
    let param = format!("/SILENT /SUPPRESSMSGBOXES /NORESTART /CLOSEAPPLICATIONS /LANG={}", taal::huidig().code());
    if dialogen::open(&pad.to_string_lossy(), &param) {
        log::info(&format!("Installer for {} started; closing", info.versie));
        zeg(&t("UpdateStart"), Belang::Belangrijk);
        // Even wachten zodat de schermlezer de melding kan zeggen.
        std::thread::sleep(Duration::from_millis(1500));
        unsafe {
            let _ = DestroyWindow(hwnd());
        }
    } else {
        log::fout("Cannot start installer");
        vragen::info(hwnd(), &f("UpdateStartFout", &[""]));
    }
}
