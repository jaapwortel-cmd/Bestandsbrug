//! Vragen en meldingen in een eigen venster (TaskDialog van Windows): afsluiten, Escape tijdens een
//! overdracht, updates en "Over". De schermlezer leest de vraag en de knoppen voor. Kiezen en bevestigen
//! kan met de Spatie en met Enter. De focus staat eerst op de knop die we als standaard opgeven.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Controls::*;

use bestandsbrug::taal::{self as crate_taal, t, zonder_amp};

use super::win::{pc, ws};

/// Een knop in een vraag: het nummer dat terugkomt en de tekst.
pub struct Knop {
    pub id: i32,
    pub tekst: String,
}

pub fn knop(id: i32, tekst: impl Into<String>) -> Knop {
    Knop { id, tekst: tekst.into() }
}

/// Stelt een vraag. `standaard` = de knop met de focus. `bij_escape` = wat Escape betekent (None: Escape doet niets).
pub fn vraag(ouder: HWND, titel: &str, tekst: &str, uitleg: &str, knoppen: &[Knop], standaard: i32, bij_escape: Option<i32>) -> i32 {
    super::begin_vraag();
    let titel_w = ws(titel);
    let tekst_w = ws(tekst);
    let uitleg_w = ws(uitleg);
    let teksten: Vec<Vec<u16>> = knoppen.iter().map(|k| ws(&k.tekst)).collect();
    let tdk: Vec<TASKDIALOG_BUTTON> = knoppen.iter().zip(&teksten).map(|(k, w)| TASKDIALOG_BUTTON { nButtonID: k.id, pszButtonText: pc(w) }).collect();
    let mut vlaggen = TDF_POSITION_RELATIVE_TO_WINDOW;
    if bij_escape.is_some() {
        vlaggen |= TDF_ALLOW_DIALOG_CANCELLATION;
    }
    let config = TASKDIALOGCONFIG {
        cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
        hwndParent: ouder,
        dwFlags: vlaggen,
        pszWindowTitle: pc(&titel_w),
        pszMainInstruction: pc(&tekst_w),
        pszContent: if uitleg.is_empty() { windows::core::PCWSTR::null() } else { pc(&uitleg_w) },
        cButtons: tdk.len() as u32,
        pButtons: tdk.as_ptr(),
        nDefaultButton: standaard,
        ..Default::default()
    };
    let mut gekozen = 0i32;
    let gelukt = unsafe { TaskDialogIndirect(&config, Some(&mut gekozen), None, None) };
    super::einde_vraag();
    if gelukt.is_err() {
        return bij_escape.unwrap_or(standaard);
    }
    // Escape (of het sluitkruisje) geeft IDCANCEL = 2.
    if gekozen == 2 && !knoppen.iter().any(|k| k.id == 2) {
        return bij_escape.unwrap_or(standaard);
    }
    gekozen
}

pub const JA: i32 = 100;
pub const NEE: i32 = 101;

/// Een vraag met Ja en Nee. Escape is altijd Nee.
pub fn ja_nee(ouder: HWND, tekst: &str, standaard_ja: bool) -> bool {
    let knoppen = [knop(JA, t("Ja")), knop(NEE, t("Nee"))];
    vraag(ouder, &crate_taal::app_naam(), tekst, "", &knoppen, if standaard_ja { JA } else { NEE }, Some(NEE)) == JA
}

/// Een melding met alleen een knop Sluiten. Escape en Enter sluiten.
pub fn info(ouder: HWND, tekst: &str) {
    let knoppen = [knop(JA, zonder_amp(&t("KnopSluiten")))];
    vraag(ouder, &crate_taal::app_naam(), tekst, "", &knoppen, JA, Some(JA));
}

pub const DOORGAAN: i32 = 110;
pub const LATER: i32 = 111;
pub const ANNULEREN: i32 = 112;

/// De vraag bij Escape (of afsluiten) tijdens een overdracht. Bij uploads en updates geen "Volgende keer verdergaan";
/// dan staat `uitleg` erbij (waarom niet).
pub fn vraag_overdracht(ouder: HWND, tekst: &str, kan_later: bool, uitleg: &str) -> i32 {
    let mut knoppen = vec![knop(DOORGAAN, t("EscDoorgaan"))];
    if kan_later {
        knoppen.push(knop(LATER, t("EscLater")));
    }
    knoppen.push(knop(ANNULEREN, t("EscAnnuleren")));
    vraag(ouder, &crate_taal::app_naam(), tekst, if kan_later { "" } else { uitleg }, &knoppen, DOORGAAN, Some(DOORGAAN))
}
