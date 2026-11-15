//! Geïnstalleerde programma's uit het register lezen (Inno Setup: Uninstall\*_is1, voor de gebruiker en voor
//! de hele computer, ook 32-bits) en een programma starten. De logica (namen en versies vergelijken) staat in
//! bestandsbrug::programmas.

use std::path::{Path, PathBuf};

use bestandsbrug::programmas::{self, Geinstalleerd};
use windows::Win32::System::Registry::*;

use super::win::{pc, ws};

const UNINSTALL: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
const UNINSTALL_32: &str = "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall";

/// Alle programma's met een Inno Setup-sleutel (*_is1).
pub fn lees() -> Vec<Geinstalleerd> {
    let mut alle: Vec<Geinstalleerd> = Vec::new();
    let plekken = [
        (HKEY_CURRENT_USER, UNINSTALL, KEY_READ),
        (HKEY_CURRENT_USER, UNINSTALL_32, KEY_READ),
        (HKEY_LOCAL_MACHINE, UNINSTALL, KEY_READ | KEY_WOW64_64KEY),
        (HKEY_LOCAL_MACHINE, UNINSTALL, KEY_READ | KEY_WOW64_32KEY),
    ];
    for (wortel, pad, rechten) in plekken {
        for p in lees_sleutel(wortel, pad, rechten) {
            if !alle.contains(&p) {
                alle.push(p);
            }
        }
    }
    alle
}

fn lees_sleutel(wortel: HKEY, pad: &str, rechten: REG_SAM_FLAGS) -> Vec<Geinstalleerd> {
    let mut uit = Vec::new();
    let p = ws(pad);
    let mut sleutel = HKEY::default();
    unsafe {
        if RegOpenKeyExW(wortel, pc(&p), None, rechten, &mut sleutel).is_err() {
            return uit;
        }
        let mut i = 0u32;
        loop {
            let mut buf = [0u16; 256];
            let mut n = buf.len() as u32;
            let r = RegEnumKeyExW(sleutel, i, Some(windows::core::PWSTR(buf.as_mut_ptr())), &mut n, None, None, None, None);
            if r.is_err() {
                break;
            }
            i += 1;
            let naam = String::from_utf16_lossy(&buf[..n as usize]);
            if !naam.to_lowercase().ends_with("_is1") {
                continue;
            }
            let lees = |waarde: &str| lees_tekst(sleutel, &naam, waarde);
            let p = Geinstalleerd { naam: lees("DisplayName"), versie: lees("DisplayVersion"), map: lees("InstallLocation"), icoon: lees("DisplayIcon") };
            if !p.naam.trim().is_empty() {
                uit.push(p);
            }
        }
        let _ = RegCloseKey(sleutel);
    }
    uit
}

/// Een tekstwaarde uit een subsleutel (leeg als hij er niet is).
fn lees_tekst(sleutel: HKEY, sub: &str, waarde: &str) -> String {
    let s = ws(sub);
    let w = ws(waarde);
    let mut grootte = 0u32;
    unsafe {
        if RegGetValueW(sleutel, pc(&s), pc(&w), RRF_RT_REG_SZ, None, None, Some(&mut grootte)).is_err() || grootte == 0 {
            return String::new();
        }
        let mut buf = vec![0u16; grootte as usize / 2 + 1];
        let mut g = (buf.len() * 2) as u32;
        if RegGetValueW(sleutel, pc(&s), pc(&w), RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr() as *mut _), Some(&mut g)).is_err() {
            return String::new();
        }
        let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..n]).trim().to_string()
    }
}

/// Het programmabestand om te starten: DisplayIcon (als dat een .exe is), anders het .exe-bestand in de
/// installatiemap dat bij de naam past.
pub fn start_pad(p: &Geinstalleerd, mapnaam: &str) -> Option<PathBuf> {
    if let Some(exe) = programmas::exe_uit_icoon(&p.icoon) {
        let pad = PathBuf::from(exe);
        if pad.is_file() {
            return Some(pad);
        }
    }
    let map = p.map.trim().trim_matches('"');
    if map.is_empty() {
        return None;
    }
    let namen: Vec<String> = std::fs::read_dir(map)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    let exe = programmas::kies_exe(&namen, mapnaam)?;
    Some(Path::new(map).join(exe))
}
