# Debugging history

This document records the actual development path that led to the working Windows 11 installer.

## 1. Original goal

Windows 11's built-in **Inverted** mouse pointer style was useful because it dynamically inverted the pixels underneath it, but the stock cursor artwork looked old and some animated states felt dated or pixelated.

The chosen upstream project was:

https://github.com/emvaized/modern_inverted_mouse_cursors

Version used throughout the project: **v3.6.2**.

The upstream project uses cursor formats that preserve Windows' native inversion behavior. Most cursors are intentionally low-color-depth/1-bit style because normal 32-bit ARGB cursors do not reliably support the same XOR-style inversion behavior.

## 2. First PowerShell installer

The first attempt wrapped the public cursor assets in a custom PowerShell installer.

It backed up the current cursor configuration, downloaded the exact v3.6.2 source archive, verified the public cursor assets, registered cursor schemes under:

```text
HKCU\Control Panel\Cursors
HKCU\Control Panel\Cursors\Schemes
```

and called `SystemParametersInfo(SPI_SETCURSORS)`.

### Failure

Windows PowerShell 5.1 hit a parser error around the scheme construction:

```powershell
$schemeString=(foreach($r in $orderedRoles){$map[$r]}) -join ','
```

The direct `foreach` expression there was not accepted by Windows PowerShell 5.1.

### Fix

It was changed to:

```powershell
$schemeValues = foreach ($r in $orderedRoles) { $map[$r] }
$schemeString = $schemeValues -join ','
```

That version is preserved as:

`archive/Modern-Inverted-v3.6.2-Installer-FIXED.ps1`

## 3. PowerShell v2 still closed unexpectedly

A more defensive PowerShell v2 was created with:

- visible error handling
- pause-on-failure
- logging
- TLS 1.2
- direct .NET download handling
- automatic archive folder detection
- restore support in the same script

Despite those changes, the script still behaved unreliably on the target machine.

That version is preserved as:

`archive/Modern-Inverted-v3.6.2-Installer-v2.ps1`

At this point the project moved away from PowerShell.

## 4. Pivot to standalone Rust

The next approach was a single-file Rust program with no Cargo project and no third-party crates.

The Rust installer:

- used `curl.exe` for downloads
- used `reg.exe` for registry backup and writes
- called Win32 APIs directly
- implemented SHA-1 internally so Git blob hashes could be verified without dependencies
- kept all changes current-user only

The first Rust source is preserved as:

`archive/modern_inverted_installer_v1.rs`

## 5. rustc was not installed / not on PATH

Initial compile attempt:

```powershell
rustc -O .\modern_inverted_installer.rs -o .\ModernInvertedInstaller.exe
```

failed with:

```text
rustc : The term 'rustc' is not recognized as the name of a cmdlet...
```

Rustup was installed with:

```powershell
winget install -e --id Rustlang.Rustup
```

The existing PowerShell session still had the old PATH, so it was refreshed with:

```powershell
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
```

## 6. MSVC linker error

After Rust became visible, compilation reached the linker and failed with:

```text
error: linker `link.exe` not found

note: the msvc targets depend on the msvc linker but `link.exe` was not found
```

Rather than installing the full Visual Studio C++ Build Tools stack just for this one utility, the GNU Rust toolchain was installed:

```powershell
rustup toolchain install stable-gnu
```

Compilation then succeeded using:

```powershell
rustc +stable-gnu -O .\modern_inverted_installer.rs -o .\ModernInvertedInstaller.exe
```

There was one harmless warning:

```text
warning: function `run_status` is never used
```

That dead helper was removed in the cleanup source preserved as:

`archive/modern_inverted_installer_warning_cleanup.rs`

## 7. Source-file-not-found confusion

When the force-live v3 source was generated, the compile command was run before the new file had actually been downloaded into the local Downloads folder:

```text
error: couldn't find file `.\modern_inverted_installer_v3_force_live.rs`
```

Once the file appeared in:

```text
C:\Users\sinet\Downloads
```

the same compilation command completed successfully.

## 8. Compile succeeded, but nothing changed

Another confusing moment was that a successful `rustc` command only **created the executable**. It did not run it.

The required second step was:

```powershell
.\ModernInvertedInstallerV3.exe
```

This distinction was important during testing.

## 9. First Rust installer ran, but the visible cursor did not change

The original Rust installer persisted the cursor scheme in the registry and requested a reload with:

`SystemParametersInfoW(SPI_SETCURSORS)`

On the test machine that was not sufficient to visibly switch the active cursor set.

This was the final major technical bug.

## 10. Final fix: force-live cursor replacement

The v3 source added direct Win32 calls:

- `LoadCursorFromFileW`
- `SetSystemCursor`

The final sequence became:

1. Download and verify assets.
2. Back up the user's previous cursor configuration.
3. Persist the selected scheme in the registry.
4. Ask Windows to reload it using `SPI_SETCURSORS`.
5. Directly replace the live system cursor handles as a fallback.

This finally changed the active cursor immediately on the target Windows 11 machine.

The user selected the **Compact** preset and confirmed:

> "okay bro that worked, thanks!"

The working final source is:

`final/modern_inverted_installer_v3_force_live.rs`

and the compiled final product is preserved by CI as:

`dist/ModernInvertedInstallerV3.exe`

## Final local commands that worked

```powershell
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

rustup toolchain install stable-gnu

cd "$env:USERPROFILE\Downloads"

rustc +stable-gnu -O .\modern_inverted_installer_v3_force_live.rs -o .\ModernInvertedInstallerV3.exe

.\ModernInvertedInstallerV3.exe
```

Then choose preset:

```text
4
```

for **Compact**.
