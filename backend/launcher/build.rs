fn main() {
    println!("cargo:rerun-if-changed=../packaging/icons/litecraft.ico");
    #[cfg(windows)]
    {
        let icon = std::path::Path::new("../packaging/icons/litecraft.ico");
        if icon.is_file() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon(icon.to_str().expect("icon path"));
            res.set("ProductName", "LiteCraft Launther");
            res.set("FileDescription", "LiteCraft Launther");
            res.set("OriginalFilename", "LiteCraft-Launther.exe");
            res.set(
                "LegalCopyright",
                "MIT; Minecraft assets belong to Mojang and Microsoft",
            );
            let _ = res.compile();
        }
    }
}
