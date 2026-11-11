//! De teksten en de timer voor de voortgangsmeldingen aan de schermlezer.
//! De voortgangsbalk zelf zegt niets; alleen deze timer van 6 seconden laat de schermlezer de voortgang horen.
//! Direct gemeld (niet pas na 6 seconden): begin, einde, fout en onderbreking.
//! Het uitspreken zelf (JAWS of UI Automation) staat in src/scherm/spraak.rs.

use std::time::{Duration, Instant};

use crate::instellingen::MELDING_INTERVAL_SECONDEN;
use crate::taal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Soort {
    Download,
    Upload,
}

/// Procent (0-100), naar beneden afgerond, zodat 100 pas komt als alles er is.
pub fn procent(gedaan: u64, totaal: u64) -> u64 {
    if totaal == 0 {
        return 0;
    }
    (gedaan.min(totaal) as u128 * 100 / totaal as u128) as u64
}

/// Hoeveel seconden nog ongeveer, op grond van de gemiddelde snelheid. None als dat nog niet te zeggen is.
pub fn seconden_over(gedaan_deze_keer: u64, nog: u64, verstreken: Duration) -> Option<u64> {
    let s = verstreken.as_secs_f64();
    if gedaan_deze_keer == 0 || s < 1.0 {
        return None;
    }
    let snelheid = gedaan_deze_keer as f64 / s;
    Some((nog as f64 / snelheid).ceil() as u64)
}

/// ", nog ongeveer 20 seconden" (afgerond op 5 seconden) of ", nog ongeveer 3 minuten".
pub fn rest_tekst(seconden: u64) -> String {
    if seconden < 60 {
        let afgerond = ((seconden + 4) / 5 * 5).max(5);
        if afgerond >= 60 {
            return taal::t("MeldNogMinuut");
        }
        taal::f("MeldNogSeconden", &[&afgerond.to_string()])
    } else {
        let minuten = (seconden + 30) / 60;
        if minuten <= 1 {
            taal::t("MeldNogMinuut")
        } else {
            taal::f("MeldNogMinuten", &[&minuten.to_string()])
        }
    }
}

/// "Downloaden, 40 procent, nog ongeveer 20 seconden."
pub fn voortgang_tekst(soort: Soort, gedaan: u64, totaal: u64, gedaan_deze_keer: u64, verstreken: Duration) -> String {
    let sleutel = match soort {
        Soort::Download => "MeldDownloadVoortgang",
        Soort::Upload => "MeldUploadVoortgang",
    };
    let mut s = taal::f(sleutel, &[&procent(gedaan, totaal).to_string()]);
    if let Some(sec) = seconden_over(gedaan_deze_keer, totaal.saturating_sub(gedaan), verstreken) {
        s.push_str(&rest_tekst(sec));
    }
    s.push('.');
    s
}

/// De timer van 6 seconden. Een overdracht die sneller klaar is, krijgt dus alleen een begin- en eindmelding.
#[derive(Debug, Clone)]
pub struct MeldTimer {
    vorige: Instant,
}

impl MeldTimer {
    pub fn start(nu: Instant) -> Self {
        MeldTimer { vorige: nu }
    }

    pub fn interval() -> Duration {
        Duration::from_secs(MELDING_INTERVAL_SECONDEN)
    }

    /// Geeft true als er nu een voortgangsmelding moet komen.
    pub fn is_tijd(&mut self, nu: Instant) -> bool {
        // Een kleine marge: een timer van Windows komt soms een paar milliseconden te vroeg.
        if nu.duration_since(self.vorige) + Duration::from_millis(150) >= Self::interval() {
            self.vorige = nu;
            true
        } else {
            false
        }
    }
}
