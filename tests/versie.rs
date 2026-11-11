//! Het versienummer staat op één plek (instellingen.rs). Cargo.toml moet hetzelfde zeggen
//! (bouw-alles.ps1 zet het gelijk).

#[test]
fn versie_in_cargo_is_gelijk() {
    assert_eq!(env!("CARGO_PKG_VERSION"), bestandsbrug::instellingen::VERSIE);
}

#[test]
fn geen_geheimen_in_de_instellingen() {
    let s = std::fs::read_to_string("src/instellingen.rs").unwrap().to_lowercase();
    for woord in ["password", "wachtwoord", "token", "secret", "access_token", "auth="] {
        assert!(!s.contains(woord), "verdacht woord in instellingen.rs: {woord}");
    }
    // Alleen de alleen-lezen updatelink: een gewone gedeelde maplink (scl/fo), geen andere Dropbox-link.
    assert_eq!(s.matches("dropbox.com").count(), 1);
    assert!(s.contains("dropbox.com/scl/fo/d7nrhucshq6vsssrgifrm/"));
}
