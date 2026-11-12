use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=GUARDIAN_EMBED_PATH");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86_64".into());

    let candidate = if let Some(explicit) = env::var_os("GUARDIAN_EMBED_PATH") {
        PathBuf::from(explicit)
    } else if target_arch == "aarch64" {
        let arm = manifest_dir.join("dist/ModernInvertedCursors-Guardian-arm64.exe");
        if arm.is_file() {
            arm
        } else {
            manifest_dir.join("dist/ModernInvertedCursors-Guardian-x64.exe")
        }
    } else if target_arch == "x86" {
        let legacy_x86 = manifest_dir.join("dist/ModernInvertedCursors-Guardian-win7-win10-x86.exe");
        if legacy_x86.is_file() {
            legacy_x86
        } else {
            manifest_dir.join("dist/ModernInvertedCursors-Guardian-x64.exe")
        }
    } else {
        let x64 = manifest_dir.join("dist/ModernInvertedCursors-Guardian-x64.exe");
        if x64.is_file() {
            x64
        } else {
            manifest_dir.join("dist/ModernInvertedCursorGuardian.exe")
        }
    };

    println!("cargo:rerun-if-changed={}", candidate.display());
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let dest = out_dir.join("embedded_guardian.exe");

    if candidate.is_file() {
        fs::copy(&candidate, &dest).expect("failed to copy guardian binary into OUT_DIR");
    } else {
        fs::write(&dest, b"").expect("failed to write placeholder embedded_guardian.exe");
    }

    // Embed explicit asInvoker + Win7-Win11 supportedOS manifest on MSVC builds so
    // 32-bit Setup EXEs never trigger Windows Installer Detection UAC prompts (error 740).
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        let manifest = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}"/>
      <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}"/>
    </application>
  </compatibility>
</assembly>
"#;
        let manifest_path = out_dir.join("app.manifest");
        fs::write(&manifest_path, manifest).expect("failed to write app.manifest");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest_path.display());
    }
}
