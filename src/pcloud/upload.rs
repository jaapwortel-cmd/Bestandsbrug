//! Uploaden naar de uploadlink (uploadtolink). Het bestand gaat in stukjes van de schijf naar pCloud,
//! nooit helemaal in het geheugen. Vooraf: kijken of de link nog werkt en of er een grens is (showuploadlink).
//! Hervatten kan niet: pCloud heeft voor uploadlinks geen methode om verder te gaan. Daarom gebruiken we
//! nopartial=1: een afgebroken upload laat geen half bestand achter bij Jaap.

use std::fs::File;
use std::io::{Chain, Cursor, Read};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde_json::Value;

use crate::fout::{io_tekst, Fout, Uitkomst};
use crate::instellingen::{LIMIET_LIJST_SECONDEN, PCLOUD_UPLOADER_NAAM, PCLOUD_UPLOAD_CODE, STUK_BYTES};
use crate::overdracht::Voortgang;

/// Kijkt of de uploadlink nog werkt en of dit bestand mag (grens aan ruimte of aantal bestanden).
pub fn controleer_vooraf(grootte: u64) -> Uitkomst<()> {
    let json = super::api_get(
        "showuploadlink",
        &[("code", PCLOUD_UPLOAD_CODE)],
        Duration::from_secs(LIMIET_LIJST_SECONDEN),
    )?;
    controleer_grens(&json, grootte)
}

/// De grens uit het antwoord van showuploadlink (0 = geen grens).
pub fn controleer_grens(json: &Value, grootte: u64) -> Uitkomst<()> {
    let getal = |k: &str| json.get(k).and_then(Value::as_u64).unwrap_or(0);
    let maxspace = getal("maxspace");
    if maxspace > 0 {
        let gebruikt = getal("space").max(getal("usedspace"));
        let mag_nog = maxspace.saturating_sub(gebruikt);
        if mag_nog == 0 {
            return Err(Fout::UploadVol);
        }
        if grootte > mag_nog {
            return Err(Fout::TeGroot { mag_nog, bestand: grootte });
        }
    }
    let maxfiles = getal("maxfiles");
    if maxfiles > 0 && getal("files") >= maxfiles {
        return Err(Fout::UploadVol);
    }
    Ok(())
}

/// Leest het bestand en telt mee voor de voortgangsbalk. Stopt meteen als de gebruiker annuleert.
struct TelLezer<'a, R> {
    binnen: R,
    v: &'a Voortgang,
    gedaan: u64,
}

impl<R: Read> Read for TelLezer<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.v.stop().is_some() {
            return Err(std::io::Error::other("stopped by user"));
        }
        let n = self.binnen.read(buf)?;
        self.gedaan += n as u64;
        self.v.gedaan.store(self.gedaan.min(self.v.totaal()), Ordering::Relaxed);
        Ok(n)
    }
}

/// Maakt de multipart-body: kop, het bestand zelf (gestreamd) en het slot.
fn body(pad: &Path, naam: &str, grens: &str) -> Uitkomst<(Chain<Chain<Cursor<Vec<u8>>, File>, Cursor<Vec<u8>>>, u64)> {
    let bestand = File::open(pad).map_err(|e| Fout::Lezen(io_tekst(&e)))?;
    let grootte = bestand.metadata().map_err(|e| Fout::Lezen(io_tekst(&e)))?.len();
    let veilige_naam = naam.replace(['"', '\r', '\n'], "_");
    let kop = format!(
        "--{grens}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{veilige_naam}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    let slot = format!("\r\n--{grens}--\r\n").into_bytes();
    let lengte = kop.len() as u64 + grootte + slot.len() as u64;
    Ok((Cursor::new(kop).chain(bestand).chain(Cursor::new(slot)), lengte))
}

/// Uploadt het bestand naar Jaaps map Uploads.
pub fn upload(pad: &Path, v: &Voortgang) -> Uitkomst<()> {
    let naam = pad.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "bestand".into());
    let grootte = std::fs::metadata(pad).map_err(|e| Fout::Lezen(io_tekst(&e)))?.len();
    controleer_vooraf(grootte)?;
    let grens = format!("----{}{:016x}", crate::instellingen::TECHNISCHE_NAAM, crate::tijd::nu_unix() as u64 ^ grootte.rotate_left(17));
    let (lichaam, lengte) = body(pad, &naam, &grens)?;
    v.totaal.store(lengte, Ordering::Relaxed);
    v.gedaan.store(0, Ordering::Relaxed);
    v.begin.store(0, Ordering::Relaxed);
    let mut lezer = std::io::BufReader::with_capacity(STUK_BYTES, TelLezer { binnen: lichaam, v, gedaan: 0 });
    let url = format!("{}/uploadtolink", super::server());
    let resp = crate::net::post(&url)
        .query("code", PCLOUD_UPLOAD_CODE)
        .query("names", PCLOUD_UPLOADER_NAAM)
        .query("nopartial", "1")
        .header("Content-Type", &format!("multipart/form-data; boundary={grens}"))
        .header("Content-Length", &lengte.to_string())
        .send(ureq::SendBody::from_reader(&mut lezer));
    if let Some(s) = v.stop() {
        return Err(Fout::Gestopt(s));
    }
    let resp = resp.map_err(crate::net::vertaal)?;
    let json: Value = crate::net::lees_json(resp)?;
    super::controleer(json)?;
    v.gedaan.store(lengte, Ordering::Relaxed);
    Ok(())
}

/// Een hele map uploaden. Een uploadlink van pCloud kan zelf geen mappen maken. Daarom maken we eerst op de eigen
/// computer één zip-bestand ("Mapnaam.zip") met de map en alles erin (mappen blijven mappen), en sturen dat.
/// Het zip-bestand komt in de gegevensmap en wordt daarna weer weggehaald. Geeft het aantal bestanden.
pub fn upload_map(map: &Path, v: &Voortgang) -> Uitkomst<usize> {
    let naam = map.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "map".into());
    let werk = crate::paden::gegevensmap().join("upload");
    let _ = std::fs::remove_dir_all(&werk);
    std::fs::create_dir_all(&werk).map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
    let zip = werk.join(format!("{}.zip", crate::pcloud::download::veilige_naam(&naam)));
    let uitkomst = maak_zip(map, &naam, &zip, v).and_then(|aantal| upload(&zip, v).map(|_| aantal));
    let _ = std::fs::remove_dir_all(&werk);
    uitkomst
}

/// Alle bestanden in een map (ook in submappen), met hun naam in de zip ("Map/sub/bestand").
pub fn bestanden_in(map: &Path, naam: &str) -> Uitkomst<Vec<(std::path::PathBuf, String)>> {
    let mut uit = Vec::new();
    let mut stapel = vec![(map.to_path_buf(), naam.to_string())];
    while let Some((m, binnen)) = stapel.pop() {
        let mut items: Vec<_> = std::fs::read_dir(&m).map_err(|e| Fout::Lezen(io_tekst(&e)))?.flatten().collect();
        items.sort_by_key(|e| e.file_name());
        for e in items {
            let soort = match e.file_type() {
                Ok(s) => s,
                Err(_) => continue,
            };
            let n = format!("{binnen}/{}", e.file_name().to_string_lossy());
            if soort.is_dir() {
                stapel.push((e.path(), n));
            } else if soort.is_file() {
                uit.push((e.path(), n));
            }
        }
    }
    uit.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(uit)
}

/// Maakt het zip-bestand, in stukjes (nooit een heel bestand in het geheugen). Stoppen kan tussendoor.
pub fn maak_zip(map: &Path, naam: &str, zip: &Path, v: &Voortgang) -> Uitkomst<usize> {
    use std::io::Write;
    let bestanden = bestanden_in(map, naam)?;
    if bestanden.is_empty() {
        return Err(Fout::LegeMap);
    }
    let uit = File::create(zip).map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
    let mut z = zip::ZipWriter::new(std::io::BufWriter::with_capacity(STUK_BYTES, uit));
    let mut buf = vec![0u8; STUK_BYTES];
    for (pad, binnen) in &bestanden {
        let mut f = File::open(pad).map_err(|e| Fout::Lezen(io_tekst(&e)))?;
        let groot = f.metadata().map(|m| m.len()).unwrap_or(0) >= 0xFFFF_FFFF;
        // Niet inpakken (Stored): snel, en apps zijn meestal al ingepakt.
        let opties = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored).large_file(groot);
        z.start_file(binnen.as_str(), opties).map_err(|e| Fout::Opslaan(e.to_string()))?;
        loop {
            if let Some(s) = v.stop() {
                return Err(Fout::Gestopt(s));
            }
            let n = f.read(&mut buf).map_err(|e| Fout::Lezen(io_tekst(&e)))?;
            if n == 0 {
                break;
            }
            z.write_all(&buf[..n]).map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
        }
    }
    z.finish().map_err(|e| Fout::Opslaan(e.to_string()))?.flush().map_err(|e| Fout::Opslaan(io_tekst(&e)))?;
    Ok(bestanden.len())
}
