fn main() {
    println!("cargo:rerun-if-env-changed=TARGET");

    // GPUI and the complete Cordis feature graph need more than the 1 MiB
    // default Windows executable stack during startup. Preserve the Windows
    // GUI main thread and reserve 8 MiB for this binary.
    if std::env::var("TARGET").is_ok_and(|target| target.ends_with("-pc-windows-msvc")) {
        println!("cargo:rustc-link-arg-bin=photag-shell=/STACK:8388608");
    }
}
