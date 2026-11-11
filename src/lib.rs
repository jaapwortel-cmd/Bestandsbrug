//! De kern van Bestandsbrug: alles behalve het scherm. Werkt ook zonder Windows, zodat de tests overal draaien.
//! Het scherm (alleen Windows) staat in src/scherm en wordt vanuit main.rs gestart.

pub mod fout;
pub mod instellingen;
pub mod log;
pub mod melding;
pub mod net;
pub mod opmaak;
pub mod overdracht;
pub mod paden;
pub mod pcloud;
pub mod taal;
pub mod tijd;
pub mod update;
pub mod voorkeuren;
