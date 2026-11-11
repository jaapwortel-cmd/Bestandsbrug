//! De boom op het tabblad Downloaden: eerst de mappen, en daaronder de bestanden. Het is een gewone
//! boom van Windows (TreeView), dus de schermlezer zegt "dichtgeklapt" of "opengeklapt" en het niveau.
//! Een map open of dicht: pijl naar rechts of links, of Enter. Een bestand: Enter = downloaden.
//! Bij vernieuwen blijven open mappen open en blijft de keuze staan.

use std::cell::RefCell;
use std::collections::HashSet;

use bestandsbrug::opmaak;
use bestandsbrug::pcloud::lijst::{self, Bestand, Map};
use windows::Win32::Foundation::LPARAM;
use windows::Win32::UI::Controls::*;

use super::win::{self, snd};
use super::*;

/// Wat er in de boom gekozen is.
pub enum Keuze {
    Bestand(usize),
    Map(String),
}

thread_local! {
    /// De mappen in de boom: pad en het onderdeel in de boom (voor open/dicht en de keuze).
    static MAPPEN: RefCell<Vec<(String, isize)>> = const { RefCell::new(Vec::new()) };
}

fn boom() -> windows::Win32::Foundation::HWND {
    hoofdscherm::onderdelen().lijst
}

/// De tekst van een bestand: naam, grootte, datum en (als die er is) de uitleg.
fn tekst_bestand(b: &Bestand) -> String {
    let mut s = format!("{}, {}, {}", b.naam, opmaak::grootte(b.grootte), opmaak::datum(&b.gewijzigd));
    if !b.uitleg.is_empty() {
        s.push_str(", ");
        s.push_str(&b.uitleg);
    }
    s
}

fn voeg_in(ouder: isize, tekst: &str, param: isize) -> isize {
    let w = win::ws(tekst);
    let item = TVITEMW { mask: TVIF_TEXT | TVIF_PARAM, pszText: windows::core::PWSTR(w.as_ptr() as *mut u16), lParam: LPARAM(param), ..Default::default() };
    let invoeg = TVINSERTSTRUCTW { hParent: HTREEITEM(ouder), hInsertAfter: TVI_LAST, Anonymous: TVINSERTSTRUCTW_0 { item } };
    snd(boom(), TVM_INSERTITEMW, 0, &invoeg as *const _ as isize)
}

fn param_van(item: isize) -> Option<isize> {
    if item == 0 {
        return None;
    }
    let mut it = TVITEMW { mask: TVIF_PARAM, hItem: HTREEITEM(item), ..Default::default() };
    (snd(boom(), TVM_GETITEMW, 0, &mut it as *mut _ as isize) != 0).then_some(it.lParam.0)
}

/// Wat er nu gekozen is.
pub fn keuze() -> Option<Keuze> {
    let p = param_van(snd(boom(), TVM_GETNEXTITEM, TVGN_CARET as usize, 0))?;
    if p >= 0 {
        Some(Keuze::Bestand(p as usize))
    } else {
        MAPPEN.with(|m| m.borrow().get((-p - 1) as usize).map(|(pad, _)| Keuze::Map(pad.clone())))
    }
}

/// Opent of sluit de gekozen map.
pub fn wissel_open() {
    let item = snd(boom(), TVM_GETNEXTITEM, TVGN_CARET as usize, 0);
    if item != 0 {
        snd(boom(), TVM_EXPAND, TVE_TOGGLE.0 as usize, item);
    }
}

pub fn aantal() -> isize {
    snd(boom(), TVM_GETCOUNT, 0, 0)
}

/// Zet de lijst als boom op het scherm. `oud` is de lijst die er nu staat (om de keuze terug te vinden).
pub fn vul(nieuw: &[Bestand], oud: &[Bestand]) {
    let h = boom();
    // Wat stond er open, en wat was gekozen?
    let open: HashSet<String> = MAPPEN.with(|m| {
        m.borrow()
            .iter()
            .filter(|(_, item)| (snd(h, TVM_GETITEMSTATE, *item as usize, TVIS_EXPANDED.0 as isize) as u32 & TVIS_EXPANDED.0) != 0)
            .map(|(pad, _)| pad.clone())
            .collect()
    });
    let gekozen_bestand = match keuze() {
        Some(Keuze::Bestand(i)) => oud.get(i).map(|b| b.fileid),
        _ => None,
    };
    let gekozen_map = match keuze() {
        Some(Keuze::Map(p)) => Some(p),
        _ => None,
    };
    snd(h, WM_SETREDRAW, 0, 0);
    snd(h, TVM_DELETEITEM, 0, TVI_ROOT.0);
    MAPPEN.with(|m| m.borrow_mut().clear());
    let mut terug = 0isize;
    voeg_map_toe(0, &lijst::maak_boom(nieuw), nieuw, &open, gekozen_bestand, gekozen_map.as_deref(), &mut terug);
    if terug == 0 {
        terug = snd(h, TVM_GETNEXTITEM, TVGN_ROOT as usize, 0);
    }
    if terug != 0 {
        snd(h, TVM_SELECTITEM, TVGN_CARET as usize, terug);
    }
    snd(h, WM_SETREDRAW, 1, 0);
    unsafe {
        let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(h), None, true);
    }
}

fn voeg_map_toe(ouder: isize, map: &Map, lijst: &[Bestand], open: &HashSet<String>, bestand: Option<u64>, mapkeuze: Option<&str>, terug: &mut isize) {
    for sub in &map.mappen {
        let k = MAPPEN.with(|m| m.borrow().len()) as isize;
        let item = voeg_in(ouder, &sub.naam, -(k + 1));
        MAPPEN.with(|m| m.borrow_mut().push((sub.pad.clone(), item)));
        if mapkeuze == Some(sub.pad.as_str()) {
            *terug = item;
        }
        voeg_map_toe(item, sub, lijst, open, bestand, mapkeuze, terug);
        if open.contains(&sub.pad) {
            snd(boom(), TVM_EXPAND, TVE_EXPAND.0 as usize, item);
        }
    }
    for &i in &map.bestanden {
        let item = voeg_in(ouder, &tekst_bestand(&lijst[i]), i as isize);
        if bestand == Some(lijst[i].fileid) {
            *terug = item;
        }
    }
}
