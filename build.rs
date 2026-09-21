use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rustc-link-lib=dylib=hackrf");
    println!("cargo:rerun-if-env-changed=RF_HACKRF_LIB_DIR");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if let Ok(dir) = env::var("RF_HACKRF_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
        if target_os != "android" {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
        }
        return;
    }

    if target_os != "android"
        && let Ok(out) = Command::new("pkg-config")
            .args(["--libs-only-L", "libhackrf"])
            .output()
        && out.status.success()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for path in text.split_whitespace().filter_map(|t| t.strip_prefix("-L")) {
            println!("cargo:rustc-link-search=native={path}");
            println!("cargo:rustc-link-arg=-Wl,-rpath,{path}");
        }
    }
}
