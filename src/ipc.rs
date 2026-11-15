//! Control channel: a Unix socket in $XDG_RUNTIME_DIR. The running overlay listens;
//! a second invocation (`--toggle`, `--quit`, …) connects, sends one command line and
//! prints the answer. This is what makes "toggle on a KDE shortcut" work under Wayland,
//! where an overlay can't grab global hotkeys itself.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Toggle,
    Show,
    Hide,
    Quit,
    Reload,
    Status,
}

impl Command {
    pub fn as_str(self) -> &'static str {
        match self {
            Command::Toggle => "toggle",
            Command::Show => "show",
            Command::Hide => "hide",
            Command::Quit => "quit",
            Command::Reload => "reload",
            Command::Status => "status",
        }
    }

    pub fn parse(s: &str) -> Option<Command> {
        Some(match s.trim() {
            "toggle" => Command::Toggle,
            "show" => Command::Show,
            "hide" => Command::Hide,
            "quit" => Command::Quit,
            "reload" => Command::Reload,
            "status" => Command::Status,
            _ => return None,
        })
    }
}

pub fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    // One overlay per Wayland session (a second display would get its own socket).
    let display = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());
    let display: String = display
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    dir.join(format!("roblox-crosshair-{display}.sock"))
}

/// Send a command to a running instance. `Ok(None)` = no instance is running.
pub fn send(cmd: Command) -> std::io::Result<Option<String>> {
    let mut stream = match UnixStream::connect(socket_path()) {
        Ok(s) => s,
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
            ) =>
        {
            return Ok(None)
        }
        Err(e) => return Err(e),
    };
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(format!("{}\n", cmd.as_str()).as_bytes())?;
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer)?;
    Ok(Some(answer.trim().to_string()))
}

/// Bind the control socket. Fails with AddrInUse if another overlay is running.
pub fn listen() -> std::io::Result<UnixListener> {
    let path = socket_path();
    match UnixListener::bind(&path) {
        Ok(l) => {
            l.set_nonblocking(true)?;
            Ok(l)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            // A live instance answers; a leftover file from a crash doesn't.
            if UnixStream::connect(&path).is_ok() {
                return Err(e);
            }
            std::fs::remove_file(&path)?;
            let l = UnixListener::bind(&path)?;
            l.set_nonblocking(true)?;
            Ok(l)
        }
        Err(e) => Err(e),
    }
}

/// Read one command from an accepted connection; the caller answers via `reply`.
pub fn read_command(stream: &UnixStream) -> Option<Command> {
    stream.set_nonblocking(false).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .ok()?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).ok()?;
    Command::parse(&line)
}

pub fn reply(mut stream: &UnixStream, text: &str) {
    let _ = stream.write_all(format!("{text}\n").as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_round_trip() {
        for c in [
            Command::Toggle,
            Command::Show,
            Command::Hide,
            Command::Quit,
            Command::Reload,
            Command::Status,
        ] {
            assert_eq!(Command::parse(c.as_str()), Some(c));
        }
        assert_eq!(Command::parse("toggle\n"), Some(Command::Toggle));
        assert_eq!(Command::parse("explode"), None);
    }

    #[test]
    fn listen_send_and_stale_socket_recovery() {
        let dir = std::env::temp_dir().join(format!("rc-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // SAFETY (test-only): no other thread reads these variables in this test process.
        unsafe {
            std::env::set_var("XDG_RUNTIME_DIR", &dir);
            std::env::set_var("WAYLAND_DISPLAY", "wayland-test");
        }
        assert_eq!(send(Command::Status).unwrap(), None, "nothing running yet");

        // Stale socket file (bound then dropped without unlink) must be recovered.
        drop(UnixListener::bind(socket_path()).unwrap());
        let listener = listen().unwrap();
        assert!(listen().is_err(), "second instance must not start");

        let t = std::thread::spawn(|| send(Command::Toggle).unwrap());
        // The probe connection from the failed second `listen()` is queued first and
        // carries no command — the overlay just skips such connections.
        let stream = loop {
            match listener.accept() {
                Ok((s, _)) => match read_command(&s) {
                    Some(c) => {
                        assert_eq!(c, Command::Toggle);
                        break s;
                    }
                    None => continue,
                },
                Err(_) => std::thread::sleep(Duration::from_millis(5)),
            }
        };
        reply(&stream, "hidden");
        drop(stream);
        assert_eq!(t.join().unwrap(), Some("hidden".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
