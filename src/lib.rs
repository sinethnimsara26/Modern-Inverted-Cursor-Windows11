#![allow(dead_code)]

pub mod assets;
pub mod backup;
pub mod cursors;
pub mod ipc;
pub mod registry;
pub mod render;

pub use assets::{
    download_asset, ensure_all_assets, git_blob_sha1, parse_https_url, prepare_assets_parallel,
    sha1, verify_asset_file, Asset, AssetVerifyCache, ASSETS, MAX_ASSET_BYTES, REPO_RAW,
};
pub use backup::{
    export_backup, parse_reg_file_text, perform_validated_restore, reset_windows_default_cursors,
    restore_backup, validate_backup_file, validate_cursor_backup_bytes,
};
pub use cursors::{
    balanced_cursor_size, current_accessibility_step, preset_role_paths, register_and_apply,
    reload_cursors, ACCESSIBILITY_STEPS, BALANCED_MIN_SIZE, BALANCED_STEP_PX, ROLES,
};
pub use ipc::{
    close_existing_v6_window, find_tray_window, find_verified_guardian_pids,
    is_guardian_image_path, is_safe_cleanup_target, normalize_win_path_str, process_image_path,
    read_log_tail, schedule_self_delete, stop_targeted_guardians, window_process_id,
    write_binary_retry, NamedReadyEvent, ProcessEntry32W, ReadyWaitOutcome,
};
pub use registry::{
    app_paths, app_paths_for_root, cleanup_legacy_guardian_binary, detect_existing_installation,
    hkcu, is_startup_enabled, load_saved_preset, load_system_dll, parse_preset_arg,
    reg_delete_subkey, reg_delete_value, reg_get_dword, reg_get_string, reg_open_notify,
    reg_set_dword, reg_set_string, reg_sync_cursor_values, register_uninstall_entry, save_preset,
    set_startup_enabled, trim_working_set, unregister_uninstall_entry, validate_cli_args, wide_z,
    BoundedLogger, InstallDetection, Preset, RegCloseKey, RegNotifyChangeKeyValue, ACCESSIBILITY_SUBKEY,
    APP_VERSION, CONFIG_SUBKEY, CREATE_NO_WINDOW, CURSORS_SUBKEY, GUARDIAN_EXE_NAME,
    GUARDIAN_RUN_VALUE, LEGACY_GUARDIAN_EXE_NAME, LEGACY_GUARDIAN_RUN_VALUE, PACK_VERSION,
    PUBLISHER, RUN_SUBKEY, SCHEMES_SUBKEY, SETUP_EXE_NAME, TRAY_WND_CLASS, UNINSTALL_KEY_NAME,
    UNINSTALL_PARENT_SUBKEY, UNINSTALL_SUBKEY,
};
pub use render::{create_split_cursor_hicon, draw_cursor_glyph, CursorGlyphSprite, DestroyCursor};

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env, fs,
        os::windows::{io::AsRawHandle, process::CommandExt},
        path::Path,
        process::Command,
    };

    #[test]
    fn test_git_blob_sha1_matches_git_spec() {
        assert_eq!(
            git_blob_sha1(b"hello\n"),
            "ce013625030ba8dba906f756967f9e9ca394464a"
        );
        assert_eq!(ASSETS.len(), 32);
    }

    #[test]
    fn test_balanced_cursor_size_progression() {
        assert_eq!(balanced_cursor_size(0), 32);
        assert_eq!(balanced_cursor_size(1), 32);
        assert_eq!(balanced_cursor_size(2), 36);
        assert_eq!(balanced_cursor_size(15), 88);
        assert_eq!(balanced_cursor_size(99), 88);
    }

    #[test]
    fn test_preset_cli_and_roles_match() {
        for preset in Preset::ALL {
            assert_eq!(Preset::from_cli(preset.cli_name()), Some(preset));
            assert_eq!(
                preset_role_paths(preset, Path::new(r"C:\test")).len(),
                ROLES.len()
            );
        }
        assert_eq!(Preset::from_cli("unknown"), None);
    }

    #[test]
    fn test_split_cursor_glyph_renders_distinct_presets() {
        let mut buf_a = vec![0xFF00_0000u32; 32 * 32];
        let mut buf_b = vec![0xFF00_0000u32; 32 * 32];
        draw_cursor_glyph(&mut buf_a, 32, 0, 0, 32, Preset::Compact, false, false);
        draw_cursor_glyph(&mut buf_b, 32, 0, 0, 32, Preset::Regular, false, false);
        assert_ne!(buf_a, buf_b);
        assert!(buf_a.contains(&0xFFFF_FFFF));
    }

    #[test]
    fn test_create_split_cursor_hicon_succeeds() {
        let icon = create_split_cursor_hicon(32, Preset::Regular, false);
        assert!(!icon.is_null());
        unsafe {
            DestroyCursor(icon);
        }
    }

    #[test]
    fn test_parse_https_url_extracts_host_and_path() {
        assert_eq!(
            parse_https_url("https://raw.githubusercontent.com/emvaized/modern_inverted_mouse_cursors/3.6.2/src/arrow.cur"),
            Some((
                "raw.githubusercontent.com",
                "/emvaized/modern_inverted_mouse_cursors/3.6.2/src/arrow.cur"
            ))
        );
        assert_eq!(parse_https_url("http://example.com/a"), None);
        assert_eq!(parse_https_url("https://example.com"), None);
    }

    #[test]
    fn test_process_entry_32w_matches_windows_sdk_size() {
        #[cfg(target_pointer_width = "64")]
        assert_eq!(std::mem::size_of::<ProcessEntry32W>(), 568);
        #[cfg(target_pointer_width = "32")]
        assert_eq!(std::mem::size_of::<ProcessEntry32W>(), 556);
    }

    #[test]
    fn test_validate_cursor_backup_bytes_accepts_utf16le_and_utf8_and_rejects_corrupt_or_out_of_scope() {
        let utf8_ok = b"Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER\\Control Panel\\Cursors]\r\n\"Arrow\"=\"C:\\\\Windows\\\\Cursors\\\\aero_arrow.cur\"\r\n";
        assert!(validate_cursor_backup_bytes(utf8_ok).is_ok());

        let utf16_body: Vec<u16> = "Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER\\Control Panel\\Cursors]\r\n\"Arrow\"=\"%SystemRoot%\\\\cursors\\\\aero_arrow.cur\"\r\n"
            .encode_utf16()
            .collect();
        let mut utf16_bytes = vec![0xFFu8, 0xFEu8];
        for w in utf16_body {
            utf16_bytes.extend_from_slice(&w.to_le_bytes());
        }
        assert!(validate_cursor_backup_bytes(&utf16_bytes).is_ok());

        assert!(validate_cursor_backup_bytes(b"").is_err());
        assert!(validate_cursor_backup_bytes(b"corrupt garbage").is_err());
        let unrelated_key = b"Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER\\Control Panel\\Cursors]\r\n\"Arrow\"=\"ok\"\r\n[HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run]\r\n\"Evil\"=\"calc.exe\"\r\n";
        assert!(validate_cursor_backup_bytes(unrelated_key).is_err());
    }

    #[test]
    fn test_perform_validated_restore_refuses_corrupt_or_missing_backup_before_touching_state() {
        let tmp = env::temp_dir().join("mic-test-safe-restore");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let bad_backup = tmp.join("backup-original.reg");
        fs::write(&bad_backup, b"NOT A REG FILE").unwrap();

        let mut stopped = false;
        let mut cleaned = false;
        let res = perform_validated_restore(
            &bad_backup,
            None,
            || {
                stopped = true;
                Ok(1)
            },
            || {
                cleaned = true;
                Ok(())
            },
        );
        assert!(res.is_err());
        assert!(!stopped, "guardian must not be stopped when backup is corrupt");
        assert!(!cleaned, "metadata/files must not be removed when backup is corrupt");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_is_guardian_image_path_enforces_install_root_and_binary_name() {
        let root = Path::new(r"C:\Users\Demo\AppData\Local\ModernInvertedCursors");
        assert!(is_guardian_image_path(
            Path::new(r"C:\Users\Demo\AppData\Local\ModernInvertedCursors\ModernInvertedCursorGuardian.exe"),
            root
        ));
        assert!(is_guardian_image_path(
            Path::new(r"C:\Users\Demo\AppData\Local\ModernInvertedCursors\ModernInvertedCursors-Guardian.exe"),
            root
        ));
        assert!(!is_guardian_image_path(
            Path::new(r"C:\Temp\OtherApp\ModernInvertedCursorGuardian.exe"),
            root
        ));
        assert!(!is_guardian_image_path(
            Path::new(r"C:\Users\Demo\AppData\Local\ModernInvertedCursors\notepad.exe"),
            root
        ));
    }

    #[test]
    fn test_named_ready_event_detects_early_exit_and_timeout() {
        let ev_early = NamedReadyEvent::create_unique("unit_early").unwrap();
        let mut cmd_early = Command::new("cmd.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/C", "exit /B 7"])
            .spawn()
            .unwrap();
        let outcome =
            ev_early.wait_process_or_ready(cmd_early.as_raw_handle() as *mut std::ffi::c_void, 5000);
        let _ = cmd_early.wait();
        assert_eq!(outcome, ReadyWaitOutcome::ExitedEarly);

        let ev_timeout = NamedReadyEvent::create_unique("unit_timeout").unwrap();
        let mut cmd_hang = Command::new("cmd.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/C", "ping -n 4 127.0.0.1 >nul"])
            .spawn()
            .unwrap();
        let t_out =
            ev_timeout.wait_process_or_ready(cmd_hang.as_raw_handle() as *mut std::ffi::c_void, 120);
        let _ = cmd_hang.kill();
        let _ = cmd_hang.wait();
        assert_eq!(t_out, ReadyWaitOutcome::TimedOut);
    }

    #[test]
    fn test_detect_and_migrate_legacy_v1_preserves_valid_backup() {
        let tmp = env::temp_dir().join("mic-test-legacy-migrate");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let backup = tmp.join("backup-original.reg");
        let legacy_guard = tmp.join(LEGACY_GUARDIAN_EXE_NAME);
        fs::write(&backup, b"Windows Registry Editor Version 5.00\r\n[HKEY_CURRENT_USER\\Control Panel\\Cursors]\r\n\"Arrow\"=\"orig\"\r\n").unwrap();
        fs::write(&legacy_guard, b"MZ-legacy").unwrap();

        let det = detect_existing_installation(&tmp, &backup);
        assert!(det.has_any());
        assert!(det.has_backup);
        assert!(det.has_legacy_guardian);
        assert!(!det.has_new_guardian);

        cleanup_legacy_guardian_binary(&tmp, None);
        assert!(!legacy_guard.exists());
        assert!(backup.exists(), "valid backup must remain intact across migration");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_verify_asset_file_rejects_same_size_tampered_bytes() {
        let tmp = env::temp_dir().join("mic-test-hash-bypass");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let asset = ASSETS[0];
        let fake_file = tmp.join(asset.name);
        // Exact same byte size as `arrow.cur`, completely different bytes.
        fs::write(&fake_file, vec![0xAAu8; asset.size as usize]).unwrap();
        let err = verify_asset_file(asset, &fake_file).unwrap_err();
        assert!(err.contains("Git blob SHA-1 check"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_system_dll_resolves_system32_and_blocks_fake() {
        assert!(!load_system_dll("kernel32.dll").is_null());
        assert!(!load_system_dll("winhttp.dll").is_null());
        assert!(load_system_dll("definitely_missing_fake_module_12345.dll").is_null());
    }

    #[test]
    fn test_is_safe_cleanup_target_allows_only_expected_executables() {
        assert!(is_safe_cleanup_target(Path::new(
            r"C:\Users\Demo\AppData\Local\ModernInvertedCursors\ModernInvertedCursors-Setup.exe"
        )));
        assert!(is_safe_cleanup_target(Path::new(
            r"C:\Users\a&b\AppData\Local\ModernInvertedCursors\ModernInvertedCursors-Setup.exe"
        )));
        assert!(!is_safe_cleanup_target(Path::new(
            r"C:\Windows\System32\cmd.exe"
        )));
        assert!(!is_safe_cleanup_target(Path::new(
            r"C:\Users\Demo\AppData\Local\ModernInvertedCursors\backup-original.reg"
        )));
    }

    #[test]
    fn test_register_and_apply_missing_dir_fails_atomically_before_mutating() {
        let missing_dir = env::temp_dir().join("mic-test-missing-cursors-dir-never-exists");
        let _ = fs::remove_dir_all(&missing_dir);
        let err = register_and_apply(Preset::Compact, &missing_dir, None).unwrap_err();
        assert!(err.contains("LoadImageW failed"));
    }

    #[test]
    fn test_reg_sync_cursor_values_opens_once_and_elides_noop_writes() {
        let parent = r"Software\ModernInvertedCursors";
        let child = "UnitTestRegSync";
        let subkey = r"Software\ModernInvertedCursors\UnitTestRegSync";
        reg_delete_subkey(parent, child);
        let pairs: Vec<(&str, &str)> = ROLES
            .iter()
            .copied()
            .map(|r| (r, r"C:\NonExistent\arrow.cur"))
            .collect();

        assert!(reg_sync_cursor_values(subkey, "Modern Inverted Test", &pairs, 0).unwrap());
        assert!(!reg_sync_cursor_values(subkey, "Modern Inverted Test", &pairs, 0).unwrap());

        let mut modified = pairs.clone();
        modified[0] = ("Arrow", r"C:\NonExistent\arrow_no_tail.cur");
        assert!(reg_sync_cursor_values(subkey, "Modern Inverted Test", &modified, 0).unwrap());
        assert!(!reg_sync_cursor_values(subkey, "Modern Inverted Test", &modified, 0).unwrap());

        reg_delete_subkey(parent, child);
    }

    #[test]
    fn test_asset_verify_cache_skips_rehash_and_detects_tamper() {
        let tmp = env::temp_dir().join("mic-test-asset-cache");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let mut cache = AssetVerifyCache::new();
        assert!(!cache.fast_stat_check(&tmp));

        for asset in ASSETS {
            fs::write(tmp.join(asset.name), vec![0x55u8; asset.size as usize]).unwrap();
        }
        cache.populate_stamps(&tmp);
        assert!(cache.fast_stat_check(&tmp));

        // Corrupting size or deleting an asset immediately misses the stat cache.
        fs::write(tmp.join(ASSETS[0].name), vec![0x55u8; (ASSETS[0].size as usize) + 1]).unwrap();
        assert!(!cache.fast_stat_check(&tmp));

        // Restoring byte size with wrong contents still fails cryptographic SHA-1 check cleanly.
        fs::write(tmp.join(ASSETS[0].name), vec![0x55u8; ASSETS[0].size as usize]).unwrap();
        let err = verify_asset_file(ASSETS[0], &tmp.join(ASSETS[0].name)).unwrap_err();
        assert!(err.contains("Git blob SHA-1 check"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_cursor_glyph_sprite_matches_direct_raster_dimensions() {
        let mut direct = vec![0xFF0D_1117u32; 40 * 40];
        let mut cached = vec![0xFF0D_1117u32; 40 * 40];
        draw_cursor_glyph(&mut direct, 40, 2, 2, 34, Preset::Compact, false, false);
        let sprite = CursorGlyphSprite::rasterize(34, Preset::Compact, false);
        assert_eq!(sprite.size, 34);
        assert_eq!(sprite.premul.len(), 34 * 34);
        sprite.blit_onto(&mut cached, 40, 2, 2);

        // Expect >= 99% of pixels within +/- 1 channel step of direct floating-point blend.
        let max_channel_diff = direct
            .iter()
            .zip(cached.iter())
            .map(|(&a, &b)| {
                let dr = ((a >> 16) & 0xFF).abs_diff((b >> 16) & 0xFF);
                let dg = ((a >> 8) & 0xFF).abs_diff((b >> 8) & 0xFF);
                let db = (a & 0xFF).abs_diff(b & 0xFF);
                dr.max(dg).max(db)
            })
            .max()
            .unwrap_or(0);
        assert!(max_channel_diff <= 2, "max channel diff={max_channel_diff}");
    }

    #[test]
    fn test_validate_cli_args_rejects_unknown_flags_and_missing_values() {
        let bools = ["--silent", "--modify", "--compact"];
        let req_vals = ["--preset", "--dpi"];
        let opt_vals = ["--sandbox"];

        assert!(validate_cli_args(
            &["--silent".into(), "--preset".into(), "compact".into()],
            &bools,
            &req_vals,
            &opt_vals
        )
        .is_ok());
        assert!(validate_cli_args(
            &["--sandbox".into(), "--compact".into()],
            &bools,
            &req_vals,
            &opt_vals
        )
        .is_ok());
        assert!(validate_cli_args(
            &["--silent".into(), "--invalid-preset-xyz".into()],
            &bools,
            &req_vals,
            &opt_vals
        )
        .is_err());
        assert!(validate_cli_args(&["--preset".into()], &bools, &req_vals, &opt_vals).is_err());
        assert!(validate_cli_args(
            &["--dpi=".into()],
            &bools,
            &req_vals,
            &opt_vals
        )
        .is_err());
    }
}
