//! Tests voor Bestandsbrug.html: alles zit erin (teksten, privacyverklaring woord voor woord, kleuren),
//! niets wordt van internet geladen, en er staan geen geheimen in.

#[test]
fn html_is_compleet_en_los() {
    let h = bestandsbrug::html::maak();
    assert!(!h.contains("/*__"), "een plekje is niet ingevuld");
    assert!(!h.contains("__TITEL__"));
    assert!(h.contains("<title>Bestandsbrug</title>"));
    // De privacyverklaring staat er woord voor woord in (als JSON-tekst).
    for p in [bestandsbrug::taal::PRIVACY_NL, bestandsbrug::taal::PRIVACY_EN] {
        let json = serde_json::to_string(p).unwrap().replace("</", "<\\/");
        assert!(h.contains(&json[1..json.len() - 1]), "privacyverklaring niet letterlijk in het HTML-bestand");
    }
    assert!(h.contains("\"Filebridge\""), "Engelse naam ontbreekt");
    assert!(h.contains(bestandsbrug::kleuren::DONKER.achtergrond));
    // Niets van internet laden (geen scripts, stijlen of lettertypen van buiten).
    for verboden in ["<script src", "<link ", "@import", "url(http"] {
        assert!(!h.contains(verboden), "laadt iets van buiten: {verboden}");
    }
    for geheim in ["password=", "access_token", "auth=", "apikey", "api_key"] {
        assert!(!h.to_lowercase().contains(geheim));
    }
    assert!(!h.contains("dropbox"), "de updatelink hoort niet in het HTML-bestand");
}

#[test]
fn kleuren_hebben_genoeg_contrast() {
    use bestandsbrug::kleuren::{contrast, DONKER, LICHT};
    for p in [LICHT, DONKER] {
        for (voor, achter, min) in [
            (p.tekst, p.kaart, 7.0),
            (p.tekst, p.achtergrond, 7.0),
            (p.zacht, p.kaart, 4.5),
            (p.op_accent, p.accent, 4.5),
            (p.op_kop, p.kop, 4.5),
            (p.focus, p.kaart, 3.0),
            (p.rand, p.kaart, 3.0),
        ] {
            let c = contrast(voor, achter);
            assert!(c >= min, "contrast {voor} op {achter} is {c:.2}, moet {min}");
        }
    }
}
