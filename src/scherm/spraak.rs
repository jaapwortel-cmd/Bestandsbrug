//! Meldingen aan de schermlezer, zonder de focus te verplaatsen. Draait JAWS, dan via de JAWS-API (SayString,
//! zonder te onderbreken). Anders via een UI Automation-melding (NVDA, Verteller en andere).
//! De melding komt ook als tekst in het vak "Melding" onderaan, zodat je hem kunt nalezen.

use windows::core::{BSTR, GUID, PCWSTR};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, CHILDID_SELF, OBJID_CLIENT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Belang {
    /// Voortgang: mag in de rij wachten.
    Gewoon,
    /// Begin, einde, fout en onderbreking.
    Belangrijk,
}

fn jaws_draait() -> bool {
    let k = super::win::ws("JFWUI2");
    unsafe { FindWindowW(super::win::pc(&k), PCWSTR::null()).map(|h| !h.is_invalid()).unwrap_or(false) }
}

/// Laat JAWS de tekst zeggen. Geeft false als dat niet lukt.
fn jaws_zeg(tekst: &str) -> bool {
    unsafe {
        let progid = super::win::ws("FreedomSci.JawsApi");
        let Ok(clsid) = CLSIDFromProgID(super::win::pc(&progid)) else { return false };
        let Ok(disp): windows::core::Result<IDispatch> = CoCreateInstance(&clsid, None, CLSCTX_ALL) else { return false };
        let naam = super::win::ws("SayString");
        let namen = [PCWSTR(naam.as_ptr())];
        let mut id = 0i32;
        if disp.GetIDsOfNames(&GUID::zeroed(), namen.as_ptr(), 1, 0, &mut id).is_err() {
            return false;
        }
        // Argumenten in omgekeerde volgorde: eerst FlushBuffer (false = niet onderbreken), dan de tekst.
        let mut args = [VARIANT::from(false), VARIANT::from(BSTR::from(tekst))];
        let params = DISPPARAMS { rgvarg: args.as_mut_ptr(), rgdispidNamedArgs: std::ptr::null_mut(), cArgs: 2, cNamedArgs: 0 };
        disp.Invoke(id, &GUID::zeroed(), 0, DISPATCH_METHOD, &params, None, None, None).is_ok()
    }
}

/// Een UI Automation-melding vanaf het venster `bron`.
fn uia_zeg(bron: HWND, tekst: &str, belang: Belang) {
    unsafe {
        if let Ok(p) = UiaHostProviderFromHwnd(bron) {
            let verwerking = match belang {
                Belang::Gewoon => NotificationProcessing_All,
                Belang::Belangrijk => NotificationProcessing_ImportantAll,
            };
            let _ = UiaRaiseNotificationEvent(&p, NotificationKind_Other, verwerking, &BSTR::from(tekst), &BSTR::from(bestandsbrug::instellingen::MELDING_ID));
        }
    }
}

/// Zeg de tekst nu (beleefd: de focus blijft waar hij is).
pub fn zeg(bron: HWND, tekst: &str, belang: Belang) {
    if tekst.is_empty() {
        return;
    }
    if jaws_draait() && jaws_zeg(tekst) {
        return;
    }
    uia_zeg(bron, tekst, belang);
}

/// Geeft een onderdeel een naam voor de schermlezer (voor onderdelen zonder label, zoals de tabbladen).
pub fn zet_naam(h: HWND, naam: &str) {
    unsafe {
        let Ok(diensten): windows::core::Result<IAccPropServices> = CoCreateInstance(&CLSID_AccPropServices, None, CLSCTX_INPROC_SERVER) else { return };
        let w = super::win::ws(naam);
        let _ = diensten.SetHwndPropStr(h, OBJID_CLIENT.0 as u32, CHILDID_SELF, PROPID_ACC_NAME, super::win::pc(&w));
    }
}

/// De voortgangsbalk krijgt de rol "voortgangsbalk" en een waarde die de schermlezer kan lezen
/// (bijvoorbeeld "40 procent"). We sturen bewust GEEN wijzigingsbericht, zodat de schermlezer niet bij
/// elke stap praat; de melding komt alleen van de timer van 6 seconden.
pub fn zet_balk_toegankelijk(balk: HWND, waarde: &str) {
    unsafe {
        let Ok(diensten): windows::core::Result<IAccPropServices> = CoCreateInstance(&CLSID_AccPropServices, None, CLSCTX_INPROC_SERVER) else { return };
        let rol = VARIANT::from(ROLE_SYSTEM_PROGRESSBAR as i32);
        let _ = diensten.SetHwndProp(balk, OBJID_CLIENT.0 as u32, CHILDID_SELF, PROPID_ACC_ROLE, &rol);
        let w = super::win::ws(waarde);
        let _ = diensten.SetHwndPropStr(balk, OBJID_CLIENT.0 as u32, CHILDID_SELF, PROPID_ACC_VALUE, super::win::pc(&w));
    }
}
