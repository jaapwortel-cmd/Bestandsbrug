// Zet het app-icoon, het manifest en de versie-informatie in Bestandsbrug.exe.
// De versie komt uit Cargo.toml, die bouw-alles.ps1 gelijk zet met src/instellingen.rs.
use std::path::PathBuf;

fn main() {
    for f in ["assets/bestandsbrug.ico", "assets/bestandsbrug.manifest", "src/instellingen.rs"] {
        println!("cargo:rerun-if-changed={f}");
    }
    if std::env::var("CARGO_CFG_WINDOWS").is_err() {
        return;
    }
    let uit = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let map = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let versie = std::env::var("CARGO_PKG_VERSION").unwrap();
    let mut d: Vec<u32> = versie.split('.').filter_map(|x| x.parse().ok()).collect();
    d.resize(4, 0);
    let pad = |p: &str| map.join(p).to_string_lossy().replace('\\', "/");
    let rc = format!(
        r#"#pragma code_page(65001)
1 ICON "{icoon}"
1 24 "{manifest}"
1 VERSIONINFO
FILEVERSION {a},{b},{c},{e}
PRODUCTVERSION {a},{b},{c},{e}
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "041304B0"
    BEGIN
      VALUE "CompanyName", "Jaap Wortel"
      VALUE "FileDescription", "Bestandsbrug (Filebridge)"
      VALUE "FileVersion", "{versie}"
      VALUE "ProductName", "Bestandsbrug"
      VALUE "ProductVersion", "{versie}"
      VALUE "OriginalFilename", "Bestandsbrug.exe"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0413, 1200
  END
END
"#,
        icoon = pad("assets/bestandsbrug.ico"),
        manifest = pad("assets/bestandsbrug.manifest"),
        a = d[0],
        b = d[1],
        c = d[2],
        e = d[3],
    );
    let rc_pad = uit.join("bestandsbrug.rc");
    std::fs::write(&rc_pad, rc).unwrap();
    #[cfg(windows)]
    {
        embed_resource::compile_for(&rc_pad, ["bestandsbrug"], embed_resource::NONE).manifest_required().unwrap();
    }
    #[cfg(not(windows))]
    {
        // Kruiscompileren vanaf Linux (alleen om te controleren of alles bouwt).
        let o = uit.join("bestandsbrug-res.o");
        let ok = std::process::Command::new("x86_64-w64-mingw32-windres")
            .arg(&rc_pad)
            .args(["-O", "coff", "-o"])
            .arg(&o)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            println!("cargo:rustc-link-arg-bin=bestandsbrug={}", o.display());
        }
    }
}
