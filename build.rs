use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");

    let output = Command::new("pkg-config")
        .args(["--variable=libdir", "libpanel-1"])
        .output()
        .expect("failed to run pkg-config for libpanel-1");

    if !output.status.success() {
        panic!(
            "pkg-config could not locate libpanel-1: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    println!(
        "cargo:rustc-link-search=native={}",
        String::from_utf8_lossy(&output.stdout).trim()
    );
}
