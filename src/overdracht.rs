//! Een lopende overdracht (download of upload): voortgang en stopknop, gedeeld tussen het scherm en
//! de achtergrond. En het onthouden van onderbroken downloads in overdrachten.json, zodat ze bij de
//! volgende start verder kunnen. (Uploads kunnen niet verder; dat kan pCloud niet bij een uploadlink.)

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::fout::Stop;

/// Gedeeld tussen het scherm en de achtergrond. Alles met atomics: het scherm hoeft nooit te wachten.
#[derive(Debug)]
pub struct Voortgang {
    pub gedaan: AtomicU64,
    pub totaal: AtomicU64,
    /// Waar deze keer begonnen is (bij hervatten), voor de schatting van de resttijd.
    pub begin: AtomicU64,
    stop: AtomicU8,
    pub gestart: Instant,
    /// Bij een hele map: de bytes van de bestanden die al klaar zijn.
    pub voor: AtomicU64,
    /// Bij een hele map: het totaal staat vast (de som van alle bestanden).
    pub vast_totaal: AtomicBool,
}

impl Default for Voortgang {
    fn default() -> Self {
        Self::new()
    }
}

impl Voortgang {
    pub fn new() -> Self {
        Voortgang {
            gedaan: AtomicU64::new(0),
            totaal: AtomicU64::new(0),
            begin: AtomicU64::new(0),
            stop: AtomicU8::new(0),
            gestart: Instant::now(),
            voor: AtomicU64::new(0),
            vast_totaal: AtomicBool::new(false),
        }
    }
    pub fn vraag_stop(&self, s: Stop) {
        self.stop.store(if s == Stop::Pauze { 1 } else { 2 }, Ordering::SeqCst);
    }
    pub fn stop(&self) -> Option<Stop> {
        match self.stop.load(Ordering::SeqCst) {
            1 => Some(Stop::Pauze),
            2 => Some(Stop::Annuleer),
            _ => None,
        }
    }
    pub fn gedaan(&self) -> u64 {
        self.gedaan.load(Ordering::Relaxed)
    }
    pub fn totaal(&self) -> u64 {
        self.totaal.load(Ordering::Relaxed)
    }
    pub fn deze_keer(&self) -> u64 {
        self.gedaan().saturating_sub(self.begin.load(Ordering::Relaxed))
    }
}

/// Een download die niet af is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Onderbroken {
    pub fileid: u64,
    pub naam: String,
    pub grootte: u64,
    /// Om te zien of het bestand in pCloud intussen veranderd is.
    pub hash: u64,
    pub doel: PathBuf,
}

impl Onderbroken {
    pub fn deel(&self) -> PathBuf {
        deel_pad(&self.doel)
    }
    pub fn procent(&self) -> u64 {
        let al = std::fs::metadata(self.deel()).map(|m| m.len()).unwrap_or(0);
        crate::melding::procent(al, self.grootte)
    }
}

/// Het halve bestand staat naast het doel, met een eigen achtervoegsel.
pub fn deel_pad(doel: &std::path::Path) -> PathBuf {
    let mut s = doel.as_os_str().to_owned();
    s.push(crate::instellingen::DEEL_ACHTERVOEGSEL);
    PathBuf::from(s)
}

const BESTAND: &str = "overdrachten.json";

pub fn laad() -> Vec<Onderbroken> {
    std::fs::read_to_string(crate::paden::bestand(BESTAND))
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<Onderbroken>>(&s).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|o| o.deel().exists())
        .collect()
}

pub fn bewaar(lijst: &[Onderbroken]) {
    let pad = crate::paden::bestand(BESTAND);
    if lijst.is_empty() {
        let _ = std::fs::remove_file(pad);
        return;
    }
    if let Ok(s) = serde_json::to_string_pretty(lijst) {
        let _ = crate::voorkeuren::schrijf_veilig(&pad, s.as_bytes());
    }
}

pub fn voeg_toe(o: Onderbroken) {
    let mut l = laad();
    l.retain(|x| x.doel != o.doel);
    l.push(o);
    bewaar(&l);
}

pub fn verwijder(doel: &std::path::Path) {
    let mut l = laad();
    l.retain(|x| x.doel != doel);
    bewaar(&l);
}

/// Gooit het halve bestand weg en vergeet de onderbreking.
pub fn gooi_weg(o: &Onderbroken) {
    let _ = std::fs::remove_file(o.deel());
    verwijder(&o.doel);
}
