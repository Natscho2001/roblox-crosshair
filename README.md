# roblox-crosshair – Fadenkreuz für Roblox unter Linux

Ein kleines Programm, das ein gut sichtbares **Fadenkreuz genau in die Bildschirmmitte**
legt – zum Beispiel für Roblox „Murder Mystery 2“, wo das eingebaute Fadenkreuz
(vier blasse Punkte) vor grauen Wänden kaum zu sehen ist.

- liegt **immer über allen Fenstern**, auch über Roblox
- ist **komplett durchklickbar** – Maus und Tastatur gehen ganz normal ans Spiel
- Form, Farbe, Größe, Rand, Deckkraft und Position sind einstellbar
- per Tastenkürzel ein- und ausschaltbar
- startet auf Wunsch automatisch zusammen mit Roblox (Sober) und geht danach wieder aus

> **Gut zu wissen:** Das ist ein reines **Bild-Overlay**. Es malt nur ein Kreuz über den
> Bildschirm – es liest nichts aus dem Spiel, verändert nichts am Spiel und schickt keine
> Eingaben. Für Roblox ist es nicht anders als ein Aufkleber auf dem Monitor.

Es gibt zwei Ausführungen – je nach Desktop:

| Desktop | Was du brauchst | Anleitung |
|---|---|---|
| **GNOME** (z. B. Bazzite GNOME) | die **GNOME-Erweiterung** | [Abschnitt GNOME](#gnome-z-b-bazzite-gnome) |
| **KDE Plasma 6**, Sway, Hyprland (Wayland) | das Programm `roblox-crosshair` (einzelne Datei oder AppImage) | ab Schritt 1 |

Welcher Desktop läuft, zeigt im Terminal: `echo $XDG_CURRENT_DESKTOP`.
Auf beiden muss nichts per `dnf` oder `rpm-ostree` installiert werden.

---

## GNOME (z. B. Bazzite GNOME)

GNOME erlaubt fremden Programmen keine Overlays – deshalb gibt es dort eine kleine
**GNOME-Erweiterung**. Sie kann sogar mehr als das Programm für KDE:

- **folgt dem Mauszeiger**, wenn ohne Shift-Lock gespielt wird (dann zielt Roblox auf
  den Zeiger), und springt in die **Bildschirmmitte**, sobald Roblox den Zeiger
  versteckt (Shift-Lock, Ego-Perspektive) – Einstellung „Position: Automatisch“
- erscheint **nur, wenn Roblox im Vordergrund ist** – kein Startskript nötig
- **Schalter „Fadenkreuz“** im Schnellmenü oben rechts und Tastenkürzel
  (Standard: **Super + Alt + C**, Super = Windows-Taste)
- alle Einstellungen in einem normalen Einstellungsfenster

### Installieren (einmalig)

1. Terminal öffnen und einfügen:
   ```bash
   curl -L -o /tmp/fadenkreuz.zip https://github.com/Natscho2001/roblox-crosshair/releases/latest/download/roblox-crosshair@natscho2001.github.io.shell-extension.zip
   gnome-extensions install --force /tmp/fadenkreuz.zip
   ```
2. **Abmelden und wieder anmelden** (GNOME lädt neue Erweiterungen erst nach einer
   neuen Anmeldung).
3. Einschalten:
   ```bash
   gnome-extensions enable roblox-crosshair@natscho2001.github.io
   ```
   oder in der App **„Erweiterungen“** bzw. **„Extension Manager“** den Schalter bei
   „Roblox Fadenkreuz“ umlegen.

Fertig. Roblox ganz normal starten – das Fadenkreuz erscheint, sobald das
Roblox-Fenster im Vordergrund ist.

### Einstellungen

In der App „Erweiterungen“ auf das Zahnrad bei „Roblox Fadenkreuz“ klicken – oder im
Terminal `gnome-extensions prefs roblox-crosshair@natscho2001.github.io`. Dort:

| Einstellung | Bedeutung |
|---|---|
| **Position** | *Automatisch* (Standard): am Mauszeiger, in der Mitte wenn Roblox den Zeiger versteckt · *Immer am Mauszeiger* · *Immer in der Mitte* |
| **Anzeigen** | nur wenn Roblox im Vordergrund ist (Standard) · solange Roblox läuft · immer |
| **Mitte von** | Bildschirm oder Roblox-Fenster (wenn es in der Mitte steht) |
| Form, Farbe, Größe, Linienstärke, Lücke, Deckkraft, Rand | wie beim KDE-Programm, siehe Tabelle in Schritt 4 |
| Nach rechts / nach unten | verschieben, falls es nicht genau passt |
| Tastenkürzel | z. B. `<Super><Alt>c` oder `F8` – eintragen und mit dem Haken bestätigen |

### Wenn etwas nicht klappt (GNOME)

| Problem | Lösung |
|---|---|
| Erweiterung taucht nicht auf | Nach dem Installieren ab- und wieder anmelden. |
| „Nicht kompatibel“ / „Veraltet“ | Die GNOME-Version ist neuer als getestet. `gnome-shell --version` ausführen und mir die Zahl sagen – dann wird die Erweiterung freigegeben. |
| Fadenkreuz erscheint nicht | Ist es eingeschaltet (Schnellmenü oben rechts)? Ist das Roblox-Fenster im Vordergrund? Testweise „Anzeigen: Immer“ wählen. |
| Fadenkreuz hängt etwas hinter dem Mauszeiger | Bei sehr schnellen Bewegungen kann es minimal nachziehen; beim Zielen in Ruhe sitzt es genau. |
| Tastenkürzel geht im Spiel nicht | Manche Spiele im Vollbild blockieren Kürzel – dann den Schalter im Schnellmenü nehmen oder ein anderes Kürzel wählen. |
| Fehlersuche | `journalctl --user -b -g roblox-crosshair` zeigt Meldungen der Erweiterung. |

**Entfernen:** `gnome-extensions uninstall roblox-crosshair@natscho2001.github.io`

---

## KDE Plasma, Sway, Hyprland

Das Programm ist eine einzelne Datei. Funktioniert unter KDE Plasma 6, Sway und
Hyprland (Wayland), **nicht** unter GNOME (→ Erweiterung oben) und nicht in einer
X11-Sitzung. Hinweis: Unter KDE bleibt das Fadenkreuz in der Bildschirmmitte – einem
Mauszeiger folgen kann ein externes Programm unter Wayland nicht.

---

## 1. Herunterladen und einrichten (einmalig, ca. 2 Minuten)

### Variante A: im Terminal (am schnellsten)

Konsole öffnen (Startmenü → „Konsole“) und diese vier Zeilen einfügen:

```bash
mkdir -p ~/Applications
curl -L -o ~/Applications/roblox-crosshair https://github.com/Natscho2001/roblox-crosshair/releases/latest/download/roblox-crosshair
chmod +x ~/Applications/roblox-crosshair
~/Applications/roblox-crosshair --install
```

Fertig. Im Startmenü gibt es jetzt **„Roblox mit Fadenkreuz“**.

### Variante B: mit der Maus

1. Diese Datei herunterladen:
   **[roblox-crosshair](https://github.com/Natscho2001/roblox-crosshair/releases/latest/download/roblox-crosshair)**
2. Im Dateimanager (Dolphin) im persönlichen Ordner einen Ordner **`Applications`** anlegen
   und die Datei aus „Downloads“ dorthin verschieben.
3. **Ausführbar machen:** Rechtsklick auf die Datei → *Eigenschaften* → Reiter
   *Berechtigungen* → Haken bei **„Ausführbar“** → *OK*.
   (Das ist dasselbe wie `chmod +x` im Terminal.)
4. Einmal den Startmenü-Eintrag anlegen lassen: Im Ordner `Applications` mit der rechten
   Maustaste ins Leere klicken → *Terminal hier öffnen* →
   `./roblox-crosshair --install` eintippen und Enter drücken.

### Variante C: als AppImage

Wer Programme lieber als AppImage verwaltet (Bazzite bringt dafür **Gear Lever** mit):

1. **[roblox-crosshair-x86_64.AppImage](https://github.com/Natscho2001/roblox-crosshair/releases/latest/download/roblox-crosshair-x86_64.AppImage)**
   herunterladen.
2. Doppelklicken. Bazzite öffnet AppImages mit Gear Lever – dort die Datei ins
   App-Menü übernehmen lassen. Danach gibt es im Startmenü **„Roblox Fadenkreuz“**:
   Ein Klick darauf blendet das Fadenkreuz ein bzw. wieder aus.
   (Ohne Gear Lever: Datei wie in Variante B ausführbar machen und doppelklicken.)
3. Für den gemeinsamen Start mit Roblox einmal im Terminal ausführen – mit dem Pfad, an
   dem die AppImage-Datei jetzt liegt (Gear Lever zeigt ihn an, meist `~/AppImages/…`):
   ```bash
   ~/AppImages/roblox-crosshair-x86_64.AppImage --install
   ```
   Wichtig: Wird die AppImage-Datei später verschoben oder umbenannt, diesen Befehl
   noch einmal ausführen.

In allen Befehlen dieser Anleitung kann statt `~/Applications/roblox-crosshair` auch der
Pfad zur AppImage-Datei stehen, z. B. beim Kurzbefehl in Schritt 3:
`/home/BENUTZERNAME/AppImages/roblox-crosshair-x86_64.AppImage --toggle`.
Das AppImage startet einen Hauch langsamer (es wird bei jedem Aufruf kurz eingehängt) –
für den Alltag ist die einfache Datei aus Variante A/B die erste Wahl.

`--install` legt drei Dinge an:

| Was | Wo |
|---|---|
| Startskript, das Roblox + Fadenkreuz gemeinsam startet | neben dem Programm, z. B. `~/Applications/roblox-mit-fadenkreuz.sh` |
| Startmenü-Eintrag „Roblox mit Fadenkreuz“ | `~/.local/share/applications/` |
| Einstellungsdatei mit Erklärungen | `~/.config/roblox-crosshair/config.toml` |

## 2. Spielen

Roblox ab jetzt über **„Roblox mit Fadenkreuz“** im Startmenü starten.
Das Fadenkreuz erscheint sofort und verschwindet wieder, wenn Roblox geschlossen wird.

**Wichtig:** Das Fadenkreuz sitzt in der Mitte des **Bildschirms**. Roblox deshalb
**maximiert** oder im Vollbild (F11) spielen – sonst ist die Spielmitte nicht die
Bildschirmmitte.

Man kann das Overlay auch ohne Roblox starten: einfach `~/Applications/roblox-crosshair`
ausführen.

## 3. Ein-/Ausschalten per Taste (empfohlen)

Unter Wayland darf ein Programm keine Tasten „mithören“ – deshalb übernimmt KDE das:

1. **Systemeinstellungen** öffnen → **Tastatur** → **Kurzbefehle**.
2. Unten auf **„+ Neu hinzufügen“** → **„Befehl oder Skript…“**.
3. Als Befehl eintragen (Benutzername anpassen – im Terminal zeigt `echo $HOME` den Pfad):
   ```
   /home/BENUTZERNAME/Applications/roblox-crosshair --toggle
   ```
4. Als Namen z. B. „Fadenkreuz an/aus“ eingeben, auf **„Kurzbefehl hinzufügen“** klicken
   und eine Taste drücken, z. B. **F8**. Mit **Anwenden** speichern.

Ab jetzt blendet F8 das Fadenkreuz aus und wieder ein. Läuft es gar nicht, startet F8 es.

Weitere Befehle (z. B. für eigene Kurzbefehle):

| Befehl | Wirkung |
|---|---|
| `roblox-crosshair --toggle` | ein-/ausblenden (startet es, falls nötig) |
| `roblox-crosshair --show` / `--hide` | einblenden / ausblenden |
| `roblox-crosshair --reload` | geänderte Einstellungen übernehmen |
| `roblox-crosshair --quit` | beenden |
| `roblox-crosshair --status` | läuft es? |
| `roblox-crosshair --list-outputs` | Monitore anzeigen |

## 4. Aussehen anpassen

Die Datei `~/.config/roblox-crosshair/config.toml` mit einem Texteditor (z. B. Kate)
öffnen – jede Zeile ist erklärt. Nach dem Speichern `roblox-crosshair --reload`
ausführen (oder Roblox neu starten).

| Einstellung | Bedeutung | Standard |
|---|---|---|
| `shape` | `"dot"` Punkt · `"cross"` Kreuz · `"cross-gap"` Kreuz mit Lücke | `"cross-gap"` |
| `color` | Farbe als `#rrggbb` oder Name (`green`, `red`, `yellow`, `cyan`, `magenta`, `white`, `orange`, `blue`) | `"#39ff14"` (Neongrün) |
| `size` | Gesamtgröße in Pixeln (Punkt: Durchmesser) | `20` |
| `thickness` | Linienstärke | `2` |
| `gap` | Lücke in der Mitte (nur `cross-gap`) | `6` |
| `opacity` | Deckkraft: `1.0` voll, `0.5` halb durchsichtig | `1.0` |
| `outline` | dunkler Rand – sichtbar auf hellen **und** dunklen Wänden | `true` |
| `outline_thickness`, `outline_color` | Randstärke und -farbe | `1`, `"#000000"` |
| `offset_x`, `offset_y` | verschieben, falls die Mitte im Spiel leicht daneben liegt (rechts/unten positiv) | `0` |
| `output` | Monitor-Name, z. B. `"DP-1"` – leer = primärer Monitor | `""` |

Alles geht auch einmalig per Kommandozeile, z. B.:

```bash
~/Applications/roblox-crosshair --shape dot --color red --size 8
```

Größen sind „Bildschirm-Pixel“ so, wie KDE sie bei der eingestellten Skalierung
(100 %, 125 %, 150 % …) anzeigt – das Fadenkreuz sieht also bei jeder Skalierung gleich
groß aus und bleibt gestochen scharf.

**Mehrere Monitore:** Standard ist der primäre Monitor (in KDE: Systemeinstellungen →
Anzeige → „Primär“). Einen anderen wählt man mit `output = "HDMI-A-1"` – die Namen
zeigt `roblox-crosshair --list-outputs`.

## 5. Wenn etwas nicht klappt

| Problem | Lösung |
|---|---|
| **„Keine Verbindung zum Wayland-Desktop“** | Beim Anmelden „Plasma (Wayland)“ wählen, nicht „Plasma (X11)“. |
| **„unterstützt kein wlr-layer-shell“** | Läuft nur unter KDE Plasma, Sway, Hyprland – nicht unter GNOME. |
| **Fadenkreuz nicht in der Spielmitte** | Roblox maximieren oder F11 (Vollbild). Bleibt ein kleiner Versatz: `offset_x` / `offset_y` anpassen. |
| **Fadenkreuz auf dem falschen Monitor** | `--list-outputs` ausführen und den Namen bei `output` eintragen. |
| **Taste (F8) tut nichts** | Im Kurzbefehl den **vollständigen** Pfad verwenden (`/home/…/Applications/roblox-crosshair --toggle`). Prüfen, ob die Taste nicht schon anderweitig belegt ist. |
| **„Roblox mit Fadenkreuz“ fehlt im Startmenü** | `~/Applications/roblox-crosshair --install` (bzw. mit dem AppImage-Pfad) noch einmal ausführen. Wurde die Datei verschoben, ebenfalls neu ausführen. |
| **AppImage startet nicht** („FUSE“ in der Fehlermeldung) | Auf Bazzite sollte das nicht vorkommen. Notlösung: einfach die normale Datei aus Variante A/B verwenden – sie braucht kein FUSE. |
| **„läuft bereits“** | Es ist schon eins aktiv – mit `--toggle` ein-/ausblenden oder mit `--quit` beenden. |
| **Fehler in der Konfigurationsdatei** | Die Meldung nennt die Zeile. Notfalls die Datei löschen und `roblox-crosshair --write-config` ausführen. |
| **Datei lässt sich nicht starten („Permission denied“)** | Ausführbar machen (Schritt 1, Punkt 3) bzw. `chmod +x ~/Applications/roblox-crosshair`. |

Überprüfen lässt sich alles im Terminal: `~/Applications/roblox-crosshair` direkt starten
zeigt Fehlermeldungen an (mit Strg+C beenden).

**Entfernen:** `~/Applications/roblox-crosshair --uninstall`, danach die Datei
`~/Applications/roblox-crosshair` und den Ordner `~/.config/roblox-crosshair` löschen.

**Aktualisieren:** Variante A aus Schritt 1 einfach noch einmal ausführen.

---

## Für Neugierige: Wie funktioniert das?

- Das Programm meldet sich beim Desktop (KWin) als **Overlay-Ebene** an
  (Wayland-Protokoll `wlr-layer-shell`, Ebene „overlay“). Diese Ebene liegt über allen
  Fenstern, auch über Vollbild-Spielen.
- Die Fläche hat eine **leere Eingabe-Region** und bekommt **nie den Tastatur-Fokus** –
  deshalb gehen alle Klicks und Tasten durch zum Spiel.
- Die Fläche bedeckt den ganzen Monitor (Leisten werden ignoriert), das Kreuz wird in
  ihrer Mitte gezeichnet. So zählt genau die Mitte, die auch der Desktop verwendet.
- Bei krummer Skalierung (125 %, 150 %) wird über `wp_fractional_scale_v1` und
  `wp_viewporter` in der echten Bildschirmauflösung gezeichnet – scharf und mittig.
- Den primären Monitor erfährt es über das KDE-Protokoll `kde_output_order_v1`.
- Ein-/Ausschalten läuft über einen kleinen Steuer-Socket in `$XDG_RUNTIME_DIR`.

**GNOME-Erweiterung:** `gnome-extension/` (GNOME 45+). Das Fadenkreuz ist ein nicht
anklickbares Element in der obersten Ebene der GNOME-Oberfläche, außerhalb der
Eingabe-Region. Die Zeigerposition liest die Erweiterung über GNOMEs eigenen
`PointerWatcher` (wie die Bildschirmlupe). Paket bauen: `gnome-extension/build.sh dist`.
Tests: `node --test gnome-extension/tests/*.test.js` und
`gnome-extension/tests/headless_gnome.sh dist/*.shell-extension.zip` (startet eine
unsichtbare GNOME Shell).

**Selbst bauen:** Rust installieren (`rustup`), dann
`cargo build --release --target x86_64-unknown-linux-musl`. Tests: `cargo test` und
`python3 tests/headless_sway.py target/x86_64-unknown-linux-musl/release/roblox-crosshair`
(braucht `sway` und `grim`, startet einen unsichtbaren Test-Desktop).
GitHub Actions baut bei jedem Tag `v*` automatisch ein Release (einfache Datei + AppImage,
beide im Test-Desktop geprüft).

Lizenz: MIT
