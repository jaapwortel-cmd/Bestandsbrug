//! Een lopende download of upload op het scherm: starten op de achtergrond, de voortgangsbalk (alleen voor
//! de ogen), de melding elke 6 seconden, de Escape-vraag, afsluiten tijdens een overdracht en het einde.
//! Eén overdracht tegelijk.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use bestandsbrug::fout::{Fout, Stop, Uitkomst};
use bestandsbrug::melding::{self, MeldTimer, Soort};
use bestandsbrug::overdracht::{self, Onderbroken, Voortgang};
use bestandsbrug::pcloud::lijst::Bestand;
use bestandsbrug::taal::{f, t};
use bestandsbrug::update::{Bron, VersieInfo};
use bestandsbrug::log;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::*;

pub enum Taak {
    Download { bestand: Bestand, doel: PathBuf },
    Upload { pad: PathBuf, grootte: u64 },
    Update { info: VersieInfo, bron: Bron },
    /// Een hele map kopiëren: alle bestanden met hun plek op de computer.
    Map { naam: String, bestanden: Vec<(Bestand, PathBuf)>, doel: PathBuf },
}

impl Taak {
    fn soort(&self) -> Soort {
        match self {
            Taak::Upload { .. } => Soort::Upload,
            _ => Soort::Download,
        }
    }
    /// Alleen een gewone download kan "volgende keer verdergaan".
    fn kan_later(&self) -> bool {
        matches!(self, Taak::Download { .. })
    }
    /// Waarom niet later verder (voor uploads en updates).
    fn uitleg(&self) -> String {
        match self {
            Taak::Upload { .. } => t("EscUploadUitleg"),
            Taak::Update { .. } => t("EscUpdateUitleg"),
            Taak::Map { .. } => t("EscMapUitleg"),
            Taak::Download { .. } => String::new(),
        }
    }
}

pub struct Lopend {
    pub taak: Taak,
    pub naam: String,
    pub v: Arc<Voortgang>,
    pub timer: MeldTimer,
    pub nummer: u64,
}

pub struct Klaar {
    nummer: u64,
    uitkomst: Uitkomst<Option<PathBuf>>,
}

static NUMMER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub fn bezig() -> bool {
    met(|s| s.lopend.is_some()).unwrap_or(false)
}

/// Start een overdracht op de achtergrond. Het scherm blijft direct reageren.
pub fn start(taak: Taak, naam: String) {
    if bezig() {
        zeg(&t("AlBezig"), Belang::Belangrijk);
        return;
    }
    let v = Arc::new(Voortgang::new());
    let nummer = NUMMER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let h = hwnd();
    let werk: Box<dyn FnOnce(&Voortgang) -> Uitkomst<Option<PathBuf>> + Send> = match &taak {
        Taak::Download { bestand, doel } => {
            let (b, d) = (bestand.clone(), doel.clone());
            log::info(&format!("Download start: {} ({} bytes)", b.naam, b.grootte));
            Box::new(move |v| bestandsbrug::pcloud::download::download(&b, &d, v).map(|_| None))
        }
        Taak::Upload { pad, grootte } => {
            let p = pad.clone();
            log::info(&format!("Upload start: {} ({} bytes)", naam, grootte));
            Box::new(move |v| bestandsbrug::pcloud::upload::upload(&p, v).map(|_| None))
        }
        Taak::Map { naam: mapnaam, bestanden, .. } => {
            let l = bestanden.clone();
            let bytes: u64 = l.iter().map(|(b, _)| b.grootte).sum();
            log::info(&format!("Folder download start: {} ({} files, {} bytes)", mapnaam, l.len(), bytes));
            Box::new(move |v| bestandsbrug::pcloud::download::download_map(&l, v).map(|_| None))
        }
        Taak::Update { info, bron } => {
            let (i, b) = (info.clone(), bron.clone());
            log::info(&format!("Update download start: {}", i.versie));
            Box::new(move |v| bestandsbrug::update::haal_update(bestandsbrug::instellingen::UPDATE_LINK, &i, &b, v).map(Some))
        }
    };
    let sleutel = if taak.soort() == Soort::Upload { "MeldUploadStart" } else { "MeldDownloadStart" };
    zeg(&f(sleutel, &[&naam]), Belang::Belangrijk);
    met(|s| s.lopend = Some(Lopend { taak, naam, v: v.clone(), timer: MeldTimer::start(Instant::now()), nummer }));
    zet_balk(0);
    unsafe {
        SetTimer(Some(h), TIMER_MELD, 500, None);
        SetTimer(Some(h), TIMER_BALK, bestandsbrug::instellingen::BALK_VERVERSEN_MS, None);
    }
    let hw = h.0 as isize;
    std::thread::spawn(move || {
        let uitkomst = werk(&v);
        let klaar = Box::new(Klaar { nummer, uitkomst });
        let hwnd = windows::Win32::Foundation::HWND(hw as *mut _);
        win::post(hwnd, WM_OVERDRACHT_KLAAR, 0, Box::into_raw(klaar) as isize);
    });
}

/// Elke halve seconde: is het tijd voor de melding van 6 seconden?
pub fn meld_timer() {
    if vraag_open() {
        return;
    }
    let tekst = met(|s| {
        let l = s.lopend.as_mut()?;
        if !l.timer.is_tijd(Instant::now()) {
            return None;
        }
        Some(melding::voortgang_tekst(l.taak.soort(), l.v.gedaan(), l.v.totaal(), l.v.deze_keer(), l.v.gestart.elapsed()))
    })
    .flatten();
    if let Some(t) = tekst {
        zeg(&t, Belang::Gewoon);
    }
}

/// Elke 0,2 seconde: de balk bijwerken (alleen voor de ogen) en de waarde voor de schermlezer klaarzetten.
pub fn balk_timer() {
    let stand = met(|s| s.lopend.as_ref().map(|l| (l.v.gedaan(), l.v.totaal()))).flatten();
    if let Some((gedaan, totaal)) = stand {
        let promille = if totaal == 0 { 0 } else { (gedaan.min(totaal) as u128 * 1000 / totaal as u128) as u32 };
        zet_balk_waarde(promille);
    }
}

fn zet_balk_waarde(promille: u32) {
    let oud = balk();
    if oud == promille {
        return;
    }
    zet_balk(promille);
    let o = hoofdscherm::onderdelen();
    if oud / 10 != promille / 10 {
        spraak::zet_balk_toegankelijk(o.balk, &f("BalkWaarde", &[&(promille / 10).to_string()]));
    }
    unsafe {
        let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(o.balk), None, false);
    }
}

/// De achtergrond is klaar (gelukt, mislukt, gestopt of geannuleerd).
pub fn klaar(k: Klaar) {
    let h = hwnd();
    let lopend = met(|s| if s.lopend.as_ref().map(|l| l.nummer) == Some(k.nummer) { s.lopend.take() } else { None }).flatten();
    let Some(l) = lopend else { return };
    unsafe {
        let _ = KillTimer(Some(h), TIMER_MELD);
        let _ = KillTimer(Some(h), TIMER_BALK);
    }
    match &k.uitkomst {
        Ok(_) => zet_balk_waarde(1000),
        Err(_) => {}
    }
    match (l.taak, k.uitkomst) {
        (Taak::Download { bestand, doel }, uitkomst) => download_klaar(bestand, doel, uitkomst.map(|_| ())),
        (Taak::Upload { grootte, .. }, uitkomst) => match uitkomst {
            Ok(_) => {
                log::info(&format!("Upload done: {} ({} bytes)", l.naam, grootte));
                zeg(&f("MeldUploadKlaar", &[&l.naam]), Belang::Belangrijk);
            }
            Err(Fout::Gestopt(_)) => {
                log::info(&format!("Upload cancelled: {}", l.naam));
                zeg(&t("MeldUploadGeannuleerd"), Belang::Belangrijk);
            }
            Err(e) => {
                log::fout(&format!("Upload failed: {}: {}", l.naam, e.log_tekst()));
                zeg(&f("MeldUploadFout", &[&e.tekst()]), Belang::Belangrijk);
            }
        },
        (Taak::Update { info, .. }, uitkomst) => super::update_ui::update_binnen(info, uitkomst),
        (Taak::Map { naam, bestanden, doel }, uitkomst) => match uitkomst {
            Ok(_) => {
                log::info(&format!("Folder download done: {} ({} files)", naam, bestanden.len()));
                let waar = doel.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                zeg(&f("MeldMapKlaar", &[&naam, &waar, &bestanden.len().to_string()]), Belang::Belangrijk);
            }
            Err(Fout::Gestopt(_)) => {
                log::info(&format!("Folder download cancelled: {}", naam));
                zeg(&t("MeldMapGeannuleerd"), Belang::Belangrijk);
            }
            Err(e) => {
                log::fout(&format!("Folder download failed: {}: {}", naam, e.log_tekst()));
                zeg(&f("MeldDownloadFout", &[&e.tekst()]), Belang::Belangrijk);
            }
        },
    }
    if !matches!(balk(), 1000) {
        zet_balk_waarde(0);
    }
    na_overdracht();
}

fn download_klaar(b: Bestand, doel: PathBuf, uitkomst: Uitkomst<()>) {
    let onderbroken = Onderbroken { fileid: b.fileid, naam: b.naam.clone(), grootte: b.grootte, hash: b.hash, doel: doel.clone() };
    match uitkomst {
        Ok(()) => {
            overdracht::verwijder(&doel);
            log::info(&format!("Download done: {} ({} bytes)", b.naam, b.grootte));
            let map = doel.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            zeg(&f("MeldDownloadKlaar", &[&b.naam, &map]), Belang::Belangrijk);
        }
        Err(Fout::Gestopt(Stop::Pauze)) => {
            log::info(&format!("Download paused: {} at {}%", b.naam, onderbroken.procent()));
            overdracht::voeg_toe(onderbroken);
            zeg(&t("MeldGepauzeerd"), Belang::Belangrijk);
        }
        Err(Fout::Gestopt(Stop::Annuleer)) => {
            overdracht::verwijder(&doel);
            log::info(&format!("Download cancelled: {}", b.naam));
            zeg(&t("MeldDownloadGeannuleerd"), Belang::Belangrijk);
        }
        Err(e) => {
            log::fout(&format!("Download failed: {} ({} bytes): {}", b.naam, b.grootte, e.log_tekst()));
            // Een half bestand na een internetfout: onthouden, zodat het later verder kan.
            if onderbroken.deel().exists() {
                overdracht::voeg_toe(onderbroken);
            }
            zeg(&f("MeldDownloadFout", &[&e.tekst()]), Belang::Belangrijk);
        }
    }
}

/// Na elke overdracht: afsluiten als dat gevraagd was, of de uitgestelde updatecontrole doen.
fn na_overdracht() {
    let (sluiten, update) = met(|s| (s.afsluiten_na_overdracht, std::mem::take(&mut s.update_uitgesteld))).unwrap_or((false, false));
    if sluiten {
        unsafe {
            let _ = DestroyWindow(hwnd());
        }
        return;
    }
    if update {
        super::update_ui::automatisch();
    }
}

/// Escape tijdens een overdracht: Doorgaan, Volgende keer verdergaan (alleen downloads) of Alles nu annuleren.
pub fn escape() {
    let Some((nummer, naam, kan_later, upload, uitleg)) =
        met(|s| s.lopend.as_ref().map(|l| (l.nummer, l.naam.clone(), l.taak.kan_later(), l.taak.soort() == Soort::Upload, l.taak.uitleg()))).flatten()
    else {
        return;
    };
    let vraag = f(if upload { "EscVraagUpload" } else { "EscVraagDownload" }, &[&naam]);
    let keuze = vragen::vraag_overdracht(hwnd(), &vraag, kan_later, &uitleg);
    pas_keuze_toe(nummer, keuze);
}

/// Voert de keuze uit, als dezelfde overdracht nog loopt. Geeft true als er gestopt wordt.
fn pas_keuze_toe(nummer: u64, keuze: i32) -> bool {
    let v = met(|s| s.lopend.as_ref().filter(|l| l.nummer == nummer).map(|l| l.v.clone())).flatten();
    let Some(v) = v else { return false };
    match keuze {
        vragen::LATER => {
            v.vraag_stop(Stop::Pauze);
            true
        }
        vragen::ANNULEREN => {
            v.vraag_stop(Stop::Annuleer);
            true
        }
        _ => {
            zeg_later(&t("MeldVerder"), Belang::Gewoon);
            false
        }
    }
}

/// Afsluiten (Alt+F4, het kruisje of het menu). Altijd eerst vragen; "Nee" staat voorop.
pub fn afsluiten() {
    let h = hwnd();
    let lopend = met(|s| s.lopend.as_ref().map(|l| (l.nummer, l.naam.clone(), l.taak.kan_later(), l.taak.uitleg()))).flatten();
    if let Some((nummer, naam, kan_later, uitleg)) = lopend {
        let vraag = format!("{}\n{}", t("AfsluitenVraag"), f("AfsluitenTijdensOverdracht", &[&naam]));
        let keuze = vragen::vraag_overdracht(h, &vraag, kan_later, &uitleg);
        if pas_keuze_toe(nummer, keuze) {
            // Afsluiten zodra de achtergrond netjes gestopt is.
            met(|s| s.afsluiten_na_overdracht = true);
        }
        return;
    }
    if vragen::ja_nee(h, &t("AfsluitenVraag"), false) {
        unsafe {
            let _ = DestroyWindow(h);
        }
    }
}

/// Windows sluit af: een lopende download onthouden, zodat hij de volgende keer verder kan.
pub fn windows_stopt() {
    let info = met(|s| {
        s.lopend.as_ref().and_then(|l| match &l.taak {
            Taak::Download { bestand, doel } => {
                l.v.vraag_stop(Stop::Pauze);
                Some(Onderbroken { fileid: bestand.fileid, naam: bestand.naam.clone(), grootte: bestand.grootte, hash: bestand.hash, doel: doel.clone() })
            }
            _ => None,
        })
    })
    .flatten();
    if let Some(o) = info {
        log::info(&format!("Windows shuts down, download paused: {}", o.naam));
        overdracht::voeg_toe(o);
    }
}
