//! Bestandsbrug starten. Hier gebeurt alleen het opstarten; het scherm staat in src/scherm.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod scherm;

fn main() {
    #[cfg(windows)]
    {
        let code = scherm::start();
        std::process::exit(code);
    }
    #[cfg(not(windows))]
    {
        eprintln!("{} draait alleen op Windows.", bestandsbrug::instellingen::TECHNISCHE_NAAM);
        std::process::exit(1);
    }
}
