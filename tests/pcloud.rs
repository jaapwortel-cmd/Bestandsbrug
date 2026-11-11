//! Tests voor de pCloud-laag zonder internet: de lijst lezen, uitleg.txt, de grens van de uploadlink,
//! en downloaden met hervatten (HTTP Range), stoppen en annuleren op een eigen kleine webserver.

mod hulp;

use bestandsbrug::fout::{Fout, Stop};
use bestandsbrug::overdracht::{deel_pad, Voortgang};
use bestandsbrug::pcloud::{download, lijst, upload};

#[test]
fn lijst_lezen_met_app_mappen() {
    let json: serde_json::Value = serde_json::from_str(r#"{"result":0,"metadata":{"contents":[
        {"name":"b.exe","fileid":2,"size":10,"modified":"Wed, 11 Nov 2026 17:42:23 +0000","hash":5,"isfolder":false},
        {"name":"Paper Pilot","folderid":3,"isfolder":true,"contents":[
            {"name":"Paper Pilot 0.0.0.16 installatie.exe","fileid":7,"size":30,"isfolder":false},
            {"name":"Diep","folderid":4,"isfolder":true,"contents":[{"name":"x.dll","fileid":8,"size":1,"isfolder":false}]}]},
        {"name":"A.exe","fileid":1,"size":20,"modified":"Wed, 11 Nov 2026 17:42:23 +0000","hash":9223372036854775808,"isfolder":false}]}}"#).unwrap();
    let l = lijst::lees_inhoud(&json);
    assert_eq!(l.len(), 3, "hoofdmap en app-map, niet dieper");
    assert_eq!(l[0].naam, "A.exe");
    assert_eq!(l[2].map, "Paper Pilot");
    assert!(!l.iter().any(|b| b.naam == "x.dll"));
    assert_eq!(l[0].hash, 9223372036854775808);
    assert_eq!(lijst::nieuwe_namen(&l[..2], &l), vec!["Paper Pilot 0.0.0.16 installatie.exe".to_string()]);
}

#[test]
fn uitleg_lezen() {
    let k = lijst::lees_uitleg("\u{feff}# uitleg\r\nPaper Pilot.exe = Een app om papier te lezen\r\nonzin\r\n");
    assert_eq!(k.get("paper pilot.exe").map(String::as_str), Some("Een app om papier te lezen"));
    assert_eq!(k.len(), 1);
}

#[test]
fn foutcodes_van_pcloud() {
    let f = |s: &str| bestandsbrug::pcloud::controleer(serde_json::from_str(s).unwrap());
    assert!(f(r#"{"result":0}"#).is_ok());
    assert!(matches!(f(r#"{"result":7004,"error":"x"}"#), Err(Fout::LinkWeg)));
    assert!(matches!(f(r#"{"result":7008,"error":"x"}"#), Err(Fout::UploadVol)));
    assert!(matches!(f(r#"{"result":5000,"error":"Internal"}"#), Err(Fout::Pcloud(5000, _))));
}

#[test]
fn grens_van_de_uploadlink() {
    let j = |s: &str| serde_json::from_str::<serde_json::Value>(s).unwrap();
    assert!(upload::controleer_grens(&j(r#"{"maxspace":0,"maxfiles":0}"#), 10_000_000_000).is_ok());
    assert!(matches!(upload::controleer_grens(&j(r#"{"maxspace":1000,"space":400}"#), 700), Err(Fout::TeGroot { mag_nog: 600, bestand: 700 })));
    assert!(upload::controleer_grens(&j(r#"{"maxspace":1000,"space":400}"#), 600).is_ok());
    assert!(matches!(upload::controleer_grens(&j(r#"{"maxfiles":5,"files":5}"#), 1), Err(Fout::UploadVol)));
}

#[test]
fn downloaden_hervatten_en_controleren() {
    let map = hulp::eigen_map("download");
    let (adres, routes) = hulp::start_server();
    let data: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
    routes.lock().unwrap().insert("/bestand".into(), (200, data.clone()));
    let url = format!("{adres}/bestand");
    let doel = map.join("app.exe");

    // Een half bestand van eerder: de download gaat daar verder (Range).
    std::fs::write(deel_pad(&doel), &data[..1_000_000]).unwrap();
    let v = Voortgang::new();
    download::download_url(&url, &doel, Some(data.len() as u64), &v).unwrap();
    assert_eq!(std::fs::read(&doel).unwrap(), data);
    assert_eq!(v.begin.load(std::sync::atomic::Ordering::Relaxed), 1_000_000);
    assert!(!deel_pad(&doel).exists());

    // Een verkeerde grootte: niet compleet.
    let r = download::download_url(&url, &map.join("b.exe"), Some(data.len() as u64 + 1), &Voortgang::new());
    assert!(matches!(r, Err(Fout::Onvolledig)));
    assert!(!deel_pad(&map.join("b.exe")).exists());
}

#[test]
fn stoppen_en_annuleren() {
    let map = hulp::eigen_map("stoppen");
    let (adres, routes) = hulp::start_server();
    routes.lock().unwrap().insert("/f".into(), (200, vec![7u8; 2_000_000]));
    let url = format!("{adres}/f");

    let v = Voortgang::new();
    v.vraag_stop(Stop::Pauze);
    let doel = map.join("pauze.exe");
    let r = download::download_url(&url, &doel, Some(2_000_000), &v);
    assert!(matches!(r, Err(Fout::Gestopt(Stop::Pauze))));
    assert!(deel_pad(&doel).exists(), "bij pauze blijft het halve bestand staan");

    let v = Voortgang::new();
    v.vraag_stop(Stop::Annuleer);
    let doel = map.join("weg.exe");
    let r = download::download_url(&url, &doel, Some(2_000_000), &v);
    assert!(matches!(r, Err(Fout::Gestopt(Stop::Annuleer))));
    assert!(!deel_pad(&doel).exists(), "bij annuleren is het halve bestand weg");
    assert!(!doel.exists());
}
