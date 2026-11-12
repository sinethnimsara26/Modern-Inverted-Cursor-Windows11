// Modern Inverted Mouse Cursors v3.6.2 - standalone Windows installer + guardian
// V5.1 Balanced: normalized Accessibility slider sizing (32..88px)
//
// Single-file Rust program. No Cargo project and no third-party crates required.
//
// Build on Windows:
//   rustc +stable-gnu -O modern_inverted_installer_v5_1_balanced.rs -o ModernInvertedInstallerV5_1.exe
//
// Run:
//   .\ModernInvertedInstallerV5_1.exe
//
// Restore the cursor setup that existed before the first install:
//   .\ModernInvertedInstallerV5_1.exe --restore
//
// What this program uses:
//   - curl.exe (included with modern Windows 10/11) to download the exact public
//     v3.6.2 cursor assets from GitHub.
//   - reg.exe (built into Windows) to back up and update HKCU cursor settings.
//   - user32 APIs to persist/reload the scheme and force-apply live cursor handles.
//   - an event-driven per-user guardian that listens for Windows Accessibility
//     cursor changes and reapplies this custom pack at the new pointer size.
//
// It installs only for the current user, under:
//   %LOCALAPPDATA%\ModernInvertedCursors\v3.6.2
//
// Upstream project:
//   https://github.com/emvaized/modern_inverted_mouse_cursors
//
// NOTE:
//   This is an independent installer for the public source assets. It is not a
//   copy of the author's Patreon package.

use std::{
    env,
    ffi::c_void,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const VERSION: &str = "3.6.2";
const REPO_RAW: &str =
    "https://raw.githubusercontent.com/emvaized/modern_inverted_mouse_cursors/3.6.2/src";

const GUARDIAN_EXE_NAME: &str = "ModernInvertedCursorGuardian.exe";
const GUARDIAN_RUN_VALUE: &str = "ModernInvertedCursorGuardian";
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const ACCESSIBILITY_SUBKEY: &str = r"Software\Microsoft\Accessibility";

// Windows exposes 15 Accessibility pointer-size positions. The native
// CursorBaseSize range is 32..256, which is far too aggressive for these
// low-bit-depth XOR/inverting cursors when LoadImageW scales them.
// V5.1 deliberately maps the same 15-position Windows slider to 32..88px
// in perfectly even 4px increments. This keeps the first jump small and
// prevents the last positions from becoming absurdly huge/pixelated.
const BALANCED_MIN_SIZE: u32 = 32;
const BALANCED_STEP_PX: u32 = 4;
const ACCESSIBILITY_STEPS: u32 = 15;

#[derive(Clone, Copy)]
struct Asset {
    name: &'static str,
    size: u64,
    git_sha1: &'static str,
}

// Exact byte sizes + Git blob SHA-1 IDs from the public GitHub v3.6.2 tree.
// Git's blob ID is SHA1("blob " + decimal_length + NUL + file_bytes).
// We verify both size and blob ID, so a bad/partial/wrong download is rejected.
const ASSETS: &[Asset] = &[
    Asset { name: "arrow.cur", size: 2246, git_sha1: "325739ebbc8a589aa757b7a7f19194bd0e6afd3a" },
    Asset { name: "arrow_no_tail.cur", size: 2384, git_sha1: "8a503406723a840324b966c46074ca7257b8b145" },
    Asset { name: "arrow_no_tail_smaller.cur", size: 2246, git_sha1: "daf30b0336f061a1a7961e1dd0dea9d1ce7ccd40" },
    Asset { name: "arrow_no_tail_smaller_v2.cur", size: 2246, git_sha1: "5e7990e298d85a4d8eb205e0b64a2b7d06b3f2b1" },
    Asset { name: "arrow_tail_detached.cur", size: 2246, git_sha1: "aa8082e301909e7bbf0bee1fdeb47db961f6d34a" },
    Asset { name: "beam.cur", size: 2246, git_sha1: "87c55d6e1a8d819ac80e4edd1d8afe4e35846f5a" },
    Asset { name: "beam_v2.cur", size: 2246, git_sha1: "b092bcf3b77e8cd9413c52e71d9a0a64afabaa69" },
    Asset { name: "busy.ani", size: 5474, git_sha1: "fa36f833b346a3d6310458d0a6bc30ec1c8657ad" },
    Asset { name: "cross.cur", size: 2246, git_sha1: "247947ea4293cb80fe60fe7143ae5c8d9633b7ad" },
    Asset { name: "diagonal_1.cur", size: 2246, git_sha1: "fe4205fd226707e165e1c9bc6ebf040419df33d2" },
    Asset { name: "diagonal_2.cur", size: 2246, git_sha1: "2244e646b88022bd3119a16ae7d17ee88aca88a9" },
    Asset { name: "help.cur", size: 2246, git_sha1: "d807f02924caa159d3bebd138af570bd6dd6ae3e" },
    Asset { name: "help_no_tail.cur", size: 2246, git_sha1: "fc3985098751b3d6e38f32ae43a9e736cafbdad1" },
    Asset { name: "help_tail_detached.cur", size: 326, git_sha1: "b3a1e4dcba4e4b2f24ade81dda0eece44e224d4d" },
    Asset { name: "horizontal.cur", size: 2246, git_sha1: "a38b02b149045421513ca7954d07581a6ae61745" },
    Asset { name: "horizontal_v2.cur", size: 2246, git_sha1: "8c7888da14a7a13441c56db830877df91024fd7d" },
    Asset { name: "link.cur", size: 2246, git_sha1: "86dae854fc5479674cca5ea25986b87c0ac2f3e0" },
    Asset { name: "link_v2.cur", size: 2246, git_sha1: "01eab41a23bbdaaf9f84e449f846a4f233777b68" },
    Asset { name: "move.cur", size: 2246, git_sha1: "6fdb0c4e3812c1cb896e5c45d4f691de53d80e84" },
    Asset { name: "move_v2.cur", size: 326, git_sha1: "6df1bcd2bea99c47a025d9f4dd03db9a3a6b3a98" },
    Asset { name: "pen.cur", size: 2246, git_sha1: "e94d2d6a7182abad2cda2b75434ed37ea7d9ebbc" },
    Asset { name: "special.cur", size: 2246, git_sha1: "dbb6b1fac453fb52ae1cfc87d09ee907373b19d3" },
    Asset { name: "unavailable.cur", size: 8774, git_sha1: "70496b18014075e2598f61c87ebd2c790174eb1c" },
    Asset { name: "unavailable_v2.cur", size: 2246, git_sha1: "4baceb98c400b4b6836a35d4e77b3fa7e76ad68d" },
    Asset { name: "unavailable_v3.cur", size: 2246, git_sha1: "d3e0901c59ec8494d8fc0a12caca418bac8b65ad" },
    Asset { name: "vertical.cur", size: 2246, git_sha1: "f6b8c7d085550f01b23a911d617b817bbac87d7d" },
    Asset { name: "vertical_v2.cur", size: 2246, git_sha1: "57b4a71f8fb6ffc70a169c65d3a561887d193aac" },
    Asset { name: "working.ani", size: 5474, git_sha1: "fcced8db8402d254a3f2e1e635bdb50dd334016c" },
    Asset { name: "working_no_tail.ani", size: 5474, git_sha1: "0c368f9a121a14c7c581ccfb60c427bf5093f77f" },
    Asset { name: "working_tail_detached.ani", size: 5474, git_sha1: "dec0cc444841031357b0b1a1b7189bd61340018a" },
    Asset { name: "working_tail_detached_v2.ani", size: 5474, git_sha1: "9f3656efe73c669aead22627e1f0bc854bab8b1a" },
    Asset { name: "working_tail_detached_v3.ani", size: 2072, git_sha1: "c47132bee21ac2941c6db45a72590f1333876907" },
];

// Minimal SHA-1 implementation for Git blob verification.
// No external crate is needed.
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut msg = data.to_vec();
    let bit_len = (msg.len() as u64) * 8;
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            let j = i * 4;
            w[i] = u32::from_be_bytes([chunk[j], chunk[j + 1], chunk[j + 2], chunk[j + 3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };

            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);

            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    for (i, word) in [h0, h1, h2, h3, h4].iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn git_blob_sha1(bytes: &[u8]) -> String {
    let header = format!("blob {}\0", bytes.len());
    let mut data = Vec::with_capacity(header.len() + bytes.len());
    data.extend_from_slice(header.as_bytes());
    data.extend_from_slice(bytes);
    hex_lower(&sha1(&data))
}

fn verify_asset_file(asset: Asset, path: &Path) -> Result<(), String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("Could not read {} for verification: {e}", path.display()))?;

    if bytes.len() as u64 != asset.size {
        return Err(format!(
            "{} has the wrong size: expected {}, got {} bytes.",
            asset.name,
            asset.size,
            bytes.len()
        ));
    }

    let got = git_blob_sha1(&bytes);
    if got != asset.git_sha1 {
        return Err(format!(
            "{} failed Git blob SHA-1 verification.\nExpected: {}\nGot:      {}",
            asset.name, asset.git_sha1, got
        ));
    }

    Ok(())
}

// Canonical Windows cursor scheme role order.
const ROLES: &[&str] = &[
    "Arrow",
    "Help",
    "AppStarting",
    "Wait",
    "Crosshair",
    "IBeam",
    "NWPen",
    "No",
    "SizeNS",
    "SizeWE",
    "SizeNWSE",
    "SizeNESW",
    "SizeAll",
    "UpArrow",
    "Hand",
    "Pin",
    "Person",
];

#[derive(Clone, Copy)]
enum Preset {
    Regular,
    NoTail,
    TailDetached,
    Compact,
}

impl Preset {
    fn name(self) -> &'static str {
        match self {
            Preset::Regular => "Modern Inverted v3.6.2 - Regular",
            Preset::NoTail => "Modern Inverted v3.6.2 - No Tail",
            Preset::TailDetached => "Modern Inverted v3.6.2 - Tail Detached",
            Preset::Compact => "Modern Inverted v3.6.2 - Compact",
        }
    }

    fn cli_name(self) -> &'static str {
        match self {
            Preset::Regular => "regular",
            Preset::NoTail => "no-tail",
            Preset::TailDetached => "tail-detached",
            Preset::Compact => "compact",
        }
    }

    fn from_cli(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "regular" | "1" => Some(Preset::Regular),
            "notail" | "no-tail" | "no_tail" | "2" => Some(Preset::NoTail),
            "detached" | "tail-detached" | "tail_detached" | "3" => {
                Some(Preset::TailDetached)
            }
            "compact" | "4" => Some(Preset::Compact),
            _ => None,
        }
    }
}

struct Logger {
    path: PathBuf,
}

impl Logger {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn line(&self, text: impl AsRef<str>) {
        let text = text.as_ref();
        println!("{text}");
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(f, "{text}");
        }
    }
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn SystemParametersInfoW(
        ui_action: u32,
        ui_param: u32,
        pv_param: *mut c_void,
        f_win_ini: u32,
    ) -> i32;

    fn LoadImageW(
        h_inst: *mut c_void,
        name: *const u16,
        image_type: u32,
        cx: i32,
        cy: i32,
        load_flags: u32,
    ) -> *mut c_void;

    fn SetSystemCursor(cursor: *mut c_void, id: u32) -> i32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetLastError() -> u32;
}

#[cfg(windows)]
#[link(name = "advapi32")]
extern "system" {
    fn RegOpenKeyExW(
        h_key: *mut c_void,
        sub_key: *const u16,
        options: u32,
        sam_desired: u32,
        result: *mut *mut c_void,
    ) -> i32;

    fn RegNotifyChangeKeyValue(
        h_key: *mut c_void,
        watch_subtree: i32,
        notify_filter: u32,
        event: *mut c_void,
        asynchronous: i32,
    ) -> i32;

    fn RegCloseKey(h_key: *mut c_void) -> i32;
}

fn pause() {
    print!("\nPress ENTER to close...");
    let _ = io::stdout().flush();
    let mut s = String::new();
    let _ = io::stdin().read_line(&mut s);
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn command_exists(name: &str) -> bool {
    Command::new("where.exe")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn parse_reg_dword(output: &str, value_name: &str) -> Option<u32> {
    for line in output.lines() {
        if !line.to_ascii_lowercase().contains(&value_name.to_ascii_lowercase()) {
            continue;
        }

        let token = line.split_whitespace().last()?;
        if let Some(hex) = token.strip_prefix("0x").or_else(|| token.strip_prefix("0X")) {
            if let Ok(v) = u32::from_str_radix(hex, 16) {
                return Some(v);
            }
        } else if let Ok(v) = token.parse::<u32>() {
            return Some(v);
        }
    }
    None
}

fn query_reg_dword(key: &str, value_name: &str) -> Option<u32> {
    let out = Command::new("reg.exe")
        .arg("query")
        .arg(key)
        .arg("/v")
        .arg(value_name)
        .output()
        .ok()?;

    if !out.status.success() {
        return None;
    }

    parse_reg_dword(&String::from_utf8_lossy(&out.stdout), value_name)
}

fn current_accessibility_step() -> u32 {
    // Prefer the Settings app's canonical 1..15 slider value. CursorBaseSize can
    // lag behind or contain a much larger raw value after Windows rebuilds the
    // accessibility cursor set, which is what made V5 jump from 32 -> 48 -> ...
    // all the way to 256.
    if let Some(step) = query_reg_dword(
        r"HKCU\Software\Microsoft\Accessibility",
        "CursorSize",
    ) {
        return step.clamp(1, ACCESSIBILITY_STEPS);
    }

    // Fallback for Windows builds where CursorSize is absent. Convert the native
    // 32..256 base-size range back to its nearest 1..15 slider position.
    if let Some(base) = query_reg_dword(r"HKCU\Control Panel\Cursors", "CursorBaseSize") {
        let base = base.clamp(32, 256);
        let rounded_step = ((base.saturating_sub(32) + 8) / 16) + 1;
        return rounded_step.clamp(1, ACCESSIBILITY_STEPS);
    }

    1
}

fn balanced_cursor_size(step: u32) -> u32 {
    let step = step.clamp(1, ACCESSIBILITY_STEPS);
    BALANCED_MIN_SIZE + (step - 1) * BALANCED_STEP_PX
}

fn reg_delete_value(key: &str, value_name: &str) {
    let _ = Command::new("reg.exe")
        .arg("delete")
        .arg(key)
        .arg("/v")
        .arg(value_name)
        .arg("/f")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn reg_add_value(
    key: &str,
    value_name: Option<&str>,
    reg_type: &str,
    data: &str,
) -> Result<(), String> {
    let mut cmd = Command::new("reg.exe");
    cmd.arg("add").arg(key);

    match value_name {
        Some(name) => {
            cmd.arg("/v").arg(name);
        }
        None => {
            cmd.arg("/ve");
        }
    }

    let status = cmd
        .arg("/t")
        .arg(reg_type)
        .arg("/d")
        .arg(data)
        .arg("/f")
        .stdout(Stdio::null())
        .status()
        .map_err(|e| format!("Could not start reg.exe: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "reg.exe failed while writing {}\\{}",
            key,
            value_name.unwrap_or("(Default)")
        ))
    }
}

fn export_backup(backup: &Path, log: &Logger) -> Result<(), String> {
    if backup.exists() {
        log.line(format!(
            "[backup] Keeping existing original backup: {}",
            backup.display()
        ));
        return Ok(());
    }

    log.line("[backup] Saving your current HKCU cursor configuration...");
    let status = Command::new("reg.exe")
        .arg("export")
        .arg(r"HKCU\Control Panel\Cursors")
        .arg(backup)
        .arg("/y")
        .status()
        .map_err(|e| format!("Could not start reg.exe for backup: {e}"))?;

    if !status.success() {
        return Err("Windows could not export the current cursor registry key.".into());
    }

    Ok(())
}

fn restore_backup(backup: &Path, log: &Logger) -> Result<(), String> {
    if !backup.exists() {
        return Err(format!(
            "No original backup exists at:\n{}",
            backup.display()
        ));
    }

    log.line(format!("[restore] Importing {}", backup.display()));
    let status = Command::new("reg.exe")
        .arg("import")
        .arg(backup)
        .status()
        .map_err(|e| format!("Could not start reg.exe for restore: {e}"))?;

    if !status.success() {
        return Err("reg.exe could not import the backup.".into());
    }

    reload_cursors()?;
    log.line("[restore] Original cursor registry configuration restored.");
    Ok(())
}

fn download_asset(asset: Asset, dir: &Path, log: &Logger) -> Result<(), String> {
    let final_path = dir.join(asset.name);

    if final_path.exists() {
        if verify_asset_file(asset, &final_path).is_ok() {
            log.line(format!("[cached+verified] {}", asset.name));
            return Ok(());
        }
        log.line(format!("[replace] Existing {} failed verification.", asset.name));
        let _ = fs::remove_file(&final_path);
    }

    let temp_path = dir.join(format!("{}.part", asset.name));
    let _ = fs::remove_file(&temp_path);

    let url = format!("{REPO_RAW}/{}", asset.name);
    log.line(format!("[download] {}", asset.name));

    let status = Command::new("curl.exe")
        .arg("--fail")
        .arg("--location")
        .arg("--silent")
        .arg("--show-error")
        .arg("--retry")
        .arg("4")
        .arg("--retry-delay")
        .arg("1")
        .arg("--connect-timeout")
        .arg("15")
        .arg("--max-time")
        .arg("90")
        .arg("--output")
        .arg(&temp_path)
        .arg(&url)
        .status()
        .map_err(|e| format!("Could not launch curl.exe: {e}"))?;

    if !status.success() {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "Download failed for {}.\nURL: {}",
            asset.name, url
        ));
    }

    if let Err(e) = verify_asset_file(asset, &temp_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("Integrity check failed after downloading {}:\n{}", asset.name, e));
    }

    fs::rename(&temp_path, &final_path).map_err(|e| {
        format!(
            "Could not move {} into the install folder: {e}",
            asset.name
        )
    })?;

    Ok(())
}

fn download_all(dir: &Path, log: &Logger) -> Result<(), String> {
    if !command_exists("curl.exe") {
        return Err(
            "curl.exe was not found. Windows 11 normally includes it in System32.".into(),
        );
    }

    for (i, asset) in ASSETS.iter().copied().enumerate() {
        log.line(format!(
            "[{}/{}] Preparing {}",
            i + 1,
            ASSETS.len(),
            asset.name
        ));
        download_asset(asset, dir, log)?;
    }
    Ok(())
}

fn file_path(dir: &Path, name: &str) -> String {
    dir.join(name).to_string_lossy().into_owned()
}

fn preset_role_paths(preset: Preset, dir: &Path) -> Vec<String> {
    let (arrow, help, working) = match preset {
        Preset::Regular => ("arrow.cur", "help.cur", "working.ani"),
        Preset::NoTail => ("arrow_no_tail.cur", "help_no_tail.cur", "working_no_tail.ani"),
        Preset::TailDetached => (
            "arrow_tail_detached.cur",
            "help_tail_detached.cur",
            "working_tail_detached_v3.ani",
        ),
        Preset::Compact => (
            "arrow_no_tail_smaller_v2.cur",
            "help_no_tail.cur",
            "working_tail_detached_v3.ani",
        ),
    };

    vec![
        file_path(dir, arrow),                       // Arrow
        file_path(dir, help),                        // Help
        file_path(dir, working),                     // AppStarting
        file_path(dir, "busy.ani"),                  // Wait
        file_path(dir, "cross.cur"),                 // Crosshair
        file_path(dir, "beam_v2.cur"),               // IBeam
        file_path(dir, "pen.cur"),                   // NWPen
        file_path(dir, "unavailable_v3.cur"),        // No
        file_path(dir, "vertical_v2.cur"),           // SizeNS
        file_path(dir, "horizontal_v2.cur"),         // SizeWE
        file_path(dir, "diagonal_1.cur"),            // SizeNWSE
        file_path(dir, "diagonal_2.cur"),            // SizeNESW
        file_path(dir, "move_v2.cur"),               // SizeAll
        file_path(dir, "special.cur"),               // UpArrow
        file_path(dir, "link_v2.cur"),               // Hand
        r"%SystemRoot%\cursors\pin_i.cur".into(),    // Pin
        r"%SystemRoot%\cursors\person_i.cur".into(), // Person
    ]
}

fn register_and_apply(preset: Preset, dir: &Path, log: &Logger) -> Result<(), String> {
    const CURSORS_KEY: &str = r"HKCU\Control Panel\Cursors";
    const SCHEMES_KEY: &str = r"HKCU\Control Panel\Cursors\Schemes";

    let values = preset_role_paths(preset, dir);

    if values.len() != ROLES.len() {
        return Err("Internal error: cursor role count mismatch.".into());
    }

    log.line(format!("[registry] Registering {}", preset.name()));

    // Save the complete scheme so it appears in classic Mouse Properties -> Pointers.
    let scheme_string = values.join(",");
    reg_add_value(
        SCHEMES_KEY,
        Some(preset.name()),
        "REG_EXPAND_SZ",
        &scheme_string,
    )?;

    // Make it the current scheme.
    reg_add_value(CURSORS_KEY, None, "REG_SZ", preset.name())?;

    for (role, value) in ROLES.iter().zip(values.iter()) {
        log.line(format!("[registry] {role} -> {value}"));
        reg_add_value(CURSORS_KEY, Some(role), "REG_EXPAND_SZ", value)?;
    }

    // 0 = custom/current-user source. This avoids marking it as a system-supplied scheme.
    reg_add_value(CURSORS_KEY, Some("Scheme Source"), "REG_DWORD", "0")?;

    match reload_cursors() {
        Ok(()) => log.line("[apply] Windows accepted SPI_SETCURSORS reload."),
        Err(e) => {
            // Keep going: certain Windows 11 builds have had a confirmed bug
            // where SPI_SETCURSORS fails even though custom cursor files load.
            log.line(format!("[apply] SPI_SETCURSORS did not reload the scheme: {e}"));
            log.line("[apply] Falling back to direct SetSystemCursor application.");
        }
    }

    force_live_system_cursors(preset, dir, log)?;
    Ok(())
}

#[cfg(windows)]
fn reload_cursors() -> Result<(), String> {
    const SPI_SETCURSORS: u32 = 0x0057;
    const SPIF_UPDATEINIFILE: u32 = 0x0001;
    const SPIF_SENDCHANGE: u32 = 0x0002;

    let ok = unsafe {
        SystemParametersInfoW(
            SPI_SETCURSORS,
            0,
            std::ptr::null_mut::<c_void>(),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
    };

    if ok == 0 {
        let err = unsafe { GetLastError() };
        Err(format!(
            "SystemParametersInfoW(SPI_SETCURSORS) returned failure (GetLastError={err})."
        ))
    } else {
        Ok(())
    }
}


#[cfg(windows)]
fn load_and_set_system_cursor(
    path: &Path,
    id: u32,
    label: &str,
    cursor_size: u32,
) -> Result<(), String> {
    const IMAGE_CURSOR: u32 = 2;
    const LR_LOADFROMFILE: u32 = 0x00000010;

    let wide: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // LoadImageW lets us request the exact size Windows Accessibility currently
    // uses. Microsoft documents that LoadImage can load both static and animated
    // cursors, including standalone files with LR_LOADFROMFILE.
    let handle = unsafe {
        LoadImageW(
            std::ptr::null_mut(),
            wide.as_ptr(),
            IMAGE_CURSOR,
            cursor_size as i32,
            cursor_size as i32,
            LR_LOADFROMFILE,
        )
    };

    if handle.is_null() {
        let err = unsafe { GetLastError() };
        return Err(format!(
            "LoadImageW failed for {label}: {} at {}px (GetLastError={err})",
            path.display(),
            cursor_size
        ));
    }

    let ok = unsafe { SetSystemCursor(handle, id) };
    if ok == 0 {
        let err = unsafe { GetLastError() };
        return Err(format!(
            "SetSystemCursor failed for {label} / id {id} (GetLastError={err})"
        ));
    }

    Ok(())
}

#[cfg(windows)]
fn force_live_system_cursors(preset: Preset, dir: &Path, log: &Logger) -> Result<(), String> {
    // This directly replaces the current session's system cursor handles.
    // It is deliberately done AFTER registry persistence + SPI_SETCURSORS.
    // It also works around Windows builds where the registry scheme is correct
    // but SPI_SETCURSORS fails to load custom cursors.
    let (arrow, help, working) = match preset {
        Preset::Regular => ("arrow.cur", "help.cur", "working.ani"),
        Preset::NoTail => ("arrow_no_tail.cur", "help_no_tail.cur", "working_no_tail.ani"),
        Preset::TailDetached => (
            "arrow_tail_detached.cur",
            "help_tail_detached.cur",
            "working_tail_detached_v3.ani",
        ),
        Preset::Compact => (
            "arrow_no_tail_smaller_v2.cur",
            "help_no_tail.cur",
            "working_tail_detached_v3.ani",        ),
    };

    let system_root = env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let system_cursors = system_root.join("Cursors");

    // Windows system-cursor IDs. The documented SetSystemCursor IDs cover the
    // common roles; Help/Pin/Person and the legacy pen identifier are also
    // standard cursor resource IDs used by Windows.
    let live: Vec<(&str, PathBuf, u32)> = vec![
        ("Normal Select", dir.join(arrow), 32512),
        ("Text Select", dir.join("beam_v2.cur"), 32513),
        ("Busy", dir.join("busy.ani"), 32514),
        ("Precision Select", dir.join("cross.cur"), 32515),
        ("Alternate Select", dir.join("special.cur"), 32516),
        ("Pen", dir.join("pen.cur"), 32631),
        ("Diagonal Resize 1", dir.join("diagonal_1.cur"), 32642),
        ("Diagonal Resize 2", dir.join("diagonal_2.cur"), 32643),
        ("Horizontal Resize", dir.join("horizontal_v2.cur"), 32644),
        ("Vertical Resize", dir.join("vertical_v2.cur"), 32645),
        ("Move", dir.join("move_v2.cur"), 32646),
        ("Unavailable", dir.join("unavailable_v3.cur"), 32648),
        ("Link Select", dir.join("link_v2.cur"), 32649),
        ("Working In Background", dir.join(working), 32650),
        ("Help Select", dir.join(help), 32651),
        ("Location Select", system_cursors.join("pin_i.cur"), 32671),
        ("Person Select", system_cursors.join("person_i.cur"), 32672),
    ];

    let slider_step = current_accessibility_step();
    let cursor_size = balanced_cursor_size(slider_step);
    log.line(format!(
        "[force-live] Accessibility size {}/15 -> balanced render size {}px.",
        slider_step,
        cursor_size
    ));
    log.line("[force-live] Directly replacing Windows system cursor handles...");
    for (label, path, id) in live {
        // Pin/person may not exist on very old Windows builds. On Windows 11
        // they normally do. Treat them as optional so they cannot block the pack.
        if !path.exists() && (id == 32671 || id == 32672) {
            log.line(format!("[force-live] Optional {label} file missing; skipped."));
            continue;
        }

        load_and_set_system_cursor(&path, id, label, cursor_size)?;
        log.line(format!("[force-live] {label} applied."));
    }

    Ok(())
}

#[cfg(not(windows))]
fn force_live_system_cursors(_preset: Preset, _dir: &Path, _log: &Logger) -> Result<(), String> {
    Err("Direct live cursor replacement is available only on Windows.".into())
}

#[cfg(not(windows))]
fn reload_cursors() -> Result<(), String> {
    Err("This installer can only apply cursor changes on Windows.".into())
}

#[cfg(windows)]
fn stop_existing_guardian() {
    let _ = Command::new("taskkill.exe")
        .arg("/IM")
        .arg(GUARDIAN_EXE_NAME)
        .arg("/F")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    thread::sleep(Duration::from_millis(250));
}

#[cfg(windows)]
fn install_guardian(root: &Path, preset: Preset, log: &Logger) -> Result<(), String> {
    let current_exe =
        env::current_exe().map_err(|e| format!("Could not locate the running EXE: {e}"))?;
    let guardian_exe = root.join(GUARDIAN_EXE_NAME);

    log.line("[guardian] Installing per-user cursor guardian...");
    stop_existing_guardian();

    if current_exe != guardian_exe {
        fs::copy(&current_exe, &guardian_exe).map_err(|e| {
            format!(
                "Could not copy the guardian to {}: {e}",
                guardian_exe.display()
            )
        })?;
    }

    let run_command = format!(
        "\"{}\" --watch --preset {} --no-pause",
        guardian_exe.display(),
        preset.cli_name()
    );

    reg_add_value(
        RUN_KEY,
        Some(GUARDIAN_RUN_VALUE),
        "REG_SZ",
        &run_command,
    )?;

    // CREATE_NO_WINDOW: the guardian runs silently in the user's session.
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    Command::new(&guardian_exe)
        .arg("--watch")
        .arg("--preset")
        .arg(preset.cli_name())
        .arg("--no-pause")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Could not start the cursor guardian: {e}"))?;

    log.line(format!("[guardian] Installed: {}", guardian_exe.display()));
    log.line("[guardian] Starts automatically at sign-in.");
    log.line("[guardian] Watches Windows Accessibility cursor changes.");
    Ok(())
}

#[cfg(windows)]
fn uninstall_guardian(root: &Path, log: &Logger) {
    log.line("[guardian] Removing startup guardian...");
    reg_delete_value(RUN_KEY, GUARDIAN_RUN_VALUE);
    stop_existing_guardian();

    let guardian_exe = root.join(GUARDIAN_EXE_NAME);
    if guardian_exe.exists() {
        if let Err(e) = fs::remove_file(&guardian_exe) {
            log.line(format!(
                "[guardian] Could not delete {} yet: {e}",
                guardian_exe.display()
            ));
        }
    }
}

#[cfg(windows)]
fn watch_accessibility_changes(
    preset: Preset,
    install_dir: &Path,
    log: &Logger,
) -> Result<(), String> {
    const KEY_NOTIFY: u32 = 0x0010;
    const REG_NOTIFY_CHANGE_LAST_SET: u32 = 0x00000004;

    let subkey: Vec<u16> = ACCESSIBILITY_SUBKEY
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // HKEY_CURRENT_USER = (HKEY)(LONG_PTR)0x80000001.
    let hkcu = (-2147483647isize) as usize as *mut c_void;
    let mut key: *mut c_void = std::ptr::null_mut();

    let open = unsafe {
        RegOpenKeyExW(
            hkcu,
            subkey.as_ptr(),
            0,
            KEY_NOTIFY,
            &mut key as *mut *mut c_void,
        )
    };

    if open != 0 || key.is_null() {
        return Err(format!(
            "Guardian could not open HKCU\\{} for change notifications (status={open}).",
            ACCESSIBILITY_SUBKEY
        ));
    }

    log.line("[guardian] Active.");
    log.line("[guardian] Waiting for pointer style/size changes from Windows Settings...");

    // Apply once at guardian startup. This is what makes the setup survive
    // sign-out/sign-in and Windows rebuilding its own accessibility cursor files.
    register_and_apply(preset, install_dir, log)?;

    loop {
        let status = unsafe {
            RegNotifyChangeKeyValue(
                key,
                0,
                REG_NOTIFY_CHANGE_LAST_SET,
                std::ptr::null_mut(),
                0,
            )
        };

        if status != 0 {
            unsafe {
                RegCloseKey(key);
            }
            return Err(format!(
                "RegNotifyChangeKeyValue failed (status={status})."
            ));
        }

        // SystemSettings.exe writes several cursor values in a burst. Waiting
        // briefly lets Windows finish the transaction before we restore our
        // custom cursor paths at the newly selected CursorBaseSize.
        thread::sleep(Duration::from_millis(500));

        log.line(format!(
            "[guardian] Accessibility cursor change detected; reapplying {}...",
            preset.name()
        ));

        if let Err(e) = register_and_apply(preset, install_dir, log) {
            log.line(format!("[guardian] Reapply failed: {e}"));
            // Keep watching. A later Settings change may succeed.
        } else {
            let step = current_accessibility_step();
            log.line(format!(
                "[guardian] Reapplied: slider {}/15 -> {}px balanced size.",
                step,
                balanced_cursor_size(step)
            ));
        }
    }
}

fn choose_preset() -> Result<Preset, String> {
    println!();
    println!("Choose the cursor style:");
    println!("  [1] Regular");
    println!("  [2] No Tail");
    println!("  [3] Tail Detached");
    println!("  [4] Compact  (recommended first)");
    println!();

    loop {
        print!("Enter 1-4: ");
        io::stdout()
            .flush()
            .map_err(|e| format!("Could not flush stdout: {e}"))?;

        let mut s = String::new();
        io::stdin()
            .read_line(&mut s)
            .map_err(|e| format!("Could not read your choice: {e}"))?;

        match s.trim() {
            "1" => return Ok(Preset::Regular),
            "2" => return Ok(Preset::NoTail),
            "3" => return Ok(Preset::TailDetached),
            "4" => return Ok(Preset::Compact),
            _ => println!("Please enter 1, 2, 3, or 4."),
        }
    }
}

fn parse_preset_arg(args: &[String]) -> Result<Option<Preset>, String> {
    for i in 0..args.len() {
        if args[i] == "--preset" {
            let value = args
                .get(i + 1)
                .ok_or_else(|| "--preset requires regular|no-tail|tail-detached|compact".to_string())?;
            return Preset::from_cli(value)
                .map(Some)
                .ok_or_else(|| format!("Unknown preset: {value}"));
        }
    }
    Ok(None)
}

fn run() -> Result<(), String> {
    #[cfg(not(windows))]
    {
        return Err("This program is intended to run on Windows 10/11.".into());
    }

    #[cfg(windows)]
    {
        let args: Vec<String> = env::args().skip(1).collect();
        let no_pause = args.iter().any(|a| a == "--no-pause");

        let local_app_data = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| "LOCALAPPDATA is not defined.".to_string())?;

        let root = local_app_data.join("ModernInvertedCursors");
        let install_dir = root.join(format!("v{VERSION}"));
        fs::create_dir_all(&install_dir)
            .map_err(|e| format!("Could not create {}: {e}", install_dir.display()))?;

        let log_path = root.join(format!("installer-{}.log", timestamp()));
        let log = Logger::new(log_path.clone());
        let backup_path = root.join("backup-original.reg");

        log.line("============================================================");
        log.line(" Modern Inverted Mouse Cursors v3.6.2 - Rust Installer v5.1 (balanced guardian)");
        log.line("============================================================");
        log.line(format!("Install folder: {}", install_dir.display()));
        log.line(format!("Log file:       {}", log_path.display()));
        log.line("");

        if !command_exists("reg.exe") {
            return Err("reg.exe was not found; Windows registry tools are unavailable.".into());
        }

        if args.iter().any(|a| a == "--watch") {
            let preset = parse_preset_arg(&args)?.unwrap_or(Preset::Compact);
            return watch_accessibility_changes(preset, &install_dir, &log);
        }

        if args.iter().any(|a| a == "--restore") {
            uninstall_guardian(&root, &log);
            restore_backup(&backup_path, &log)?;
            log.line("");
            log.line("Restore complete.");
            if !no_pause {
                pause();
            }
            return Ok(());
        }

        let preset = match parse_preset_arg(&args)? {
            Some(p) => p,
            None => choose_preset()?,
        };

        log.line(format!("Selected preset: {}", preset.name()));
        log.line("");

        export_backup(&backup_path, &log)?;

        log.line("");
        log.line("[download] Fetching exact public v3.6.2 cursor assets...");
        download_all(&install_dir, &log)?;

        log.line("");
        log.line("[apply] Writing the Windows cursor scheme...");
        register_and_apply(preset, &install_dir, &log)?;

        log.line("");
        install_guardian(&root, preset, &log)?;

        log.line("");
        log.line("SUCCESS.");
        log.line(format!("Applied: {}", preset.name()));
        log.line(format!("Files:   {}", install_dir.display()));
        log.line(format!("Backup:  {}", backup_path.display()));
        log.line("");
        log.line("Restore later with:");
        log.line("  ModernInvertedInstallerV5_1.exe --restore");
        log.line("");
        log.line("NATIVE INTEGRATION:");
        log.line("  The per-user guardian is now running silently.");
        log.line("  Windows' 15-position Accessibility size slider is normalized");
        log.line("  to a balanced 32..88px range in 4px steps.");
        log.line("  This removes the huge 32->48 first jump and prevents the");
        log.line("  256px maximum that made the XOR cursor extremely pixelated.");
        log.line("  Windows may briefly load its stock cursor after a slider move;");
        log.line("  the guardian then reapplies this pack at the balanced new size.");
        log.line("  The guardian also starts automatically when you sign in.");

        if !no_pause {
            pause();
        }

        Ok(())
    }
}

fn main() {
    // Prevent a double-clicked EXE from vanishing on a Rust panic.
    std::panic::set_hook(Box::new(|info| {
        eprintln!("\nFATAL RUST PANIC\n================");
        eprintln!("{info}");
        eprintln!("\nThe installer did not intentionally hide this error.");
        pause();
    }));

    if let Err(e) = run() {
        eprintln!("\nINSTALLER ERROR\n===============");
        eprintln!("{e}");
        eprintln!("\nNothing else will be attempted after this failure.");
        pause();
        std::process::exit(1);
    }
}