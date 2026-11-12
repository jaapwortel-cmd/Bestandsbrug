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
    /// De map waar het bestand in staat, met "/" tussen de mappen (leeg = de hoofdmap).
    #[serde(default)]
    pub map: String,
    #[serde(default)]
    pub uitleg: String,
}

/// Haalt de lijst op: alle bestanden in de openbare map en alle mappen daaronder, gesorteerd op map en naam.
/// Op het scherm komen ze als een boom (maak_boom). uitleg.txt zelf staat niet in de lijst.
pub fn haal_lijst() -> Uitkomst<Vec<Bestand>> {
    let json = super::api_get(
        "showpublink",
        &[("code", PCLOUD_DOWNLOAD_CODE)],
        Duration::from_secs(LIMIET_LIJST_SECONDEN),
    )?;
    let mut lijst = lees_inhoud(&json);
    let uitlegbestanden: Vec<Bestand> = lijst.iter().filter(|b| b.naam.eq_ignore_ascii_case(UITLEG_BESTAND)).cloned().collect();
    lijst.retain(|b| !b.naam.eq_ignore_ascii_case(UITLEG_BESTAND));
    for u in uitlegbestanden {
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

/// Leest de bestanden uit het antwoord van showpublink (alle mappen, ook diep).
pub fn lees_inhoud(json: &Value) -> Vec<Bestand> {
    let mut lijst = Vec::new();
    if let Some(m) = json.get("metadata") {
        verzamel(m, "", &mut lijst);
    }
    lijst.sort_by_key(|b| (b.map.to_lowercase(), b.naam.to_lowercase()));
    lijst
}

fn verzamel(map: &Value, mapnaam: &str, lijst: &mut Vec<Bestand>) {
    let Some(inhoud) = map.get("contents").and_then(Value::as_array) else { return };
    for x in inhoud {
        let naam = x.get("name").and_then(Value::as_str).unwrap_or("");
        if x.get("isfolder").and_then(Value::as_bool).unwrap_or(false) {
            let pad = if mapnaam.is_empty() { naam.to_string() } else { format!("{mapnaam}/{naam}") };
            verzamel(x, &pad, lijst);
            continue;
        }
        let Some(fileid) = x.get("fileid").and_then(Value::as_u64) else { continue };
        if is_systeembestand(naam) {
            continue;
        }
        lijst.push(Bestand {
            fileid,
            naam: naam.to_string(),
            grootte: x.get("size").and_then(Value::as_u64).unwrap_or(0),
            gewijzigd: x.get("modified").and_then(Value::as_str).unwrap_or("").to_string(),
            hash: x.get("hash").and_then(Value::as_u64).unwrap_or(0),
            map: mapnaam.to_string(),
            uitleg: String::new(),
        });
    }
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

/// Een map in de boom: de mappen erin en de bestanden erin (als nummer in de lijst).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Map {
    pub naam: String,
    /// Het hele pad, met "/" (leeg = de hoofdmap).
    pub pad: String,
    pub mappen: Vec<Map>,
    pub bestanden: Vec<usize>,
}

/// Maakt van de lijst een boom: in elke map eerst de mappen, dan de bestanden, allebei op naam.
pub fn maak_boom(lijst: &[Bestand]) -> Map {
    let mut wortel = Map::default();
    for (i, b) in lijst.iter().enumerate() {
        let mut m = &mut wortel;
        if !b.map.is_empty() {
            for deel in b.map.split('/') {
                let pad = if m.pad.is_empty() { deel.to_string() } else { format!("{}/{deel}", m.pad) };
                let plek = match m.mappen.iter().position(|x| x.naam == deel) {
                    Some(p) => p,
                    None => {
                        m.mappen.push(Map { naam: deel.to_string(), pad, ..Default::default() });
                        m.mappen.len() - 1
                    }
                };
                m = &mut m.mappen[plek];
            }
        }
        m.bestanden.push(i);
    }
    sorteer(&mut wortel, lijst);
    wortel
}

fn sorteer(m: &mut Map, lijst: &[Bestand]) {
    m.mappen.sort_by_key(|x| x.naam.to_lowercase());
    m.bestanden.sort_by_key(|&i| lijst[i].naam.to_lowercase());
    for x in m.mappen.iter_mut() {
        sorteer(x, lijst);
    }
}

/// Verborgen bestanden van Windows en Mac die niemand wil downloaden (zoals desktop.ini).
pub fn is_systeembestand(naam: &str) -> bool {
    let n = naam.to_lowercase();
    ["desktop.ini", "thumbs.db", ".ds_store", "ehthumbs.db"].contains(&n.as_str()) || n.starts_with("~$") || n.starts_with("._")
}

/// Een bestand herkennen, ook als het vervangen is door een nieuwe versie met dezelfde naam.
pub fn sleutel(b: &Bestand) -> String {
    format!("{}:{}", b.fileid, b.hash)
}

const GEZIEN: &str = "gezien.json";

/// De bestanden die de vorige keer al in de lijst stonden. None = de app heeft nog nooit een lijst gezien.
pub fn laad_gezien() -> Option<std::collections::HashSet<String>> {
    let s = std::fs::read_to_string(crate::paden::bestand(GEZIEN)).ok()?;
    serde_json::from_str::<Vec<String>>(&s).ok().map(|v| v.into_iter().collect())
}

pub fn bewaar_gezien(lijst: &[Bestand]) {
    let v: Vec<String> = lijst.iter().map(sleutel).collect();
    if let Ok(s) = serde_json::to_string(&v) {
        let _ = crate::voorkeuren::schrijf_veilig(&crate::paden::bestand(GEZIEN), s.as_bytes());
    }
}

/// Welke bestanden nieuw zijn ten opzichte van `gezien` (als nummer in de lijst).
pub fn nieuw_sinds(gezien: &std::collections::HashSet<String>, lijst: &[Bestand]) -> Vec<usize> {
    lijst.iter().enumerate().filter(|(_, b)| !gezien.contains(&sleutel(b))).map(|(i, _)| i).collect()
}

/// Namen die nieuw zijn in de nieuwe lijst (om kort te melden).
pub fn nieuwe_namen(oud: &[Bestand], nieuw: &[Bestand]) -> Vec<String> {
    nieuw
        .iter()
        .filter(|n| !oud.iter().any(|o| o.fileid == n.fileid))
        .map(|n| n.naam.clone())
        .collect()
}
