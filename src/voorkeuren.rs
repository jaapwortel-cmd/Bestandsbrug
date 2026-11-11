//! De keuzes van de gebruiker, bewaard op de eigen computer: taal, lichte of donkere weergave,
//! en de versie waarvoor "Later" is gekozen bij een update. Bestand: voorkeuren.json in de gegevensmap.

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Voorkeuren {
    /// "nl" of "en". Leeg = nog nooit gekozen (dan de taal van Windows).
    #[serde(default)]
    pub taal: Option<String>,
    /// "licht" of "donker". Leeg = nog nooit gekozen (dan de instelling van Windows).
    #[serde(default)]
    pub weergave: Option<String>,
    /// Voor deze versie (en lager) is "Later" gekozen: niet meer automatisch melden.
    #[serde(default)]
    pub update_later: Option<String>,
}

const BESTAND: &str = "voorkeuren.json";

pub fn laad() -> Voorkeuren {
    std::fs::read_to_string(crate::paden::bestand(BESTAND))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn bewaar(v: &Voorkeuren) {
    if let Ok(s) = serde_json::to_string_pretty(v) {
        let _ = schrijf_veilig(&crate::paden::bestand(BESTAND), s.as_bytes());
    }
}

/// Schrijft eerst naar een tijdelijk bestand en zet het daarna op zijn plek, zodat het nooit half is.
pub fn schrijf_veilig(pad: &std::path::Path, inhoud: &[u8]) -> std::io::Result<()> {
    let tmp = pad.with_extension("tmp");
    std::fs::write(&tmp, inhoud)?;
    std::fs::rename(&tmp, pad)
}
