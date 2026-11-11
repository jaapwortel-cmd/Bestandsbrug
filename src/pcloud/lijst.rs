//! De lijst met bestanden in de openbare map ophalen (showpublink), met de uitleg uit uitleg.txt.
//! De laatst bekende lijst staat ook op de schijf (lijst.json), zodat hij bij het opstarten meteen te zien is.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::fout::{Fout, Uitkomst};
use crate::instellingen::{LIMIET_LIJST_SECONDEN, PCLOUD_DOWNLOAD_CODE, UITLEG_BESTAND};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bestand {
    pub fileid: u64,
    pub naam: String,
    pub grootte: u64,
    /// Datum zoals pCloud hem geeft (UTC).
    pub gewijzigd: String,
    pub hash: u64,
    #[serde(default)]
    pub uitleg: String,
}

/// Haalt de lijst op: alleen bestanden (geen mappen), op naam gesorteerd. uitleg.txt zelf staat niet in de lijst.
pub fn haal_lijst() -> Uitkomst<Vec<Bestand>> {
    let json = super::api_get(
        "showpublink",
        &[("code", PCLOUD_DOWNLOAD_CODE)],
        Duration::from_secs(LIMIET_LIJST_SECONDEN),
    )?;
    let mut lijst = lees_inhoud(&json);
    let uitleg_bestand = lijst.iter().position(|b| b.naam.eq_ignore_ascii_case(UITLEG_BESTAND));
    if let Some(i) = uitleg_bestand {
        let u = lijst.remove(i);
        // De uitleg is een extraatje: lukt het niet, dan gewoon zonder.
        if let Ok(tekst) = haal_tekst(u.fileid) {
            let kaart = lees_uitleg(&tekst);
            for b in lijst.iter_mut() {
                if let Some(x) = kaart.get(&b.naam.to_lowercase()) {
                    b.uitleg = x.clone();
                }
            }
        }
    }
    Ok(lijst)
}

/// Leest de bestanden uit het antwoord van showpublink.
pub fn lees_inhoud(json: &Value) -> Vec<Bestand> {
    let mut lijst: Vec<Bestand> = json
        .pointer("/metadata/contents")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter(|x| !x.get("isfolder").and_then(Value::as_bool).unwrap_or(false))
                .filter_map(|x| {
                    Some(Bestand {
                        fileid: x.get("fileid")?.as_u64()?,
                        naam: x.get("name")?.as_str()?.to_string(),
                        grootte: x.get("size").and_then(Value::as_u64).unwrap_or(0),
                        gewijzigd: x.get("modified").and_then(Value::as_str).unwrap_or("").to_string(),
                        hash: x.get("hash").and_then(Value::as_u64).unwrap_or(0),
                        uitleg: String::new(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    lijst.sort_by_key(|b| b.naam.to_lowercase());
    lijst
}

/// Leest uitleg.txt: regels "bestandsnaam = korte uitleg". De naam is niet hoofdlettergevoelig.
pub fn lees_uitleg(tekst: &str) -> HashMap<String, String> {
    tekst
        .trim_start_matches('\u{feff}')
        .lines()
        .filter_map(|r| {
            let r = r.trim();
            if r.is_empty() || r.starts_with('#') {
                return None;
            }
            let (k, v) = r.split_once('=')?;
            Some((k.trim().to_lowercase(), v.trim().to_string()))
        })
        .collect()
}

/// Een klein tekstbestand uit de openbare map lezen (hooguit 64 KB).
fn haal_tekst(fileid: u64) -> Uitkomst<String> {
    let url = super::download::download_adres(fileid)?;
    let resp = crate::net::met_limiet(crate::net::get(&url), Duration::from_secs(LIMIET_LIJST_SECONDEN))
        .call()
        .map_err(crate::net::vertaal)?;
    let mut buf = Vec::new();
    use std::io::Read;
    resp.into_body().into_reader().take(64 * 1024).read_to_end(&mut buf).map_err(|e| crate::net::vertaal_io(&e))?;
    String::from_utf8(buf).map_err(|_| Fout::Anders("uitleg.txt is not UTF-8".into()))
}

const CACHE: &str = "lijst.json";

/// De laatst bekende lijst van de schijf.
pub fn laad_cache() -> Option<Vec<Bestand>> {
    let s = std::fs::read_to_string(crate::paden::bestand(CACHE)).ok()?;
    serde_json::from_str(&s).ok()
}

pub fn bewaar_cache(lijst: &[Bestand]) {
    if let Ok(s) = serde_json::to_string(lijst) {
        let _ = crate::voorkeuren::schrijf_veilig(&crate::paden::bestand(CACHE), s.as_bytes());
    }
}

/// Namen die nieuw zijn in de nieuwe lijst (om kort te melden).
pub fn nieuwe_namen(oud: &[Bestand], nieuw: &[Bestand]) -> Vec<String> {
    nieuw
        .iter()
        .filter(|n| !oud.iter().any(|o| o.fileid == n.fileid))
        .map(|n| n.naam.clone())
        .collect()
}
