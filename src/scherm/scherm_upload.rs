//! Tabblad Uploaden: een bestand kiezen (de standaard bestandskiezer van Windows) en uploaden naar Jaap.

use bestandsbrug::taal::{f, t};

use super::overdracht_ui::{self, Taak};
use super::*;

pub fn kies_bestand() {
    let Some(pad) = dialogen::kies_openen() else { return };
    let naam = pad.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    win::zet_tekst(hoofdscherm::onderdelen().gekozen, &naam);
    met(|s| s.upload_pad = Some(pad));
    zeg_later(&f("GekozenMelding", &[&naam]), Belang::Gewoon);
}

pub fn start_upload() {
    if overdracht_ui::bezig() {
        zeg(&t("AlBezig"), Belang::Belangrijk);
        return;
    }
    let Some(pad) = met(|s| s.upload_pad.clone()).flatten() else {
        zeg(&t("GeenUploadKeuze"), Belang::Belangrijk);
        return;
    };
    let grootte = match std::fs::metadata(&pad) {
        Ok(m) => m.len(),
        Err(e) => {
            zeg(&f("FoutLezen", &[&bestandsbrug::fout::io_tekst(&e)]), Belang::Belangrijk);
            return;
        }
    };
    if grootte == 0 {
        zeg(&t("LeegBestand"), Belang::Belangrijk);
        return;
    }
    let naam = pad.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    overdracht_ui::start(Taak::Upload { pad, grootte }, naam);
}
