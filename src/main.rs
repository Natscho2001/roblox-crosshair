//! roblox-crosshair — click-through crosshair overlay for Wayland (KDE Plasma, Sway, …).
//! It only draws a picture on top of the screen; it never reads or changes the game.

mod config;
mod geometry;
mod install;
mod ipc;
mod overlay;

use clap::Parser;
use config::Overrides;
use ipc::Command;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "roblox-crosshair",
    version,
    about = "Fadenkreuz-Overlay für Wayland (KDE Plasma) – durchklickbar, immer im Vordergrund.",
    after_help = "Ohne Befehl startet das Overlay. Einstellungen: ~/.config/roblox-crosshair/config.toml"
)]
struct Cli {
    #[command(flatten)]
    overrides: Overrides,

    /// Andere Konfigurationsdatei verwenden
    #[arg(long, value_name = "DATEI")]
    config: Option<PathBuf>,

    /// Laufendes Overlay ein-/ausblenden (startet es, falls es nicht läuft) – für die KDE-Kurzbefehle
    #[arg(long, group = "action")]
    toggle: bool,
    /// Laufendes Overlay einblenden (startet es, falls es nicht läuft)
    #[arg(long, group = "action")]
    show: bool,
    /// Laufendes Overlay ausblenden
    #[arg(long, group = "action")]
    hide: bool,
    /// Laufendes Overlay beenden
    #[arg(long, group = "action")]
    quit: bool,
    /// Einstellungen neu laden (nach dem Bearbeiten der config.toml)
    #[arg(long, group = "action")]
    reload: bool,
    /// Zeigt, ob das Overlay läuft (Exit-Code 0) oder nicht (1)
    #[arg(long, group = "action")]
    status: bool,
    /// Monitore auflisten (Namen für --output)
    #[arg(long, group = "action")]
    list_outputs: bool,
    /// Beispiel-Einstellungsdatei anlegen (falls noch keine existiert)
    #[arg(long, group = "action")]
    write_config: bool,
    /// Startskript + Startmenü-Eintrag „Roblox mit Fadenkreuz“ einrichten
    #[arg(long, group = "action")]
    install: bool,
    /// Startskript + Startmenü-Eintrag wieder entfernen
    #[arg(long, group = "action")]
    uninstall: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let config_path = cli.config.clone().unwrap_or_else(config::default_path);
    match run(cli, config_path) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("roblox-crosshair: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli, config_path: PathBuf) -> Result<ExitCode, String> {
    if cli.list_outputs {
        overlay::list_outputs()?;
        return Ok(ExitCode::SUCCESS);
    }
    if cli.write_config {
        install::write_example_config(&config_path)?;
        return Ok(ExitCode::SUCCESS);
    }
    if cli.install {
        install::install(&config_path)?;
        return Ok(ExitCode::SUCCESS);
    }
    if cli.uninstall {
        install::uninstall()?;
        return Ok(ExitCode::SUCCESS);
    }

    // Commands for a running instance.
    let remote = [
        (cli.toggle, Command::Toggle),
        (cli.show, Command::Show),
        (cli.hide, Command::Hide),
        (cli.quit, Command::Quit),
        (cli.reload, Command::Reload),
        (cli.status, Command::Status),
    ]
    .into_iter()
    .find(|(on, _)| *on)
    .map(|(_, c)| c);

    if let Some(cmd) = remote {
        match ipc::send(cmd).map_err(|e| format!("Steuerung: {e}"))? {
            Some(answer) => {
                println!("{answer}");
                return Ok(if answer.starts_with("error") {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                });
            }
            None if matches!(cmd, Command::Toggle | Command::Show) => {} // not running → start it
            None => {
                println!("not running");
                return Ok(ExitCode::from(1));
            }
        }
    }

    // Start the overlay (only one per desktop session).
    let listener = match ipc::listen() {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            println!("roblox-crosshair läuft bereits (ein-/ausblenden: roblox-crosshair --toggle)");
            return Ok(ExitCode::SUCCESS);
        }
        Err(e) => {
            return Err(format!(
                "Steuer-Socket {}: {e}",
                ipc::socket_path().display()
            ))
        }
    };
    let result = overlay::run(config_path, cli.overrides, listener);
    let _ = std::fs::remove_file(ipc::socket_path());
    result.map(|_| ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn actions_are_mutually_exclusive_and_options_parse() {
        assert!(Cli::try_parse_from(["x", "--toggle", "--quit"]).is_err());
        let c = Cli::try_parse_from([
            "x",
            "--shape",
            "dot",
            "--color",
            "red",
            "--offset-x",
            "-3",
            "--opacity",
            "0.7",
        ])
        .unwrap();
        assert_eq!(c.overrides.shape, Some(config::Shape::Dot));
        assert_eq!(c.overrides.offset_x, Some(-3.0));
    }
}
