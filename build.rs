use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=GUARDIAN_EMBED_PATH");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86_64".into());

    let candidate = if let Some(explicit) = env::var_os("GUARDIAN_EMBED_PATH") {
        PathBuf::from(explicit)
    } else if target_arch == "aarch64" {
        let arm = manifest_dir.join("dist/ModernInvertedCursorGuardian-arm64.exe");
        if arm.is_file() {
            arm
        } else {
            manifest_dir.join("dist/ModernInvertedCursorGuardian.exe")
        }
    } else {
        manifest_dir.join("dist/ModernInvertedCursorGuardian.exe")
    };

    println!("cargo:rerun-if-changed={}", candidate.display());
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let dest = out_dir.join("embedded_guardian.exe");

    if candidate.is_file() {
        fs::copy(&candidate, &dest).expect("failed to copy guardian binary into OUT_DIR");
    } else {
        fs::write(&dest, b"").expect("failed to write placeholder embedded_guardian.exe");
    }
}
