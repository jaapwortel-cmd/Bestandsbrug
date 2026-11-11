//! De pCloud-laag: de openbare API voor gedeelde links (zonder inloggen, zonder sleutel).
//! lijst.rs haalt de lijst op, download.rs downloadt (streamen, hervatten), upload.rs uploadt (streamen).
//! Alleen de twee openbare codes uit instellingen.rs worden gebruikt.

pub mod download;
pub mod lijst;
pub mod upload;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::Value;

use crate::fout::{Fout, Uitkomst};
use crate::instellingen::PCLOUD_API_SERVERS;

/// De server die het laatst werkte (eerst de Europese).
static SERVER: AtomicUsize = AtomicUsize::new(0);

pub fn server() -> &'static str {
    PCLOUD_API_SERVERS[SERVER.load(Ordering::Relaxed) % PCLOUD_API_SERVERS.len()]
}

/// Roept een API-methode aan met GET. Bij geen verbinding met de ene server wordt de andere geprobeerd.
pub fn api_get(methode: &str, params: &[(&str, &str)], limiet: Duration) -> Uitkomst<Value> {
    let mut laatste = Fout::GeenInternet(String::new());
    for poging in 0..PCLOUD_API_SERVERS.len() {
        let index = (SERVER.load(Ordering::Relaxed) + poging) % PCLOUD_API_SERVERS.len();
        let url = format!("{}/{}", PCLOUD_API_SERVERS[index], methode);
        let mut req = crate::net::get(&url);
        for (k, v) in params {
            req = req.query(*k, *v);
        }
        match crate::net::met_limiet(req, limiet).call() {
            Ok(resp) => {
                let json: Value = crate::net::lees_json(resp)?;
                SERVER.store(index, Ordering::Relaxed);
                return controleer(json);
            }
            Err(e) => {
                let f = crate::net::vertaal(e);
                if !f.is_geen_internet() {
                    return Err(f);
                }
                laatste = f;
            }
        }
    }
    Err(laatste)
}

/// Kijkt naar "result" in het antwoord van pCloud en maakt er zo nodig een duidelijke fout van.
pub fn controleer(json: Value) -> Uitkomst<Value> {
    let code = json.get("result").and_then(Value::as_i64).unwrap_or(-1);
    if code == 0 {
        return Ok(json);
    }
    let bericht = json.get("error").and_then(Value::as_str).unwrap_or("").to_string();
    Err(match code {
        7001 | 7002 | 7004 | 7005 | 7006 => Fout::LinkWeg,
        7007 | 7008 => Fout::UploadVol,
        _ => Fout::Pcloud(code, bericht),
    })
}
