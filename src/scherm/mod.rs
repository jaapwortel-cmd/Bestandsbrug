//! Het scherm van Bestandsbrug (alleen Windows): echte Windows-onderdelen met eigen kleuren, zodat de
//! schermlezer alles kent (namen, rollen, toestanden). Hier: opstarten, de berichtenlus met de sneltoetsen,
//! en de gedeelde toestand van het scherm.

mod verkenner;
mod dialogen;
mod hoofdscherm;
mod menu;
mod overdracht_ui;
mod scherm_download;
mod scherm_upload;
mod spraak;
mod tekenen;
mod tekstvenster;
mod thema;
mod update_ui;
mod venster;
mod vragen;
mod win;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::time::Instant;

use bestandsbrug::pcloud::lijst::Bestand;
use bestandsbrug::taal::{self, Taal};
use bestandsbrug::voorkeuren::{self, Voorkeuren};
use bestandsbrug::{instellingen, log};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Controls::{InitCommonControlsEx, ICC_LISTVIEW_CLASSES, ICC_STANDARD_CLASSES, ICC_TAB_CLASSES, INITCOMMONCONTROLSEX};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub use spraak::Belang;
pub use thema::Weergave;

/// Eigen berichten van de achtergrond naar het scherm.
pub const WM_LIJST_KLAAR: u32 = WM_APP + 1;
pub const WM_OVERDRACHT_KLAAR: u32 = WM_APP + 2;
pub const WM_UPDATE_INFO: u32 = WM_APP + 3;
pub const WM_NA_START: u32 = WM_APP + 4;
pub const WM_MENU_FOCUS: u32 = WM_APP + 5;

/// Timers.
pub const TIMER_MELD: usize = 1;
pub const TIMER_BALK: usize = 2;
pub const TIMER_UPDATE: usize = 3;
pub const TIMER_LATER_ZEGGEN: usize = 4;
pub const TIMER_SLUIT: usize = 5;

/// Opties bij het starten (voor de proeven in de bouwronde).
#[derive(Debug, Default, Clone)]
pub struct Opties {
    pub taal: Option<Taal>,
    pub weergave: Option<Weergave>,
    pub meet: Option<PathBuf>,
    pub sluit_na: Option<u32>,
    pub geen_update: bool,
}

/// De toestand van het scherm. Alleen kort lenen (met `met`), nooit tijdens een vraag of venster.
pub struct Staat {
    pub hwnd: HWND,
    pub weergave: Weergave,
    pub pagina: usize,
    pub lijst: Vec<Bestand>,
    pub lijst_van_internet: bool,
    pub upload_pad: Option<PathBuf>,
    pub lopend: Option<overdracht_ui::Lopend>,
    pub voorkeuren: Voorkeuren,
    pub update_uitgesteld: bool,
    pub afsluiten_na_overdracht: bool,
    pub onderbroken_vragen: bool,
    pub later_zeggen: Vec<(String, Belang)>,
    pub opties: Opties,
    pub start: Instant,
    pub metingen: Vec<String>,
    pub laatste_focus: HWND,
}

thread_local! {
    static STAAT: RefCell<Option<Staat>> = const { RefCell::new(None) };
    /// Hoeveel vragen of vensters er nu open zijn (dan geen voortgangsmelding tussendoor).
    static VRAGEN: Cell<u32> = const { Cell::new(0) };
    /// Stand van de voortgangsbalk, 0 tot 1000 (los van STAAT, zodat tekenen nooit hoeft te wachten).
    static BALK: Cell<u32> = const { Cell::new(0) };
}

/// Doe iets met de toestand. Geeft None als de toestand er (nog) niet is of al geleend wordt.
pub fn met<R>(f: impl FnOnce(&mut Staat) -> R) -> Option<R> {
    STAAT.with(|s| s.try_borrow_mut().ok().and_then(|mut b| b.as_mut().map(f)))
}

pub fn hwnd() -> HWND {
    met(|s| s.hwnd).unwrap_or_default()
}

pub fn begin_vraag() {
    VRAGEN.with(|v| v.set(v.get() + 1));
}

pub fn einde_vraag() {
    VRAGEN.with(|v| v.set(v.get().saturating_sub(1)));
}

pub fn vraag_open() -> bool {
    VRAGEN.with(|v| v.get() > 0)
}

pub fn balk() -> u32 {
    BALK.with(|b| b.get())
}

pub fn zet_balk(promille: u32) {
    BALK.with(|b| b.set(promille.min(1000)));
}

/// Zeg iets via de schermlezer en zet het in het vak Melding.
pub fn zeg(tekst: &str, belang: Belang) {
    let o = hoofdscherm::onderdelen();
    win::zet_tekst(o.melding, tekst);
    spraak::zeg(o.melding, tekst, belang);
}

/// Zeg iets over 0,4 seconde: na het sluiten van een menu of venster, zodat de schermlezer eerst
/// de focus noemt en daarna deze melding (en elkaar niet onderbreken).
pub fn zeg_later(tekst: &str, belang: Belang) {
    let h = hwnd();
    met(|s| s.later_zeggen.push((tekst.to_string(), belang)));
    unsafe {
        SetTimer(Some(h), TIMER_LATER_ZEGGEN, 400, None);
    }
}

pub fn log_meting(regel: String) {
    log::info(&regel);
    met(|s| s.metingen.push(regel));
}

fn lees_opties() -> Opties {
    let mut o = Opties::default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let volgende = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--taal" => {
                o.taal = Taal::van_code(&volgende);
                i += 1;
            }
            "--weergave" => {
                o.weergave = Weergave::van_code(&volgende);
                i += 1;
            }
            "--meet" => {
                o.meet = Some(PathBuf::from(volgende));
                i += 1;
            }
            "--sluit-na" => {
                o.sluit_na = volgende.parse().ok();
                i += 1;
            }
            "--geen-update" => o.geen_update = true,
            _ => {}
        }
        i += 1;
    }
    o
}

/// Start de app en geeft de afsluitcode.
pub fn start() -> i32 {
    let begin = Instant::now();
    let opties = lees_opties();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_LISTVIEW_CLASSES | ICC_TAB_CLASSES | ICC_STANDARD_CLASSES,
        };
        let _ = InitCommonControlsEx(&icc);
    }
    let voorkeuren = voorkeuren::laad();
    let gekozen_taal = opties
        .taal
        .or_else(|| voorkeuren.taal.as_deref().and_then(Taal::van_code))
        .unwrap_or_else(taal::taal_van_systeem);
    taal::zet(gekozen_taal);
    let weergave = opties
        .weergave
        .or_else(|| voorkeuren.weergave.as_deref().and_then(Weergave::van_code))
        .unwrap_or_else(thema::weergave_van_windows);
    log::info(&format!("Start, version {}, language {}, view {}", instellingen::VERSIE, gekozen_taal.code(), weergave.code()));

    let Some(h) = venster::maak(weergave) else {
        log::fout("Cannot create the main window");
        return 1;
    };
    STAAT.with(|s| {
        *s.borrow_mut() = Some(Staat {
            hwnd: h,
            weergave,
            pagina: 0,
            lijst: Vec::new(),
            lijst_van_internet: false,
            upload_pad: None,
            lopend: None,
            voorkeuren,
            update_uitgesteld: false,
            afsluiten_na_overdracht: false,
            onderbroken_vragen: !bestandsbrug::overdracht::laad().is_empty(),
            later_zeggen: Vec::new(),
            opties: opties.clone(),
            start: begin,
            metingen: Vec::new(),
            laatste_focus: HWND::default(),
        })
    });
    hoofdscherm::maak_onderdelen(h);
    menu::bouw(h);
    hoofdscherm::zet_teksten();
    // De lijst van de vorige keer meteen laten zien; de nieuwe wordt op de achtergrond opgehaald.
    if let Some(oud) = bestandsbrug::pcloud::lijst::laad_cache() {
        scherm_download::toon_lijst(oud, false);
    }
    hoofdscherm::indeling();
    unsafe {
        let _ = ShowWindow(h, SW_SHOW);
        let _ = windows::Win32::Graphics::Gdi::UpdateWindow(h);
    }
    hoofdscherm::focus_start();
    log_meting(format!("Window shown after {} ms", begin.elapsed().as_millis()));
    win::post(h, WM_NA_START, 0, 0);
    if let Some(n) = opties.sluit_na {
        unsafe {
            SetTimer(Some(h), TIMER_SLUIT, n * 1000, None);
        }
    }
    berichtenlus()
}

fn berichtenlus() -> i32 {
    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if voor_toets(&msg) {
                continue;
            }
            let root = GetAncestor(msg.hwnd, GA_ROOT);
            if !root.is_invalid() && msg.message != WM_SYSCHAR && IsDialogMessageW(root, &msg).as_bool() {
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    msg.wParam.0 as i32
}

fn toets_ingedrukt(vk: VIRTUAL_KEY) -> bool {
    unsafe { GetKeyState(vk.0 as i32) < 0 }
}

/// Is dit een tekstvak met meerdere regels? Die houden Tab vast; wij verplaatsen dan zelf de focus.
fn is_meerregelig(h: HWND) -> bool {
    let stijl = unsafe { GetWindowLongW(h, GWL_STYLE) } as u32;
    let klasse = {
        let mut buf = [0u16; 32];
        let n = unsafe { GetClassNameW(h, &mut buf) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    };
    klasse.eq_ignore_ascii_case("Edit") && (stijl & ES_MULTILINE as u32) != 0
}

fn tab_verder(root: HWND, focus: HWND, terug: bool) {
    unsafe {
        if let Ok(volgende) = GetNextDlgTabItem(root, Some(focus), terug) {
            let _ = SetFocus(Some(volgende));
        }
    }
}

/// De sneltoetsen. Geeft true als de toets hier is afgehandeld.
fn voor_toets(msg: &MSG) -> bool {
    let hoofd = hwnd();
    if hoofd.is_invalid() {
        return false;
    }
    let root = unsafe { GetAncestor(msg.hwnd, GA_ROOT) };
    if msg.message == WM_SYSCHAR && root == hoofd {
        // Alt+letter voor een knop (de letter met & in de naam). Anders gaat Alt+letter naar het menu.
        let letter = char::from_u32(msg.wParam.0 as u32).unwrap_or(' ').to_lowercase().next().unwrap_or(' ');
        return hoofdscherm::klik_sneltoets(letter);
    }
    if msg.message != WM_KEYDOWN {
        return false;
    }
    let vk = VIRTUAL_KEY(msg.wParam.0 as u16);
    let ctrl = toets_ingedrukt(VK_CONTROL);
    let shift = toets_ingedrukt(VK_SHIFT);
    let focus = unsafe { GetFocus() };
    if tekstvenster::is_tekstvenster(root) {
        if vk == VK_ESCAPE {
            tekstvenster::sluit(root);
            return true;
        }
        if vk == VK_TAB && is_meerregelig(focus) {
            tab_verder(root, focus, shift);
            return true;
        }
        return false;
    }
    if root != hoofd {
        return false;
    }
    match vk {
        VK_ESCAPE if !ctrl && !shift => {
            if met(|s| s.lopend.is_some()).unwrap_or(false) {
                overdracht_ui::escape();
                return true;
            }
            false
        }
        VK_F1 => {
            tekstvenster::uitleg();
            true
        }
        VK_F5 => {
            scherm_download::vernieuw(true);
            true
        }
        VK_TAB if ctrl => {
            let p = met(|s| s.pagina).unwrap_or(0);
            hoofdscherm::kies_pagina(1 - p, true);
            true
        }
        VK_TAB if is_meerregelig(focus) => {
            tab_verder(root, focus, shift);
            true
        }
        VK_1 if ctrl => {
            hoofdscherm::kies_pagina(0, true);
            true
        }
        VK_2 if ctrl => {
            hoofdscherm::kies_pagina(1, true);
            true
        }
        VK_RETURN if !ctrl => hoofdscherm::enter(focus),
        VK_BACK if focus == hoofdscherm::onderdelen().lijst => {
            verkenner::omhoog();
            true
        }
        _ => false,
    }
}
