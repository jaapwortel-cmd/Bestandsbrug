//! Een eigen venster met gewone, leesbare tekst: de Uitleg en de Privacyverklaring. Je loopt erdoor met de
//! pijltjes; Escape of de knop Sluiten sluit het venster. Ook "Over" staat hier.

use std::cell::Cell;

use bestandsbrug::instellingen::VERSIE;
use bestandsbrug::taal::{self, f, t, zonder_amp};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Controls::{DRAWITEMSTRUCT, EM_SETMARGINS, EM_SETSEL};
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, SetFocus};
use windows::Win32::UI::WindowsAndMessaging::*;

use super::win::{self, snd};
use super::*;

const ID_TEKST: u16 = 1;
const ID_SLUITEN: u16 = 2;

thread_local! {
    static OPEN: Cell<(HWND, HWND, HWND)> = Cell::new((HWND::default(), HWND::default(), HWND::default()));
}

pub fn is_tekstvenster(h: HWND) -> bool {
    !h.is_invalid() && OPEN.with(|o| o.get().0) == h
}

pub fn uitleg() {
    toon(&t("UitlegTitel"), &taal::uitleg(taal::huidig()));
}

pub fn privacy() {
    toon(&t("PrivacyTitel"), taal::privacy(taal::huidig()));
}

/// Over: naam en versie, met een knop naar de privacyverklaring.
pub fn over() {
    const PRIVACY: i32 = 400;
    const SLUITEN: i32 = 401;
    let knoppen = [vragen::knop(PRIVACY, zonder_amp(&t("OverPrivacy"))), vragen::knop(SLUITEN, zonder_amp(&t("KnopSluiten")))];
    let tekst = f("OverTekst", &[VERSIE]);
    let (kop, rest) = tekst.split_once('\n').unwrap_or((&tekst, ""));
    if vragen::vraag(hwnd(), &t("OverTitel"), kop, rest, &knoppen, SLUITEN, Some(SLUITEN)) == PRIVACY {
        privacy();
    }
}

/// Maakt de tekst klaar voor het tekstvak: een lege regel voor elk genummerd kopje, en Windows-regeleinden.
pub fn opmaak(tekst: &str) -> String {
    let mut uit = String::new();
    for (i, regel) in tekst.lines().enumerate() {
        let kopje = regel.split_once(". ").is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        if i > 0 && kopje && !uit.ends_with("\r\n\r\n") {
            uit.push_str("\r\n");
        }
        uit.push_str(regel.trim_end());
        uit.push_str("\r\n");
    }
    uit
}

fn toon(titel: &str, tekst: &str) {
    let eigenaar = hwnd();
    if OPEN.with(|o| !o.get().0.is_invalid()) {
        return;
    }
    unsafe {
        let klasse = win::ws(bestandsbrug::instellingen::KLASSE_TEKSTVENSTER);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(proc_),
            hInstance: win::hinst(),
            hIcon: LoadIconW(Some(win::hinst()), windows::core::PCWSTR(1 as *const u16)).unwrap_or_default(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: win::pc(&klasse),
            ..Default::default()
        };
        RegisterClassExW(&wc);
        let mut r = RECT::default();
        let _ = GetWindowRect(eigenaar, &mut r);
        let (b, h) = ((r.right - r.left) * 9 / 10, (r.bottom - r.top) * 9 / 10);
        let titel_w = win::ws(titel);
        let Ok(venster) = CreateWindowExW(
            WS_EX_CONTROLPARENT | WS_EX_DLGMODALFRAME,
            win::pc(&klasse),
            win::pc(&titel_w),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            r.left + (r.right - r.left - b) / 2,
            r.top + (r.bottom - r.top - h) / 2,
            b,
            h,
            Some(eigenaar),
            None,
            Some(win::hinst()),
            None,
        ) else {
            return;
        };
        let th = thema::huidig();
        thema::titelbalk(venster, th.weergave == Weergave::Donker && !th.hoog_contrast);
        let vak = win::maak(
            "EDIT",
            &opmaak(tekst),
            WS_TABSTOP.0 | WS_VSCROLL.0 | ES_MULTILINE as u32 | ES_READONLY as u32 | ES_AUTOVSCROLL as u32,
            0,
            venster,
            ID_TEKST,
        );
        let knop = win::maak("BUTTON", &t("KnopSluiten"), WS_TABSTOP.0 | BS_OWNERDRAW as u32, 0, venster, ID_SLUITEN);
        snd(vak, WM_SETFONT, th.letter.0 as usize, 1);
        snd(knop, WM_SETFONT, th.letter_vet.0 as usize, 1);
        snd(vak, EM_SETMARGINS, (EC_LEFTMARGIN | EC_RIGHTMARGIN) as usize, (th.regel / 2 | (th.regel / 2) << 16) as isize);
        thema::stijl_onderdeel(vak, th.weergave == Weergave::Donker && !th.hoog_contrast);
        OPEN.with(|o| o.set((venster, vak, knop)));
        indeling(venster);
        begin_vraag();
        let _ = EnableWindow(eigenaar, false);
        let _ = ShowWindow(venster, SW_SHOW);
        let _ = SetFocus(Some(vak));
        snd(vak, EM_SETSEL, 0, 0);
    }
}

fn indeling(venster: HWND) {
    let (_, vak, knop) = OPEN.with(|o| o.get());
    let th = thema::huidig();
    let mut r = RECT::default();
    unsafe {
        let _ = GetClientRect(venster, &mut r);
    }
    let m = th.regel * 2 / 3;
    let knop_h = th.regel + th.regel * 2 / 3;
    let knop_b = th.regel * 7;
    win::verplaats(vak, m, m, r.right - 2 * m, r.bottom - 3 * m - knop_h);
    win::verplaats(knop, r.right - m - knop_b, r.bottom - m - knop_h, knop_b, knop_h);
}

pub fn sluit(venster: HWND) {
    unsafe {
        let _ = DestroyWindow(venster);
    }
}

extern "system" fn proc_(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_SIZE => {
                indeling(h);
                LRESULT(0)
            }
            WM_ERASEBKGND => {
                let th = thema::huidig();
                let mut r = RECT::default();
                let _ = GetClientRect(h, &mut r);
                FillRect(HDC(wp.0 as *mut _), &r, th.kwast_achtergrond);
                LRESULT(1)
            }
            WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT => {
                let th = thema::huidig();
                let dc = HDC(wp.0 as *mut _);
                SetTextColor(dc, th.k.tekst);
                SetBkColor(dc, th.k.kaart);
                LRESULT(th.kwast_kaart.0 as isize)
            }
            WM_DRAWITEM => {
                tekenen::item(&*(lp.0 as *const DRAWITEMSTRUCT));
                LRESULT(1)
            }
            WM_COMMAND => {
                let id = win::lo(wp.0) as u16;
                if id == ID_SLUITEN || id == IDCANCEL.0 as u16 {
                    sluit(h);
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                sluit(h);
                LRESULT(0)
            }
            WM_DESTROY => {
                OPEN.with(|o| o.set((HWND::default(), HWND::default(), HWND::default())));
                einde_vraag();
                let eigenaar = hwnd();
                let _ = EnableWindow(eigenaar, true);
                let _ = SetForegroundWindow(eigenaar);
                LRESULT(0)
            }
            _ => DefWindowProcW(h, msg, wp, lp),
        }
    }
}
