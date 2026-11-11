//! Het uiterlijk voor ziende mensen: de kop met de naam, kaarten met ronde hoeken, ronde knoppen met een
//! duidelijke focusrand, tabbladen als "pillen", de voortgangsbalk en de kolomkoppen van de lijst.
//! Alleen tekenen: namen, rollen en toestanden voor de schermlezer blijven die van Windows zelf.

use bestandsbrug::taal::zonder_amp;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetFocus;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::*;

use super::hoofdscherm::{self, is_hoofdknop};
use super::thema::{self, Thema};
use super::win;

fn rondje(dc: HDC, r: &RECT, vulling: COLORREF, rand: COLORREF, randdikte: i32, straal: i32) {
    unsafe {
        let kwast = CreateSolidBrush(vulling);
        let pen = if randdikte > 0 { CreatePen(PS_SOLID, randdikte, rand) } else { CreatePen(PS_NULL, 0, rand) };
        let ok = SelectObject(dc, kwast.into());
        let op = SelectObject(dc, pen.into());
        let _ = RoundRect(dc, r.left, r.top, r.right, r.bottom, straal, straal);
        SelectObject(dc, ok);
        SelectObject(dc, op);
        let _ = DeleteObject(kwast.into());
        let _ = DeleteObject(pen.into());
    }
}

/// Een rand zonder vulling (voor de focus).
fn randje(dc: HDC, r: &RECT, kleur: COLORREF, dikte: i32, straal: i32) {
    unsafe {
        let pen = CreatePen(PS_SOLID, dikte, kleur);
        let op = SelectObject(dc, pen.into());
        let ok = SelectObject(dc, GetStockObject(NULL_BRUSH));
        let _ = RoundRect(dc, r.left, r.top, r.right, r.bottom, straal, straal);
        SelectObject(dc, op);
        SelectObject(dc, ok);
        let _ = DeleteObject(pen.into());
    }
}

fn tekst(dc: HDC, s: &str, r: &RECT, kleur: COLORREF, letter: HFONT, vorm: DRAW_TEXT_FORMAT) {
    unsafe {
        let oud = SelectObject(dc, letter.into());
        SetTextColor(dc, kleur);
        SetBkMode(dc, TRANSPARENT);
        let mut v: Vec<u16> = s.encode_utf16().collect();
        let mut rr = *r;
        DrawTextW(dc, &mut v, &mut rr, vorm);
        SelectObject(dc, oud);
    }
}

fn vul(dc: HDC, r: &RECT, kleur: COLORREF) {
    unsafe {
        let k = CreateSolidBrush(kleur);
        FillRect(dc, r, k);
        let _ = DeleteObject(k.into());
    }
}

/// Het hoofdvenster: achtergrond, kop met naam en icoon, en de twee kaarten.
pub fn venster(h: HWND) {
    let th = thema::huidig();
    let mt = hoofdscherm::maten();
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(h, &mut ps);
        let mut alles = RECT::default();
        let _ = GetClientRect(h, &mut alles);
        FillRect(dc, &alles, th.kwast_achtergrond);
        // Kop met de naam van de app (in de gekozen taal) en het icoon.
        vul(dc, &mt.kop, th.k.kop);
        if th.hoog_contrast {
            let lijn = RECT { top: mt.kop.bottom - 2, ..mt.kop };
            vul(dc, &lijn, th.k.tekst);
        }
        let icoon_maat = th.regel * 3 / 2;
        let iy = (mt.kop.bottom - icoon_maat) / 2;
        if let Ok(icoon) = LoadImageW(Some(win::hinst()), windows::core::PCWSTR(1 as *const u16), IMAGE_ICON, icoon_maat, icoon_maat, LR_DEFAULTCOLOR) {
            let _ = DrawIconEx(dc, mt.marge, iy, HICON(icoon.0), icoon_maat, icoon_maat, 0, None, DI_NORMAL);
            let _ = DestroyIcon(HICON(icoon.0));
        }
        let naam_r = RECT { left: mt.marge * 2 + icoon_maat, ..mt.kop };
        tekst(dc, &bestandsbrug::taal::app_naam(), &naam_r, th.k.op_kop, th.letter_kop, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        // Kaarten.
        let straal = th.regel;
        rondje(dc, &mt.pagina_kaart, th.k.kaart, th.k.rand, 1, straal);
        rondje(dc, &mt.status_kaart, th.k.kaart, th.k.rand, 1, straal);
        let _ = EndPaint(h, &ps);
    }
}

/// Knoppen en de voortgangsbalk.
pub fn item(dis: &DRAWITEMSTRUCT) {
    let th = thema::huidig();
    match dis.CtlType {
        ODT_BUTTON => knop(dis, &th),
        ODT_STATIC => balk(dis, &th),
        _ => {}
    }
}

fn knop(dis: &DRAWITEMSTRUCT, th: &Thema) {
    let dc = dis.hDC;
    let r = dis.rcItem;
    let id = dis.CtlID as u16;
    let ingedrukt = (dis.itemState.0 & ODS_SELECTED.0) != 0;
    let focus = (dis.itemState.0 & ODS_FOCUS.0) != 0;
    let geen_sneltoets = (dis.itemState.0 & ODS_NOACCEL.0) != 0;
    vul(dc, &r, th.k.kaart);
    let d = (th.regel / 8).max(2);
    let binnen = RECT { left: r.left + d + 1, top: r.top + d + 1, right: r.right - d - 1, bottom: r.bottom - d - 1 };
    let straal = (binnen.bottom - binnen.top) * 2 / 3;
    let (vulling, tekstkleur, rand) = if is_hoofdknop(id) {
        (if ingedrukt { th.k.accent_ingedrukt } else { th.k.accent }, th.k.op_accent, th.k.accent)
    } else {
        (if ingedrukt { th.k.achtergrond } else { th.k.kaart }, th.k.tekst, th.k.rand)
    };
    rondje(dc, &binnen, vulling, rand, 2, straal);
    let mut vorm = DT_CENTER | DT_VCENTER | DT_SINGLELINE;
    if geen_sneltoets {
        vorm |= DT_HIDEPREFIX;
    }
    tekst(dc, &win::lees_tekst(dis.hwndItem), &binnen, tekstkleur, th.letter_vet, vorm);
    if focus {
        // Dikke, goed zichtbare focusrand om de hele knop.
        let fr = RECT { left: r.left + d / 2, top: r.top + d / 2, right: r.right - d / 2, bottom: r.bottom - d / 2 };
        randje(dc, &fr, th.k.focus, d, straal + d);
    }
}

fn balk(dis: &DRAWITEMSTRUCT, th: &Thema) {
    let dc = dis.hDC;
    let r = dis.rcItem;
    vul(dc, &r, th.k.kaart);
    let straal = r.bottom - r.top;
    rondje(dc, &r, th.k.achtergrond, th.k.rand, 1, straal);
    let promille = super::balk() as i32;
    if promille > 0 {
        let b = ((r.right - r.left) as i64 * promille as i64 / 1000) as i32;
        let gevuld = RECT { right: r.left + b.max(straal), ..r };
        rondje(dc, &gevuld, th.k.accent, th.k.accent, 1, straal);
        let pct = format!("{} %", promille / 10);
        let kleur = if b > (r.right - r.left) / 2 { th.k.op_accent } else { th.k.tekst };
        tekst(dc, &pct, &r, kleur, th.letter_vet, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    }
}

/// De tabbladen tekenen als "pillen". Het onderdeel blijft een echt tabblad-onderdeel van Windows.
pub fn tabbladen(tab: HWND) {
    unsafe {
        let _ = SetWindowSubclass(tab, Some(tab_proc), 1, 0);
    }
}

extern "system" fn tab_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM, _id: usize, _ref: usize) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            teken_tabs(h);
            LRESULT(0)
        }
        WM_SETFOCUS | WM_KILLFOCUS => {
            let r = unsafe { DefSubclassProc(h, msg, wp, lp) };
            unsafe {
                let _ = InvalidateRect(Some(h), None, true);
            }
            r
        }
        _ => unsafe { DefSubclassProc(h, msg, wp, lp) },
    }
}

fn teken_tabs(h: HWND) {
    let th = thema::huidig();
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(h, &mut ps);
        let mut alles = RECT::default();
        let _ = GetClientRect(h, &mut alles);
        FillRect(dc, &alles, th.kwast_achtergrond);
        let aantal = win::snd(h, TCM_GETITEMCOUNT, 0, 0);
        let gekozen = win::snd(h, TCM_GETCURSEL, 0, 0);
        let focus = GetFocus() == h;
        let d = (th.regel / 8).max(2);
        for i in 0..aantal {
            let mut r = RECT::default();
            win::snd(h, TCM_GETITEMRECT, i as usize, &mut r as *mut _ as isize);
            let mut buf = [0u16; 128];
            let mut item = TCITEMW { mask: TCIF_TEXT, pszText: windows::core::PWSTR(buf.as_mut_ptr()), cchTextMax: 128, ..Default::default() };
            win::snd(h, TCM_GETITEMW, i as usize, &mut item as *mut _ as isize);
            let naam = String::from_utf16_lossy(&buf[..buf.iter().position(|&c| c == 0).unwrap_or(0)]);
            let pil = RECT { left: r.left + d, top: alles.top + d, right: r.right - d, bottom: alles.bottom - d };
            let straal = pil.bottom - pil.top;
            if i == gekozen {
                rondje(dc, &pil, th.k.keuze, th.k.keuze, 2, straal);
                tekst(dc, &naam, &pil, th.k.op_keuze, th.letter_vet, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
                if focus {
                    let fr = RECT { left: pil.left - d / 2 - 1, top: pil.top - d / 2 - 1, right: pil.right + d / 2 + 1, bottom: pil.bottom + d / 2 + 1 };
                    randje(dc, &fr, th.k.focus, d, straal + d);
                }
            } else {
                rondje(dc, &pil, th.k.kaart, th.k.rand, 1, straal);
                tekst(dc, &naam, &pil, th.k.tekst, th.letter_vet, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            }
        }
        let _ = EndPaint(h, &ps);
    }
}

/// De kolomkoppen van de lijst in de kleuren van de weergave.
pub fn kop_van_lijst(lijst: HWND) {
    unsafe {
        let _ = SetWindowSubclass(lijst, Some(lijst_proc), 2, 0);
    }
}

extern "system" fn lijst_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM, _id: usize, _ref: usize) -> LRESULT {
    if msg == WM_NOTIFY {
        let kop = unsafe { &*(lp.0 as *const NMHDR) };
        if kop.code == NM_CUSTOMDRAW {
            let kopje = HWND(win::snd(h, LVM_GETHEADER, 0, 0) as *mut _);
            if kop.hwndFrom == kopje {
                let cd = unsafe { &*(lp.0 as *const NMCUSTOMDRAW) };
                let th = thema::huidig();
                if cd.dwDrawStage == CDDS_PREPAINT {
                    return LRESULT(CDRF_NOTIFYITEMDRAW as isize);
                }
                if cd.dwDrawStage == CDDS_ITEMPREPAINT {
                    let r = cd.rc;
                    vul(cd.hdc, &r, th.k.achtergrond);
                    let lijn = RECT { top: r.bottom - 2, ..r };
                    vul(cd.hdc, &lijn, th.k.rand);
                    let mut buf = [0u16; 128];
                    let mut item = HDITEMW { mask: HDI_TEXT, pszText: windows::core::PWSTR(buf.as_mut_ptr()), cchTextMax: 128, ..Default::default() };
                    win::snd(kopje, HDM_GETITEMW, cd.dwItemSpec, &mut item as *mut _ as isize);
                    let naam = String::from_utf16_lossy(&buf[..buf.iter().position(|&c| c == 0).unwrap_or(0)]);
                    let tr = RECT { left: r.left + th.regel / 3, ..r };
                    tekst(cd.hdc, &zonder_amp(&naam), &tr, th.k.tekst, th.letter_vet, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);
                    return LRESULT(CDRF_SKIPDEFAULT as isize);
                }
            }
        }
    }
    unsafe { DefSubclassProc(h, msg, wp, lp) }
}
