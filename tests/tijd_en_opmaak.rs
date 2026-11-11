//! Tests voor datums van pCloud en voor het opschrijven van groottes.

use bestandsbrug::taal::{self, Taal};
use bestandsbrug::tijd;

#[test]
fn pcloud_datum_lezen() {
    let u = tijd::lees_pcloud_datum("Wed, 11 Nov 2026 17:42:23 +0000").unwrap();
    let d = tijd::utc_datum(u);
    assert_eq!((d.jaar, d.maand, d.dag, d.uur, d.minuut, d.seconde), (2026, 11, 11, 17, 42, 23));
    let u2 = tijd::lees_pcloud_datum("Wed, 11 Nov 2026 18:42:23 +0100").unwrap();
    assert_eq!(u, u2);
    assert_eq!(tijd::naar_unix(&tijd::utc_datum(1_700_000_000)), 1_700_000_000);
    assert!(tijd::lees_pcloud_datum("onzin").is_none());
}

#[test]
fn grootte_in_woorden() {
    taal::zet(Taal::Nl);
    assert_eq!(bestandsbrug::opmaak::grootte(500), "500 bytes");
    assert_eq!(bestandsbrug::opmaak::grootte(6_685_226), "6,4 MB");
    taal::zet(Taal::En);
    assert_eq!(bestandsbrug::opmaak::grootte(6_685_226), "6.4 MB");
    taal::zet(Taal::Nl);
}
