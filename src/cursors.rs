use crate::registry::{
    reg_get_dword, reg_set_dword, reg_set_string, trim_working_set, wide_z, BoundedLogger, Preset,
    ACCESSIBILITY_SUBKEY, CURSORS_SUBKEY, SCHEMES_SUBKEY,
};
use std::{
    env,
    ffi::c_void,
    path::{Path, PathBuf},
};

pub const BALANCED_MIN_SIZE: u32 = 32;
pub const BALANCED_STEP_PX: u32 = 4;
pub const ACCESSIBILITY_STEPS: u32 = 15;

pub const ROLES: &[&str] = &[
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
    fn DestroyCursor(cursor: *mut c_void) -> i32;
    fn GetSystemMetrics(n_index: i32) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetLastError() -> u32;
}

pub fn current_accessibility_step() -> u32 {
    if let Some(step) = reg_get_dword(ACCESSIBILITY_SUBKEY, "CursorSize") {
        return step.clamp(1, ACCESSIBILITY_STEPS);
    }
    if let Some(base) = reg_get_dword(CURSORS_SUBKEY, "CursorBaseSize") {
        let base = base.clamp(32, 256);
        let rounded_step = ((base.saturating_sub(32) + 8) / 16) + 1;
        return rounded_step.clamp(1, ACCESSIBILITY_STEPS);
    }
    // SAFETY: `GetSystemMetrics(SM_CXCURSOR = 13)` is read-only.
    let metric = unsafe { GetSystemMetrics(13) };
    if metric > 32 {
        let px = (metric as u32).clamp(32, 256);
        let rounded_step = ((px.saturating_sub(32) + 8) / 16) + 1;
        return rounded_step.clamp(1, ACCESSIBILITY_STEPS);
    }
    1
}

pub fn balanced_cursor_size(step: u32) -> u32 {
    let step = step.clamp(1, ACCESSIBILITY_STEPS);
    BALANCED_MIN_SIZE + (step - 1) * BALANCED_STEP_PX
}

pub fn preset_role_paths(preset: Preset, dir: &Path) -> Vec<String> {
    let (arrow, help, working) = preset.custom_files();
    let p = |name: &str| dir.join(name).to_string_lossy().into_owned();
    vec![
        p(arrow),
        p(help),
        p(working),
        p("busy.ani"),
        p("cross.cur"),
        p("beam_v2.cur"),
        p("pen.cur"),
        p("unavailable_v3.cur"),
        p("vertical_v2.cur"),
        p("horizontal_v2.cur"),
        p("diagonal_1.cur"),
        p("diagonal_2.cur"),
        p("move_v2.cur"),
        p("special.cur"),
        p("link_v2.cur"),
        r"%SystemRoot%\cursors\pin_i.cur".into(),
        r"%SystemRoot%\cursors\person_i.cur".into(),
    ]
}

pub fn reload_cursors() -> Result<(), String> {
    const SPI_SETCURSORS: u32 = 0x0057;
    const SPIF_UPDATEINIFILE: u32 = 0x0001;
    const SPIF_SENDCHANGE: u32 = 0x0002;
    // SAFETY: `SPI_SETCURSORS` ignores `pv_param` when zero/null.
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_SETCURSORS,
            0,
            std::ptr::null_mut(),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
    };
    if ok == 0 {
        let err = unsafe { GetLastError() };
        Err(format!("SPI_SETCURSORS failed (GetLastError={err})"))
    } else {
        Ok(())
    }
}

fn load_cursor_handle(path: &Path, label: &str, cursor_size: u32) -> Result<*mut c_void, String> {
    const IMAGE_CURSOR: u32 = 2;
    const LR_LOADFROMFILE: u32 = 0x0000_0010;
    let wide = wide_z(&path.to_string_lossy());

    // SAFETY: Loads a non-shared cursor handle (`LR_LOADFROMFILE`); caller owns it until passed to `SetSystemCursor` or freed via `DestroyCursor`.
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
            "LoadImageW failed for {label}: {} at {cursor_size}px (err={err})",
            path.display()
        ));
    }
    Ok(handle)
}

pub fn register_and_apply(
    preset: Preset,
    dir: &Path,
    log: Option<&BoundedLogger>,
) -> Result<(u32, u32), String> {
    let values = preset_role_paths(preset, dir);
    if values.len() != ROLES.len() {
        return Err("Internal cursor role count mismatch.".into());
    }

    let (arrow, help, working) = preset.custom_files();
    let system_root = env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let system_cursors = system_root.join("Cursors");

    let live: [(&str, PathBuf, u32); 17] = [
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

    let step = current_accessibility_step();
    let px = balanced_cursor_size(step);

    // Phase 1: Pre-load and validate ALL cursor handles BEFORE mutating registry or live handles.
    let mut loaded: Vec<(&str, u32, bool, *mut c_void)> = Vec::with_capacity(live.len());
    for (label, path, id) in live {
        let optional = id == 32631 || id == 32671 || id == 32672;
        if !path.exists() && optional {
            continue;
        }
        match load_cursor_handle(&path, label, px) {
            Ok(h) => loaded.push((label, id, optional, h)),
            Err(err) => {
                if !optional {
                    // Clean up every already-loaded handle cleanly so nothing mutates on failure.
                    for (_, _, _, prev_h) in loaded {
                        // SAFETY: Non-shared handle not yet passed to `SetSystemCursor`.
                        unsafe {
                            DestroyCursor(prev_h);
                        }
                    }
                    return Err(err);
                }
            }
        }
    }

    // Phase 2: Persist cursor scheme in HKCU once all assets loaded cleanly.
    let scheme_string = values.join(",");
    reg_set_string(SCHEMES_SUBKEY, Some(preset.name()), true, &scheme_string)?;
    reg_set_string(CURSORS_SUBKEY, None, false, preset.name())?;
    for (role, value) in ROLES.iter().zip(values.iter()) {
        reg_set_string(CURSORS_SUBKEY, Some(role), true, value)?;
    }
    reg_set_dword(CURSORS_SUBKEY, "Scheme Source", 0)?;

    let _ = reload_cursors();

    // Phase 3: Atomically swap loaded handles; roll back with `reload_cursors()` if a required swap fails.
    for idx in 0..loaded.len() {
        let (label, id, optional, handle) = loaded[idx];
        // SAFETY: `SetSystemCursor` destroys `handle` on success; on failure we free remaining handles cleanly.
        let ok = unsafe { SetSystemCursor(handle, id) };
        if ok == 0 {
            let err = unsafe { GetLastError() };
            unsafe {
                DestroyCursor(handle);
            }
            if !optional {
                for &(_, _, _, rem_h) in &loaded[idx + 1..] {
                    unsafe {
                        DestroyCursor(rem_h);
                    }
                }
                let _ = reload_cursors();
                return Err(format!("SetSystemCursor failed for {label} / {id} (err={err})"));
            }
        }
    }

    if let Some(l) = log {
        l.line(format!(
            "Applied {} at {}px (Accessibility {}/{})",
            preset.short_name(),
            px,
            step,
            ACCESSIBILITY_STEPS
        ));
    }
    trim_working_set();
    Ok((step, px))
}
