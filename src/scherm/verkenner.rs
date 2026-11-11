//! Het tabblad Downloaden werkt zoals de Verkenner van Windows: je ziet de mappen en bestanden van één map.
//! Enter op een map = die map openen. Backspace = terug naar de map erboven. Enter op een bestand = downloaden.
//! Het is een gewone lijst van Windows met kolommen (Naam, Soort, Grootte, Datum, Uitleg).

use std::cell::RefCell;

use bestandsbrug::opmaak;
use bestandsbrug::pcloud::lijst::{self, Bestand, Map};
use bestandsbrug::taal::{f, t};
use windows::Win32::Foundation::LPARAM;
use windows::Win32::UI::Controls::*;

use super::win::{self, snd};
use super::*;

/// Een regel in de lijst.
#[derive(Clone)]
pub enum Regel {
    Map { naam: String, pad: String, aantal: usize },
    Bestand(usize),
}

thread_local! {
    /// De map die nu open is ("" = de hoofdmap) en de regels die nu in de lijst staan.
    static HUIDIG: RefCell<(String, Vec<Regel>)> = const { RefCell::new((String::new(), Vec::new())) };
}

pub fn huidige_map() -> String {
    HUIDIG.with(|h| h.borrow().0.clone())
}

fn lijst_hwnd() -> windows::Win32::Foundation::HWND {
    hoofdscherm::onderdelen().lijst
}

/// Zoekt een map in de boom op zijn pad.
fn zoek<'a>(m: &'a Map, pad: &str) -> Option<&'a Map> {
    if pad.is_empty() {
        return Some(m);
    }
    let mut huidig = m;
    for deel in pad.split('/') {
        huidig = huidig.mappen.iter().find(|x| x.naam == deel)?;
    }
    Some(huidig)
}

fn tel(m: &Map) -> usize {
    m.mappen.len() + m.bestanden.len()
}

/// Wat er nu gekozen is.
pub fn keuze() -> Option<Regel> {
    let i = snd(lijst_hwnd(), LVM_GETNEXTITEM, usize::MAX, LVNI_SELECTED as isize);
    if i < 0 {
        return None;
    }
    HUIDIG.with(|h| h.borrow().1.get(i as usize).cloned())
}

/// Toont de map `pad` met de lijst `bestanden`. `kies` = de regel die gekozen moet zijn (naam van een map of fileid).
fn toon(bestanden: &[Bestand], pad: &str, kies: Option<Kies>) {
    let boom = lijst::maak_boom(bestanden);
    let (pad, map) = match zoek(&boom, pad) {
        Some(m) => (pad.to_string(), m.clone()),
        None => (String::new(), boom.clone()),
    };
    let mut regels: Vec<Regel> = map.mappen.iter().map(|m| Regel::Map { naam: m.naam.clone(), pad: m.pad.clone(), aantal: tel(m) }).collect();
    regels.extend(map.bestanden.iter().map(|&i| Regel::Bestand(i)));
    let nieuw = met(|s| s.nieuw.clone()).unwrap_or_default();
    let is_nieuw = |b: &Bestand| nieuw.contains(&lijst::sleutel(b));
    let map_nieuw = |p: &str| bestanden.iter().any(|b| (b.map == p || b.map.starts_with(&format!("{p}/"))) && is_nieuw(b));
    let met_nieuw = |soort: String, ja: bool| if ja { f("SoortNieuw", &[&soort]) } else { soort };
    let h = lijst_hwnd();
    snd(h, WM_SETREDRAW, 0, 0);
    snd(h, LVM_DELETEALLITEMS, 0, 0);
    let mut gekozen = 0usize;
    for (i, r) in regels.iter().enumerate() {
        let (naam, soort, grootte, datum, uitleg) = match r {
            Regel::Map { naam, aantal, pad } => {
                (naam.clone(), met_nieuw(t("SoortMap"), map_nieuw(pad)), f("AantalItems", &[&aantal.to_string()]), String::new(), String::new())
            }
            Regel::Bestand(j) => {
                let b = &bestanden[*j];
                (b.naam.clone(), met_nieuw(t("SoortBestand"), is_nieuw(b)), opmaak::grootte(b.grootte), opmaak::datum(&b.gewijzigd), b.uitleg.clone())
            }
        };
        let hit = match (&kies, r) {
            (Some(Kies::Map(n)), Regel::Map { naam, .. }) => n == naam,
            (Some(Kies::Bestand(id)), Regel::Bestand(j)) => bestanden[*j].fileid == *id,
            _ => false,
        };
        if hit {
            gekozen = i;
        }
        let w = win::ws(&naam);
        let item = LVITEMW { mask: LVIF_TEXT | LVIF_PARAM, iItem: i as i32, pszText: windows::core::PWSTR(w.as_ptr() as *mut u16), lParam: LPARAM(i as isize), ..Default::default() };
        snd(h, LVM_INSERTITEMW, 0, &item as *const _ as isize);
        for (kol, tekst) in [(1, soort), (2, grootte), (3, datum), (4, uitleg)] {
            let v = win::ws(&tekst);
            let sub = LVITEMW { iSubItem: kol, pszText: windows::core::PWSTR(v.as_ptr() as *mut u16), ..Default::default() };
            snd(h, LVM_SETITEMTEXTW, i, &sub as *const _ as isize);
        }
    }
    if !regels.is_empty() {
        let vlag = LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0);
        let staat = LVITEMW { stateMask: vlag, state: vlag, ..Default::default() };
        snd(h, LVM_SETITEMSTATE, gekozen, &staat as *const _ as isize);
        snd(h, LVM_ENSUREVISIBLE, gekozen, 0);
    }
    snd(h, WM_SETREDRAW, 1, 0);
    unsafe {
        let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(h), None, true);
    }
    // Het label boven de lijst zegt in welke map je bent.
    let label = if pad.is_empty() { t("LijstNaam") } else { f("LijstInMap", &[&pad.replace('/', " > ")]) };
    win::zet_tekst(hoofdscherm::onderdelen().lijst_label, &label);
    HUIDIG.with(|x| *x.borrow_mut() = (pad, regels));
}

enum Kies {
    Map(String),
    Bestand(u64),
}

/// De lijst opnieuw tonen (nieuwe lijst van pCloud, of andere taal). Je blijft in dezelfde map en op dezelfde regel.
pub fn vul(nieuw: &[Bestand], oud: &[Bestand]) {
    let kies = match keuze() {
        Some(Regel::Map { naam, .. }) => Some(Kies::Map(naam)),
        Some(Regel::Bestand(i)) => oud.get(i).map(|b| Kies::Bestand(b.fileid)),
        None => None,
    };
    toon(nieuw, &huidige_map(), kies);
}

/// Enter op een map: die map openen en zeggen waar je bent.
pub fn open(pad: &str) {
    let lijst = met(|s| s.lijst.clone()).unwrap_or_default();
    toon(&lijst, pad, None);
    let naam = pad.rsplit('/').next().unwrap_or(pad);
    let aantal = HUIDIG.with(|h| h.borrow().1.len());
    zeg(&f("MapGeopend", &[naam, &aantal.to_string()]), Belang::Belangrijk);
}

/// Backspace: naar de map erboven. De map waar je vandaan kwam is dan gekozen.
pub fn omhoog() {
    let pad = huidige_map();
    if pad.is_empty() {
        zeg(&t("AlInHoofdmap"), Belang::Belangrijk);
        return;
    }
    let (boven, naam) = match pad.rsplit_once('/') {
        Some((b, n)) => (b.to_string(), n.to_string()),
        None => (String::new(), pad.clone()),
    };
    let lijst = met(|s| s.lijst.clone()).unwrap_or_default();
    toon(&lijst, &boven, Some(Kies::Map(naam)));
    let melding = if boven.is_empty() { t("TerugHoofdmap") } else { f("TerugInMap", &[boven.rsplit('/').next().unwrap_or(&boven)]) };
    zeg(&melding, Belang::Belangrijk);
}

pub fn aantal() -> isize {
    snd(lijst_hwnd(), LVM_GETITEMCOUNT, 0, 0)
}
