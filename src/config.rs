//! Crosshair settings: defaults, the optional TOML file and command-line overrides.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Shape of the crosshair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
#[value(rename_all = "kebab-case")]
pub enum Shape {
    /// Punkt
    Dot,
    /// Kreuz
    Cross,
    /// Kreuz mit Lücke in der Mitte
    CrossGap,
}

/// An RGB colour, written as `#rrggbb` or a few names (`green`, `red`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const NEON_GREEN: Color = Color {
        r: 0x39,
        g: 0xff,
        b: 0x14,
    };
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };
}

impl FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = s.trim().to_ascii_lowercase();
        let named = match t.as_str() {
            "green" | "neon-green" | "neongreen" | "neongruen" | "neongrün" | "grün" | "gruen" => {
                Some("#39ff14")
            }
            "red" | "rot" => Some("#ff1a1a"),
            "yellow" | "gelb" => Some("#ffe600"),
            "cyan" | "türkis" | "tuerkis" => Some("#00ffff"),
            "magenta" | "pink" => Some("#ff00ff"),
            "white" | "weiß" | "weiss" => Some("#ffffff"),
            "black" | "schwarz" => Some("#000000"),
            "orange" => Some("#ff8c00"),
            "blue" | "blau" => Some("#2f7bff"),
            _ => None,
        };
        let hex = named.unwrap_or(t.as_str());
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "unbekannte Farbe {s:?} – bitte als #rrggbb angeben (z. B. #39ff14) oder einen Namen wie green, red, yellow"
            ));
        }
        let v = u32::from_str_radix(hex, 16).map_err(|e| e.to_string())?;
        Ok(Color {
            r: (v >> 16) as u8,
            g: (v >> 8) as u8,
            b: v as u8,
        })
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl Serialize for Color {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// Everything that defines how the crosshair looks and where it sits.
/// All lengths are in *logical* pixels (what KDE calls "pixels" at the chosen scaling).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub shape: Shape,
    pub color: Color,
    /// Total width/height of the crosshair (dot: diameter).
    pub size: f64,
    /// Line thickness of the cross arms.
    pub thickness: f64,
    /// Empty space in the middle for `cross-gap`.
    pub gap: f64,
    /// 0.0 (invisible) … 1.0 (fully opaque).
    pub opacity: f64,
    /// Draw a dark outline so the crosshair stays visible on light and dark backgrounds.
    pub outline: bool,
    pub outline_thickness: f64,
    pub outline_color: Color,
    /// Shift from the exact screen centre (positive = right / down).
    pub offset_x: f64,
    pub offset_y: f64,
    /// Monitor name (e.g. "DP-1"). Empty = primary monitor.
    pub output: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            shape: Shape::CrossGap,
            color: Color::NEON_GREEN,
            size: 20.0,
            thickness: 2.0,
            gap: 6.0,
            opacity: 1.0,
            outline: true,
            outline_thickness: 1.0,
            outline_color: Color::BLACK,
            offset_x: 0.0,
            offset_y: 0.0,
            output: String::new(),
        }
    }
}

/// Command-line overrides; `None` = keep the value from the file / default.
#[derive(Debug, Clone, Default, clap::Args)]
pub struct Overrides {
    /// Form: dot, cross oder cross-gap
    #[arg(long, value_enum)]
    pub shape: Option<Shape>,
    /// Farbe, z. B. #39ff14 oder green/red/yellow/cyan/white
    #[arg(long)]
    pub color: Option<Color>,
    /// Gesamtgröße in Pixeln (beim Punkt: Durchmesser)
    #[arg(long)]
    pub size: Option<f64>,
    /// Linienstärke in Pixeln
    #[arg(long)]
    pub thickness: Option<f64>,
    /// Lücke in der Mitte in Pixeln (nur cross-gap)
    #[arg(long)]
    pub gap: Option<f64>,
    /// Deckkraft 0.0 bis 1.0 (oder 0 bis 100 als Prozent)
    #[arg(long)]
    pub opacity: Option<f64>,
    /// Schwarzen Rand ein (true) oder aus (false)
    #[arg(long)]
    pub outline: Option<bool>,
    /// Randstärke in Pixeln
    #[arg(long)]
    pub outline_thickness: Option<f64>,
    /// Randfarbe
    #[arg(long)]
    pub outline_color: Option<Color>,
    /// Versatz nach rechts (negativ: links) in Pixeln
    #[arg(long, allow_hyphen_values = true)]
    pub offset_x: Option<f64>,
    /// Versatz nach unten (negativ: oben) in Pixeln
    #[arg(long, allow_hyphen_values = true)]
    pub offset_y: Option<f64>,
    /// Monitor-Name (siehe --list-outputs). Standard: primärer Monitor
    #[arg(long)]
    pub output: Option<String>,
}

impl Config {
    pub fn apply(&mut self, o: &Overrides) {
        if let Some(v) = o.shape {
            self.shape = v;
        }
        if let Some(v) = o.color {
            self.color = v;
        }
        if let Some(v) = o.size {
            self.size = v;
        }
        if let Some(v) = o.thickness {
            self.thickness = v;
        }
        if let Some(v) = o.gap {
            self.gap = v;
        }
        if let Some(v) = o.opacity {
            self.opacity = v;
        }
        if let Some(v) = o.outline {
            self.outline = v;
        }
        if let Some(v) = o.outline_thickness {
            self.outline_thickness = v;
        }
        if let Some(v) = o.outline_color {
            self.outline_color = v;
        }
        if let Some(v) = o.offset_x {
            self.offset_x = v;
        }
        if let Some(v) = o.offset_y {
            self.offset_y = v;
        }
        if let Some(v) = &o.output {
            self.output = v.clone();
        }
    }

    /// Clamp values into sane ranges (and accept opacity given in percent).
    pub fn sanitize(&mut self) {
        if self.opacity > 1.0 {
            self.opacity /= 100.0; // "80" means 80 %
        }
        self.opacity = self.opacity.clamp(0.0, 1.0);
        self.size = self.size.clamp(1.0, 500.0);
        self.thickness = self.thickness.clamp(1.0, self.size);
        self.gap = self.gap.clamp(0.0, self.size);
        self.outline_thickness = self.outline_thickness.clamp(0.0, 20.0);
        self.offset_x = self.offset_x.clamp(-5000.0, 5000.0);
        self.offset_y = self.offset_y.clamp(-5000.0, 5000.0);
    }

    /// Default + file (if it exists) + CLI overrides.
    pub fn load(path: &Path, overrides: &Overrides) -> Result<Config, String> {
        let mut cfg = match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str::<Config>(&text).map_err(|e| {
                format!("Fehler in der Konfigurationsdatei {}:\n{e}", path.display())
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
            Err(e) => return Err(format!("Kann {} nicht lesen: {e}", path.display())),
        };
        cfg.apply(overrides);
        cfg.sanitize();
        Ok(cfg)
    }
}

pub fn default_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"));
    base.join("roblox-crosshair").join("config.toml")
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// Commented example config (written by `--write-config`).
pub const EXAMPLE_CONFIG: &str = r##"# Einstellungen für roblox-crosshair
# Nach dem Ändern speichern und `roblox-crosshair --reload` ausführen
# (oder Roblox neu starten). Alle Längen in Pixeln.

# Form: "dot" (Punkt), "cross" (Kreuz), "cross-gap" (Kreuz mit Lücke in der Mitte)
shape = "cross-gap"

# Farbe als #rrggbb oder Name: green, red, yellow, cyan, magenta, white, orange, blue
color = "#39ff14"

# Gesamtgröße (beim Punkt: Durchmesser)
size = 20

# Linienstärke der Kreuz-Arme
thickness = 2

# Lücke in der Mitte (nur bei "cross-gap")
gap = 6

# Deckkraft: 1.0 = voll sichtbar, 0.5 = halb durchsichtig
opacity = 1.0

# Dunkler Rand um das Fadenkreuz – damit es auf hellen UND dunklen Wänden sichtbar ist
outline = true
outline_thickness = 1
outline_color = "#000000"

# Verschieben, falls die Mitte im Spiel nicht ganz passt (rechts/unten = positiv)
offset_x = 0
offset_y = 0

# Monitor, z. B. "DP-1" (Liste: roblox-crosshair --list-outputs). Leer = primärer Monitor
output = ""
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_names() {
        assert_eq!("#39ff14".parse::<Color>().unwrap(), Color::NEON_GREEN);
        assert_eq!("39FF14".parse::<Color>().unwrap(), Color::NEON_GREEN);
        assert_eq!("green".parse::<Color>().unwrap(), Color::NEON_GREEN);
        assert_eq!(
            "Rot".parse::<Color>().unwrap(),
            Color {
                r: 0xff,
                g: 0x1a,
                b: 0x1a
            }
        );
        assert!("#12345".parse::<Color>().is_err());
        assert!("lila-ish".parse::<Color>().is_err());
    }

    #[test]
    fn example_config_parses_to_the_defaults() {
        let cfg: Config = toml::from_str(EXAMPLE_CONFIG).unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn partial_file_keeps_defaults_and_rejects_typos() {
        let cfg: Config = toml::from_str("shape = \"dot\"\nsize = 8").unwrap();
        assert_eq!(cfg.shape, Shape::Dot);
        assert_eq!(cfg.size, 8.0);
        assert_eq!(cfg.color, Color::NEON_GREEN);
        assert!(toml::from_str::<Config>("colour = \"red\"").is_err());
    }

    #[test]
    fn cli_overrides_file_and_sanitize_clamps() {
        let mut cfg = Config::default();
        cfg.apply(&Overrides {
            shape: Some(Shape::Cross),
            opacity: Some(80.0),
            thickness: Some(999.0),
            offset_x: Some(-3.0),
            ..Default::default()
        });
        cfg.sanitize();
        assert_eq!(cfg.shape, Shape::Cross);
        assert!((cfg.opacity - 0.8).abs() < 1e-9);
        assert_eq!(cfg.thickness, cfg.size);
        assert_eq!(cfg.offset_x, -3.0);
    }

    #[test]
    fn load_missing_file_gives_defaults() {
        let cfg = Config::load(Path::new("/nonexistent/x.toml"), &Overrides::default()).unwrap();
        assert_eq!(cfg, Config::default());
    }
}
