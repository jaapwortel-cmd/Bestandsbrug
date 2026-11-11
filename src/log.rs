//! Het logbestand: korte Engelse regels over opstarten, afsluiten, de lijst, downloads, uploads, onderbrekingen,
//! updates en fouten. Geen wachtwoorden, geen sleutels, geen paden, geen persoonlijke gegevens.
//! Het bestand blijft klein: alleen het laatste stuk (zie LOG_MAX_BYTES) blijft bewaard.

use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Mutex;

use crate::instellingen::{LOG_HOUD_BYTES, LOG_MAX_BYTES};

static SLOT: Mutex<()> = Mutex::new(());

pub fn info(tekst: &str) {
    schrijf("INFO", tekst);
}

pub fn fout(tekst: &str) {
    schrijf("ERROR", tekst);
}

fn schrijf(soort: &str, tekst: &str) {
    let _slot = SLOT.lock();
    let pad = crate::paden::logbestand();
    let regel = format!("{} {} {}\r\n", crate::tijd::nu_tekst(), soort, tekst.replace(['\r', '\n'], " "));
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&pad) {
        let _ = f.write_all(regel.as_bytes());
        if let Ok(m) = f.metadata() {
            if m.len() > LOG_MAX_BYTES {
                drop(f);
                let _ = kort_in(&pad, LOG_HOUD_BYTES);
            }
        }
    }
}

/// Houdt alleen de laatste `houd` bytes over, vanaf het begin van een regel.
pub fn kort_in(pad: &Path, houd: u64) -> std::io::Result<()> {
    let mut f = std::fs::File::open(pad)?;
    let lengte = f.metadata()?.len();
    if lengte <= houd {
        return Ok(());
    }
    f.seek(SeekFrom::Start(lengte - houd))?;
    let mut rest = Vec::with_capacity(houd as usize);
    f.read_to_end(&mut rest)?;
    drop(f);
    let begin = rest.iter().position(|&b| b == b'\n').map(|p| p + 1).unwrap_or(0);
    let tmp = pad.with_extension("log.tmp");
    std::fs::write(&tmp, &rest[begin..])?;
    std::fs::rename(&tmp, pad)
}
