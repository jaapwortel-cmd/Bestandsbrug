//! Het thema van het scherm: de kleuren (uit src/kleuren.rs, dezelfde als in Bestandsbrug.html), plus de letters (grootte volgt Windows en de DPI).
//! Staat hoog contrast van Windows aan, dan gebruiken we de kleuren van Windows zelf.

use std::cell::RefCell;

use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Accessibility::{HIGHCONTRASTW, HCF_HIGHCONTRASTON};
use windows::Win32::UI::HiDpi::SystemParametersInfoForDpi;
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weergave {
    Licht,
    Donker,
}

impl Weergave {
    pub fn code(self) -> &'static str {
        match self {
            Weergave::Licht => "licht",
            Weergave::Donker => "donker",
        }
    }
    pub fn van_code(s: &str) -> Option<Self> {
        match s {
            "licht" => Some(Weergave::Licht),
            "donker" => Some(Weergave::Donker),
            _ => None,
        }
    }
}

/// De kleuren van één weergave.
#[derive(Debug, Clone, Copy)]
pub struct Kleuren {
    pub achtergrond: COLORREF,
    pub kaart: COLORREF,
    pub tekst: COLORREF,
    pub zacht: COLORREF,
    pub rand: COLORREF,
    pub accent: COLORREF,
    pub op_accent: COLORREF,
    pub accent_ingedrukt: COLORREF,
    pub focus: COLORREF,
    pub kop: COLORREF,
    pub op_kop: COLORREF,
    pub keuze: COLORREF,
    pub op_keuze: COLORREF,
}

/// Een kleur uit src/kleuren.rs ("#RRGGBB") naar een Windows-kleur.
fn kleur(hex: &str) -> COLORREF {
    let (r, g, b) = bestandsbrug::kleuren::rgb(hex);
    COLORREF(r as u32 | (g as u32) << 8 | (b as u32) << 16)
}

/// De kleuren van src/kleuren.rs (dezelfde als in Bestandsbrug.html).
fn van_palet(p: &bestandsbrug::kleuren::Palet) -> Kleuren {
    Kleuren {
        achtergrond: kleur(p.achtergrond),
        kaart: kleur(p.kaart),
        tekst: kleur(p.tekst),
        zacht: kleur(p.zacht),
        rand: kleur(p.rand),
        accent: kleur(p.accent),
        op_accent: kleur(p.op_accent),
        accent_ingedrukt: kleur(p.accent_ingedrukt),
        focus: kleur(p.focus),
        kop: kleur(p.kop),
        op_kop: kleur(p.op_kop),
        keuze: kleur(p.accent),
        op_keuze: kleur(p.op_accent),
    }
}

/// Kleuren van Windows zelf, voor hoog contrast.
fn systeemkleuren() -> Kleuren {
    let k = |i: SYS_COLOR_INDEX| COLORREF(unsafe { GetSysColor(i) });
    Kleuren {
        achtergrond: k(COLOR_WINDOW),
        kaart: k(COLOR_WINDOW),
        tekst: k(COLOR_WINDOWTEXT),
        zacht: k(COLOR_WINDOWTEXT),
        rand: k(COLOR_WINDOWTEXT),
        accent: k(COLOR_HIGHLIGHT),
        op_accent: k(COLOR_HIGHLIGHTTEXT),
        accent_ingedrukt: k(COLOR_HIGHLIGHT),
        focus: k(COLOR_WINDOWTEXT),
        kop: k(COLOR_WINDOW),
        op_kop: k(COLOR_WINDOWTEXT),
        keuze: k(COLOR_HIGHLIGHT),
        op_keuze: k(COLOR_HIGHLIGHTTEXT),
    }
}

pub fn hoog_contrast() -> bool {
    let mut hc = HIGHCONTRASTW { cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32, ..Default::default() };
    unsafe {
        SystemParametersInfoW(SPI_GETHIGHCONTRAST, hc.cbSize, Some(&mut hc as *mut _ as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)).is_ok()
            && (hc.dwFlags.0 & HCF_HIGHCONTRASTON.0) != 0
    }
}

/// De weergave van Windows (voor de allereerste start): donker als apps donker moeten zijn.
pub fn weergave_van_windows() -> Weergave {
    use windows::Win32::System::Registry::*;
    let mut waarde: u32 = 1;
    let mut grootte = 4u32;
    let pad = super::win::ws("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let naam = super::win::ws("AppsUseLightTheme");
    let ok = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            super::win::pc(&pad),
            super::win::pc(&naam),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut waarde as *mut u32 as *mut _),
            Some(&mut grootte),
        )
    };
    if ok.is_ok() && waarde == 0 {
        Weergave::Donker
    } else {
        Weergave::Licht
    }
}

/// Het thema dat nu gebruikt wordt: kleuren, kwasten en letters.
#[derive(Debug, Clone, Copy)]
pub struct Thema {
    pub weergave: Weergave,
    pub hoog_contrast: bool,
    pub k: Kleuren,
    pub kwast_achtergrond: HBRUSH,
    pub kwast_kaart: HBRUSH,
    pub letter: HFONT,
    pub letter_vet: HFONT,
    pub letter_kop: HFONT,
    /// Hoogte van een regel tekst in beeldpunten (alles op het scherm schaalt hiermee mee).
    pub regel: i32,
}

thread_local! {
    static THEMA: RefCell<Option<Thema>> = const { RefCell::new(None) };
}

pub fn huidig() -> Thema {
    THEMA.with(|t| t.borrow().expect("thema"))
}

/// Maakt het thema (opnieuw), bijvoorbeeld bij een andere weergave, DPI of lettergrootte.
pub fn maak(weergave: Weergave, dpi: u32) -> Thema {
    let hc = hoog_contrast();
    let k = if hc {
        systeemkleuren()
    } else {
        match weergave {
            Weergave::Licht => van_palet(&bestandsbrug::kleuren::LICHT),
            Weergave::Donker => van_palet(&bestandsbrug::kleuren::DONKER),
        }
    };
    // De lettergrootte van Windows (volgt ook "Tekst groter maken"), iets groter voor de leesbaarheid.
    let mut ncm = NONCLIENTMETRICSW { cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32, ..Default::default() };
    unsafe {
        let _ = SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS.0, ncm.cbSize, Some(&mut ncm as *mut _ as *mut _), 0, dpi);
    }
    let mut lf = ncm.lfMessageFont;
    let basis = lf.lfHeight.abs().max(12);
    lf.lfHeight = -(basis * 115 / 100);
    lf.lfQuality = CLEARTYPE_QUALITY;
    let letter = unsafe { CreateFontIndirectW(&lf) };
    let mut lv = lf;
    lv.lfWeight = 600;
    let letter_vet = unsafe { CreateFontIndirectW(&lv) };
    let mut lk = lf;
    lk.lfWeight = 700;
    lk.lfHeight = -(basis * 200 / 100);
    let letter_kop = unsafe { CreateFontIndirectW(&lk) };
    let nieuw = Thema {
        weergave,
        hoog_contrast: hc,
        k,
        kwast_achtergrond: unsafe { CreateSolidBrush(k.achtergrond) },
        kwast_kaart: unsafe { CreateSolidBrush(k.kaart) },
        letter,
        letter_vet,
        letter_kop,
        regel: (basis * 115 / 100) * 3 / 2,
    };
    THEMA.with(|t| {
        if let Some(oud) = t.borrow_mut().replace(nieuw) {
            unsafe {
                let _ = DeleteObject(oud.kwast_achtergrond.into());
                let _ = DeleteObject(oud.kwast_kaart.into());
                let _ = DeleteObject(oud.letter.into());
                let _ = DeleteObject(oud.letter_vet.into());
                let _ = DeleteObject(oud.letter_kop.into());
            }
        }
    });
    nieuw
}

/// Donkere titelbalk van Windows aan of uit.
pub fn titelbalk(hwnd: HWND, donker: bool) {
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
    let waarde: i32 = donker as i32;
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &waarde as *const i32 as *const _, 4);
    }
}

/// Schuifbalken en lijstkop in de stijl van de weergave (alleen het uiterlijk, niets voor de schermlezer).
pub fn stijl_onderdeel(h: HWND, donker: bool) {
    let naam = super::win::ws(if donker { "DarkMode_Explorer" } else { "Explorer" });
    unsafe {
        let _ = windows::Win32::UI::Controls::SetWindowTheme(h, super::win::pc(&naam), windows::core::PCWSTR::null());
    }
}
