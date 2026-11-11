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
