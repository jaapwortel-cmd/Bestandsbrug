//! Eén soort fout voor de hele app, met een tekst in gewone taal voor de gebruiker
//! en een korte Engelse tekst voor het logbestand.

use crate::taal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// Gestopt, en volgende keer verdergaan (alleen bij downloaden).
    Pauze,
    /// Alles geannuleerd; het halve bestand is weg.
    Annuleer,
}

#[derive(Debug)]
pub enum Fout {
    GeenInternet(String),
    TeLang,
    /// pCloud gaf een foutcode met een bericht.
    Pcloud(i64, String),
    LinkWeg,
    UploadVol,
    TeGroot { mag_nog: u64, bestand: u64 },
    Opslaan(String),
    Lezen(String),
    Onvolledig,
    Beschadigd,
    /// Een map zonder bestanden.
    LegeMap,
    Gestopt(Stop),
    Anders(String),
}

pub type Uitkomst<T> = Result<T, Fout>;

impl Fout {
    /// Tekst voor de gebruiker, in de gekozen taal.
    pub fn tekst(&self) -> String {
        match self {
            Fout::GeenInternet(_) => taal::t("FoutGeenInternet"),
            Fout::TeLang => taal::t("FoutTeLang"),
            Fout::Pcloud(_, m) => taal::f("FoutPcloud", &[m]),
            Fout::LinkWeg => taal::t("FoutLinkWeg"),
            Fout::UploadVol => taal::t("FoutUploadVol"),
            Fout::TeGroot { mag_nog, bestand } => taal::f(
                "FoutTeGroot",
                &[&crate::opmaak::grootte(*mag_nog), &crate::opmaak::grootte(*bestand)],
            ),
            Fout::Opslaan(m) => taal::f("FoutOpslaan", &[m]),
            Fout::Lezen(m) => taal::f("FoutLezen", &[m]),
            Fout::Onvolledig => taal::t("FoutOnvolledig"),
            Fout::Beschadigd => taal::t("UpdateBeschadigd"),
            Fout::LegeMap => taal::t("MapLeeg"),
            Fout::Gestopt(_) => String::new(),
            Fout::Anders(m) => taal::f("FoutOnbekend", &[m]),
        }
    }

    /// Korte Engelse tekst voor het logbestand. Geen paden, geen persoonlijke gegevens.
    pub fn log_tekst(&self) -> String {
        match self {
            Fout::GeenInternet(d) => format!("no internet connection ({d})"),
            Fout::TeLang => "timeout".into(),
            Fout::Pcloud(c, m) => format!("pCloud error {c}: {m}"),
            Fout::LinkWeg => "link deleted or expired".into(),
            Fout::UploadVol => "upload link is full".into(),
            Fout::TeGroot { mag_nog, bestand } => format!("file too big ({bestand} bytes, {mag_nog} left)"),
            Fout::Opslaan(m) => format!("cannot save: {m}"),
            Fout::Lezen(m) => format!("cannot read: {m}"),
            Fout::Onvolledig => "download incomplete".into(),
            Fout::Beschadigd => "SHA-256 does not match".into(),
            Fout::LegeMap => "folder is empty".into(),
            Fout::Gestopt(Stop::Pauze) => "paused".into(),
            Fout::Gestopt(Stop::Annuleer) => "cancelled".into(),
            Fout::Anders(m) => m.clone(),
        }
    }

    pub fn is_geen_internet(&self) -> bool {
        matches!(self, Fout::GeenInternet(_) | Fout::TeLang)
    }
}

/// Een fout van de schijf, zonder het pad (daar kan de gebruikersnaam in staan).
pub fn io_tekst(e: &std::io::Error) -> String {
    use std::io::ErrorKind::*;
    match e.kind() {
        NotFound => "not found".into(),
        PermissionDenied => "no permission".into(),
        StorageFull => "disk full".into(),
        _ => format!("{}", e.kind()),
    }
}
