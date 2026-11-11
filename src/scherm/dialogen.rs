//! De standaardvensters van Windows: bestand kiezen, opslaan als, en het logbestand of de map openen.

use std::path::{Path, PathBuf};

use bestandsbrug::taal::t;
use windows::core::PCWSTR;
use windows::Win32::UI::Controls::Dialogs::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::win::{pc, ws};
use super::*;

/// De map Downloads van de gebruiker (via Windows, zodat een verplaatste map ook klopt).
pub fn downloads_map() -> PathBuf {
    unsafe {
        if let Ok(p) = SHGetKnownFolderPath(&FOLDERID_Downloads, KNOWN_FOLDER_FLAG(0), None) {
            let s = p.to_string().unwrap_or_default();
            windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as *const _));
            if !s.is_empty() {
                return PathBuf::from(s);
            }
        }
    }
    bestandsbrug::paden::downloads()
}

fn filter() -> Vec<u16> {
    let mut v: Vec<u16> = t("FilterAlles").encode_utf16().collect();
    v.push(0);
    v.extend("*.*".encode_utf16());
    v.extend([0, 0]);
    v
}

/// "Opslaan als": standaard in Downloads, met de naam van het bestand. Vraagt zelf NIET om te overschrijven;
/// dat doen wij daarna in de eigen taal.
pub fn kies_opslaan(naam: &str) -> Option<PathBuf> {
    let mut buf = vec![0u16; 4096];
    for (i, c) in naam.encode_utf16().take(4000).enumerate() {
        buf[i] = c;
    }
    let map = ws(&downloads_map().to_string_lossy());
    let titel = ws(&t_naam("OpslaanTitel", naam));
    let f = filter();
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd(),
        lpstrFilter: pc(&f),
        lpstrFile: windows::core::PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrInitialDir: pc(&map),
        lpstrTitle: pc(&titel),
        Flags: OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_HIDEREADONLY,
        ..Default::default()
    };
    begin_vraag();
    let ok = unsafe { GetSaveFileNameW(&mut ofn) }.as_bool();
    einde_vraag();
    ok.then(|| PathBuf::from(String::from_utf16_lossy(&buf[..buf.iter().position(|&c| c == 0).unwrap_or(0)])))
}

fn t_naam(sleutel: &str, naam: &str) -> String {
    bestandsbrug::taal::f(sleutel, &[naam])
}

/// Een bestand kiezen om te uploaden.
pub fn kies_openen() -> Option<PathBuf> {
    let mut buf = vec![0u16; 4096];
    let titel = ws(&t("UploadKiezenTitel"));
    let f = filter();
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd(),
        lpstrFilter: pc(&f),
        lpstrFile: windows::core::PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrTitle: pc(&titel),
        Flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_HIDEREADONLY,
        ..Default::default()
    };
    begin_vraag();
    let ok = unsafe { GetOpenFileNameW(&mut ofn) }.as_bool();
    einde_vraag();
    ok.then(|| PathBuf::from(String::from_utf16_lossy(&buf[..buf.iter().position(|&c| c == 0).unwrap_or(0)])))
}

/// Opent een bestand of programma met Windows. Geeft true als het lukte.
pub fn open(bestand: &str, parameters: &str) -> bool {
    let b = ws(bestand);
    let p = ws(parameters);
    let open = ws("open");
    let r = unsafe { ShellExecuteW(Some(hwnd()), pc(&open), pc(&b), if parameters.is_empty() { PCWSTR::null() } else { pc(&p) }, PCWSTR::null(), SW_SHOWNORMAL) };
    r.0 as isize > 32
}

/// Help, Logbestand openen: in de standaard teksteditor, anders in Kladblok.
pub fn open_log() {
    let pad = bestandsbrug::paden::logbestand();
    if !pad.exists() {
        vragen::info(hwnd(), &t("LogNietGevonden"));
        return;
    }
    let s = pad.to_string_lossy().to_string();
    if !open(&s, "") && !open("notepad.exe", &format!("\"{s}\"")) {
        vragen::info(hwnd(), &bestandsbrug::taal::f("LogOpenFout", &[""]));
    }
}

/// Help, Map met logbestand openen: de Verkenner, met het logbestand gekozen.
pub fn open_logmap() {
    let pad = bestandsbrug::paden::logbestand();
    let map = pad.parent().map(Path::to_path_buf).unwrap_or_default();
    if pad.exists() {
        open("explorer.exe", &format!("/select,\"{}\"", pad.to_string_lossy()));
    } else {
        open(&map.to_string_lossy(), "");
    }
}
