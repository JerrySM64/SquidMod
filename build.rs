fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set("FileDescription", "SquidMod");
        res.set("ProductName", "SquidMod");
        if std::path::Path::new("assets/icon.ico").exists() {
            let _ = res.set_icon("assets/icon.ico");
        }
        let _ = res.compile();
    }
}