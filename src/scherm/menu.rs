//! De menubalk: Bestand, Taal, Toegankelijkheid en Help. Echte Windows-menu's, zodat de schermlezer alles kent.
//! Bij Taal en Toegankelijkheid zijn de keuzes keuzerondjes (radio-items): de schermlezer hoort "aangevinkt".
//! Als zo'n menu opengaat, staat de keuze die nu geldt al geselecteerd.

use std::cell::Cell;

use bestandsbrug::taal::{self, t, Taal};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::win;
use super::*;

pub const ID_DOWNLOADEN: u16 = 1001;
pub const ID_UPLOAD_KIEZEN: u16 = 1002;
pub const ID_UPLOADEN: u16 = 1003;
pub const ID_VERNIEUWEN: u16 = 1004;
pub const ID_NAAR_DOWNLOAD: u16 = 1005;
pub const ID_NAAR_UPLOAD: u16 = 1006;
pub const ID_AFSLUITEN: u16 = 1007;
pub const ID_UPLOAD_MAP_KIEZEN: u16 = 1008;
pub const ID_NL: u16 = 1101;
pub const ID_EN: u16 = 1102;
pub const ID_LICHT: u16 = 1201;
pub const ID_DONKER: u16 = 1202;
pub const ID_UITLEG: u16 = 1301;
pub const ID_UPDATES: u16 = 1302;
pub const ID_PRIVACY: u16 = 1303;
pub const ID_LOG: u16 = 1304;
pub const ID_LOGMAP: u16 = 1305;
pub const ID_OVER: u16 = 1306;

#[derive(Clone, Copy, Default)]
struct Menus {
    taal: HMENU,
    toegankelijkheid: HMENU,
}

thread_local! {
    static MENUS: Cell<Menus> = Cell::new(Menus::default());
}

fn item(menu: HMENU, id: u16, sleutel: &str) {
    let v = win::ws(&t(sleutel));
    unsafe {
        let _ = AppendMenuW(menu, MF_STRING, id as usize, win::pc(&v));
    }
}

fn streep(menu: HMENU) {
    unsafe {
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, windows::core::PCWSTR::null());
    }
}

fn sub(balk: HMENU, menu: HMENU, sleutel: &str) {
    let v = win::ws(&t(sleutel));
    unsafe {
        let _ = AppendMenuW(balk, MF_POPUP, menu.0 as usize, win::pc(&v));
    }
}

/// Bouwt de menubalk (opnieuw, bijvoorbeeld na het wisselen van taal).
pub fn bouw(h: HWND) {
    unsafe {
        let balk = CreateMenu().unwrap_or_default();
        let bestand = CreatePopupMenu().unwrap_or_default();
        item(bestand, ID_DOWNLOADEN, "MenuDownloaden");
        item(bestand, ID_VERNIEUWEN, "MenuVernieuwen");
        streep(bestand);
        item(bestand, ID_UPLOAD_KIEZEN, "MenuUploadKiezen");
        item(bestand, ID_UPLOAD_MAP_KIEZEN, "MenuUploadMapKiezen");
        item(bestand, ID_UPLOADEN, "MenuUploaden");
        streep(bestand);
        item(bestand, ID_NAAR_DOWNLOAD, "MenuNaarDownload");
        item(bestand, ID_NAAR_UPLOAD, "MenuNaarUpload");
        streep(bestand);
        item(bestand, ID_AFSLUITEN, "MenuAfsluiten");
        sub(balk, bestand, "MenuBestand");

        let taalmenu = CreatePopupMenu().unwrap_or_default();
        item(taalmenu, ID_NL, "MenuNederlands");
        item(taalmenu, ID_EN, "MenuEngels");
        sub(balk, taalmenu, "MenuTaal");

        let toeg = CreatePopupMenu().unwrap_or_default();
        item(toeg, ID_LICHT, "MenuLicht");
        item(toeg, ID_DONKER, "MenuDonker");
        sub(balk, toeg, "MenuToegankelijkheid");

        let help = CreatePopupMenu().unwrap_or_default();
        item(help, ID_UITLEG, "MenuUitleg");
        item(help, ID_UPDATES, "MenuUpdates");
        item(help, ID_PRIVACY, "MenuPrivacy");
        streep(help);
        item(help, ID_LOG, "MenuLogOpenen");
        item(help, ID_LOGMAP, "MenuLogMap");
        streep(help);
        item(help, ID_OVER, "MenuOver");
        sub(balk, help, "MenuHelp");

        MENUS.with(|m| m.set(Menus { taal: taalmenu, toegankelijkheid: toeg }));
        let oud = GetMenu(h);
        let _ = SetMenu(h, Some(balk));
        if !oud.is_invalid() {
            let _ = DestroyMenu(oud);
        }
        zet_vinkjes();
        let _ = DrawMenuBar(h);
    }
}

/// Zet het keuzerondje bij de gekozen taal en weergave (MFT_RADIOCHECK + aangevinkt).
pub fn zet_vinkjes() {
    let m = MENUS.with(|m| m.get());
    let taal_id = if taal::huidig() == Taal::Nl { ID_NL } else { ID_EN };
    let weergave = met(|s| s.weergave).unwrap_or(Weergave::Licht);
    let w_id = if weergave == Weergave::Licht { ID_LICHT } else { ID_DONKER };
    unsafe {
        let _ = CheckMenuRadioItem(m.taal, ID_NL as u32, ID_EN as u32, taal_id as u32, MF_BYCOMMAND.0);
        let _ = CheckMenuRadioItem(m.toegankelijkheid, ID_LICHT as u32, ID_DONKER as u32, w_id as u32, MF_BYCOMMAND.0);
    }
}

/// Een menu gaat open. Bij Taal en Toegankelijkheid: de huidige keuze selecteren (na het openen).
pub fn popup_open(h: HWND, menu: HMENU) {
    let m = MENUS.with(|m| m.get());
    let index = if menu == m.taal {
        Some(if taal::huidig() == Taal::Nl { 0 } else { 1 })
    } else if menu == m.toegankelijkheid {
        Some(if met(|s| s.weergave).unwrap_or(Weergave::Licht) == Weergave::Licht { 0 } else { 1 })
    } else {
        None
    };
    if let Some(i) = index {
        if i > 0 {
            win::post(h, WM_MENU_FOCUS, i, 0);
        }
    }
}

/// MN_SELECTITEM: selecteert een regel in het menu dat nu open is.
const MN_SELECTITEM: u32 = 0x01E5;

pub fn kies_aangevinkt(index: usize) {
    let k = win::ws("#32768");
    unsafe {
        if let Ok(menuvenster) = FindWindowW(win::pc(&k), windows::core::PCWSTR::null()) {
            if !menuvenster.is_invalid() {
                win::snd(menuvenster, MN_SELECTITEM, index, 0);
            }
        }
    }
}
