//! Tests voor het herkennen van geïnstalleerde programma's en het vergelijken van versies (zonder Windows).

use std::cmp::Ordering;

use bestandsbrug::pcloud::lijst::Bestand;
use bestandsbrug::programmas::*;

fn b(map: &str, naam: &str) -> Bestand {
    Bestand { fileid: 1, naam: naam.into(), grootte: 1, gewijzigd: String::new(), hash: 0, map: map.into(), uitleg: String::new() }
}

fn p(naam: &str, versie: &str) -> Geinstalleerd {
    Geinstalleerd { naam: naam.into(), versie: versie.into(), ..Default::default() }
}

fn lijst() -> Vec<Bestand> {
    vec![
        b("Bestandsbrug", "Bestandsbrug-Setup.exe"),
        b("Bestandsbrug", "versie.txt"),
        b("Computerhulp Maatje", "Computerhulp Maatje 0.0.0.11 installatie.exe"),
        b("Computerhulp Maatje", "Computerhulp Maatje 0.0.0.11.zip"),
        b("Paper Pilot", "Paper Pilot 0.0.0.16 installatie.exe"),
        b("Paper Pilot", "Paper Pilot 0.0.0.9 installatie.zip"),
        b("Verhalenschrijver", "Verhalenschrijver 0.2.0.4 installatie.exe"),
        b("Verhalenschrijver", "Verhalenschrijver 0.2.0.5 installatie.exe"),
        b("Jaaps Audio Editor", "Jaaps Audio Editor Installatie 3.0.4.0.exe"),
    ]
}

#[test]
fn versie_uit_de_naam() {
    assert_eq!(versie_uit_naam("Paper Pilot 0.0.0.16 installatie.exe").as_deref(), Some("0.0.0.16"));
    assert_eq!(versie_uit_naam("Jaaps Audio Editor Installatie 3.0.4.0.exe").as_deref(), Some("3.0.4.0"));
    assert_eq!(versie_uit_naam("Bestandsbrug-Setup.exe"), None);
    assert_eq!(versie_uit_naam("Paper Pilot 0.0.0.16"), Some("0.0.0.16".into()));
    assert_eq!(versie_uit_naam("App 2 setup.exe"), None, "een los getal is geen versie");
}

#[test]
fn versies_als_getal_vergelijken() {
    assert_eq!(vergelijk("0.0.0.16", "0.0.0.9"), Ordering::Greater);
    assert_eq!(vergelijk("0.2.0.5", "0.2.0.4"), Ordering::Greater);
    assert_eq!(vergelijk("1.2", "1.2.0"), Ordering::Equal);
    assert_eq!(vergelijk("1.10", "1.9.9"), Ordering::Greater);
}

#[test]
fn installatiebestanden_herkennen() {
    assert!(is_installatie("Paper Pilot 0.0.0.16 installatie.exe"));
    assert!(is_installatie("Bestandsbrug-Setup.exe"));
    assert!(is_installatie("Jaaps Audio Editor Installatie 3.0.4.0.exe"));
    assert!(!is_installatie("Paper Pilot 0.0.0.9 installatie.zip"));
    assert!(!is_installatie("Paper Pilot.exe"));
}

#[test]
fn nieuwste_installatie_kiezen() {
    let l = lijst();
    let (i, v) = nieuwste_installatie(&l, "Verhalenschrijver").unwrap();
    assert_eq!(l[i].naam, "Verhalenschrijver 0.2.0.5 installatie.exe");
    assert_eq!(v.as_deref(), Some("0.2.0.5"));
    let (i, v) = nieuwste_installatie(&l, "Paper Pilot").unwrap();
    assert_eq!(l[i].naam, "Paper Pilot 0.0.0.16 installatie.exe", "de zip telt niet");
    assert_eq!(v.as_deref(), Some("0.0.0.16"));
    let (i, v) = nieuwste_installatie(&l, "Bestandsbrug").unwrap();
    assert_eq!(l[i].naam, "Bestandsbrug-Setup.exe");
    assert_eq!(v, None);
    assert_eq!(nieuwste_installatie(&l, "Bestaat niet"), None);
}

#[test]
fn programma_bij_map_zoeken() {
    let ps = vec![p("Paper Pilot 0.0.0.16", ""), p("computerhulp maatje", "0.0.0.11"), p("Paper Pilotje", "9.9")];
    assert_eq!(zoek_programma("Paper Pilot", &ps).unwrap().naam, "Paper Pilot 0.0.0.16");
    assert_eq!(zoek_programma("Computerhulp Maatje", &ps).unwrap().versie, "0.0.0.11");
    assert!(zoek_programma("Paper", &ps).is_none(), "alleen een heel woord telt");
    assert!(zoek_programma("Verhalenschrijver", &ps).is_none());
    assert!(zoek_programma("", &ps).is_none());
    assert!(zoek_programma("Paper Pilot", &[p("Paper Pilot (64-bit)", "1.0")]).is_some());
}

#[test]
fn stand_van_een_map() {
    let l = lijst();
    let ps = vec![
        p("Paper Pilot", "0.0.0.16"),
        p("Verhalenschrijver", "0.2.0.4"),
        p("Computerhulp Maatje 0.0.0.11", ""),
        p("Bestandsbrug", "1.0.1"),
    ];
    assert_eq!(stand("Paper Pilot", &l, &ps), Some(Stand { versie: Some("0.0.0.16".into()), nieuwer: None }));
    let s = stand("Verhalenschrijver", &l, &ps).unwrap();
    assert_eq!(s.versie.as_deref(), Some("0.2.0.4"));
    let (i, v) = s.nieuwer.unwrap();
    assert_eq!((l[i].naam.as_str(), v.as_str()), ("Verhalenschrijver 0.2.0.5 installatie.exe", "0.2.0.5"));
    assert_eq!(stand("Computerhulp Maatje", &l, &ps).unwrap().versie.as_deref(), Some("0.0.0.11"), "versie uit DisplayName");
    assert_eq!(stand("Bestandsbrug", &l, &ps).unwrap().nieuwer, None, "Setup zonder versie: niet bijwerken");
    assert_eq!(stand("Jaaps Audio Editor", &l, &ps), None, "niet geïnstalleerd");
    // Onbekende geïnstalleerde versie: nooit "nieuwer".
    assert_eq!(stand("Verhalenschrijver", &l, &[p("Verhalenschrijver", "")]).unwrap(), Stand { versie: None, nieuwer: None });
}

#[test]
fn programma_starten_pad() {
    assert_eq!(exe_uit_icoon(r#""C:\Programs\Paper Pilot\Paper Pilot.exe",0"#).as_deref(), Some(r"C:\Programs\Paper Pilot\Paper Pilot.exe"));
    assert_eq!(exe_uit_icoon(r"C:\P\App.exe").as_deref(), Some(r"C:\P\App.exe"));
    assert_eq!(exe_uit_icoon(r"C:\P\app.ico"), None);
    assert_eq!(exe_uit_icoon(r"C:\P\unins000.exe,0"), None);
    assert_eq!(exe_uit_icoon(""), None);
    let namen: Vec<String> = ["unins000.exe", "Paper Pilot.exe", "helper.exe"].iter().map(|s| s.to_string()).collect();
    assert_eq!(kies_exe(&namen, "Paper Pilot").as_deref(), Some("Paper Pilot.exe"));
    let namen: Vec<String> = ["unins000.exe", "PaperPilot.exe", "readme.txt"].iter().map(|s| s.to_string()).collect();
    assert_eq!(kies_exe(&namen, "Paper Pilot").as_deref(), Some("PaperPilot.exe"));
    let namen: Vec<String> = ["unins000.exe", "a.exe", "b.exe"].iter().map(|s| s.to_string()).collect();
    assert_eq!(kies_exe(&namen, "Paper Pilot"), None);
}
