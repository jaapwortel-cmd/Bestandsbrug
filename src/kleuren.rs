//! Alle kleuren van de lichte en donkere weergave op één plek. De Windows-app (src/scherm/thema.rs) en
//! Bestandsbrug.html (via src/html.rs) gebruiken allebei deze lijst. Alle tekstkleuren halen minstens
//! contrast 4,5 : 1 (WCAG AA) op hun achtergrond; een test controleert dat.

#[derive(Debug, Clone, Copy)]
pub struct Palet {
    pub achtergrond: &'static str,
    pub kaart: &'static str,
    pub tekst: &'static str,
    pub zacht: &'static str,
    pub rand: &'static str,
    pub accent: &'static str,
    pub op_accent: &'static str,
    pub accent_ingedrukt: &'static str,
    pub focus: &'static str,
    pub kop: &'static str,
    pub op_kop: &'static str,
}

pub const LICHT: Palet = Palet {
    achtergrond: "#EEF2F7",
    kaart: "#FFFFFF",
    tekst: "#1A1F29",
    zacht: "#4A5363",
    rand: "#8A94A6",
    accent: "#0B5CAD",
    op_accent: "#FFFFFF",
    accent_ingedrukt: "#084685",
    focus: "#B34000",
    kop: "#0B4A8C",
    op_kop: "#FFFFFF",
};

pub const DONKER: Palet = Palet {
    achtergrond: "#12161D",
    kaart: "#1E242E",
    tekst: "#EEF1F6",
    zacht: "#B9C2D0",
    rand: "#6B778A",
    accent: "#6CB4FF",
    op_accent: "#0A1220",
    accent_ingedrukt: "#9CCBFF",
    focus: "#FFC857",
    kop: "#172B47",
    op_kop: "#FFFFFF",
};

/// "#RRGGBB" naar (r, g, b).
pub fn rgb(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    let d = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    (d(0), d(2), d(4))
}

/// Contrastverhouding volgens WCAG (1 tot 21).
pub fn contrast(a: &str, b: &str) -> f64 {
    fn lum(hex: &str) -> f64 {
        let (r, g, b) = rgb(hex);
        let k = |c: u8| {
            let c = c as f64 / 255.0;
            if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * k(r) + 0.7152 * k(g) + 0.0722 * k(b)
    }
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// De kleuren als CSS-variabelen (voor Bestandsbrug.html).
pub fn css(p: &Palet) -> String {
    format!(
        "--achtergrond:{};--kaart:{};--tekst:{};--zacht:{};--rand:{};--accent:{};--op-accent:{};--accent-ingedrukt:{};--focus:{};--kop:{};--op-kop:{};",
        p.achtergrond, p.kaart, p.tekst, p.zacht, p.rand, p.accent, p.op_accent, p.accent_ingedrukt, p.focus, p.kop, p.op_kop
    )
}
