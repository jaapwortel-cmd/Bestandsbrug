//! Maakt Bestandsbrug.html: één los bestand met alles erin. De bron is web/bestandsbrug.html; daarin
//! komen de teksten (talen/nl.txt en en.txt), de uitleg, de privacyverklaring (talen/privacy-*.txt),
//! de kleuren (src/kleuren.rs), het versienummer en de twee openbare pCloud-codes. Geen geheimen.

use serde_json::{json, Map, Value};

use crate::instellingen::*;
use crate::kleuren;
use crate::taal::{self, Taal};

pub const SJABLOON: &str = include_str!("../web/bestandsbrug.html");

fn teksten(t: Taal) -> Value {
    let mut m = Map::new();
    for k in taal::sleutels(t) {
        m.insert(k.clone(), Value::String(taal::zonder_amp(&taal::t_in(t, &k)).replace('\t', " ")));
    }
    Value::Object(m)
}

/// De gegevens voor de pagina, als JSON.
pub fn gegevens() -> Value {
    json!({
        "versie": VERSIE,
        "downloadCode": PCLOUD_DOWNLOAD_CODE,
        "uploadCode": PCLOUD_UPLOAD_CODE,
        "uploader": PCLOUD_UPLOADER_NAAM,
        "servers": PCLOUD_API_SERVERS,
        "uitlegBestand": UITLEG_BESTAND,
        "meldSeconden": MELDING_INTERVAL_SECONDEN,
        "teksten": { "nl": teksten(Taal::Nl), "en": teksten(Taal::En) },
        "uitleg": { "nl": taal::uitleg(Taal::Nl), "en": taal::uitleg(Taal::En) },
        "privacy": { "nl": taal::privacy(Taal::Nl), "en": taal::privacy(Taal::En) },
    })
}

/// De hele pagina.
pub fn maak() -> String {
    // "</" in de JSON zou het script kunnen afbreken; daarom "<\/".
    let data = serde_json::to_string(&gegevens()).unwrap().replace("</", "<\\/");
    SJABLOON
        .replace("/*__GEGEVENS__*/null", &data)
        .replace("/*__LICHT__*/", &kleuren::css(&kleuren::LICHT))
        .replace("/*__DONKER__*/", &kleuren::css(&kleuren::DONKER))
        .replace("__TITEL__", &taal::t_in(Taal::Nl, "AppNaam"))
}
