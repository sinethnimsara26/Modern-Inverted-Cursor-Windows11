use crate::ui::push_worker_update;
use crate::{Screen, WorkerTarget, WorkerUpdate};
use modern_inverted_cursor::{
    balanced_cursor_size, cleanup_legacy_guardian_binary, current_accessibility_step,
    export_backup, find_tray_window, normalize_win_path_str, perform_validated_restore,
    prepare_assets_parallel, read_log_tail, reg_delete_value, reg_get_string, reg_set_string,
    register_uninstall_entry, save_preset, set_startup_enabled, stop_targeted_guardians,
    unregister_uninstall_entry, window_process_id, write_binary_retry, schedule_self_delete,
    BoundedLogger, NamedReadyEvent, Preset, ReadyWaitOutcome, APP_VERSION, CONFIG_SUBKEY,
    GUARDIAN_EXE_NAME, GUARDIAN_RUN_VALUE, LEGACY_GUARDIAN_EXE_NAME,
    LEGACY_GUARDIAN_RUN_VALUE, RUN_SUBKEY, SETUP_EXE_NAME, UNINSTALL_SUBKEY,
};
use std::{
    env,
    ffi::c_void,
    fs,
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

pub const GUARDIAN_EXE_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/embedded_guardian.exe"));

pub fn verify_child_readiness(
    mut child: Child,
    ev: &NamedReadyEvent,
    timeout_ms: u32,
    log_path: &Path,
) -> Result<Child, String> {
    let proc_handle = child.as_raw_handle() as *mut c_void;
    match ev.wait_process_or_ready(proc_handle, timeout_ms) {
        ReadyWaitOutcome::Ready => {
            if let Ok(Some(status)) = child.try_wait() {
                let tail = read_log_tail(log_path, 4);
                return Err(format!(
                    "Guardian exited right after signaling ready ({status}). Log: {tail}"
                ));
            }
            Ok(child)
        }
        ReadyWaitOutcome::ExitedEarly => {
            let status = child.wait().ok();
            let code_desc = status
                .and_then(|s| s.code())
                .map(|c| format!("exit code {c}"))
                .unwrap_or_else(|| "unknown exit status".into());
            let tail = read_log_tail(log_path, 4);
            let suffix = if tail.is_empty() {
                String::new()
            } else {
                format!(" Recent log: {tail}")
            };
            Err(format!(
                "Guardian process terminated prematurely ({code_desc}) before cursor initialization.{suffix}"
            ))
        }
        ReadyWaitOutcome::TimedOut => {
            let _ = child.kill();
            let _ = child.wait();
            let tail = read_log_tail(log_path, 4);
            Err(format!(
                "Guardian did not confirm readiness within {timeout_ms}ms.{}{tail}",
                if tail.is_empty() { "" } else { " Recent log: " }
            ))
        }
        ReadyWaitOutcome::WaitError(code) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(format!(
                "WaitForMultipleObjects failed while starting guardian (code={code})."
            ))
        }
    }
}

pub fn launch_and_verify_guardian(
    root: &Path,
    guardian_exe: &Path,
    preset: Preset,
    timeout_ms: u32,
) -> Result<(), String> {
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    let ev = NamedReadyEvent::create_unique("setup")?;
    let child = Command::new(guardian_exe)
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args([
            "--tray",
            "--preset",
            preset.cli_name(),
            "--ready-event",
            &ev.name,
        ])
        .spawn()
        .map_err(|e| format!("Could not launch {}: {e}", guardian_exe.display()))?;

    let child = verify_child_readiness(child, &ev, timeout_ms, &root.join("guardian.log"))?;
    let expected_pid = child.id();

    let tray_hwnd = find_tray_window();
    let tray_pid = window_process_id(tray_hwnd);
    if tray_pid != Some(expected_pid) {
        return Err(format!(
            "Guardian readiness event fired, but tray window verification failed (expected pid={expected_pid}, found={tray_pid:?})."
        ));
    }

    let runtime_ver = reg_get_string(CONFIG_SUBKEY, Some("RuntimeVersion"));
    if runtime_ver.as_deref() != Some(APP_VERSION) {
        return Err(format!(
            "Guardian version mismatch after startup (expected {APP_VERSION}, got {:?}).",
            runtime_ver
        ));
    }
    Ok(())
}

/// Installs the persistent maintenance copy of Setup into `%LOCALAPPDATA%\ModernInvertedCursors\ModernInvertedCursors-Setup.exe`
/// unless already running from that exact path.
pub fn sync_setup_maintenance_copy(current_exe: &Path, dest_setup: &Path) -> Result<(), String> {
    if normalize_win_path_str(current_exe) == normalize_win_path_str(dest_setup) {
        return Ok(());
    }
    if let Some(parent) = dest_setup.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    let tmp = dest_setup.with_extension("exe.tmp");
    let _ = fs::remove_file(&tmp);
    fs::copy(current_exe, &tmp).map_err(|e| {
        format!(
            "Could not copy setup maintenance binary {} -> {}: {e}",
            current_exe.display(),
            tmp.display()
        )
    })?;
    if dest_setup.exists() {
        let _ = fs::remove_file(dest_setup);
    }
    fs::rename(&tmp, dest_setup).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("Could not finalize {}: {e}", dest_setup.display())
    })?;
    Ok(())
}

fn dir_size_bytes(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    let mut sum = 0u64;
    for entry in entries.flatten() {
        let p = entry.path();
        if let Ok(m) = entry.metadata() {
            if m.is_file() {
                sum += m.len();
            } else if m.is_dir() {
                sum += dir_size_bytes(&p);
            }
        }
    }
    sum
}

pub fn compute_estimated_kb(root: &Path, setup_exe: &Path, guardian_exe: &Path) -> u32 {
    let assets_b = dir_size_bytes(&root.join("v3.6.2"));
    let setup_b = fs::metadata(setup_exe).map(|m| m.len()).unwrap_or(0);
    let guard_b = fs::metadata(guardian_exe)
        .map(|m| m.len())
        .unwrap_or(GUARDIAN_EXE_BYTES.len() as u64);
    ((assets_b + setup_b + guard_b).div_ceil(1024)) as u32
}

struct InstallSnapshot {
    prev_run_cmd: Option<String>,
    prev_legacy_run_cmd: Option<String>,
    prev_preset: Option<String>,
    had_uninstall: bool,
    prev_guardian: Option<Vec<u8>>,
    prev_setup: Option<Vec<u8>>,
}

impl InstallSnapshot {
    fn capture(guardian_exe: &Path, setup_exe: &Path) -> Self {
        Self {
            prev_run_cmd: reg_get_string(RUN_SUBKEY, Some(GUARDIAN_RUN_VALUE)),
            prev_legacy_run_cmd: reg_get_string(RUN_SUBKEY, Some(LEGACY_GUARDIAN_RUN_VALUE)),
            prev_preset: reg_get_string(CONFIG_SUBKEY, Some("Preset")),
            had_uninstall: reg_get_string(UNINSTALL_SUBKEY, Some("UninstallString")).is_some(),
            prev_guardian: fs::read(guardian_exe).ok(),
            prev_setup: fs::read(setup_exe).ok(),
        }
    }

    fn rollback(
        &self,
        root: &Path,
        guardian_exe: &Path,
        setup_exe: &Path,
        backup_path: &Path,
        log: &BoundedLogger,
    ) {
        log.line("[rollback] Reverting install state after guardian verification failure...");
        let _ = stop_targeted_guardians(root, Some(log));
        match self.prev_run_cmd.as_deref() {
            Some(cmd) => {
                let _ = reg_set_string(RUN_SUBKEY, Some(GUARDIAN_RUN_VALUE), false, cmd);
            }
            None => reg_delete_value(RUN_SUBKEY, GUARDIAN_RUN_VALUE),
        }
        if let Some(cmd) = self.prev_legacy_run_cmd.as_deref() {
            let _ = reg_set_string(RUN_SUBKEY, Some(LEGACY_GUARDIAN_RUN_VALUE), false, cmd);
        }
        match self.prev_preset.as_deref() {
            Some(p) => {
                let _ = reg_set_string(CONFIG_SUBKEY, Some("Preset"), false, p);
            }
            None => reg_delete_value(CONFIG_SUBKEY, "Preset"),
        }
        if !self.had_uninstall {
            unregister_uninstall_entry();
        }
        match self.prev_guardian.as_deref() {
            Some(bytes) => {
                let _ = write_binary_retry(guardian_exe, bytes);
            }
            None => {
                let _ = fs::remove_file(guardian_exe);
                let _ = modern_inverted_cursor::restore_backup(backup_path, Some(log));
            }
        }
        if self.prev_setup.is_none() && fs::metadata(setup_exe).is_ok() {
            let cur = env::current_exe().unwrap_or_default();
            if normalize_win_path_str(&cur) != normalize_win_path_str(setup_exe) {
                let _ = fs::remove_file(setup_exe);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn execute_install_pipeline(
    root: &Path,
    install_dir: &Path,
    backup_path: &Path,
    preset: Preset,
    startup: bool,
    register_uninstall: bool,
    sandbox_only: bool,
    target: Option<&Arc<WorkerTarget>>,
    shared: Option<&Arc<Mutex<WorkerUpdate>>>,
) -> Result<(u32, u32), String> {
    let log = BoundedLogger::new(root.join("guardian.log"), false);
    let report = |pct: f32, stage: usize, msg: String| {
        if let Some(sh) = shared {
            push_worker_update(
                target,
                sh,
                WorkerUpdate {
                    pct,
                    stage,
                    status: msg,
                    finished: false,
                    is_restore: false,
                    error: None,
                    applied_step: 1,
                    applied_px: 32,
                    install_root: root.to_path_buf(),
                },
            );
        }
    };

    report(
        0.08,
        0,
        "Backing up pre-existing Windows cursor scheme...".into(),
    );
    if !sandbox_only {
        export_backup(backup_path, Some(&log))?;
    } else {
        fs::create_dir_all(root).map_err(|e| format!("Could not create {}: {e}", root.display()))?;
    }

    report(
        0.18,
        1,
        "Preparing 32 upstream v3.6.2 cursor files...".into(),
    );
    prepare_assets_parallel(install_dir, Some(&log), |done, total, name| {
        let pct = 0.18 + 0.52 * (done as f32 / total as f32);
        report(
            pct,
            1,
            format!("Verified {done}/{total} cursor assets ({name})"),
        );
    })?;

    report(
        0.76,
        2,
        "Installing setup maintenance copy & tray guardian...".into(),
    );
    let guardian_exe = root.join(GUARDIAN_EXE_NAME);
    let setup_exe = root.join(SETUP_EXE_NAME);
    let snapshot = if !sandbox_only {
        let s = InstallSnapshot::capture(&guardian_exe, &setup_exe);
        stop_targeted_guardians(root, Some(&log))?;
        Some(s)
    } else {
        None
    };

    if let Err(err) = (|| -> Result<(), String> {
        write_binary_retry(&guardian_exe, GUARDIAN_EXE_BYTES)?;
        if let Ok(cur_exe) = env::current_exe() {
            if sandbox_only {
                let _ = sync_setup_maintenance_copy(&cur_exe, &setup_exe);
            } else {
                sync_setup_maintenance_copy(&cur_exe, &setup_exe)?;
            }
        }
        Ok(())
    })() {
        if let Some(ref s) = snapshot {
            s.rollback(root, &guardian_exe, &setup_exe, backup_path, &log);
        }
        return Err(err);
    }

    if !sandbox_only {
        if let Err(err) = (|| -> Result<(), String> {
            save_preset(preset)?;
            set_startup_enabled(&guardian_exe, startup)?;
            if register_uninstall {
                let kb = compute_estimated_kb(root, &setup_exe, &guardian_exe);
                register_uninstall_entry(root, &setup_exe, kb)?;
            } else {
                unregister_uninstall_entry();
            }
            report(
                0.90,
                3,
                "Applying balanced cursors & verifying tray guardian startup...".into(),
            );
            launch_and_verify_guardian(root, &guardian_exe, preset, 10_000)?;
            cleanup_legacy_guardian_binary(root, Some(&log));
            Ok(())
        })() {
            if let Some(ref s) = snapshot {
                s.rollback(root, &guardian_exe, &setup_exe, backup_path, &log);
            }
            return Err(err);
        }
    }

    let st = current_accessibility_step();
    let px = balanced_cursor_size(st);

    log.line(format!(
        "Setup v{APP_VERSION} installed & verified {} ({}px, sandbox={sandbox_only})",
        preset.short_name(),
        px
    ));
    Ok((st, px))
}

pub fn execute_uninstall_pipeline(
    root: &Path,
    install_dir: &Path,
    backup_path: &Path,
    remove_assets_and_self: bool,
    log: &BoundedLogger,
    on_stage: impl Fn(f32, usize, &str),
) -> Result<(), String> {
    on_stage(0.25, 0, "Validating original cursor backup...");
    perform_validated_restore(
        backup_path,
        Some(log),
        || {
            on_stage(0.55, 1, "Stopping tray guardian & restoring backup...");
            stop_targeted_guardians(root, Some(log))
        },
        || {
            on_stage(0.85, 2, "Cleaning startup & Installed Apps entries...");
            reg_delete_value(RUN_SUBKEY, GUARDIAN_RUN_VALUE);
            reg_delete_value(RUN_SUBKEY, LEGACY_GUARDIAN_RUN_VALUE);
            reg_delete_value(CONFIG_SUBKEY, "Preset");
            reg_delete_value(CONFIG_SUBKEY, "RuntimeVersion");
            unregister_uninstall_entry();

            for gname in [GUARDIAN_EXE_NAME, LEGACY_GUARDIAN_EXE_NAME] {
                let p = root.join(gname);
                if p.exists() {
                    let _ = fs::remove_file(&p);
                }
            }
            if remove_assets_and_self {
                if install_dir.exists() {
                    let _ = fs::remove_dir_all(install_dir);
                }
                let installed_setup = root.join(SETUP_EXE_NAME);
                if installed_setup.exists() {
                    let cur = env::current_exe().unwrap_or_default();
                    if normalize_win_path_str(&cur) == normalize_win_path_str(&installed_setup) {
                        schedule_self_delete(&installed_setup);
                    } else {
                        let _ = fs::remove_file(&installed_setup);
                    }
                }
            }
            Ok(())
        },
    )
}

pub fn start_install_worker(state: &mut crate::GuiState, is_restore: bool) {
    state.screen = Screen::Working;
    state.is_restore = is_restore;
    state.anim_pct = 0.04;
    state.target_pct = 0.08;
    state.worker_stage = 0;
    state.worker_error = None;
    state.worker_status = if is_restore {
        "Validating original cursor backup...".into()
    } else {
        "Initializing installer...".into()
    };

    let target = Arc::clone(&state.worker_target);
    let shared = Arc::clone(&state.shared_update);
    let preset = state.preset;
    let startup = state.startup;
    let reg_un = state.register_uninstall;

    state.anim_active = true;
    state.dirty = true;

    // SAFETY: `state.hwnd` is valid while `GuiState` lives in UI thread.
    unsafe {
        crate::ui::SetTimer(state.hwnd, crate::TIMER_ANIM, 30, std::ptr::null_mut());
        crate::ui::InvalidateRect(state.hwnd, std::ptr::null(), 0);
    }

    std::thread::spawn(move || {
        let paths = modern_inverted_cursor::app_paths();
        let res = match paths {
            Ok((root, install_dir, backup_path)) => {
                if is_restore {
                    let log = BoundedLogger::new(root.join("guardian.log"), false);
                    execute_uninstall_pipeline(
                        &root,
                        &install_dir,
                        &backup_path,
                        true,
                        &log,
                        |pct, stage, msg| {
                            push_worker_update(
                                Some(&target),
                                &shared,
                                WorkerUpdate {
                                    pct,
                                    stage,
                                    status: msg.into(),
                                    finished: false,
                                    is_restore: true,
                                    error: None,
                                    applied_step: 1,
                                    applied_px: 32,
                                    install_root: root.clone(),
                                },
                            );
                        },
                    )
                    .map(|()| (root, 1u32, 32u32))
                } else {
                    execute_install_pipeline(
                        &root,
                        &install_dir,
                        &backup_path,
                        preset,
                        startup,
                        reg_un,
                        false,
                        Some(&target),
                        Some(&shared),
                    )
                    .map(|(st, px)| (root, st, px))
                }
            }
            Err(e) => Err(e),
        };

        match res {
            Ok((root, st, px)) => {
                push_worker_update(
                    Some(&target),
                    &shared,
                    WorkerUpdate {
                        pct: 1.0,
                        stage: 4,
                        status: "Complete!".into(),
                        finished: true,
                        is_restore,
                        error: None,
                        applied_step: st,
                        applied_px: px,
                        install_root: root,
                    },
                );
            }
            Err(err) => {
                push_worker_update(
                    Some(&target),
                    &shared,
                    WorkerUpdate {
                        pct: 1.0,
                        stage: 4,
                        status: "Failed".into(),
                        finished: true,
                        is_restore,
                        error: Some(err),
                        applied_step: 1,
                        applied_px: 32,
                        install_root: PathBuf::new(),
                    },
                );
            }
        }
    });
}
