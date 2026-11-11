//! Hulp voor de tests: een kleine webserver op de eigen computer (127.0.0.1), met ondersteuning voor Range.
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

pub type Routes = Arc<Mutex<HashMap<String, (u16, Vec<u8>)>>>;

/// Start een server. Geeft het adres ("http://127.0.0.1:1234") en de routes (pad zonder query -> status, inhoud).
pub fn start_server() -> (String, Routes) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let adres = format!("http://{}", l.local_addr().unwrap());
    let routes: Routes = Arc::new(Mutex::new(HashMap::new()));
    let r2 = routes.clone();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let routes = r2.clone();
            std::thread::spawn(move || {
                let mut lezer = BufReader::new(s.try_clone().unwrap());
                let mut eerste = String::new();
                if lezer.read_line(&mut eerste).is_err() {
                    return;
                }
                let mut range: Option<u64> = None;
                let mut lengte = 0usize;
                loop {
                    let mut r = String::new();
                    if lezer.read_line(&mut r).is_err() || r.trim().is_empty() {
                        break;
                    }
                    let k = r.to_ascii_lowercase();
                    if let Some(x) = k.strip_prefix("range: bytes=") {
                        range = x.trim().trim_end_matches('-').parse().ok();
                    }
                    if let Some(x) = k.strip_prefix("content-length:") {
                        lengte = x.trim().parse().unwrap_or(0);
                    }
                }
                let mut lichaam = vec![0u8; lengte];
                let _ = lezer.read_exact(&mut lichaam);
                let pad = eerste.split_whitespace().nth(1).unwrap_or("/").split('?').next().unwrap().to_string();
                let antwoord = routes.lock().unwrap().get(&pad).cloned();
                let mut s = s;
                match antwoord {
                    Some((status, inhoud)) => {
                        if let (Some(van), 200) = (range, status) {
                            let deel = &inhoud[van as usize..];
                            let _ = write!(s, "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nConnection: close\r\n\r\n", deel.len(), van, inhoud.len() - 1, inhoud.len());
                            let _ = s.write_all(deel);
                        } else {
                            let _ = write!(s, "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", inhoud.len());
                            let _ = s.write_all(&inhoud);
                        }
                    }
                    None => {
                        let _ = write!(s, "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    }
                }
            });
        }
    });
    (adres, routes)
}

/// Een eigen gegevensmap per test, zodat tests elkaar niet storen.
pub fn eigen_map(naam: &str) -> std::path::PathBuf {
    let m = std::env::temp_dir().join(format!("bestandsbrug-test-{naam}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&m);
    std::fs::create_dir_all(&m).unwrap();
    m
}
