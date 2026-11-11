//! Tests voor het updaten, met een nep-versie (hoger nummer) op een eigen kleine webserver.
//! Getest: versie.txt lezen, versies vergelijken, "Later", los ophalen, de zip-reserve, en de SHA-256-controle.

mod hulp;

use std::io::Write;
use std::time::Duration;

use bestandsbrug::fout::Fout;
use bestandsbrug::overdracht::Voortgang;
use bestandsbrug::update::{self, Bron};

fn versie_txt(sha: &str) -> String {
    format!("versie = 9.1.0\r\nbestand = Bestandsbrug-Setup.exe\r\nsha256 = {sha}\r\nnieuw_nl = Iets nieuws.\r\nnieuw_nl = Nog iets.\r\nnieuw_en = Something new.\r\n")
}

fn sha_van(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn versies_vergelijken() {
    assert!(update::is_nieuwer("1.1.0", "1.0.0"));
    assert!(update::is_nieuwer("1.10.0", "1.9.0"));
    assert!(update::is_nieuwer("2", "1.9.9"));
    assert!(!update::is_nieuwer("1.0.0", "1.0.0"));
    assert!(!update::is_nieuwer("1.0", "1.0.0"));
    assert!(!update::is_nieuwer("onzin", "1.0.0"));
}

#[test]
fn later_onthouden() {
    assert!(update::moet_automatisch_melden("99.0.0", None));
    assert!(!update::moet_automatisch_melden("99.0.0", Some("99.0.0")));
    assert!(update::moet_automatisch_melden("99.0.1", Some("99.0.0")));
    assert!(!update::moet_automatisch_melden(bestandsbrug::instellingen::VERSIE, None));
}

#[test]
fn versie_txt_lezen() {
    let sha = "a".repeat(64);
    let i = update::lees_versie_txt(&versie_txt(&sha)).unwrap();
    assert_eq!(i.versie, "9.1.0");
    assert_eq!(i.nieuw_nl, "Iets nieuws.\nNog iets.");
    assert_eq!(i.nieuw(bestandsbrug::taal::Taal::En), "Something new.");
    assert!(update::lees_versie_txt("<html>geen versie</html>").is_none());
    assert!(update::lees_versie_txt("versie = 1.0.0\nsha256 = kort").is_none());
    // Geen map in de bestandsnaam (veiligheid).
    assert!(update::lees_versie_txt(&format!("versie = 1.0.0\nbestand = ..\\boos.exe\nsha256 = {sha}")).is_none());
}

#[test]
fn adres_van_de_dropbox_link() {
    let link = bestandsbrug::instellingen::UPDATE_LINK;
    let a = update::adres(link, Some("versie.txt"));
    assert!(a.starts_with("https://www.dropbox.com/scl/fo/"));
    assert!(a.contains("/versie.txt?rlkey=") && a.ends_with("&dl=1"));
    assert!(update::adres(link, None).ends_with("&dl=1"));
}

// Eén test voor beide manieren, na elkaar: ze delen de gegevensmap (BESTANDSBRUG_MAP).
#[test]
fn update_los_en_via_zip() {
    std::env::set_var("BESTANDSBRUG_MAP", hulp::eigen_map("update"));
    update_los_met_goede_en_foute_sha();
    update_via_zip_van_de_hele_map();
}

fn update_los_met_goede_en_foute_sha() {
    let (adres, routes) = hulp::start_server();
    let setup = b"MZ nep-installatiebestand voor de test".repeat(1000);
    routes.lock().unwrap().insert("/scl/fo/abc/def/versie.txt".into(), (200, versie_txt(&sha_van(&setup)).into_bytes()));
    routes.lock().unwrap().insert("/scl/fo/abc/def/Bestandsbrug-Setup.exe".into(), (200, setup.clone()));
    let link = format!("{adres}/scl/fo/abc/def?rlkey=xyz&st=1&dl=0");

    let (info, bron) = update::haal_versie_info(&link, Duration::from_secs(5)).unwrap();
    assert_eq!(info.versie, "9.1.0");
    assert!(matches!(bron, Bron::Los));
    assert!(update::moet_automatisch_melden(&info.versie, None));
    let v = Voortgang::new();
    let pad = update::haal_update(&link, &info, &bron, &v).unwrap();
    assert_eq!(std::fs::read(&pad).unwrap(), setup);
    assert_eq!(v.gedaan(), setup.len() as u64);

    // Een kapot bestand: de SHA-256 klopt niet, dan wordt niets geïnstalleerd.
    let mut kapot = setup.clone();
    kapot[10] ^= 0xff;
    routes.lock().unwrap().insert("/scl/fo/abc/def/Bestandsbrug-Setup.exe".into(), (200, kapot));
    let _ = std::fs::remove_file(&pad);
    let r = update::haal_update(&link, &info, &bron, &Voortgang::new());
    assert!(matches!(r, Err(Fout::Beschadigd)), "{r:?}");
    assert!(!pad.exists());
}

fn update_via_zip_van_de_hele_map() {
    let (adres, routes) = hulp::start_server();
    let setup = b"MZ nog een nep-installatiebestand".repeat(500);
    // Dropbox geeft voor een los bestand soms een webpagina: dan moet de zip-reserve werken.
    routes.lock().unwrap().insert("/scl/fo/abc/def/versie.txt".into(), (200, b"<html>Dropbox</html>".to_vec()));
    let mut zip = Vec::new();
    {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut zip));
        let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        w.start_file("versie.txt", opt).unwrap();
        w.write_all(versie_txt(&sha_van(&setup)).as_bytes()).unwrap();
        w.start_file("Bestandsbrug-Setup.exe", opt).unwrap();
        w.write_all(&setup).unwrap();
        w.finish().unwrap();
    }
    routes.lock().unwrap().insert("/scl/fo/abc/def".into(), (200, zip));
    let link = format!("{adres}/scl/fo/abc/def?rlkey=xyz&dl=0");
    let (info, bron) = update::haal_versie_info(&link, Duration::from_secs(5)).unwrap();
    assert!(matches!(bron, Bron::Zip(_)));
    let pad = update::haal_update(&link, &info, &bron, &Voortgang::new()).unwrap();
    assert_eq!(std::fs::read(&pad).unwrap(), setup);
}

#[test]
fn geen_internet_geeft_een_duidelijke_fout() {
    // Poort 9 op de eigen computer: daar luistert niets.
    let r = update::haal_versie_info("http://127.0.0.1:9/scl/fo/a/b?rlkey=x&dl=0", Duration::from_secs(3));
    match r {
        Err(f) => assert!(f.is_geen_internet(), "{f:?}"),
        Ok(_) => panic!("zou moeten mislukken"),
    }
}
