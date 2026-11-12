//! Tabblad Uploaden: een bestand of een hele map kiezen (de standaardvensters van Windows) en uploaden naar Jaap.
//! Een map gaat als één zip-bestand met alles erin (een uploadlink van pCloud kan zelf geen mappen maken).

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

/// De tekst in het vak "Gekozen": de naam, met "Map:" ervoor bij een map.
pub fn gekozen_tekst(pad: &std::path::Path) -> String {
    let naam = pad.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if pad.is_dir() { f("GekozenMap", &[&naam]) } else { naam }
}

pub fn kies_map() {
    let Some(pad) = dialogen::kies_map(&t("UploadMapKiezenTitel")) else { return };
    let naam = pad.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    win::zet_tekst(hoofdscherm::onderdelen().gekozen, &gekozen_tekst(&pad));
    met(|s| s.upload_pad = Some(pad));
    zeg_later(&f("GekozenMapMelding", &[&naam]), Belang::Gewoon);
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
    if pad.is_dir() {
        let naam = pad.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        overdracht_ui::start(Taak::UploadMap { pad }, f("MapNaam", &[&naam]));
        return;
    }
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
