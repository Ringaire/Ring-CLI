fn main() {
    #[cfg(target_os = "windows")]
    {
        let ico_path = "assets/ring.ico";
        if std::path::Path::new(ico_path).exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon(ico_path);
            res.set("ProductName", "RingCLI");
            res.set("FileDescription", "RingCLI - Terminal AI Coding Assistant");
            res.set("LegalCopyright", "Ringaire");
            if let Err(e) = res.compile() {
                eprintln!("cargo:warning=Failed to compile Windows resource: {e}");
            }
        } else {
            println!("cargo:warning=assets/ring.ico not found, skipping icon embedding");
        }
    }
}
