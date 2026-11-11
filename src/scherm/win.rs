//! Kleine hulpjes voor de Windows-API: tekst omzetten, berichten sturen, onderdelen maken.

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Tekst naar UTF-16 met een 0 aan het eind.
pub fn ws(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn pc(v: &[u16]) -> PCWSTR {
    PCWSTR(v.as_ptr())
}

pub fn snd(h: HWND, msg: u32, w: usize, l: isize) -> isize {
    unsafe { SendMessageW(h, msg, Some(WPARAM(w)), Some(LPARAM(l))).0 }
}

pub fn post(h: HWND, msg: u32, w: usize, l: isize) {
    unsafe {
        let _ = PostMessageW(Some(h), msg, WPARAM(w), LPARAM(l));
    }
}

pub fn hinst() -> HINSTANCE {
    unsafe { GetModuleHandleW(None).map(|m| HINSTANCE(m.0)).unwrap_or_default() }
}

pub fn zet_tekst(h: HWND, s: &str) {
    let v = ws(s);
    unsafe {
        let _ = SetWindowTextW(h, pc(&v));
    }
}

pub fn lees_tekst(h: HWND) -> String {
    unsafe {
        let n = GetWindowTextLengthW(h);
        let mut buf = vec![0u16; n as usize + 1];
        let m = GetWindowTextW(h, &mut buf);
        String::from_utf16_lossy(&buf[..m as usize])
    }
}

/// Maakt een onderdeel (knop, lijst, tekstvak, ...) in het venster `ouder`.
pub fn maak(klasse: &str, tekst: &str, stijl: u32, exstijl: u32, ouder: HWND, id: u16) -> HWND {
    let k = ws(klasse);
    let t = ws(tekst);
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(exstijl),
            pc(&k),
            pc(&t),
            WINDOW_STYLE(stijl | WS_CHILD.0 | WS_VISIBLE.0),
            0,
            0,
            10,
            10,
            Some(ouder),
            Some(HMENU(id as usize as *mut core::ffi::c_void)),
            Some(hinst()),
            None,
        )
        .unwrap_or_default()
    }
}

pub fn verplaats(h: HWND, x: i32, y: i32, b: i32, hoogte: i32) {
    unsafe {
        let _ = MoveWindow(h, x, y, b.max(1), hoogte.max(1), true);
    }
}

pub fn toon(h: HWND, zichtbaar: bool) {
    unsafe {
        let _ = ShowWindow(h, if zichtbaar { SW_SHOW } else { SW_HIDE });
    }
}

pub fn is_zichtbaar(h: HWND) -> bool {
    unsafe { IsWindowVisible(h).as_bool() }
}

pub fn lo(x: usize) -> u32 {
    (x & 0xffff) as u32
}

pub fn hi(x: usize) -> u32 {
    ((x >> 16) & 0xffff) as u32
}
