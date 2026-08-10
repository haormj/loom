fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dist = std::path::PathBuf::from(&manifest_dir).join("embedded/dist");
    if !dist.exists() {
        std::fs::create_dir_all(&dist).ok();
        std::fs::write(
            dist.join("index.html"),
            "<!DOCTYPE html><html><body>Frontend not built. Run npm build in frontend/.</body></html>",
        )
        .ok();
    }
    println!("cargo:rerun-if-changed=embedded/dist");
}
