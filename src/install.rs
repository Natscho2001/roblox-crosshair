//! `--install` / `--uninstall`: put the launcher script next to the binary, a start-menu
//! entry into ~/.local/share/applications and an example config into ~/.config.

use crate::config::{self, EXAMPLE_CONFIG};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const LAUNCHER: &str = include_str!("../packaging/roblox-mit-fadenkreuz.sh");
const DESKTOP: &str = include_str!("../packaging/roblox-mit-fadenkreuz.desktop");
const LAUNCHER_NAME: &str = "roblox-mit-fadenkreuz.sh";
const DESKTOP_NAME: &str = "roblox-mit-fadenkreuz.desktop";

fn applications_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| config::home_dir().join(".local/share"))
        .join("applications")
}

/// Quote a path for a .desktop Exec line (spaces etc. need double quotes).
fn desktop_quote(p: &Path) -> String {
    let s = p.to_string_lossy();
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "/._-+".contains(c))
    {
        s.into_owned()
    } else {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\\\\\")
                .replace('"', "\\\"")
                .replace('`', "\\`")
                .replace('$', "\\$")
        )
    }
}

pub fn desktop_entry(script: &Path) -> String {
    DESKTOP.replace("@SCRIPT@", &desktop_quote(script))
}

pub fn install(config_path: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("Programmpfad unbekannt: {e}"))?;
    let dir = exe.parent().ok_or("Programmordner unbekannt")?;
    let script = dir.join(LAUNCHER_NAME);
    fs::write(&script, LAUNCHER).map_err(|e| format!("{}: {e}", script.display()))?;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    println!("✔ Startskript:       {}", script.display());

    let apps = applications_dir();
    fs::create_dir_all(&apps).map_err(|e| format!("{}: {e}", apps.display()))?;
    let desktop = apps.join(DESKTOP_NAME);
    fs::write(&desktop, desktop_entry(&script))
        .map_err(|e| format!("{}: {e}", desktop.display()))?;
    println!(
        "✔ Startmenü-Eintrag: {}  („Roblox mit Fadenkreuz“)",
        desktop.display()
    );

    write_example_config(config_path)?;
    println!("\nFertig. Im Startmenü findest du jetzt „Roblox mit Fadenkreuz“.");
    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    let desktop = applications_dir().join(DESKTOP_NAME);
    let _ = fs::remove_file(&desktop);
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        let _ = fs::remove_file(dir.join(LAUNCHER_NAME));
    }
    println!("Startmenü-Eintrag und Startskript entfernt (Programm und Einstellungen bleiben).");
    Ok(())
}

pub fn write_example_config(path: &Path) -> Result<(), String> {
    if path.exists() {
        println!(
            "✔ Einstellungen:     {} (schon vorhanden, nicht überschrieben)",
            path.display()
        );
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(path, EXAMPLE_CONFIG).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("✔ Einstellungen:     {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_entry_has_absolute_quoted_exec() {
        let e = desktop_entry(Path::new("/home/kid/Applications/roblox-mit-fadenkreuz.sh"));
        assert!(e.contains("Exec=/home/kid/Applications/roblox-mit-fadenkreuz.sh %u"));
        let e = desktop_entry(Path::new("/home/kid/Meine Programme/x.sh"));
        assert!(
            e.contains("Exec=\"/home/kid/Meine Programme/x.sh\" %u"),
            "{e}"
        );
    }

    #[test]
    fn launcher_runs_sober_and_quits_only_its_own_overlay() {
        assert!(LAUNCHER.contains("flatpak run org.vinegarhq.Sober"));
        assert!(LAUNCHER.contains("--quit"));
        assert!(LAUNCHER.starts_with("#!/bin/sh"));
    }
}
