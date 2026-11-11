//! Getallen en datums netjes opschrijven in de gekozen taal: "6,4 MB" en "11 november 2026, 18:42".

use crate::taal;

/// Bestandsgrootte in gewone woorden.
pub fn grootte(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    let (getal, eenheid) = if b < KB {
        return format!("{bytes} bytes");
    } else if b < KB * KB {
        (b / KB, "KB")
    } else if b < KB * KB * KB {
        (b / KB / KB, "MB")
    } else {
        (b / KB / KB / KB, "GB")
    };
    let tekst = if getal >= 100.0 { format!("{getal:.0}") } else { format!("{getal:.1}") };
    format!("{} {eenheid}", tekst.replace('.', &taal::t("Decimaalteken")))
}

/// Datum van pCloud naar de eigen tijd, in woorden.
pub fn datum(pcloud: &str) -> String {
    match crate::tijd::lees_pcloud_datum(pcloud) {
        Some(u) => {
            let d = crate::tijd::lokaal(u);
            taal::t("DatumVorm")
                .replace("{d}", &d.dag.to_string())
                .replace("{maand}", &taal::t(&format!("Maand{}", d.maand)))
                .replace("{jaar}", &d.jaar.to_string())
                .replace("{uur}", &format!("{:02}", d.uur))
                .replace("{min}", &format!("{:02}", d.minuut))
        }
        None => pcloud.to_string(),
    }
}
