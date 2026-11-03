/// Lists every file under `themes/` (the built-in themes, with their fonts)
/// so the app can carry them inside the executable.
fn embed_themes() {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .map(|r| r.flatten().map(|e| e.path()).collect())
            .unwrap_or_default();
        entries.sort();
        for p in entries {
            if p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
                continue;
            }
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
    let root = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("themes");
    println!("cargo:rerun-if-changed=themes");
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut code = String::from("/// (path under themes/, contents)\npub const BUILT_IN_FILES: &[(&str, &[u8])] = &[\n");
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let rel = f.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        code.push_str(&format!("    ({rel:?}, include_bytes!({:?})),\n", f.display().to_string()));
    }
    code.push_str("];\n");
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("built_in_themes.rs");
    std::fs::write(out, code).unwrap();
}

fn main() {
    embed_themes();
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
