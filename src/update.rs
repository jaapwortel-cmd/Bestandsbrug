//! Updates via Jaaps alleen-lezen Dropbox-map. In de map staan Bestandsbrug-Setup.exe en versie.txt.
//! versie.txt heeft regels "sleutel = waarde": versie, bestand, sha256, nieuw_nl en nieuw_en (mag vaker).
//! Eerst proberen we versie.txt los op te halen. Lukt dat niet, dan downloaden we de hele map als zip,
//! zoals Jaaps andere programma's doen. Het installatiebestand wordt altijd met de SHA-256 gecontroleerd.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::fout::{io_tekst, Fout, Uitkomst};
use crate::instellingen::{MAX_UPDATEMAP_BYTES, SETUP_BESTAND, VERSIE, VERSIE_BESTAND};
use crate::overdracht::Voortgang;
use crate::taal::Taal;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VersieInfo {
    pub versie: String,
    pub bestand: String,
    pub sha256: String,
    pub nieuw_nl: String,
    pub nieuw_en: String,
}

impl VersieInfo {
    pub fn nieuw(&self, t: Taal) -> &str {
        match t {
            Taal::Nl if !self.nieuw_nl.is_empty() => &self.nieuw_nl,
            Taal::En if !self.nieuw_en.is_empty() => &self.nieuw_en,
            _ if !self.nieuw_nl.is_empty() => &self.nieuw_nl,
            _ => &self.nieuw_en,
        }
    }
}

/// Waar de update vandaan komt: losse bestanden, of de hele map als zip (al gedownload).
#[derive(Debug, Clone)]
pub enum Bron {
    Los,
    Zip(PathBuf),
}

/// Leest versie.txt. Geeft None als het geen geldig versiebestand is.
pub fn lees_versie_txt(tekst: &str) -> Option<VersieInfo> {
    let mut i = VersieInfo::default();
    for regel in tekst.trim_start_matches('\u{feff}').lines() {
        let r = regel.trim();
        if r.is_empty() || r.starts_with('#') {
            continue;
        }
        let Some((k, w)) = r.split_once('=') else { continue };
        let w = w.trim().to_string();
        match k.trim().to_ascii_lowercase().as_str() {
            "versie" | "version" => i.versie = w,
            "bestand" | "file" => i.bestand = w,
            "sha256" => i.sha256 = w.to_ascii_lowercase(),
            "nieuw_nl" => voeg_regel_toe(&mut i.nieuw_nl, &w),
            "nieuw_en" => voeg_regel_toe(&mut i.nieuw_en, &w),
            _ => {}
        }
    }
    if versie_delen(&i.versie).is_none() || i.sha256.len() != 64 || !i.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    if i.bestand.is_empty() {
        i.bestand = SETUP_BESTAND.to_string();
    }
    // Veiligheid: alleen een gewone bestandsnaam, geen map.
    if i.bestand.contains(['/', '\\']) || i.bestand.contains("..") {
        return None;
    }
    Some(i)
}

fn voeg_regel_toe(s: &mut String, regel: &str) {
    if !s.is_empty() {
        s.push('\n');
    }
    s.push_str(regel);
}

fn versie_delen(v: &str) -> Option<Vec<u64>> {
    let d: Option<Vec<u64>> = v.trim().split('.').map(|x| x.parse().ok()).collect();
    d.filter(|x| !x.is_empty())
}

/// Is versie a nieuwer dan b? (1.10.0 is nieuwer dan 1.9.0.)
pub fn is_nieuwer(a: &str, b: &str) -> bool {
    match (versie_delen(a), versie_delen(b)) {
        (Some(mut x), Some(mut y)) => {
            let n = x.len().max(y.len());
            x.resize(n, 0);
            y.resize(n, 0);
            x > y
        }
        _ => false,
    }
}

/// Moet de automatische controle dit melden? Alleen als het nieuwer is dan deze versie
/// en nieuwer dan de versie waarvoor de gebruiker "Later" koos.
pub fn moet_automatisch_melden(nieuw: &str, later: Option<&str>) -> bool {
    is_nieuwer(nieuw, VERSIE) && later.is_none_or(|l| is_nieuwer(nieuw, l))
}

/// Het directe adres van een bestand in de gedeelde map, of van de hele map (bestand = None, geeft een zip).
pub fn adres(link: &str, bestand: Option<&str>) -> String {
    let (basis, query) = link.split_once('?').unwrap_or((link, ""));
    let rlkey = query.split('&').find(|p| p.starts_with("rlkey=")).unwrap_or("");
    let basis = basis.trim_end_matches('/');
    let pad = match bestand {
        Some(b) => format!("{basis}/{}", b.replace(' ', "%20")),
        None => basis.to_string(),
    };
    if rlkey.is_empty() {
        format!("{pad}?dl=1")
    } else {
        format!("{pad}?{rlkey}&dl=1")
    }
}

/// Kijkt in de updatemap welke versie er staat.
pub fn haal_versie_info(link: &str, limiet: Duration) -> Uitkomst<(VersieInfo, Bron)> {
    // 1. versie.txt los ophalen (klein en snel).
    let los = crate::net::met_limiet(crate::net::get(&adres(link, Some(VERSIE_BESTAND))), limiet).call();
    match los {
        Ok(resp) => {
            let mut buf = Vec::new();
            let _ = resp.into_body().into_reader().take(64 * 1024).read_to_end(&mut buf);
            if let Some(i) = lees_versie_txt(&String::from_utf8_lossy(&buf)) {
                return Ok((i, Bron::Los));
            }
        }
        Err(e) => {
            let f = crate::net::vertaal(e);
            if f.is_geen_internet() {
                return Err(f);
            }
        }
    }
    // 2. De hele map als zip, zoals Jaaps andere programma's.
    let zip = paden_zip();
    let resp = crate::net::met_limiet(crate::net::get(&adres(link, None)), limiet.max(Duration::from_secs(60))).call().map_err(crate::net::vertaal)?;
    {
        let mut uit = std::fs::File::create(&zip).map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
        let n = std::io::copy(&mut resp.into_body().into_reader().take(MAX_UPDATEMAP_BYTES + 1), &mut uit).map_err(|e| crate::net::vertaal_io(&e))?;
        if n > MAX_UPDATEMAP_BYTES {
            return Err(Fout::Anders("update folder too big".into()));
        }
    }
    let tekst = lees_uit_zip(&zip, VERSIE_BESTAND, 64 * 1024)?;
    let info = lees_versie_txt(&String::from_utf8_lossy(&tekst)).ok_or_else(|| Fout::Anders("no valid versie.txt in update folder".into()))?;
    Ok((info, Bron::Zip(zip)))
}

fn paden_zip() -> PathBuf {
    crate::paden::tijdelijk().join("updatemap.zip")
}

/// Leest één bestand uit de zip (alleen op naam, mappen in de zip worden genegeerd).
fn lees_uit_zip(zip: &Path, naam: &str, max: u64) -> Uitkomst<Vec<u8>> {
    let f = std::fs::File::open(zip).map_err(|e| Fout::Lezen(io_tekst(&e)))?;
    let mut archief = zip::ZipArchive::new(f).map_err(|e| Fout::Anders(format!("zip: {e}")))?;
    for i in 0..archief.len() {
        let mut item = archief.by_index(i).map_err(|e| Fout::Anders(format!("zip: {e}")))?;
        let itemnaam = item.name().rsplit('/').next().unwrap_or("").to_string();
        if itemnaam.eq_ignore_ascii_case(naam) && item.is_file() {
            let mut buf = Vec::new();
            (&mut item).take(max).read_to_end(&mut buf).map_err(|e| Fout::Lezen(io_tekst(&e)))?;
            return Ok(buf);
        }
    }
    Err(Fout::Anders(format!("{naam} not found in update folder")))
}

/// Downloadt de nieuwe versie (met voortgang) en controleert de SHA-256. Geeft het pad van het installatiebestand.
pub fn haal_update(link: &str, info: &VersieInfo, bron: &Bron, v: &Voortgang) -> Uitkomst<PathBuf> {
    let doel = crate::paden::tijdelijk().join(&info.bestand);
    match bron {
        Bron::Los => crate::pcloud::download::download_url(&adres(link, Some(&info.bestand)), &doel, None, v)?,
        Bron::Zip(zip) => {
            let data = lees_uit_zip(zip, &info.bestand, MAX_UPDATEMAP_BYTES)?;
            std::fs::write(&doel, &data).map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
            v.totaal.store(data.len() as u64, std::sync::atomic::Ordering::Relaxed);
            v.gedaan.store(data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        }
    }
    if !sha256_klopt(&doel, &info.sha256)? {
        let _ = std::fs::remove_file(&doel);
        return Err(Fout::Beschadigd);
    }
    Ok(doel)
}

/// SHA-256 van een bestand, in stukjes gelezen.
pub fn sha256_van(pad: &Path) -> Uitkomst<String> {
    let mut f = std::fs::File::open(pad).map_err(|e| Fout::Lezen(io_tekst(&e)))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; crate::instellingen::STUK_BYTES];
    loop {
        let n = f.read(&mut buf).map_err(|e| Fout::Lezen(io_tekst(&e)))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn sha256_klopt(pad: &Path, verwacht: &str) -> Uitkomst<bool> {
    Ok(sha256_van(pad)?.eq_ignore_ascii_case(verwacht.trim()))
}
