//! Taal kiezen en teksten ophalen. Alle teksten staan in talen/nl.txt en talen/en.txt (in de app ingebouwd).
//! {naam} wordt overal de naam van de app in de gekozen taal: Bestandsbrug of Filebridge.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

pub const NL_TEKST: &str = include_str!("../talen/nl.txt");
pub const EN_TEKST: &str = include_str!("../talen/en.txt");
pub const PRIVACY_NL: &str = include_str!("../talen/privacy-nl.txt");
pub const PRIVACY_EN: &str = include_str!("../talen/privacy-en.txt");
pub const UITLEG_NL: &str = include_str!("../talen/uitleg-nl.txt");
pub const UITLEG_EN: &str = include_str!("../talen/uitleg-en.txt");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taal {
    Nl,
    En,
}

impl Taal {
    pub fn code(self) -> &'static str {
        match self {
            Taal::Nl => "nl",
            Taal::En => "en",
        }
    }
    pub fn van_code(c: &str) -> Option<Taal> {
        match c.trim().to_ascii_lowercase().as_str() {
            "nl" => Some(Taal::Nl),
            "en" => Some(Taal::En),
            _ => None,
        }
    }
}

/// Leest een taalbestand: regels "sleutel = tekst". Lege regels en regels met # worden overgeslagen.
pub fn lees_taalbestand(inhoud: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for regel in inhoud.trim_start_matches('\u{feff}').lines() {
        let r = regel.trim();
        if r.is_empty() || r.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = r.split_once('=') {
            let waarde = v.trim().replace("\\n", "\n").replace("\\t", "\t");
            m.insert(k.trim().to_string(), waarde);
        }
    }
    m
}

struct Talen {
    nl: HashMap<String, String>,
    en: HashMap<String, String>,
}

fn talen() -> &'static Talen {
    static T: OnceLock<Talen> = OnceLock::new();
    T.get_or_init(|| Talen { nl: lees_taalbestand(NL_TEKST), en: lees_taalbestand(EN_TEKST) })
}

static HUIDIG: RwLock<Taal> = RwLock::new(Taal::Nl);

pub fn zet(t: Taal) {
    if let Ok(mut h) = HUIDIG.write() {
        *h = t;
    }
}

pub fn huidig() -> Taal {
    HUIDIG.read().map(|h| *h).unwrap_or(Taal::Nl)
}

fn kaart(t: Taal) -> &'static HashMap<String, String> {
    match t {
        Taal::Nl => &talen().nl,
        Taal::En => &talen().en,
    }
}

/// Tekst in een bepaalde taal, met {naam} ingevuld.
pub fn t_in(t: Taal, sleutel: &str) -> String {
    let k = kaart(t);
    let tekst = k.get(sleutel).cloned().unwrap_or_else(|| sleutel.to_string());
    if sleutel == "AppNaam" {
        return tekst;
    }
    let naam = k.get("AppNaam").map(String::as_str).unwrap_or("");
    tekst.replace("{naam}", naam)
}

/// Tekst in de gekozen taal.
pub fn t(sleutel: &str) -> String {
    t_in(huidig(), sleutel)
}

/// Tekst met {0}, {1}, ... ingevuld.
pub fn f(sleutel: &str, waarden: &[&str]) -> String {
    vul_in(&t(sleutel), waarden)
}

pub fn vul_in(tekst: &str, waarden: &[&str]) -> String {
    let mut s = tekst.to_string();
    for (i, w) in waarden.iter().enumerate() {
        s = s.replace(&format!("{{{i}}}"), w);
    }
    s
}

/// De naam van de app in de gekozen taal: Bestandsbrug of Filebridge.
pub fn app_naam() -> String {
    t("AppNaam")
}

/// Tekst zonder & (voor namen die de schermlezer voorleest, en meldingen).
pub fn zonder_amp(s: &str) -> String {
    s.replace("&&", "\u{0}").replace('&', "").replace('\u{0}', "&")
}

pub fn privacy(t: Taal) -> &'static str {
    match t {
        Taal::Nl => PRIVACY_NL,
        Taal::En => PRIVACY_EN,
    }
}

pub fn uitleg(t: Taal) -> String {
    let tekst = match t {
        Taal::Nl => UITLEG_NL,
        Taal::En => UITLEG_EN,
    };
    tekst.replace("{naam}", &t_in(t, "AppNaam"))
}

/// De taal van Windows: Nederlands (of Fries) wordt nl, al het andere en.
pub fn taal_van_systeem() -> Taal {
    #[cfg(windows)]
    {
        let id = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
        let primair = id & 0x3ff;
        // 0x13 = Nederlands, 0x62 = Fries
        if primair == 0x13 || primair == 0x62 {
            Taal::Nl
        } else {
            Taal::En
        }
    }
    #[cfg(not(windows))]
    {
        let lang = std::env::var("LANG").unwrap_or_default().to_ascii_lowercase();
        if lang.starts_with("nl") || lang.starts_with("fy") {
            Taal::Nl
        } else {
            Taal::En
        }
    }
}

/// Alle sleutels van een taal (voor de tests).
pub fn sleutels(t: Taal) -> Vec<String> {
    let mut v: Vec<String> = kaart(t).keys().cloned().collect();
    v.sort();
    v
}
