//! Tests voor het logbestand (klein houden) en het onthouden van onderbroken downloads.

mod hulp;

use bestandsbrug::overdracht::{self, Onderbroken};

#[test]
fn log_blijft_klein_en_begint_op_een_hele_regel() {
    let map = hulp::eigen_map("log");
    let pad = map.join("test.log");
    let regel = "2026-11-11 18:00:00 INFO Download done, 1234 bytes\r\n";
    std::fs::write(&pad, regel.repeat(20_000)).unwrap();
    bestandsbrug::log::kort_in(&pad, 250 * 1024).unwrap();
    let s = std::fs::read_to_string(&pad).unwrap();
    assert!(s.len() <= 250 * 1024);
    assert!(s.starts_with("2026-11-11"));
}

#[test]
fn onderbroken_download_onthouden_en_weggooien() {
    let map = hulp::eigen_map("overdracht");
    std::env::set_var("BESTANDSBRUG_MAP", &map);
    let o = Onderbroken { fileid: 1, naam: "App.exe".into(), grootte: 100, hash: 5, doel: map.join("App.exe") };
    std::fs::write(o.deel(), vec![0u8; 40]).unwrap();
    overdracht::voeg_toe(o.clone());
    let l = overdracht::laad();
    assert_eq!(l, vec![o.clone()]);
    assert_eq!(l[0].procent(), 40);
    overdracht::gooi_weg(&o);
    assert!(overdracht::laad().is_empty());
    assert!(!o.deel().exists());
}
