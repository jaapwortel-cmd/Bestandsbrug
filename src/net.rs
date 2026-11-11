//! Eén gedeelde internetverbinding (agent) voor pCloud en Dropbox. Zo wordt de verbinding hergebruikt.
//! Certificaten komen uit Windows zelf, en de proxy-instelling van Windows wordt gevolgd.
//! Hier staan ook de tijdslimieten en het vertalen van internetfouten naar een Fout.

use std::sync::OnceLock;
use std::time::Duration;

use ureq::typestate::{WithBody, WithoutBody};
use ureq::RequestBuilder;

use crate::fout::Fout;
use crate::instellingen::{LIMIET_LEZEN_SECONDEN, LIMIET_VERBINDEN_SECONDEN, TECHNISCHE_NAAM, VERSIE};

pub fn agent() -> &'static ureq::Agent {
    static A: OnceLock<ureq::Agent> = OnceLock::new();
    A.get_or_init(|| {
        let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
        ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(LIMIET_VERBINDEN_SECONDEN)))
            .timeout_per_read(Some(Duration::from_secs(LIMIET_LEZEN_SECONDEN)))
            .timeout_per_write(Some(Duration::from_secs(LIMIET_LEZEN_SECONDEN)))
            .user_agent(format!("{TECHNISCHE_NAAM}/{VERSIE}"))
            .proxy(ureq::Proxy::try_from_env())
            .tls_config(tls)
            .build()
            .into()
    })
}

pub fn get(url: &str) -> RequestBuilder<WithoutBody> {
    agent().get(url)
}

pub fn post(url: &str) -> RequestBuilder<WithBody> {
    agent().post(url)
}

/// Een tijdslimiet voor de hele vraag (van verbinden tot het hele antwoord).
pub fn met_limiet(r: RequestBuilder<WithoutBody>, limiet: Duration) -> RequestBuilder<WithoutBody> {
    r.config().timeout_global(Some(limiet)).build()
}

/// Een fout van ureq naar een Fout in gewone taal.
pub fn vertaal(e: ureq::Error) -> Fout {
    match e {
        ureq::Error::StatusCode(code) => Fout::Anders(format!("HTTP {code}")),
        ureq::Error::Timeout(_) => Fout::TeLang,
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => Fout::GeenInternet(e.to_string()),
        ureq::Error::Io(io) => vertaal_io(&io),
        anders => Fout::Anders(anders.to_string()),
    }
}

/// Een fout tijdens het lezen van de stroom: tijdslimiet of verbinding weg.
pub fn vertaal_io(e: &std::io::Error) -> Fout {
    match e.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => Fout::TeLang,
        _ => {
            // ureq stopt soms een eigen fout in een io::Error.
            if let Some(u) = e.get_ref().and_then(|x| x.downcast_ref::<ureq::Error>()) {
                if matches!(u, ureq::Error::Timeout(_)) {
                    return Fout::TeLang;
                }
            }
            Fout::GeenInternet(format!("{:?}", e.kind()))
        }
    }
}

/// Een JSON-antwoord lezen.
pub fn lees_json(resp: ureq::http::Response<ureq::Body>) -> Result<serde_json::Value, Fout> {
    resp.into_body().read_json::<serde_json::Value>().map_err(vertaal)
}
