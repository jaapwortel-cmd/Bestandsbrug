//! Maakt het losse HTML-bestand. Gebruik: maak-html <uitvoerbestand>
fn main() {
    let uit = std::env::args().nth(1).unwrap_or_else(|| bestandsbrug::instellingen::HTML_BESTAND.into());
    std::fs::write(&uit, bestandsbrug::html::maak()).expect("kan het HTML-bestand niet schrijven");
    println!("{uit} gemaakt");
}
