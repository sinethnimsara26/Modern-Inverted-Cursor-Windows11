use crate::cursors::reload_cursors;
use crate::registry::{
    reg_get_string, reg_set_dword, reg_set_string, BoundedLogger, CREATE_NO_WINDOW, CURSORS_SUBKEY,
};
use std::{fs, os::windows::process::CommandExt, path::Path, process::Command};

const MAX_BACKUP_BYTES: usize = 512 * 1024;

pub fn parse_reg_file_text(bytes: &[u8]) -> Result<String, String> {
    if bytes.is_empty() {
        return Err("Backup file is empty (0 bytes).".into());
    }
    if bytes.len() > MAX_BACKUP_BYTES {
        return Err(format!(
            "Backup file exceeds maximum expected size ({} bytes).",
            bytes.len()
        ));
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let body = &bytes[2..];
        if body.len() % 2 != 0 {
            return Err("Backup UTF-16LE registry file has truncated trailing byte.".into());
        }
        let words: Vec<u16> = body
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16(&words)
            .map_err(|_| "Backup file contains invalid UTF-16LE data.".into());
    }
    let utf8_slice = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    String::from_utf8(utf8_slice.to_vec())
        .map_err(|_| "Backup file is neither UTF-16LE .reg nor valid UTF-8 text.".into())
}

pub fn validate_cursor_backup_bytes(bytes: &[u8]) -> Result<(), String> {
    let text = parse_reg_file_text(bytes)?;
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with(';'));

    let header = lines
        .next()
        .ok_or_else(|| "Backup file has no Windows Registry header.".to_string())?;
    let header_ok = header.eq_ignore_ascii_case("Windows Registry Editor Version 5.00")
        || header.eq_ignore_ascii_case("REGEDIT4");
    if !header_ok {
        return Err(format!("Unexpected .reg backup header: {header}"));
    }

    let mut saw_cursors_section = false;
    let mut saw_entry = false;
    for line in lines {
        if line.starts_with('[') && line.ends_with(']') {
            let section_lower = line[1..line.len() - 1].trim().to_ascii_lowercase();
            let valid_scope = section_lower == r"hkey_current_user\control panel\cursors"
                || section_lower.starts_with(r"hkey_current_user\control panel\cursors\");
            if !valid_scope {
                return Err(format!(
                    "Backup file contains disallowed registry section [{section_lower}]."
                ));
            }
            if section_lower == r"hkey_current_user\control panel\cursors" {
                saw_cursors_section = true;
            }
        } else if saw_cursors_section && line.contains('=') {
            saw_entry = true;
        }
    }

    if !saw_cursors_section || !saw_entry {
        return Err("Backup file does not contain a valid Control Panel\\Cursors key.".into());
    }
    Ok(())
}

pub fn validate_backup_file(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!(
            "No original cursor backup exists at:\n{}",
            path.display()
        ));
    }
    let meta = fs::metadata(path)
        .map_err(|e| format!("Could not inspect backup {}: {e}", path.display()))?;
    if meta.len() > MAX_BACKUP_BYTES as u64 {
        return Err(format!(
            "Backup file {} is unexpectedly large ({} bytes).",
            path.display(),
            meta.len()
        ));
    }
    let bytes =
        fs::read(path).map_err(|e| format!("Could not read backup {}: {e}", path.display()))?;
    validate_cursor_backup_bytes(&bytes)
        .map_err(|reason| format!("Cursor backup {} is invalid: {reason}", path.display()))
}

pub fn export_backup(backup: &Path, log: Option<&BoundedLogger>) -> Result<(), String> {
    if backup.exists() {
        match validate_backup_file(backup) {
            Ok(()) => {
                if let Some(l) = log {
                    l.line(format!(
                        "[backup] Keeping existing valid {}",
                        backup.display()
                    ));
                }
                return Ok(());
            }
            Err(reason) => {
                let current_arrow =
                    reg_get_string(CURSORS_SUBKEY, Some("Arrow")).unwrap_or_default();
                if current_arrow
                    .to_ascii_lowercase()
                    .contains("moderninvertedcursors")
                {
                    return Err(format!(
                        "Existing backup is invalid ({reason}) and current cursors already point to Modern Inverted Cursors; refusing to overwrite backup with inverted paths."
                    ));
                }
                if let Some(l) = log {
                    l.line(format!(
                        "[backup] Existing backup was invalid ({reason}); re-exporting pre-install cursor scheme."
                    ));
                }
            }
        }
    }
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    let temp_backup = backup.with_extension("reg.tmp");
    let _ = fs::remove_file(&temp_backup);
    let out = Command::new("reg.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["export", r"HKCU\Control Panel\Cursors"])
        .arg(&temp_backup)
        .arg("/y")
        .output()
        .map_err(|e| format!("Could not start reg.exe for backup: {e}"))?;
    if !out.status.success() {
        let _ = fs::remove_file(&temp_backup);
        let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!(
            "Windows could not export the current cursor registry key ({detail})."
        ));
    }
    validate_backup_file(&temp_backup).map_err(|e| {
        let _ = fs::remove_file(&temp_backup);
        e
    })?;
    fs::rename(&temp_backup, backup).map_err(|e| {
        let _ = fs::remove_file(&temp_backup);
        format!("Could not finalize backup {}: {e}", backup.display())
    })?;
    if let Some(l) = log {
        l.line(format!(
            "[backup] Saved & verified original cursors to {}",
            backup.display()
        ));
    }
    Ok(())
}

pub fn restore_backup(backup: &Path, log: Option<&BoundedLogger>) -> Result<(), String> {
    validate_backup_file(backup)?;
    let out = Command::new("reg.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["import"])
        .arg(backup)
        .output()
        .map_err(|e| format!("Could not start reg.exe for restore: {e}"))?;
    if !out.status.success() {
        let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!(
            "reg.exe could not import original cursor backup {} ({detail}).",
            backup.display()
        ));
    }
    if let Err(err) = reload_cursors() {
        let arrow = reg_get_string(CURSORS_SUBKEY, Some("Arrow")).unwrap_or_default();
        if arrow.to_ascii_lowercase().contains("moderninvertedcursors") {
            return Err(format!("Cursor reload after restore failed: {err}"));
        }
    }
    if let Some(l) = log {
        l.line("[restore] Original Windows cursor configuration restored and verified.");
    }
    Ok(())
}

pub fn reset_windows_default_cursors(log: Option<&BoundedLogger>) -> Result<(), String> {
    const STANDARD_DEFAULTS: [(&str, &str); 17] = [
        ("Arrow", r"%SystemRoot%\cursors\aero_arrow.cur"),
        ("Help", r"%SystemRoot%\cursors\aero_helpsel.cur"),
        ("AppStarting", r"%SystemRoot%\cursors\aero_working.ani"),
        ("Wait", r"%SystemRoot%\cursors\aero_busy.ani"),
        ("Crosshair", ""),
        ("IBeam", ""),
        ("NWPen", r"%SystemRoot%\cursors\aero_pen.cur"),
        ("No", r"%SystemRoot%\cursors\aero_unavail.cur"),
        ("SizeNS", r"%SystemRoot%\cursors\aero_ns.cur"),
        ("SizeWE", r"%SystemRoot%\cursors\aero_ew.cur"),
        ("SizeNWSE", r"%SystemRoot%\cursors\aero_nwse.cur"),
        ("SizeNESW", r"%SystemRoot%\cursors\aero_nesw.cur"),
        ("SizeAll", r"%SystemRoot%\cursors\aero_move.cur"),
        ("UpArrow", r"%SystemRoot%\cursors\aero_up.cur"),
        ("Hand", r"%SystemRoot%\cursors\aero_link.cur"),
        ("Pin", r"%SystemRoot%\cursors\aero_pin.cur"),
        ("Person", r"%SystemRoot%\cursors\aero_person.cur"),
    ];
    reg_set_string(CURSORS_SUBKEY, None, false, "Windows Default")?;
    for (role, val) in STANDARD_DEFAULTS {
        reg_set_string(CURSORS_SUBKEY, Some(role), true, val)?;
    }
    reg_set_dword(CURSORS_SUBKEY, "Scheme Source", 2)?;
    reload_cursors()?;
    if let Some(l) = log {
        l.line("[restore] Reset HKCU cursors to built-in Windows Default scheme.");
    }
    Ok(())
}

pub fn perform_validated_restore<FStop, FClean>(
    backup: &Path,
    log: Option<&BoundedLogger>,
    stop_guardians: FStop,
    cleanup_after_restore: FClean,
) -> Result<(), String>
where
    FStop: FnOnce() -> Result<usize, String>,
    FClean: FnOnce() -> Result<(), String>,
{
    validate_backup_file(backup)?;
    stop_guardians()?;
    restore_backup(backup, log)?;
    cleanup_after_restore()?;
    Ok(())
}
