use std::process::Command;

fn main() {
    // Re-run if UI source changes
    println!("cargo:rerun-if-changed=ui/src");
    println!("cargo:rerun-if-changed=ui/index.html");
    println!("cargo:rerun-if-changed=ui/package.json");
    println!("cargo:rerun-if-changed=ui/vite.config.ts");

    // Allow skipping the UI build for faster Rust-only iteration
    if std::env::var("SKIP_UI_BUILD").is_ok() {
        println!("cargo:warning=Skipping UI build (SKIP_UI_BUILD set)");
        // Ensure dist/ exists so rust-embed doesn't fail
        std::fs::create_dir_all("ui/dist").unwrap();
        return;
    }

    let ui_dir = std::path::Path::new("ui");
    if !ui_dir.join("node_modules").exists() {
        let status = Command::new("npm")
            .args(["install"])
            .current_dir(ui_dir)
            .status()
            .expect("Failed to run npm install");
        if !status.success() {
            panic!("npm install failed");
        }
    }

    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir(ui_dir)
        .status()
        .expect("Failed to run npm run build");
    if !status.success() {
        panic!("npm run build failed");
    }
}
