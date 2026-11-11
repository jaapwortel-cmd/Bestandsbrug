//! Tests voor de voortgangsmeldingen en de timer van 6 seconden.

use std::time::{Duration, Instant};

use bestandsbrug::melding::{self, MeldTimer, Soort};
use bestandsbrug::taal::{self, Taal};

#[test]
fn procent_klopt() {
    assert_eq!(melding::procent(0, 100), 0);
    assert_eq!(melding::procent(399, 1000), 39);
    assert_eq!(melding::procent(1000, 1000), 100);
    assert_eq!(melding::procent(5, 0), 0);
}

#[test]
fn voortgangstekst_in_beide_talen() {
    taal::zet(Taal::Nl);
    // 40 van 100 MB in 10 seconden: nog 60 MB = 15 seconden.
    let s = melding::voortgang_tekst(Soort::Download, 40, 100, 40, Duration::from_secs(10));
    assert_eq!(s, "Downloaden, 40 procent, nog ongeveer 15 seconden.");
    let s = melding::voortgang_tekst(Soort::Upload, 10, 100, 10, Duration::from_secs(10));
    assert_eq!(s, "Uploaden, 10 procent, nog ongeveer 2 minuten.");
    taal::zet(Taal::En);
    let s = melding::voortgang_tekst(Soort::Download, 40, 100, 40, Duration::from_secs(10));
    assert_eq!(s, "Downloading, 40 percent, about 15 seconds left.");
    taal::zet(Taal::Nl);
}

#[test]
fn timer_meldt_pas_na_6_seconden() {
    assert_eq!(MeldTimer::interval(), Duration::from_secs(bestandsbrug::instellingen::MELDING_INTERVAL_SECONDEN));
    let t0 = Instant::now();
    let mut t = MeldTimer::start(t0);
    assert!(!t.is_tijd(t0 + Duration::from_secs(3)));
    assert!(!t.is_tijd(t0 + Duration::from_secs(5)));
    assert!(t.is_tijd(t0 + Duration::from_secs(6)));
    assert!(!t.is_tijd(t0 + Duration::from_secs(9)));
    assert!(t.is_tijd(t0 + Duration::from_secs(12)));
}
