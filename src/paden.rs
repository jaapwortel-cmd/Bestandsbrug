//! Waar de app zijn bestanden bewaart: de map in AppData (voorkeuren, log, lijst, onderbroken downloads)
//! en de standaardmap Downloads.

use std::path::PathBuf;

use crate::instellingen::TECHNISCHE_NAAM;

/// %APPDATA%\Bestandsbrug op Windows. Tests kunnen een eigen map kiezen met BESTANDSBRUG_MAP.
pub fn gegevensmap() -> PathBuf {
    if let Some(m) = std::env::var_os("BESTANDSBRUG_MAP") {
        return PathBuf::from(m);
    }
    let basis = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(std::env::temp_dir);
    basis.join(TECHNISCHE_NAAM)
}

/// Maakt de gegevensmap als die er nog niet is, en geeft het pad van een bestand daarin.
pub fn bestand(naam: &str) -> PathBuf {
    let map = gegevensmap();
    let _ = std::fs::create_dir_all(&map);
    map.join(naam)
}

pub fn logbestand() -> PathBuf {
    bestand(&format!("{TECHNISCHE_NAAM}.log"))
}

/// De map Downloads van de gebruiker (reserve; het scherm vraagt Windows om de echte map).
pub fn downloads() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|h| PathBuf::from(h).join("Downloads"))
        .unwrap_or_else(std::env::temp_dir)
}

/// Een eigen map voor het downloaden van updates (in de gegevensmap, wordt na gebruik leeggemaakt).
pub fn tijdelijk() -> PathBuf {
    let m = gegevensmap().join("update");
    let _ = std::fs::create_dir_all(&m);
    m
}
