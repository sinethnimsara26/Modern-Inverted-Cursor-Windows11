#![windows_subsystem = "windows"]

mod install;
mod ui;

use install::{execute_install_pipeline, execute_uninstall_pipeline, GUARDIAN_EXE_BYTES};
use modern_inverted_cursor::{
    app_paths, app_paths_for_root, detect_existing_installation, load_saved_preset,
    parse_preset_arg, perform_validated_restore, reg_delete_value, reset_windows_default_cursors,
    stop_targeted_guardians, unregister_uninstall_entry, validate_cli_args, wide_z, BoundedLogger,
    Preset, CONFIG_SUBKEY, GUARDIAN_EXE_NAME, GUARDIAN_RUN_VALUE, LEGACY_GUARDIAN_RUN_VALUE,
    RUN_SUBKEY,
};
use std::{
    env,
    ffi::c_void,
    fs,
    io::Write,
    path::{Path, PathBuf},
    ptr::null_mut,
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc, Mutex,
    },
};
use ui::{
    bring_existing_setup_to_front, create_setup_window, new_gui_state, run_smoke_ui, Backbuffer,
    CloseHandle, CreateMutexW, DestroyIcon, DispatchMessageW, FontSet, GetLastError, GetMessageW,
    GlyphSpriteCache, MessageBoxW, Msg, Rect, TranslateMessage,
};

pub const TIMER_ANIM: usize = 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ControlId {
    PresetCard(usize),
    ToggleStartup,
    ToggleUninstall,
    BtnInstall,
    BtnRestore,
    BtnOpenFolder,
    BtnFinish,
}

#[derive(Clone)]
pub struct HitZone {
    pub id: ControlId,
    pub rect: Rect,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Setup,
    Working,
    Done,
}

#[derive(Clone)]
pub struct WorkerUpdate {
    pub pct: f32,
    pub stage: usize,
    pub status: String,
    pub finished: bool,
    pub is_restore: bool,
    pub error: Option<String>,
    pub applied_step: u32,
    pub applied_px: u32,
    pub install_root: PathBuf,
}

pub struct WorkerTarget {
    pub alive: AtomicBool,
    pub hwnd: AtomicUsize,
}

pub struct GuiState {
    pub hwnd: *mut c_void,
    pub dpi: u32,
    pub screen: Screen,
    pub preset: Preset,
    pub startup: bool,
    pub register_uninstall: bool,
    pub has_existing_install: bool,
    pub is_modify_mode: bool,
    pub current_step: u32,
    pub current_px: u32,
    pub hover: Option<ControlId>,
    pub pressed: Option<ControlId>,
    pub focus: Option<ControlId>,
    pub tracking_mouse: bool,
    pub dirty: bool,
    pub anim_active: bool,
    pub trimmed_initial: bool,
    pub hits: Vec<HitZone>,
    pub anim_pct: f32,
    pub target_pct: f32,
    pub worker_stage: usize,
    pub worker_status: String,
    pub worker_error: Option<String>,
    pub is_restore: bool,
    pub install_root: PathBuf,
    pub worker_target: Arc<WorkerTarget>,
    pub shared_update: Arc<Mutex<WorkerUpdate>>,
    pub fonts: FontSet,
    pub glyph_cache: GlyphSpriteCache,
    pub backbuffer: Option<Backbuffer>,
    pub app_icon: *mut c_void,
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == flag {
            return args.get(i + 1).cloned();
        }
        if let Some(v) = args[i].strip_prefix(&format!("{flag}=")) {
            return Some(v.to_string());
        }
    }
    None
}

fn is_headless_invocation(args: &[String]) -> bool {
    args.iter().any(|a| {
        a == "--silent"
            || a == "--restore"
            || a == "--restore-system-defaults"
            || a == "--sandbox"
            || a.starts_with("--sandbox=")
            || a == "--smoke-ui"
            || a.starts_with("--smoke-ui=")
    })
}

fn validate_setup_args(args: &[String]) -> Result<(), String> {
    validate_cli_args(
        args,
        &[
            "--silent",
            "--modify",
            "--no-startup",
            "--uninstall",
            "--restore",
            "--restore-system-defaults",
            "--compact",
            "--no-tail",
            "--tail-detached",
            "--detached",
            "--regular",
        ],
        &["--sandbox", "--smoke-ui", "--dpi", "--preset"],
        &[],
    )
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    validate_setup_args(&args)?;

    if let Some(smoke_dir) = arg_value(&args, "--smoke-ui") {
        let dpi_override = arg_value(&args, "--dpi").and_then(|v| v.parse::<u32>().ok());
        return run_smoke_ui(Path::new(&smoke_dir), dpi_override);
    }

    if let Some(sandbox_dir) = arg_value(&args, "--sandbox") {
        let preset = parse_preset_arg(&args)?.unwrap_or(Preset::Compact);
        let (root, install_dir, backup_path) = app_paths_for_root(PathBuf::from(sandbox_dir));
        let (step, px) = execute_install_pipeline(
            &root,
            &install_dir,
            &backup_path,
            preset,
            false,
            false,
            true,
            None,
            None,
        )?;
        let report = format!(
            "SANDBOX_OK preset={} step={} px={} assets=32 guardian_bytes={}\n",
            preset.cli_name(),
            step,
            px,
            GUARDIAN_EXE_BYTES.len()
        );
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("sandbox-report.txt"))
            .and_then(|mut f| f.write_all(report.as_bytes()));
        return Ok(());
    }

    if args.iter().any(|a| a == "--restore") {
        let (root, _, backup_path) = app_paths()?;
        let log = BoundedLogger::new(root.join("guardian.log"), false);
        return perform_validated_restore(
            &backup_path,
            Some(&log),
            || stop_targeted_guardians(&root, Some(&log)),
            || {
                reg_delete_value(RUN_SUBKEY, GUARDIAN_RUN_VALUE);
                reg_delete_value(RUN_SUBKEY, LEGACY_GUARDIAN_RUN_VALUE);
                reg_delete_value(CONFIG_SUBKEY, "Preset");
                unregister_uninstall_entry();
                let gexe = root.join(GUARDIAN_EXE_NAME);
                if gexe.exists() {
                    let _ = fs::remove_file(&gexe);
                }
                Ok(())
            },
        );
    }

    if args.iter().any(|a| a == "--uninstall") {
        let silent = args.iter().any(|a| a == "--silent");
        if !silent {
            let title = wide_z("Uninstall Modern Inverted Cursors");
            let prompt = wide_z(
                "Restore your original Windows cursor backup, disable startup launch, and uninstall Modern Inverted Cursors?",
            );
            let ans = unsafe {
                MessageBoxW(
                    null_mut(),
                    prompt.as_ptr(),
                    title.as_ptr(),
                    0x0001_0024, // MB_YESNO | MB_ICONQUESTION | MB_SETFOREGROUND
                )
            };
            if ans != 6 {
                return Ok(());
            }
        }
        let (root, install_dir, backup_path) = app_paths()?;
        let log = BoundedLogger::new(root.join("guardian.log"), false);
        return execute_uninstall_pipeline(
            &root,
            &install_dir,
            &backup_path,
            true,
            &log,
            |_, _, _| {},
        );
    }

    if args.iter().any(|a| a == "--restore-system-defaults") {
        let (root, _, _) = app_paths()?;
        let log = BoundedLogger::new(root.join("guardian.log"), false);
        stop_targeted_guardians(&root, Some(&log))?;
        reset_windows_default_cursors(Some(&log))?;
        reg_delete_value(RUN_SUBKEY, GUARDIAN_RUN_VALUE);
        reg_delete_value(RUN_SUBKEY, LEGACY_GUARDIAN_RUN_VALUE);
        reg_delete_value(CONFIG_SUBKEY, "Preset");
        unregister_uninstall_entry();
        return Ok(());
    }

    if args.iter().any(|a| a == "--silent") {
        let preset = parse_preset_arg(&args)?.unwrap_or(Preset::Compact);
        let no_startup = args.iter().any(|a| a == "--no-startup");
        let (root, install_dir, backup_path) = app_paths()?;
        let _ = execute_install_pipeline(
            &root,
            &install_dir,
            &backup_path,
            preset,
            !no_startup,
            true,
            false,
            None,
            None,
        )?;
        return Ok(());
    }

    let setup_mutex_name = wide_z("Local\\ModernInvertedCursors_Setup_V1_Mutex");
    let setup_mutex = unsafe { CreateMutexW(null_mut(), 0, setup_mutex_name.as_ptr()) };
    if !setup_mutex.is_null() && unsafe { GetLastError() } == 183 {
        let _ = bring_existing_setup_to_front();
        unsafe {
            CloseHandle(setup_mutex);
        }
        return Ok(());
    }

    let (root, _, backup_path) = app_paths()?;
    let det = detect_existing_installation(&root, &backup_path);
    let is_modify = args.iter().any(|a| a == "--modify");
    let has_existing = is_modify || det.has_any();
    let initial_preset = parse_preset_arg(&args)?.unwrap_or_else(load_saved_preset);

    let state_ptr = new_gui_state(initial_preset, true, true, has_existing, is_modify, root);
    let hwnd = match create_setup_window(state_ptr, true) {
        Ok(h) => h,
        Err(err) => {
            unsafe {
                let mut state = Box::from_raw(state_ptr);
                if !state.app_icon.is_null() {
                    DestroyIcon(state.app_icon);
                }
                let _ = &mut state;
                if !setup_mutex.is_null() {
                    CloseHandle(setup_mutex);
                }
            }
            return Err(err);
        }
    };

    unsafe {
        let mut msg: Msg = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let mut state = Box::from_raw(state_ptr);
        if let Some(mut bb) = state.backbuffer.take() {
            bb.destroy();
        }
        state.fonts.destroy();
        if !state.app_icon.is_null() {
            DestroyIcon(state.app_icon);
        }
        if !setup_mutex.is_null() {
            CloseHandle(setup_mutex);
        }
        let _ = hwnd;
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let headless = is_headless_invocation(&args);
    if let Err(err) = run() {
        eprintln!("{err}");
        if let Ok((root, _, _)) = app_paths() {
            let log = BoundedLogger::new(root.join("setup-error.log"), false);
            log.line(format!("[setup-error] {err}"));
        }
        if !headless {
            let title = wide_z("Modern Inverted Cursors Setup Error");
            let body = wide_z(&err);
            unsafe {
                MessageBoxW(null_mut(), body.as_ptr(), title.as_ptr(), 0x0001_0010);
            }
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use install::{sync_setup_maintenance_copy, verify_child_readiness};
    use modern_inverted_cursor::{NamedReadyEvent, CREATE_NO_WINDOW};
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use ui::{focus_order, BitmapInfoHeader, MonitorInfo, PaintStruct, TrackMouseEvent};

    #[test]
    fn test_win32_setup_structs_match_sdk() {
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(std::mem::size_of::<TrackMouseEvent>(), 24);
            assert_eq!(std::mem::size_of::<MonitorInfo>(), 40);
            assert_eq!(std::mem::size_of::<PaintStruct>(), 72);
            assert_eq!(std::mem::size_of::<BitmapInfoHeader>(), 40);
        }
        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(std::mem::size_of::<TrackMouseEvent>(), 16);
            assert_eq!(std::mem::size_of::<MonitorInfo>(), 40);
            assert_eq!(std::mem::size_of::<PaintStruct>(), 64);
            assert_eq!(std::mem::size_of::<BitmapInfoHeader>(), 40);
        }
    }

    #[test]
    fn test_embedded_guardian_is_nonempty_pe_binary() {
        assert!(GUARDIAN_EXE_BYTES.len() > 10_000);
        assert_eq!(&GUARDIAN_EXE_BYTES[..2], b"MZ");
        let pe_off = u32::from_le_bytes([
            GUARDIAN_EXE_BYTES[0x3C],
            GUARDIAN_EXE_BYTES[0x3D],
            GUARDIAN_EXE_BYTES[0x3E],
            GUARDIAN_EXE_BYTES[0x3F],
        ]) as usize;
        assert_eq!(&GUARDIAN_EXE_BYTES[pe_off..pe_off + 4], b"PE\0\0");
        let machine = u16::from_le_bytes([
            GUARDIAN_EXE_BYTES[pe_off + 4],
            GUARDIAN_EXE_BYTES[pe_off + 5],
        ]);
        #[cfg(target_arch = "x86_64")]
        assert_eq!(machine, 0x8664);
        #[cfg(target_arch = "aarch64")]
        assert_eq!(machine, 0xAA64);
        #[cfg(target_arch = "x86")]
        assert_eq!(machine, 0x014C);
    }

    #[test]
    fn test_verify_child_readiness_detects_crash_and_timeout() {
        let ev_crash = NamedReadyEvent::create_unique("setup_crash").unwrap();
        let child_crash = Command::new("cmd.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/C", "exit /B 13"])
            .spawn()
            .unwrap();
        let tmp_log = env::temp_dir().join("mic-test-readiness-b3.log");
        let _ = fs::write(&tmp_log, "line1\n[error] fake crash\n");
        let err = verify_child_readiness(child_crash, &ev_crash, 5000, &tmp_log).unwrap_err();
        assert!(err.contains("exit code 13"));
        assert!(err.contains("fake crash"));

        let ev_hang = NamedReadyEvent::create_unique("setup_hang").unwrap();
        let child_hang = Command::new("cmd.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/C", "ping -n 4 127.0.0.1 >nul"])
            .spawn()
            .unwrap();
        let err_timeout = verify_child_readiness(child_hang, &ev_hang, 120, &tmp_log).unwrap_err();
        assert!(err_timeout.contains("did not confirm readiness"));
        let _ = fs::remove_file(&tmp_log);
    }

    #[test]
    fn test_headless_detection_covers_silent_and_automated_flags() {
        assert!(is_headless_invocation(&[
            "--silent".into(),
            "--preset".into(),
            "compact".into()
        ]));
        assert!(is_headless_invocation(&["--sandbox=C:\\temp".into()]));
        assert!(is_headless_invocation(&["--restore".into()]));
        assert!(!is_headless_invocation(&[]));
        assert!(!is_headless_invocation(&["--uninstall".into()]));
    }

    #[test]
    fn test_sync_setup_maintenance_copy_skips_same_path_and_copies_external() {
        let tmp = env::temp_dir().join("mic-test-setup-sync");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let src = tmp.join("ModernInvertedCursors-Setup-v1.1.0-rc.1-x64.exe");
        let dest = tmp.join("ModernInvertedCursors-Setup.exe");
        fs::write(&src, b"MZ-setup-payload").unwrap();

        sync_setup_maintenance_copy(&src, &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"MZ-setup-payload");
        sync_setup_maintenance_copy(&dest, &dest).unwrap();

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_keyboard_focus_order_tracks_screen_and_install_state() {
        let fresh = focus_order(Screen::Setup, false, false, false);
        assert_eq!(fresh.len(), 7);
        assert_eq!(fresh[0], ControlId::PresetCard(0));
        assert_eq!(fresh[6], ControlId::BtnInstall);

        let existing = focus_order(Screen::Setup, true, false, false);
        assert_eq!(existing.len(), 8);
        assert_eq!(existing[6], ControlId::BtnRestore);
        assert_eq!(existing[7], ControlId::BtnInstall);

        let done_ok = focus_order(Screen::Done, true, false, false);
        assert_eq!(
            done_ok,
            vec![ControlId::BtnOpenFolder, ControlId::BtnFinish]
        );

        let done_err = focus_order(Screen::Done, true, true, false);
        assert_eq!(
            done_err,
            vec![ControlId::BtnOpenFolder, ControlId::BtnFinish]
        );

        let working = focus_order(Screen::Working, true, false, false);
        assert!(working.is_empty());
    }
}
