//! Welke programma's van Jaap staan al op de computer, en is er op pCloud een nieuwere versie?
//! Het lezen van Windows (het register) staat in src/scherm/programmas.rs; hier staat alleen de logica
//! (vergelijken van namen en versies), zodat de tests ook zonder Windows draaien.
//!
//! Een map in de hoofdmap van pCloud (zoals "Paper Pilot") hoort bij een geïnstalleerd programma als de
//! naam in Windows (DisplayName) gelijk is aan de mapnaam of ermee begint ("Paper Pilot 0.0.0.16").
//! Het nieuwste installatiebestand in die map is het .exe-bestand met "install" of "setup" in de naam en
//! het hoogste versienummer in de naam.

use std::cmp::Ordering;

use crate::pcloud::lijst::Bestand;

/// Een geïnstalleerd programma, zoals Windows het kent (Uninstall-sleutel in het register).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Geinstalleerd {
    /// DisplayName, bijvoorbeeld "Paper Pilot" of "Paper Pilot 0.0.0.16".
    pub naam: String,
    /// DisplayVersion (kan leeg zijn).
    pub versie: String,
    /// InstallLocation (kan leeg zijn).
    pub map: String,
    /// DisplayIcon, bijvoorbeeld "C:\...\Paper Pilot.exe,0" (kan leeg zijn).
    pub icoon: String,
}

impl Geinstalleerd {
    /// De versie die op de computer staat: DisplayVersion, anders het nummer in de naam.
    pub fn bekende_versie(&self) -> Option<String> {
        let v = self.versie.trim();
        if versie_delen(v).is_some() {
            return Some(v.to_string());
        }
        versie_uit_naam(&self.naam)
    }
}

/// Wat de lijst over een programmamap weet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stand {
    /// De geïnstalleerde versie (None = onbekend).
    pub versie: Option<String>,
    /// Een nieuwer installatiebestand: (nummer in de lijst, versie).
    pub nieuwer: Option<(usize, String)>,
}

/// "0.0.0.16" -> [0, 0, 0, 16]. None als het geen versienummer is.
pub fn versie_delen(v: &str) -> Option<Vec<u64>> {
    let v = v.trim();
    if v.is_empty() {
        return None;
    }
    v.split('.').map(|d| if !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()) { d.parse().ok() } else { None }).collect()
}

/// Vergelijkt twee versies per deel, als getal. Een ontbrekend deel telt als 0 (1.2 = 1.2.0).
pub fn vergelijk(a: &str, b: &str) -> Ordering {
    let (a, b) = (versie_delen(a).unwrap_or_default(), versie_delen(b).unwrap_or_default());
    for i in 0..a.len().max(b.len()) {
        let o = a.get(i).copied().unwrap_or(0).cmp(&b.get(i).copied().unwrap_or(0));
        if o != Ordering::Equal {
            return o;
        }
    }
    Ordering::Equal
}

/// Het versienummer in een naam: het laatste stuk met cijfers en punten, met minstens één punt.
/// "Paper Pilot 0.0.0.16 installatie.exe" -> "0.0.0.16". "Bestandsbrug-Setup.exe" -> None.
pub fn versie_uit_naam(naam: &str) -> Option<String> {
    let mut gevonden = None;
    let mut huidig = String::new();
    for c in naam.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() || c == '.' {
            huidig.push(c);
            continue;
        }
        let stuk = huidig.trim_matches('.');
        if stuk.contains('.') && versie_delen(stuk).is_some() {
            gevonden = Some(stuk.to_string());
        }
        huidig.clear();
    }
    gevonden
}

/// Is dit een installatiebestand? Een .exe met "install" (ook "installatie") of "setup" in de naam.
pub fn is_installatie(naam: &str) -> bool {
    let n = naam.to_lowercase();
    n.ends_with(".exe") && (n.contains("install") || n.contains("setup"))
}

/// Het nieuwste installatiebestand direct in de map `map`: (nummer in de lijst, versie of None).
/// Bestanden met een versie gaan voor; zonder versie wordt het eerste installatiebestand genomen.
pub fn nieuwste_installatie(lijst: &[Bestand], map: &str) -> Option<(usize, Option<String>)> {
    let kandidaten: Vec<usize> = (0..lijst.len()).filter(|&i| lijst[i].map == map && is_installatie(&lijst[i].naam)).collect();
    let met_versie = kandidaten
        .iter()
        .filter_map(|&i| versie_uit_naam(&lijst[i].naam).map(|v| (i, v)))
        .max_by(|a, b| vergelijk(&a.1, &b.1));
    match met_versie {
        Some((i, v)) => Some((i, Some(v))),
        None => kandidaten.first().map(|&i| (i, None)),
    }
}

/// Het geïnstalleerde programma dat bij de map `map` hoort (niet hoofdlettergevoelig).
/// De naam in Windows moet gelijk zijn aan de mapnaam, of ermee beginnen gevolgd door een versie of iets tussen
/// haakjes ("Paper Pilot 0.0.0.16", "Paper Pilot versie 2", "Paper Pilot (64-bit)"). Zo hoort "Paper Pilotje"
/// niet bij de map "Paper Pilot", en "Paper Pilot" niet bij een map "Paper".
pub fn zoek_programma<'a>(map: &str, programmas: &'a [Geinstalleerd]) -> Option<&'a Geinstalleerd> {
    let m = map.trim().to_lowercase();
    if m.is_empty() {
        return None;
    }
    let past = |p: &Geinstalleerd| {
        let n = p.naam.trim().to_lowercase();
        n == m
            || n.strip_prefix(&m).is_some_and(|rest| {
                let na = rest.trim_start_matches([' ', '-', '_']);
                rest.starts_with([' ', '-', '_', '(']) && (na.starts_with(|c: char| c.is_ascii_digit() || c == '(' || c == 'v'))
            })
    };
    programmas.iter().find(|p| p.naam.trim().to_lowercase() == m).or_else(|| programmas.iter().find(|p| past(p)))
}

/// De stand van een programmamap: None als het programma niet op de computer staat.
pub fn stand(map: &str, lijst: &[Bestand], programmas: &[Geinstalleerd]) -> Option<Stand> {
    let p = zoek_programma(map, programmas)?;
    let versie = p.bekende_versie();
    let nieuwer = match (&versie, nieuwste_installatie(lijst, map)) {
        (Some(oud), Some((i, Some(nieuw)))) if vergelijk(&nieuw, oud) == Ordering::Greater => Some((i, nieuw)),
        _ => None,
    };
    Some(Stand { versie, nieuwer })
}

/// DisplayIcon schoonmaken: aanhalingstekens en ",0" eraf. Alleen een .exe telt (geen .ico).
pub fn exe_uit_icoon(icoon: &str) -> Option<String> {
    let mut s = icoon.trim().to_string();
    if let Some((voor, na)) = s.rsplit_once(',') {
        if na.trim().trim_start_matches('-').chars().all(|c| c.is_ascii_digit()) {
            s = voor.to_string();
        }
    }
    let s = s.trim().trim_matches('"').trim().to_string();
    let n = s.to_lowercase();
    let bestand = n.rsplit(['\\', '/']).next().unwrap_or("");
    (n.ends_with(".exe") && !bestand.starts_with("unins")).then_some(s)
}

/// Kiest uit de .exe-bestanden in de installatiemap het programma zelf. Het verwijderprogramma (unins000.exe)
/// telt niet mee. Eerst een naam die past bij het programma; anders het enige .exe-bestand.
pub fn kies_exe(namen: &[String], programma: &str) -> Option<String> {
    let exes: Vec<&String> = namen
        .iter()
        .filter(|n| {
            let l = n.to_lowercase();
            l.ends_with(".exe") && !l.starts_with("unins")
        })
        .collect();
    let gewoon = |s: &str| s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
    let doel = gewoon(programma);
    if let Some(n) = exes.iter().find(|n| gewoon(n.trim_end_matches(".exe").trim_end_matches(".EXE")) == doel) {
        return Some((*n).clone());
    }
    if exes.len() == 1 {
        return Some(exes[0].clone());
    }
    exes.iter().find(|n| !doel.is_empty() && gewoon(n).starts_with(&doel)).map(|n| (*n).clone())
}
