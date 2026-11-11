//! Downloaden uit de openbare map: het adres ophalen (getpublinkdownload) en het bestand in stukjes
//! naar de schijf schrijven. Hervatten met HTTP Range. Na afloop: controleren of het bestand compleet is.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde_json::Value;

use crate::fout::{io_tekst, Fout, Stop, Uitkomst};
use crate::instellingen::{LIMIET_LIJST_SECONDEN, PCLOUD_DOWNLOAD_CODE, STUK_BYTES};
use crate::overdracht::{deel_pad, Voortgang};
use crate::pcloud::lijst::Bestand;

/// Het downloadadres van een bestand. Pas aanroepen als de gebruiker echt wil downloaden (zo vraagt pCloud het).
pub fn download_adres(fileid: u64) -> Uitkomst<String> {
    let id = fileid.to_string();
    let json = super::api_get(
        "getpublinkdownload",
        &[("code", PCLOUD_DOWNLOAD_CODE), ("fileid", &id), ("forcedownload", "1")],
        Duration::from_secs(LIMIET_LIJST_SECONDEN),
    )?;
    let host = json.pointer("/hosts/0").and_then(Value::as_str).ok_or_else(|| Fout::Anders("no download host".into()))?;
    let pad = json.get("path").and_then(Value::as_str).ok_or_else(|| Fout::Anders("no download path".into()))?;
    Ok(format!("https://{host}{pad}"))
}

/// Downloadt een bestand uit de lijst naar `doel`. Staat er al een half bestand, dan gaat hij daar verder.
/// Bij Pauze blijft het halve bestand staan; bij Annuleer wordt het weggehaald.
pub fn download(b: &Bestand, doel: &Path, v: &Voortgang) -> Uitkomst<()> {
    let url = download_adres(b.fileid)?;
    download_url(&url, doel, Some(b.grootte), v)
}

/// Een hele map downloaden: elk bestand naar zijn eigen plek (de mappen worden gemaakt). Eén voortgang voor alles.
/// Stoppen kan alleen met Annuleer: het halve bestand gaat weg, de bestanden die al klaar zijn blijven staan.
pub fn download_map(bestanden: &[(Bestand, std::path::PathBuf)], v: &Voortgang) -> Uitkomst<()> {
    let totaal: u64 = bestanden.iter().map(|(b, _)| b.grootte).sum();
    v.vast_totaal.store(true, Ordering::Relaxed);
    v.totaal.store(totaal, Ordering::Relaxed);
    let mut klaar = 0u64;
    for (b, doel) in bestanden {
        if let Some(s) = v.stop() {
            return Err(Fout::Gestopt(s));
        }
        if let Some(map) = doel.parent() {
            std::fs::create_dir_all(map).map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
        }
        v.voor.store(klaar, Ordering::Relaxed);
        download(b, doel, v)?;
        klaar += b.grootte;
    }
    v.gedaan.store(totaal, Ordering::Relaxed);
    Ok(())
}

/// Een naam die Windows goed vindt (geen tekens als : of ?).
pub fn veilige_naam(naam: &str) -> String {
    let s: String = naam.chars().map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c }).collect();
    let s = s.trim_end_matches(['.', ' ']).to_string();
    if s.is_empty() || s == ".." { "_".into() } else { s }
}

/// Alle bestanden in de map `pad` (ook in submappen), elk met de plek waar het komt: `doel` + de mappen eronder.
pub fn bestanden_in_map(lijst: &[Bestand], pad: &str, doel: &Path) -> Vec<(Bestand, std::path::PathBuf)> {
    let mut uit = Vec::new();
    for b in lijst {
        let rest = if b.map == pad {
            ""
        } else if let Some(r) = b.map.strip_prefix(&format!("{pad}/")) {
            r
        } else {
            continue;
        };
        let mut plek = doel.to_path_buf();
        for deel in rest.split('/').filter(|d| !d.is_empty()) {
            plek.push(veilige_naam(deel));
        }
        plek.push(veilige_naam(&b.naam));
        uit.push((b.clone(), plek));
    }
    uit
}

/// Downloadt een adres naar `doel` (via een half bestand naast het doel). Ook gebruikt voor updates.
pub fn download_url(url: &str, doel: &Path, verwacht: Option<u64>, v: &Voortgang) -> Uitkomst<()> {
    let deel = deel_pad(doel);
    let mut al = std::fs::metadata(&deel).map(|m| m.len()).unwrap_or(0);
    if let Some(t) = verwacht {
        if al > t {
            al = 0;
        }
    }
    let uitkomst = stroom(url, &deel, al, verwacht, v);
    match uitkomst {
        Ok(()) => {
            let lengte = std::fs::metadata(&deel).map(|m| m.len()).unwrap_or(0);
            if verwacht.is_some_and(|t| t != lengte) {
                let _ = std::fs::remove_file(&deel);
                return Err(Fout::Onvolledig);
            }
            if doel.exists() {
                std::fs::remove_file(doel).map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
            }
            std::fs::rename(&deel, doel).map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
            Ok(())
        }
        Err(Fout::Gestopt(Stop::Pauze)) => Err(Fout::Gestopt(Stop::Pauze)),
        Err(f) => {
            // Bij annuleren gaat het halve bestand weg. Bij een fout van internet blijft het staan,
            // zodat een nieuwe poging verder kan.
            if matches!(f, Fout::Gestopt(Stop::Annuleer) | Fout::Opslaan(_) | Fout::Onvolledig) {
                let _ = std::fs::remove_file(&deel);
            }
            Err(f)
        }
    }
}

fn stroom(url: &str, deel: &Path, al: u64, verwacht: Option<u64>, v: &Voortgang) -> Uitkomst<()> {
    let mut req = crate::net::get(url);
    if al > 0 {
        req = req.header("Range", &format!("bytes={al}-"));
    }
    let resp = req.call().map_err(crate::net::vertaal)?;
    let hervat = al > 0 && resp.status().as_u16() == 206;
    let begin = if hervat { al } else { 0 };
    let rest: Option<u64> = resp.headers().get("content-length").and_then(|h| h.to_str().ok()).and_then(|s| s.parse().ok());
    let totaal = verwacht.or(rest.map(|r| r + begin)).unwrap_or(0);
    let voor = v.voor.load(Ordering::Relaxed);
    if !v.vast_totaal.load(Ordering::Relaxed) {
        v.totaal.store(totaal, Ordering::Relaxed);
    }
    v.gedaan.store(voor + begin, Ordering::Relaxed);
    if voor == 0 {
        v.begin.store(begin, Ordering::Relaxed);
    }

    let bestand: File = if hervat {
        OpenOptions::new().append(true).open(deel)
    } else {
        File::create(deel)
    }
    .map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
    let mut uit = BufWriter::with_capacity(STUK_BYTES, bestand);
    let mut lezer = resp.into_body().into_reader();
    let mut buf = vec![0u8; STUK_BYTES];
    let mut gedaan = begin;
    loop {
        if let Some(s) = v.stop() {
            uit.flush().map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
            return Err(Fout::Gestopt(s));
        }
        let n = match lezer.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                let _ = uit.flush();
                return Err(crate::net::vertaal_io(&e));
            }
        };
        uit.write_all(&buf[..n]).map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
        gedaan += n as u64;
        v.gedaan.store(voor + gedaan, Ordering::Relaxed);
    }
    uit.flush().map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
    Ok(())
}
