//! Het hoofdvenster: maken en de afhandeling van alle berichten (de "window procedure").

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::*;

pub fn maak(weergave: Weergave) -> Option<HWND> {
    unsafe {
        let icoon = LoadIconW(Some(win::hinst()), PCWSTR(1 as *const u16)).unwrap_or_default();
        let klasse = win::ws(bestandsbrug::instellingen::KLASSE_HOOFDVENSTER);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(proc_),
            hInstance: win::hinst(),
            hIcon: icoon,
            hIconSm: icoon,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: win::pc(&klasse),
            ..Default::default()
        };
        RegisterClassExW(&wc);
        let titel = win::ws(&bestandsbrug::taal::app_naam());
        let h = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            win::pc(&klasse),
            win::pc(&titel),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            Some(win::hinst()),
            None,
        )
        .ok()?;
        let dpi = GetDpiForWindow(h).max(96);
        thema::maak(weergave, dpi);
        thema::titelbalk(h, weergave == Weergave::Donker && !thema::hoog_contrast());
        // Begingrootte: ruim, maar binnen het scherm, en in het midden.
        let schaal = |x: i32| x * dpi as i32 / 96;
        let mut werk = RECT::default();
        let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut werk as *mut _ as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
        let b = schaal(960).min(werk.right - werk.left);
        let hoogte = schaal(680).min(werk.bottom - werk.top);
        let x = werk.left + (werk.right - werk.left - b) / 2;
        let y = werk.top + (werk.bottom - werk.top - hoogte) / 2;
        let _ = SetWindowPos(h, None, x, y, b, hoogte, SWP_NOZORDER);
        Some(h)
    }
}

use windows::core::PCWSTR;

extern "system" fn proc_(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match bericht(h, msg, wp, lp) {
        Some(r) => r,
        None => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

fn bericht(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
    match msg {
        WM_SIZE => {
            hoofdscherm::indeling();
            Some(LRESULT(0))
        }
        WM_GETMINMAXINFO => {
            let dpi = unsafe { GetDpiForWindow(h) }.max(96) as i32;
            let mmi = unsafe { &mut *(lp.0 as *mut MINMAXINFO) };
            mmi.ptMinTrackSize = POINT { x: 520 * dpi / 96, y: 460 * dpi / 96 };
            Some(LRESULT(0))
        }
        WM_ERASEBKGND => Some(LRESULT(1)),
        WM_PAINT => {
            tekenen::venster(h);
            Some(LRESULT(0))
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN => {
            let t = thema::huidig();
            let dc = HDC(wp.0 as *mut _);
            // Labels iets zachter dan gewone tekst (contrast blijft ruim boven 7 : 1).
            let label = unsafe { GetDlgCtrlID(HWND(lp.0 as *mut _)) } as u16;
            let kleur = if [hoofdscherm::ID_LIJST_LABEL, hoofdscherm::ID_UPLOAD_UITLEG_LABEL, hoofdscherm::ID_GEKOZEN_LABEL, hoofdscherm::ID_VOORTGANG_LABEL, hoofdscherm::ID_MELDING_LABEL].contains(&label) { t.k.zacht } else { t.k.tekst };
            unsafe {
                SetTextColor(dc, kleur);
                SetBkColor(dc, t.k.kaart);
            }
            Some(LRESULT(t.kwast_kaart.0 as isize))
        }
        WM_DRAWITEM => {
            let dis = unsafe { &*(lp.0 as *const DRAWITEMSTRUCT) };
            tekenen::item(dis);
            Some(LRESULT(1))
        }
        WM_COMMAND => {
            let id = win::lo(wp.0) as u16;
            let code = win::hi(wp.0);
            if lp.0 == 0 || code == BN_CLICKED {
                opdracht(id);
            }
            Some(LRESULT(0))
        }
        WM_NOTIFY => {
            let kop = unsafe { &*(lp.0 as *const NMHDR) };
            hoofdscherm::melding_van_onderdeel(kop, lp)
        }
        WM_TIMER => {
            timer(h, wp.0);
            Some(LRESULT(0))
        }
        WM_INITMENUPOPUP => {
            menu::popup_open(h, HMENU(wp.0 as *mut _));
            None
        }
        WM_MENU_FOCUS => {
            menu::kies_aangevinkt(wp.0);
            Some(LRESULT(0))
        }
        WM_NA_START => {
            na_start(h);
            Some(LRESULT(0))
        }
        WM_LIJST_KLAAR => {
            let uitkomst = unsafe { Box::from_raw(lp.0 as *mut bestandsbrug::fout::Uitkomst<Vec<bestandsbrug::pcloud::lijst::Bestand>>) };
            scherm_download::lijst_klaar(*uitkomst, wp.0 == 1);
            Some(LRESULT(0))
        }
        WM_OVERDRACHT_KLAAR => {
            let klaar = unsafe { Box::from_raw(lp.0 as *mut overdracht_ui::Klaar) };
            overdracht_ui::klaar(*klaar);
            Some(LRESULT(0))
        }
        WM_UPDATE_INFO => {
            let info = unsafe { Box::from_raw(lp.0 as *mut update_ui::Gevonden) };
            update_ui::gevonden(*info, wp.0 == 1);
            Some(LRESULT(0))
        }
        WM_ACTIVATE => {
            if win::lo(wp.0) == WA_INACTIVE as u32 {
                let f = unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() };
                if !f.is_invalid() && unsafe { IsChild(h, f) }.as_bool() {
                    met(|s| s.laatste_focus = f);
                }
            }
            None
        }
        WM_SETFOCUS => {
            let f = met(|s| s.laatste_focus).unwrap_or_default();
            if !f.is_invalid() && win::is_zichtbaar(f) {
                unsafe {
                    let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(f));
                }
            } else {
                hoofdscherm::focus_start();
            }
            Some(LRESULT(0))
        }
        WM_CLOSE => {
            overdracht_ui::afsluiten();
            Some(LRESULT(0))
        }
        WM_QUERYENDSESSION => Some(LRESULT(1)),
        WM_ENDSESSION => {
            if wp.0 != 0 {
                overdracht_ui::windows_stopt();
            }
            Some(LRESULT(0))
        }
        WM_SETTINGCHANGE | WM_SYSCOLORCHANGE | WM_THEMECHANGED => {
            hoofdscherm::nieuw_thema(None);
            None
        }
        WM_DPICHANGED => {
            let r = unsafe { &*(lp.0 as *const RECT) };
            unsafe {
                let _ = SetWindowPos(h, None, r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
            }
            hoofdscherm::nieuw_thema(None);
            Some(LRESULT(0))
        }
        WM_DESTROY => {
            bewaar_metingen();
            bestandsbrug::log::info("Stop");
            unsafe { PostQuitMessage(0) };
            Some(LRESULT(0))
        }
        _ => None,
    }
}

fn timer(h: HWND, id: usize) {
    match id {
        TIMER_MELD => overdracht_ui::meld_timer(),
        TIMER_BALK => overdracht_ui::balk_timer(),
        TIMER_UPDATE => {
            unsafe {
                let _ = KillTimer(Some(h), TIMER_UPDATE);
            }
            update_ui::automatisch();
        }
        TIMER_LATER_ZEGGEN => {
            unsafe {
                let _ = KillTimer(Some(h), TIMER_LATER_ZEGGEN);
            }
            let rij = met(|s| std::mem::take(&mut s.later_zeggen)).unwrap_or_default();
            for (tekst, belang) in rij {
                zeg(&tekst, belang);
            }
        }
        TIMER_SLUIT => unsafe {
            let _ = KillTimer(Some(h), TIMER_SLUIT);
            let _ = DestroyWindow(h);
        },
        _ => {}
    }
}

fn na_start(h: HWND) {
    // Een oud installatiebestand van een eerdere update opruimen.
    let _ = std::fs::remove_dir_all(bestandsbrug::paden::gegevensmap().join("update"));
    scherm_download::vernieuw(false);
    let geen_update = met(|s| s.opties.geen_update).unwrap_or(false);
    if !geen_update {
        unsafe {
            SetTimer(Some(h), TIMER_UPDATE, bestandsbrug::instellingen::UPDATECONTROLE_WACHT_MS, None);
        }
    }
}

/// Een menukeuze of een knop.
pub fn opdracht(id: u16) {
    use hoofdscherm as hs;
    use menu as m;
    match id {
        hs::ID_KNOP_DOWNLOAD | m::ID_DOWNLOADEN => scherm_download::start_download(),
        hs::ID_KNOP_VERNIEUWEN | m::ID_VERNIEUWEN => scherm_download::vernieuw(true),
        hs::ID_KNOP_OMHOOG => verkenner::omhoog(),
        hs::ID_KNOP_KIEZEN | m::ID_UPLOAD_KIEZEN => {
            hoofdscherm::kies_pagina(1, false);
            scherm_upload::kies_bestand();
        }
        hs::ID_KNOP_MAP_KIEZEN | m::ID_UPLOAD_MAP_KIEZEN => {
            hoofdscherm::kies_pagina(1, false);
            scherm_upload::kies_map();
        }
        hs::ID_KNOP_UPLOAD | m::ID_UPLOADEN => {
            hoofdscherm::kies_pagina(1, false);
            scherm_upload::start_upload();
        }
        m::ID_NAAR_DOWNLOAD => hoofdscherm::kies_pagina(0, true),
        m::ID_NAAR_UPLOAD => hoofdscherm::kies_pagina(1, true),
        m::ID_AFSLUITEN => overdracht_ui::afsluiten(),
        m::ID_NL => hoofdscherm::kies_taal(bestandsbrug::taal::Taal::Nl),
        m::ID_EN => hoofdscherm::kies_taal(bestandsbrug::taal::Taal::En),
        m::ID_LICHT => hoofdscherm::kies_weergave(Weergave::Licht),
        m::ID_DONKER => hoofdscherm::kies_weergave(Weergave::Donker),
        m::ID_UITLEG => tekstvenster::uitleg(),
        m::ID_UPDATES => update_ui::handmatig(),
        m::ID_PRIVACY => tekstvenster::privacy(),
        m::ID_LOG => dialogen::open_log(),
        m::ID_LOGMAP => dialogen::open_logmap(),
        m::ID_OVER => tekstvenster::over(),
        _ => {}
    }
}

/// Bij --meet: de meetcijfers in een bestand zetten.
fn bewaar_metingen() {
    let (pad, regels) = met(|s| (s.opties.meet.clone(), s.metingen.clone())).unwrap_or((None, Vec::new()));
    if let Some(p) = pad {
        let _ = std::fs::write(p, regels.join("\r\n") + "\r\n");
    }
}
