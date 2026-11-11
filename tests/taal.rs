//! Tests voor de taalbestanden: nl en en hebben dezelfde sleutels, de app-naam komt uit het taalbestand
//! en staat nergens vast in de code, en de privacyverklaring is op elke plek dezelfde bron.

use bestandsbrug::taal::{self, Taal};

#[test]
fn nl_en_en_hebben_dezelfde_sleutels() {
    let nl = taal::sleutels(Taal::Nl);
    let en = taal::sleutels(Taal::En);
    let alleen_nl: Vec<_> = nl.iter().filter(|k| !en.contains(k)).collect();
    let alleen_en: Vec<_> = en.iter().filter(|k| !nl.contains(k)).collect();
    assert!(alleen_nl.is_empty() && alleen_en.is_empty(), "Alleen in nl: {alleen_nl:?}. Alleen in en: {alleen_en:?}");
}

#[test]
fn geen_lege_teksten() {
    for t in [Taal::Nl, Taal::En] {
        for k in taal::sleutels(t) {
            assert!(!taal::t_in(t, &k).trim().is_empty(), "Lege tekst: {k} in {t:?}");
        }
    }
}

#[test]
fn app_naam_hangt_af_van_de_taal() {
    assert_eq!(taal::t_in(Taal::Nl, "AppNaam"), "Bestandsbrug");
    assert_eq!(taal::t_in(Taal::En, "AppNaam"), "Filebridge");
    assert_eq!(taal::t_in(Taal::Nl, "AfsluitenVraag"), "Weet je zeker dat je Bestandsbrug wilt afsluiten?");
    assert_eq!(taal::t_in(Taal::En, "AfsluitenVraag"), "Are you sure you want to quit Filebridge?");
    assert_eq!(taal::t_in(Taal::Nl, "MenuOver"), "&Over Bestandsbrug");
    assert_eq!(taal::t_in(Taal::En, "MenuOver"), "&About Filebridge");
    assert!(taal::t_in(Taal::En, "UpdateMelding").starts_with("There is a new version of Filebridge."));
    assert!(taal::uitleg(Taal::En).contains("Filebridge") && !taal::uitleg(Taal::En).contains("{naam}"));
}

#[test]
fn elke_sleutel_in_de_code_bestaat() {
    // Zoek t("..."), f("...") en tm("...") in de code en kijk of die sleutel in beide talen staat.
    let nl = taal::sleutels(Taal::Nl);
    let mut missend = Vec::new();
    for pad in rs_bestanden("src") {
        let code = std::fs::read_to_string(&pad).unwrap();
        for patroon in ["t(\"", "f(\"", "tm(\""] {
            for (i, _) in code.match_indices(patroon) {
                let voor = code[..i].chars().last().unwrap_or(' ');
                if voor.is_alphanumeric() || voor == '_' {
                    continue;
                }
                let rest = &code[i + patroon.len()..];
                let sleutel: String = rest.chars().take_while(|c| c.is_alphanumeric()).collect();
                if !sleutel.is_empty() && rest[sleutel.len()..].starts_with('"') && !nl.contains(&sleutel) {
                    missend.push(format!("{}: {sleutel}", pad.display()));
                }
            }
        }
    }
    assert!(missend.is_empty(), "Sleutels die niet in de taalbestanden staan: {missend:?}");
}

#[test]
fn app_naam_staat_niet_vast_in_de_code() {
    // In tekst tussen aanhalingstekens mag "Filebridge" nooit staan, en "Bestandsbrug" alleen in instellingen.rs
    // (de technische naam voor mappen en bestanden).
    let mut fout = Vec::new();
    for pad in rs_bestanden("src") {
        let naam = pad.file_name().unwrap().to_string_lossy().to_string();
        let code = std::fs::read_to_string(&pad).unwrap();
        for (nr, regel) in code.lines().enumerate() {
            let r = regel.trim_start();
            if r.starts_with("//") {
                continue;
            }
            for tekst in tekst_tussen_aanhalingstekens(regel) {
                if tekst.contains("Filebridge") || (tekst.contains("Bestandsbrug") && naam != "instellingen.rs") {
                    fout.push(format!("{}:{}: {tekst}", pad.display(), nr + 1));
                }
            }
        }
    }
    assert!(fout.is_empty(), "App-naam vast in de code: {fout:?}");
}

#[test]
fn privacyverklaring_is_compleet() {
    let nl = taal::privacy(Taal::Nl);
    let en = taal::privacy(Taal::En);
    assert!(nl.starts_with("PRIVACYVERKLARING BESTANDSBRUG"));
    assert!(en.starts_with("PRIVACY STATEMENT FILEBRIDGE"));
    assert!(nl.trim_end().ends_with("Bovenaan staat de datum."));
    assert!(en.trim_end().ends_with("The date is at the top."));
    for n in 1..=10 {
        assert!(nl.contains(&format!("\n{n}. ")), "Kopje {n} mist in nl");
        assert!(en.contains(&format!("\n{n}. ")), "Kopje {n} mist in en");
    }
}

#[test]
fn zonder_amp_haalt_sneltoetsteken_weg() {
    assert_eq!(taal::zonder_amp("&Downloaden"), "Downloaden");
    assert_eq!(taal::zonder_amp("Tom && Jerry"), "Tom & Jerry");
}

pub fn rs_bestanden(map: &str) -> Vec<std::path::PathBuf> {
    let mut uit = Vec::new();
    let mut stapel = vec![std::path::PathBuf::from(map)];
    while let Some(m) = stapel.pop() {
        for e in std::fs::read_dir(&m).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stapel.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                uit.push(p);
            }
        }
    }
    uit
}

fn tekst_tussen_aanhalingstekens(regel: &str) -> Vec<String> {
    let mut uit = Vec::new();
    let mut binnen = false;
    let mut huidig = String::new();
    let mut vorige = ' ';
    for c in regel.chars() {
        if c == '"' && vorige != '\\' {
            if binnen {
                uit.push(std::mem::take(&mut huidig));
            }
            binnen = !binnen;
        } else if binnen {
            huidig.push(c);
        }
        vorige = c;
    }
    uit
}
