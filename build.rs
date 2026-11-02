fn main() {
    println!("cargo:rerun-if-changed=packaging/icons/PresentCraft.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("packaging/icons/PresentCraft.ico")
            .set("ProductName", "PresentCraft")
            .set(
                "FileDescription",
                "PresentCraft Markdown presentation editor",
            )
            .compile()
            .expect("Failed to embed Windows app icon; install the Windows SDK resource compiler");
    }
}
