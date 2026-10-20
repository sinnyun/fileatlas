use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=TARGET");
    let target = env::var("TARGET").unwrap_or_default();
    if !target.ends_with("x86_64-pc-windows-msvc") {
        return;
    }

    let workspace =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir")).join("../..");
    let sdk = workspace.join("vendor/everything-sdk");
    println!(
        "cargo:rerun-if-changed={}",
        sdk.join("lib/Everything3_x64.lib").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        sdk.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=Everything3_x64");
}
