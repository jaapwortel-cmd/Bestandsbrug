//! Tests tegen de ECHTE pCloud-links. Ze staan uit (ignore), want ze hebben internet nodig.
//! Draaien: cargo test --test echt -- --ignored --nocapture
//! De upload-test stuurt een klein tekstbestand naar Jaaps map Uploads; die draait alleen met
//! BESTANDSBRUG_ECHTE_UPLOAD=1, zodat de map niet volloopt.

mod hulp;

use std::time::Instant;

use bestandsbrug::overdracht::Voortgang;
use bestandsbrug::pcloud::{download, lijst, upload};

#[test]
#[ignore]
fn echte_lijst_en_download() {
    let map = hulp::eigen_map("echt");
    std::env::set_var("BESTANDSBRUG_MAP", &map);
    let t = Instant::now();
    let l = lijst::haal_lijst().expect("lijst ophalen");
    println!("Lijst opgehaald in {} ms: {} bestanden", t.elapsed().as_millis(), l.len());
    for b in &l {
        println!("  {} ({} bytes, {})", b.naam, b.grootte, b.gewijzigd);
    }
    let b = l.iter().find(|b| b.naam.to_lowercase().contains("paper pilot")).or(l.first()).expect("een bestand");
    let doel = map.join(&b.naam);
    let v = Voortgang::new();
    let t = Instant::now();
    download::download(b, &doel, &v).expect("downloaden");
    let s = t.elapsed().as_secs_f64();
    let lengte = std::fs::metadata(&doel).unwrap().len();
    assert_eq!(lengte, b.grootte);
    println!("Download: {} bytes in {:.2} s = {:.1} MB/s", lengte, s, lengte as f64 / s / 1_048_576.0);
}

#[test]
#[ignore]
fn echte_upload() {
    if std::env::var("BESTANDSBRUG_ECHTE_UPLOAD").ok().as_deref() != Some("1") {
        println!("Overgeslagen: zet BESTANDSBRUG_ECHTE_UPLOAD=1");
        return;
    }
    let map = hulp::eigen_map("echt-upload");
    let pad = map.join("Bestandsbrug-test-upload.txt");
    std::fs::write(&pad, "Testbestand van Bestandsbrug. Dit bestand mag weg.\r\n".repeat(20_000)).unwrap();
    let v = Voortgang::new();
    let t = Instant::now();
    upload::upload(&pad, &v).expect("uploaden");
    let s = t.elapsed().as_secs_f64();
    let n = std::fs::metadata(&pad).unwrap().len();
    println!("Upload: {} bytes in {:.2} s = {:.1} MB/s", n, s, n as f64 / s / 1_048_576.0);
}
