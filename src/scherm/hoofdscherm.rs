//! Het hoofdscherm: de onderdelen maken (tabbladen, lijst met mappen en bestanden, knoppen, voortgang, melding), de teksten in de
//! gekozen taal, de indeling op het scherm, en wisselen van tabblad, taal en weergave.
//! Volgorde op het scherm = volgorde met Tab = volgorde voor de schermlezer.

use std::cell::Cell;

use bestandsbrug::taal::{self, t, zonder_amp, Taal};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetFocus, SetFocus};
use windows::Win32::UI::WindowsAndMessaging::*;

use super::win::{self, snd};
use super::*;

pub const ID_TAB: u16 = 10;
pub const ID_LIJST_LABEL: u16 = 11;
pub const ID_LIJST: u16 = 12;
pub const ID_KNOP_DOWNLOAD: u16 = 13;
pub const ID_KNOP_VERNIEUWEN: u16 = 14;
pub const ID_KNOP_OMHOOG: u16 = 15;
pub const ID_UPLOAD_UITLEG_LABEL: u16 = 20;
pub const ID_UPLOAD_UITLEG: u16 = 21;
pub const ID_KNOP_KIEZEN: u16 = 22;
pub const ID_GEKOZEN_LABEL: u16 = 23;
pub const ID_GEKOZEN: u16 = 24;
pub const ID_KNOP_UPLOAD: u16 = 25;
pub const ID_VOORTGANG_LABEL: u16 = 30;
pub const ID_BALK: u16 = 31;
pub const ID_MELDING_LABEL: u16 = 32;
pub const ID_MELDING: u16 = 33;

/// De knoppen met een gevulde (opvallende) kleur.
pub fn is_hoofdknop(id: u16) -> bool {
    id == ID_KNOP_DOWNLOAD || id == ID_KNOP_UPLOAD
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Onderdelen {
    pub tab: HWND,
    pub lijst_label: HWND,
    pub lijst: HWND,
    pub knop_download: HWND,
    pub knop_vernieuwen: HWND,
    pub knop_omhoog: HWND,
    pub upload_uitleg_label: HWND,
    pub upload_uitleg: HWND,
    pub knop_kiezen: HWND,
    pub gekozen_label: HWND,
    pub gekozen: HWND,
    pub knop_upload: HWND,
    pub voortgang_label: HWND,
    pub balk: HWND,
    pub melding_label: HWND,
    pub melding: HWND,
}

impl Onderdelen {
    fn download_pagina(&self) -> [HWND; 5] {
        [self.lijst_label, self.lijst, self.knop_download, self.knop_omhoog, self.knop_vernieuwen]
    }
    fn upload_pagina(&self) -> [HWND; 6] {
        [self.upload_uitleg_label, self.upload_uitleg, self.knop_kiezen, self.gekozen_label, self.gekozen, self.knop_upload]
    }
    pub fn knoppen(&self) -> [HWND; 5] {
        [self.knop_download, self.knop_omhoog, self.knop_vernieuwen, self.knop_kiezen, self.knop_upload]
    }
}

thread_local! {
    static ONDERDELEN: Cell<Onderdelen> = Cell::new(Onderdelen::default());
}

pub fn onderdelen() -> Onderdelen {
    ONDERDELEN.with(|o| o.get())
}

const KNOP: u32 = WS_TABSTOP.0 | BS_OWNERDRAW as u32;
/// Stijlen van een STATIC-onderdeel: geen & als sneltoets, en zelf getekend (de voortgangsbalk).
const SS_NOPREFIX: u32 = 0x80;
const SS_OWNERDRAW: u32 = 0x0D;
const LABEL: u32 = SS_NOPREFIX;
const LEESVAK: u32 = WS_TABSTOP.0 | ES_READONLY as u32 | ES_MULTILINE as u32 | ES_AUTOVSCROLL as u32;

/// Maakt alle onderdelen, in de volgorde waarin Tab en de schermlezer ze tegenkomen.
pub fn maak_onderdelen(h: HWND) {
    let m = |k: &str, stijl: u32, ex: u32, id: u16| win::maak(k, "", stijl, ex, h, id);
    let o = Onderdelen {
        tab: m("SysTabControl32", WS_TABSTOP.0 | TCS_FOCUSONBUTTONDOWN as u32 | TCS_FIXEDWIDTH as u32 | WS_CLIPSIBLINGS.0, 0, ID_TAB),
        lijst_label: m("STATIC", LABEL, 0, ID_LIJST_LABEL),
        lijst: m(
            "SysListView32",
            WS_TABSTOP.0 | LVS_REPORT as u32 | LVS_SINGLESEL as u32 | LVS_SHOWSELALWAYS as u32,
            0,
            ID_LIJST,
        ),
        knop_download: m("BUTTON", KNOP, 0, ID_KNOP_DOWNLOAD),
        knop_omhoog: m("BUTTON", KNOP, 0, ID_KNOP_OMHOOG),
        knop_vernieuwen: m("BUTTON", KNOP, 0, ID_KNOP_VERNIEUWEN),
        upload_uitleg_label: m("STATIC", LABEL, 0, ID_UPLOAD_UITLEG_LABEL),
        upload_uitleg: m("EDIT", LEESVAK, 0, ID_UPLOAD_UITLEG),
        knop_kiezen: m("BUTTON", KNOP, 0, ID_KNOP_KIEZEN),
        gekozen_label: m("STATIC", LABEL, 0, ID_GEKOZEN_LABEL),
        gekozen: m("EDIT", WS_TABSTOP.0 | ES_READONLY as u32 | ES_AUTOHSCROLL as u32, 0, ID_GEKOZEN),
        knop_upload: m("BUTTON", KNOP, 0, ID_KNOP_UPLOAD),
        voortgang_label: m("STATIC", LABEL, 0, ID_VOORTGANG_LABEL),
        balk: m("STATIC", SS_OWNERDRAW | LABEL, 0, ID_BALK),
        melding_label: m("STATIC", LABEL, 0, ID_MELDING_LABEL),
        melding: m("EDIT", LEESVAK, 0, ID_MELDING),
    };
    ONDERDELEN.with(|c| c.set(o));
    // Lijst: hele regel kiezen, met kolomkoppen.
    snd(o.lijst, LVM_SETEXTENDEDLISTVIEWSTYLE, 0, (LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER | LVS_EX_LABELTIP) as isize);
    for i in 0..5 {
        let mut kol = LVCOLUMNW { mask: LVCF_WIDTH | LVCF_TEXT | LVCF_SUBITEM, cx: 100, iSubItem: i, ..Default::default() };
        let leeg = win::ws("");
        kol.pszText = windows::core::PWSTR(leeg.as_ptr() as *mut u16);
        snd(o.lijst, LVM_INSERTCOLUMNW, i as usize, &kol as *const _ as isize);
    }
    tekenen::kop_van_lijst(o.lijst);
    tekenen::tabbladen(o.tab);
    super::spraak::zet_balk_toegankelijk(o.balk, "");
    for v in [o.upload_uitleg, o.melding] {
        snd(v, EM_SETMARGINS, (EC_LEFTMARGIN | EC_RIGHTMARGIN) as usize, 0);
    }
    pas_thema_toe();
    kies_pagina(0, false);
}

/// Alle teksten in de gekozen taal (ook na het wisselen van taal).
pub fn zet_teksten() {
    let o = onderdelen();
    let h = hwnd();
    win::zet_tekst(h, &taal::app_naam());
    // Tabbladen (met een naam voor de schermlezer: er staat geen label voor).
    win::zet_tekst(o.tab, &t("TabsNaam"));
    super::spraak::zet_naam(o.tab, &t("TabsNaam"));
    snd(o.tab, TCM_DELETEALLITEMS, 0, 0);
    for (i, k) in ["TabDownloaden", "TabUploaden"].iter().enumerate() {
        let tekst = win::ws(&t(k));
        let item = TCITEMW { mask: TCIF_TEXT, pszText: windows::core::PWSTR(tekst.as_ptr() as *mut u16), ..Default::default() };
        snd(o.tab, TCM_INSERTITEMW, i, &item as *const _ as isize);
    }
    let p = met(|s| s.pagina).unwrap_or(0);
    snd(o.tab, TCM_SETCURSEL, p, 0);
    win::zet_tekst(o.lijst_label, &t("LijstNaam"));
    for (i, k) in ["KolomNaam", "KolomSoort", "KolomGrootte", "KolomDatum", "KolomUitleg"].iter().enumerate() {
        let tekst = win::ws(&t(k));
        let kol = LVCOLUMNW { mask: LVCF_TEXT, pszText: windows::core::PWSTR(tekst.as_ptr() as *mut u16), ..Default::default() };
        snd(o.lijst, LVM_SETCOLUMNW, i, &kol as *const _ as isize);
    }
    win::zet_tekst(o.knop_download, &t("KnopDownloaden"));
    win::zet_tekst(o.knop_vernieuwen, &t("KnopVernieuwen"));
    win::zet_tekst(o.knop_omhoog, &t("KnopOmhoog"));
    win::zet_tekst(o.upload_uitleg_label, &t("UploadUitlegNaam"));
    win::zet_tekst(o.upload_uitleg, &t("UploadUitleg").replace('\n', "\r\n"));
    win::zet_tekst(o.knop_kiezen, &t("KnopKiezen"));
    win::zet_tekst(o.gekozen_label, &t("GekozenNaam"));
    let gekozen = met(|s| s.upload_pad.clone()).flatten();
    win::zet_tekst(
        o.gekozen,
        &gekozen.and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())).unwrap_or_else(|| t("GeenBestand")),
    );
    win::zet_tekst(o.knop_upload, &t("KnopUploaden"));
    win::zet_tekst(o.voortgang_label, &t("VoortgangNaam"));
    win::zet_tekst(o.balk, &t("VoortgangNaam"));
    win::zet_tekst(o.melding_label, &t("MeldingNaam"));
    // De lijst opnieuw (datums en groottes in de nieuwe taal).
    let lijst = met(|s| s.lijst.clone()).unwrap_or_default();
    super::scherm_download::vul_lijst(&lijst);
    indeling();
}

/// Kleuren, letters en stijl op alle onderdelen zetten.
pub fn pas_thema_toe() {
    let o = onderdelen();
    let th = thema::huidig();
    let donker = th.weergave == Weergave::Donker && !th.hoog_contrast;
    let alle = [
        o.tab, o.lijst_label, o.lijst, o.knop_download, o.knop_omhoog, o.knop_vernieuwen, o.upload_uitleg_label, o.upload_uitleg, o.knop_kiezen,
        o.gekozen_label, o.gekozen, o.knop_upload, o.voortgang_label, o.balk, o.melding_label, o.melding,
    ];
    for h in alle {
        let letter = if o.knoppen().contains(&h) || h == o.tab { th.letter_vet } else { th.letter };
        snd(h, WM_SETFONT, letter.0 as usize, 1);
    }
    for h in [o.lijst, o.upload_uitleg, o.melding, o.gekozen] {
        thema::stijl_onderdeel(h, donker);
    }
    snd(o.lijst, LVM_SETBKCOLOR, 0, th.k.kaart.0 as isize);
    snd(o.lijst, LVM_SETTEXTBKCOLOR, 0, th.k.kaart.0 as isize);
    snd(o.lijst, LVM_SETTEXTCOLOR, 0, th.k.tekst.0 as isize);
    // Ruimere regels in de lijst: een onzichtbaar plaatje van 1 punt breed bepaalt de regelhoogte.
    unsafe {
        let hoogte = th.regel + th.regel / 2;
        let lijstje = ImageList_Create(1, hoogte, ILC_COLOR32, 1, 0);
        let oud = snd(o.lijst, LVM_SETIMAGELIST, LVSIL_SMALL as usize, lijstje.0 as isize);
        if oud != 0 {
            let _ = ImageList_Destroy(Some(HIMAGELIST(oud)));
        }
    }
    let h = hwnd();
    thema::titelbalk(h, donker);
    unsafe {
        let _ = InvalidateRect(Some(h), None, true);
        for c in alle {
            let _ = InvalidateRect(Some(c), None, true);
        }
    }
}

/// De maten op het scherm, uitgerekend uit de letterhoogte (alles groeit mee met grotere tekst).
pub struct Maten {
    pub marge: i32,
    pub kop: RECT,
    pub tab: RECT,
    pub pagina_kaart: RECT,
    pub status_kaart: RECT,
}

pub fn maten() -> Maten {
    let h = hwnd();
    let th = thema::huidig();
    let mut r = RECT::default();
    unsafe {
        let _ = GetClientRect(h, &mut r);
    }
    let m = th.regel * 2 / 3;
    let kop_h = th.regel * 2 + m;
    let tab_h = th.regel + m;
    let status_h = th.regel * 5 + m * 3;
    let kop = RECT { left: 0, top: 0, right: r.right, bottom: kop_h };
    let tab = RECT { left: m, top: kop_h + m, right: r.right - m, bottom: kop_h + m + tab_h };
    let status_kaart = RECT { left: m, top: r.bottom - m - status_h, right: r.right - m, bottom: r.bottom - m };
    let pagina_kaart = RECT { left: m, top: tab.bottom + m / 2, right: r.right - m, bottom: status_kaart.top - m };
    Maten { marge: m, kop, tab, pagina_kaart, status_kaart }
}

fn tekstbreedte(tekst: &str, letter: HFONT) -> i32 {
    let h = hwnd();
    unsafe {
        let dc = GetDC(Some(h));
        let oud = SelectObject(dc, letter.into());
        let v: Vec<u16> = zonder_amp(tekst).encode_utf16().collect();
        let mut s = SIZE::default();
        let _ = GetTextExtentPoint32W(dc, &v, &mut s);
        SelectObject(dc, oud);
        ReleaseDC(Some(h), dc);
        s.cx
    }
}

/// Zet alles op de goede plek.
pub fn indeling() {
    let o = onderdelen();
    if o.tab.is_invalid() {
        return;
    }
    let th = thema::huidig();
    let mt = maten();
    let m = mt.marge;
    let regel = th.regel;
    let knop_h = regel + regel * 2 / 3;
    let knop_b = |h: HWND| tekstbreedte(&win::lees_tekst(h), th.letter_vet) + regel * 2;

    // Tabbladen: even breed, breed genoeg voor de langste naam.
    let tab_b = ["TabDownloaden", "TabUploaden"].iter().map(|k| tekstbreedte(&t(k), th.letter_vet)).max().unwrap_or(100) + regel * 2;
    snd(o.tab, TCM_SETITEMSIZE, 0, ((tab_b & 0xffff) | ((mt.tab.bottom - mt.tab.top) << 16)) as isize);
    win::verplaats(o.tab, mt.tab.left, mt.tab.top, tab_b * 2 + m, mt.tab.bottom - mt.tab.top);

    // Pagina Downloaden.
    let p = &mt.pagina_kaart;
    let (x, b) = (p.left + m, p.right - p.left - 2 * m);
    win::verplaats(o.lijst_label, x, p.top + m, b, regel);
    let knop_y = p.bottom - m - knop_h;
    let lijst_y = p.top + m + regel + m / 3;
    win::verplaats(o.lijst, x, lijst_y, b, knop_y - m - lijst_y);
    let b1 = knop_b(o.knop_download);
    win::verplaats(o.knop_download, x, knop_y, b1, knop_h);
    let b2 = knop_b(o.knop_omhoog);
    win::verplaats(o.knop_omhoog, x + b1 + m, knop_y, b2, knop_h);
    win::verplaats(o.knop_vernieuwen, x + b1 + b2 + 2 * m, knop_y, knop_b(o.knop_vernieuwen), knop_h);
    let lb = b - regel;
    for (i, deel) in [40, 10, 14, 22, 14].iter().enumerate() {
        snd(o.lijst, LVM_SETCOLUMNWIDTH, i, (lb * deel / 100) as isize);
    }

    // Pagina Uploaden.
    let mut y = p.top + m;
    win::verplaats(o.upload_uitleg_label, x, y, b, regel);
    y += regel + m / 3;
    let uitleg_h = regel * 3 + m;
    win::verplaats(o.upload_uitleg, x, y, b, uitleg_h);
    y += uitleg_h + m;
    win::verplaats(o.knop_kiezen, x, y, knop_b(o.knop_kiezen), knop_h);
    y += knop_h + m;
    win::verplaats(o.gekozen_label, x, y, b, regel);
    y += regel + m / 3;
    win::verplaats(o.gekozen, x, y, b, regel + m / 2);
    y += regel + m / 2 + m;
    win::verplaats(o.knop_upload, x, y, knop_b(o.knop_upload), knop_h);

    // Voortgang en melding.
    let s = &mt.status_kaart;
    let (x, b) = (s.left + m, s.right - s.left - 2 * m);
    let mut y = s.top + m;
    win::verplaats(o.voortgang_label, x, y, b, regel);
    y += regel + m / 3;
    win::verplaats(o.balk, x, y, b, regel);
    y += regel + m / 2;
    win::verplaats(o.melding_label, x, y, b, regel);
    y += regel + m / 3;
    win::verplaats(o.melding, x, y, b, s.bottom - m - y);
    unsafe {
        let _ = InvalidateRect(Some(hwnd()), None, true);
    }
}

/// Wissel van tabblad. `melden` = de focus naar het eerste onderdeel van de pagina (bij Ctrl+Tab).
pub fn kies_pagina(n: usize, focus: bool) {
    let o = onderdelen();
    met(|s| s.pagina = n);
    snd(o.tab, TCM_SETCURSEL, n, 0);
    for h in o.download_pagina() {
        win::toon(h, n == 0);
    }
    for h in o.upload_pagina() {
        win::toon(h, n == 1);
    }
    unsafe {
        let _ = InvalidateRect(Some(o.tab), None, true);
    }
    if focus {
        unsafe {
            let _ = SetFocus(Some(o.tab));
        }
    }
}

/// Waar de focus begint: in de lijst (of op de tabbladen als de lijst leeg is).
pub fn focus_start() {
    let o = onderdelen();
    let p = met(|s| s.pagina).unwrap_or(0);
    let doel = if p == 0 && super::verkenner::aantal() > 0 { o.lijst } else { o.tab };
    unsafe {
        let _ = SetFocus(Some(doel));
    }
}

/// Enter: in de lijst op een map = die map openen, op een bestand = downloaden; op een knop = die knop.
pub fn enter(focus: HWND) -> bool {
    let o = onderdelen();
    if focus == o.lijst {
        super::scherm_download::enter_in_lijst();
        return true;
    }
    if o.knoppen().contains(&focus) {
        klik(focus);
        return true;
    }
    false
}

fn klik(knop: HWND) {
    let id = unsafe { GetDlgCtrlID(knop) } as u16;
    super::venster::opdracht(id);
}

/// Alt+letter: de knop met die onderstreepte letter (alleen op de zichtbare pagina).
pub fn klik_sneltoets(letter: char) -> bool {
    let o = onderdelen();
    for k in o.knoppen() {
        if !win::is_zichtbaar(k) {
            continue;
        }
        let tekst = win::lees_tekst(k).to_lowercase();
        if let Some(i) = tekst.find('&') {
            if tekst[i + 1..].starts_with(letter) {
                unsafe {
                    let _ = SetFocus(Some(k));
                }
                klik(k);
                return true;
            }
        }
    }
    false
}

/// Meldingen van de tabbladen en de lijst.
pub fn melding_van_onderdeel(kop: &NMHDR, lp: LPARAM) -> Option<LRESULT> {
    let o = onderdelen();
    if kop.hwndFrom == o.tab && kop.code == TCN_SELCHANGE {
        let n = snd(o.tab, TCM_GETCURSEL, 0, 0).max(0) as usize;
        kies_pagina(n, false);
        return Some(LRESULT(0));
    }
    // Dubbelklik = hetzelfde als Enter (map openen of bestand downloaden).
    if kop.hwndFrom == o.lijst && kop.code == NM_DBLCLK {
        super::scherm_download::enter_in_lijst();
        return Some(LRESULT(0));
    }
    let _ = lp;
    None
}

pub fn kies_taal(nieuw: Taal) {
    taal::zet(nieuw);
    let tijdelijk = met(|s| s.opties.taal.is_some()).unwrap_or(false);
    met(|s| {
        s.voorkeuren.taal = Some(nieuw.code().to_string());
        if !tijdelijk {
            bestandsbrug::voorkeuren::bewaar(&s.voorkeuren);
        }
    });
    bestandsbrug::log::info(&format!("Language: {}", nieuw.code()));
    super::menu::bouw(hwnd());
    zet_teksten();
    zeg_later(&t("TaalGekozen"), Belang::Belangrijk);
}

pub fn kies_weergave(w: Weergave) {
    met(|s| {
        s.weergave = w;
        s.voorkeuren.weergave = Some(w.code().to_string());
        if s.opties.weergave.is_none() {
            bestandsbrug::voorkeuren::bewaar(&s.voorkeuren);
        }
    });
    bestandsbrug::log::info(&format!("View: {}", w.code()));
    nieuw_thema(Some(w));
    super::menu::zet_vinkjes();
    zeg_later(&t(if w == Weergave::Licht { "WeergaveLicht" } else { "WeergaveDonker" }), Belang::Belangrijk);
}

/// Thema opnieuw maken: andere weergave, DPI, lettergrootte of hoog contrast.
pub fn nieuw_thema(w: Option<Weergave>) {
    let h = hwnd();
    if h.is_invalid() {
        return;
    }
    let weergave = w.or_else(|| met(|s| s.weergave)).unwrap_or(Weergave::Licht);
    let dpi = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(h) }.max(96);
    thema::maak(weergave, dpi);
    pas_thema_toe();
    indeling();
}

/// Is de focus nu in het hoofdvenster?
pub fn focus_hier() -> bool {
    let f = unsafe { GetFocus() };
    f == hwnd() || unsafe { IsChild(hwnd(), f) }.as_bool()
}
