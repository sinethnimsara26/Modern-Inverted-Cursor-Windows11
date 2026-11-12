use modern_inverted_cursor::{
    cleanup_legacy_guardian_binary, export_backup, perform_validated_restore, validate_cli_args,
    validate_cursor_backup_bytes, BoundedLogger, Preset, ASSETS, ROLES,
};
use std::{
    env, fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

fn fresh_temp_dir(label: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("mic-it-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_simulated_v1_0_2_migration_preserves_valid_backup_and_cleans_legacy_exe() {
    let root = fresh_temp_dir("v102-migrate");
    let backup = root.join("backup-original.reg");
    let valid_reg = b"Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER\\Control Panel\\Cursors]\r\n@=\"Windows Default\"\r\n\"Arrow\"=\"C:\\\\Windows\\\\Cursors\\\\aero_arrow.cur\"\r\n";
    fs::write(&backup, valid_reg).unwrap();

    let legacy_exe = root.join("ModernInvertedCursorGuardian.exe");
    fs::write(&legacy_exe, b"MZ-legacy-v102").unwrap();

    let log = BoundedLogger::new(root.join("guardian.log"), false);
    // Updating over existing v1.0.2 installation must keep existing valid backup untouched.
    export_backup(&backup, Some(&log)).unwrap();
    assert_eq!(fs::read(&backup).unwrap(), valid_reg);

    cleanup_legacy_guardian_binary(&root, Some(&log));
    assert!(!legacy_exe.exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_corrupt_truncated_and_out_of_scope_backups_abort_restore_preserving_state() {
    let root = fresh_temp_dir("bad-backups");
    let guardian = root.join("ModernInvertedCursors-Guardian.exe");
    fs::write(&guardian, b"MZ-active").unwrap();
    let log = BoundedLogger::new(root.join("guardian.log"), false);

    let cases: &[(&str, &[u8])] = &[
        ("empty", b""),
        ("truncated", b"Windows Registry Editor Version 5.00\r\n"),
        (
            "out_of_scope",
            b"Windows Registry Editor Version 5.00\r\n\r\n[HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Services\\Bad]\r\n\"Start\"=dword:00000001\r\n",
        ),
    ];

    for (name, payload) in cases {
        let backup = root.join(format!("backup-{name}.reg"));
        fs::write(&backup, payload).unwrap();
        let stop_called = AtomicBool::new(false);
        let cleanup_called = AtomicBool::new(false);

        let err = perform_validated_restore(
            &backup,
            Some(&log),
            || {
                stop_called.store(true, Ordering::SeqCst);
                Ok(0)
            },
            || {
                cleanup_called.store(true, Ordering::SeqCst);
                let _ = fs::remove_file(&guardian);
                Ok(())
            },
        )
        .unwrap_err();

        assert!(
            !err.is_empty(),
            "Expected non-empty error for corrupt backup case {name}"
        );
        assert!(
            !stop_called.load(Ordering::SeqCst),
            "stop_fn must not run on invalid backup {name}"
        );
        assert!(
            !cleanup_called.load(Ordering::SeqCst),
            "cleanup_fn must not run on invalid backup {name}"
        );
        assert!(guardian.exists(), "Guardian binary destroyed on {name}");
        assert!(backup.exists(), "Backup file destroyed on {name}");
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_utf16le_cursor_backup_parses_cleanly_and_assets_roles_stay_complete() {
    let text = "Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER\\Control Panel\\Cursors]\r\n\"Arrow\"=\"C:\\\\Windows\\\\Cursors\\\\aero_arrow.cur\"\r\n\"Scheme Source\"=dword:00000002\r\n";
    let mut utf16le = vec![0xFFu8, 0xFEu8];
    for unit in text.encode_utf16() {
        utf16le.extend_from_slice(&unit.to_le_bytes());
    }
    assert!(validate_cursor_backup_bytes(&utf16le).is_ok());

    assert_eq!(ASSETS.len(), 32);
    assert_eq!(ROLES.len(), 17);
    assert_eq!(Preset::ALL.len(), 4);
    assert!(validate_cli_args(
        &["--silent".into(), "--preset".into(), "compact".into()],
        &["--silent"],
        &["--preset"],
        &["--sandbox"],
    )
    .is_ok());
}
