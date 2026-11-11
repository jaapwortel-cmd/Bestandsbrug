//! Tijd en datum: de tijd voor het logbestand en het omzetten van de datum die pCloud geeft naar de eigen tijdzone.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Datum {
    pub jaar: i32,
    pub maand: u32,
    pub dag: u32,
    pub uur: u32,
    pub minuut: u32,
    pub seconde: u32,
}

/// Seconden sinds 1970 naar een datum in UTC.
pub fn utc_datum(unix: i64) -> Datum {
    let dagen = unix.div_euclid(86_400);
    let rest = unix.rem_euclid(86_400) as u32;
    // Algoritme van Howard Hinnant (days_from_civil omgekeerd).
    let z = dagen + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    Datum { jaar: (y + if m <= 2 { 1 } else { 0 }) as i32, maand: m, dag: d, uur: rest / 3600, minuut: rest / 60 % 60, seconde: rest % 60 }
}

/// Datum (UTC) naar seconden sinds 1970.
pub fn naar_unix(d: &Datum) -> i64 {
    let y = d.jaar as i64 - if d.maand <= 2 { 1 } else { 0 };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = d.maand as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d.dag as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let dagen = era * 146_097 + doe - 719_468;
    dagen * 86_400 + d.uur as i64 * 3600 + d.minuut as i64 * 60 + d.seconde as i64
}

/// Leest een datum van pCloud, zoals "Wed, 11 Nov 2026 17:42:23 +0000". Geeft seconden sinds 1970.
pub fn lees_pcloud_datum(s: &str) -> Option<i64> {
    let delen: Vec<&str> = s.split_whitespace().collect();
    if delen.len() < 5 {
        return None;
    }
    let dag: u32 = delen[1].parse().ok()?;
    let maanden = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
    let maand = maanden.iter().position(|m| delen[2].to_ascii_lowercase().starts_with(m))? as u32 + 1;
    let jaar: i32 = delen[3].parse().ok()?;
    let t: Vec<u32> = delen[4].split(':').filter_map(|x| x.parse().ok()).collect();
    if t.len() != 3 {
        return None;
    }
    let mut unix = naar_unix(&Datum { jaar, maand, dag, uur: t[0], minuut: t[1], seconde: t[2] });
    if let Some(z) = delen.get(5) {
        // +0100 betekent: de tijd is 1 uur voor op UTC.
        if z.len() == 5 {
            let teken = if z.starts_with('-') { -1 } else { 1 };
            let u: i64 = z[1..3].parse().unwrap_or(0);
            let m: i64 = z[3..5].parse().unwrap_or(0);
            unix -= teken * (u * 3600 + m * 60);
        }
    }
    Some(unix)
}

/// Seconden sinds 1970 naar de eigen tijd (met zomer- en wintertijd van Windows).
pub fn lokaal(unix: i64) -> Datum {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
        use windows::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};
        let ft100 = (unix + 11_644_473_600) as u64 * 10_000_000;
        let ft = FILETIME { dwLowDateTime: ft100 as u32, dwHighDateTime: (ft100 >> 32) as u32 };
        let mut st = SYSTEMTIME::default();
        let mut lt = SYSTEMTIME::default();
        unsafe {
            if FileTimeToSystemTime(&ft, &mut st).is_ok() && SystemTimeToTzSpecificLocalTime(None, &st, &mut lt).is_ok() {
                return Datum {
                    jaar: lt.wYear as i32,
                    maand: lt.wMonth as u32,
                    dag: lt.wDay as u32,
                    uur: lt.wHour as u32,
                    minuut: lt.wMinute as u32,
                    seconde: lt.wSecond as u32,
                };
            }
        }
        utc_datum(unix)
    }
    #[cfg(not(windows))]
    {
        utc_datum(unix)
    }
}

pub fn nu_unix() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// De tijd van nu voor het logbestand: "2026-11-11 18:40:05".
pub fn nu_tekst() -> String {
    let d = lokaal(nu_unix());
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", d.jaar, d.maand, d.dag, d.uur, d.minuut, d.seconde)
}
