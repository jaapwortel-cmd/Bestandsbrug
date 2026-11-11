// Zet het app-icoon en het manifest (moderne Windows-onderdelen, scherpe letters bij grote tekst) in de exe.
fn main() {
    println!("cargo:rerun-if-changed=assets/bestandsbrug.rc");
    println!("cargo:rerun-if-changed=assets/bestandsbrug.manifest");
    println!("cargo:rerun-if-changed=assets/bestandsbrug.ico");
    #[cfg(windows)]
    {
        let _ = embed_resource::compile("assets/bestandsbrug.rc", embed_resource::NONE);
    }
    #[cfg(not(windows))]
    {
        // Bij kruiscompileren vanaf Linux naar Windows: het doel bepaalt het.
        if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
            let _ = std::process::Command::new("x86_64-w64-mingw32-windres")
                .args(["assets/bestandsbrug.rc", "-O", "coff", "-o"])
                .arg(format!("{}/bestandsbrug-res.o", std::env::var("OUT_DIR").unwrap()))
                .status();
            println!("cargo:rustc-link-arg-bins={}/bestandsbrug-res.o", std::env::var("OUT_DIR").unwrap());
        }
    }
}
