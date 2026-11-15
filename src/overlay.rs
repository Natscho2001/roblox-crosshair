//! The Wayland side: one full-output wlr-layer-shell surface on the `overlay` layer,
//! click-through (empty input region, no keyboard focus), redrawn only when the size,
//! scale, config or visibility changes.
//!
//! The surface covers the whole output (anchored to all edges, exclusive zone -1 so
//! panels don't shrink it) and the crosshair is drawn at its centre. That way the
//! compositor's own idea of "the middle of the screen" is used — no guessing about
//! panel sizes or rounding — and an X/Y offset is just a shift inside the buffer.
//!
//! HiDPI: with wp_fractional_scale_v1 + wp_viewporter (KWin, Sway) the buffer is
//! rendered at the exact physical resolution (e.g. 1.25 × logical) and mapped 1:1 via
//! the viewport, so lines stay crisp. Without them, the integer buffer scale is used.

use crate::config::{Config, Overrides};
use crate::geometry::render;
use crate::ipc::{self, Command};
use rustix::event::{poll, PollFd, PollFlags};
use rustix::fs::{ftruncate, memfd_create, MemfdFlags};
use rustix::mm::{mmap, munmap, MapFlags, ProtFlags};
use std::os::fd::AsFd;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use wayland_client::globals::{registry_queue_init, GlobalList, GlobalListContents};
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_output, wl_region, wl_registry, wl_shm, wl_shm_pool, wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1, wp_fractional_scale_v1,
};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};
use wayland_protocols_plasma::output_order::v1::client::kde_output_order_v1;
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub output: wl_output::WlOutput,
    pub name: String,
    pub description: String,
    pub scale: i32,
}

struct State {
    outputs: Vec<OutputInfo>,
    /// Output names in KDE's order (first = primary), if the compositor tells us.
    output_order: Vec<String>,
    order_done: bool,
    // Surface state
    configured: bool,
    width: u32,
    height: u32,
    /// Fractional scale × 120 (wp_fractional_scale_v1), 0 = unknown.
    scale120: u32,
    /// Integer scale (wl_surface.preferred_buffer_scale or the output's scale).
    int_scale: i32,
    dirty: bool,
    closed: bool,
}

impl State {
    fn new() -> Self {
        State {
            outputs: Vec::new(),
            output_order: Vec::new(),
            order_done: false,
            configured: false,
            width: 0,
            height: 0,
            scale120: 0,
            int_scale: 1,
            dirty: false,
            closed: false,
        }
    }
}

/// Connect, bind the globals we need and learn the outputs.
struct Session {
    conn: Connection,
    queue: EventQueue<State>,
    globals: GlobalList,
    state: State,
}

impl Session {
    fn connect() -> Result<Session, String> {
        let conn = Connection::connect_to_env().map_err(|e| {
            format!(
                "Keine Verbindung zum Wayland-Desktop ({e}).\n\
                 Läuft das Programm in einer Plasma-Wayland-Sitzung? (X11 wird nicht unterstützt)"
            )
        })?;
        let (globals, mut queue) =
            registry_queue_init::<State>(&conn).map_err(|e| format!("Wayland-Registry: {e}"))?;
        let qh = queue.handle();
        let mut state = State::new();
        globals.contents().with_list(|list| {
            for g in list.iter().filter(|g| g.interface == "wl_output") {
                let output: wl_output::WlOutput =
                    globals.registry().bind(g.name, g.version.min(4), &qh, ());
                state.outputs.push(OutputInfo {
                    output,
                    name: String::new(),
                    description: String::new(),
                    scale: 1,
                });
            }
        });
        let order: Option<kde_output_order_v1::KdeOutputOrderV1> =
            globals.bind(&qh, 1..=1, ()).ok();
        // Two roundtrips: one for the binds, one for the events they trigger.
        queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
        queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
        if let Some(o) = order {
            o.destroy();
        }
        Ok(Session {
            conn,
            queue,
            globals,
            state,
        })
    }

    /// The output to show the crosshair on: by name, else KDE's primary, else let the
    /// compositor decide (None).
    fn pick_output(&self, wanted: &str) -> Result<Option<wl_output::WlOutput>, String> {
        let outs = &self.state.outputs;
        if !wanted.is_empty() {
            return outs
                .iter()
                .find(|o| o.name == wanted)
                .map(|o| Some(o.output.clone()))
                .ok_or_else(|| {
                    format!(
                        "Monitor {wanted:?} nicht gefunden. Vorhanden: {}",
                        outs.iter()
                            .map(|o| o.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                });
        }
        if let Some(primary) = self.state.output_order.first() {
            if let Some(o) = outs.iter().find(|o| &o.name == primary) {
                return Ok(Some(o.output.clone()));
            }
        }
        Ok(None)
    }
}

/// `--list-outputs`
pub fn list_outputs() -> Result<(), String> {
    let s = Session::connect()?;
    let primary = s.state.output_order.first().cloned();
    if s.state.outputs.is_empty() {
        println!("Keine Monitore gemeldet.");
    }
    for o in &s.state.outputs {
        let mark = if primary.as_deref() == Some(o.name.as_str()) {
            "  (primär)"
        } else {
            ""
        };
        println!(
            "{}{}  –  {} (Skalierung {}x)",
            o.name, mark, o.description, o.scale
        );
    }
    if primary.is_none() {
        println!(
            "\nHinweis: Der Desktop meldet keinen primären Monitor; ohne --output wählt er selbst."
        );
    }
    Ok(())
}

/// Run the overlay until `quit`, Ctrl+C or the output disappears.
pub fn run(
    config_path: PathBuf,
    overrides: Overrides,
    listener: UnixListener,
) -> Result<(), String> {
    let mut cfg = Config::load(&config_path, &overrides)?;
    let mut s = Session::connect()?;
    let qh = s.queue.handle();

    let compositor: wl_compositor::WlCompositor = s
        .globals
        .bind(&qh, 4..=6, ())
        .map_err(|_| "Der Desktop bietet kein wl_compositor >= 4 an".to_string())?;
    let shm: wl_shm::WlShm = s
        .globals
        .bind(&qh, 1..=1, ())
        .map_err(|_| "Kein wl_shm".to_string())?;
    let layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1 =
        s.globals.bind(&qh, 1..=4, ()).map_err(|_| {
            "Der Desktop unterstützt kein wlr-layer-shell (Overlay-Fenster).\n\
         KDE Plasma 6, Sway und Hyprland können das – GNOME leider nicht."
                .to_string()
        })?;
    let viewporter: Option<wp_viewporter::WpViewporter> = s.globals.bind(&qh, 1..=1, ()).ok();
    let frac_mgr: Option<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1> =
        s.globals.bind(&qh, 1..=1, ()).ok();

    let output = s.pick_output(&cfg.output)?;
    if let Some(o) = &output {
        if let Some(info) = s.state.outputs.iter().find(|i| &i.output == o) {
            s.state.int_scale = info.scale.max(1);
            eprintln!("roblox-crosshair: Monitor {}", info.name);
        }
    }

    let surface = compositor.create_surface(&qh, ());
    // Empty input region = every click and key goes to the window underneath.
    let region = compositor.create_region(&qh, ());
    surface.set_input_region(Some(&region));
    region.destroy();

    let layer = layer_shell.get_layer_surface(
        &surface,
        output.as_ref(),
        zwlr_layer_shell_v1::Layer::Overlay,
        "roblox-crosshair".into(),
        &qh,
        (),
    );
    use zwlr_layer_surface_v1::Anchor;
    layer.set_anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right);
    layer.set_size(0, 0); // = fill the anchored area (the whole output)
    layer.set_exclusive_zone(-1); // ignore panels: always the full screen
    layer.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);

    let viewport = match (&viewporter, &frac_mgr) {
        (Some(vp), Some(_)) => Some(vp.get_viewport(&surface, &qh, ())),
        _ => None,
    };
    let fractional = match (&frac_mgr, &viewport) {
        (Some(m), Some(_)) => Some(m.get_fractional_scale(&surface, &qh, ())),
        _ => None,
    };
    surface.commit(); // initial commit without buffer → compositor sends configure

    let mut visible = true;
    let mut drawn_once = false;
    while !s.state.closed {
        // Flush, then wait for Wayland events or a control connection.
        s.conn.flush().map_err(|e| format!("Wayland: {e}"))?;
        if let Some(guard) = s.queue.prepare_read() {
            let wl_ready = {
                let wl_fd = guard.connection_fd();
                let mut fds = [
                    PollFd::new(&wl_fd, PollFlags::IN),
                    PollFd::new(&listener, PollFlags::IN),
                ];
                match poll(&mut fds, None) {
                    Ok(_) => {}
                    Err(rustix::io::Errno::INTR) => continue,
                    Err(e) => return Err(format!("poll: {e}")),
                }
                !fds[0].revents().is_empty()
            };
            if wl_ready {
                guard
                    .read()
                    .map_err(|e| format!("Wayland-Verbindung verloren: {e}"))?;
            }
        }
        s.queue
            .dispatch_pending(&mut s.state)
            .map_err(|e| format!("Wayland: {e}"))?;

        // Control commands (several may be queued).
        while let Ok((stream, _)) = listener.accept() {
            // Connections without a command (e.g. the probe of a second instance) are skipped.
            let Some(cmd) = ipc::read_command(&stream) else {
                ipc::reply(&stream, "error: unbekannter Befehl");
                continue;
            };
            match cmd {
                Command::Toggle => visible = !visible,
                Command::Show => visible = true,
                Command::Hide => visible = false,
                Command::Quit => s.state.closed = true,
                Command::Reload => match Config::load(&config_path, &overrides) {
                    Ok(new) => {
                        if new.output != cfg.output {
                            eprintln!("roblox-crosshair: Monitor-Wechsel wirkt erst nach Neustart");
                        }
                        cfg = new;
                    }
                    Err(e) => {
                        ipc::reply(&stream, &format!("error: {e}"));
                        continue;
                    }
                },
                Command::Status => {}
            }
            s.state.dirty = true;
            let answer = if s.state.closed {
                "quit"
            } else if visible {
                "visible"
            } else {
                "hidden"
            };
            ipc::reply(&stream, answer);
        }

        if s.state.configured && (s.state.dirty || !drawn_once) && !s.state.closed {
            draw(
                &mut s.state,
                &qh,
                &shm,
                &surface,
                viewport.as_ref(),
                &cfg,
                visible,
            )?;
            drawn_once = true;
        }
    }

    if let Some(f) = fractional {
        f.destroy();
    }
    if let Some(v) = viewport {
        v.destroy();
    }
    layer.destroy();
    surface.destroy();
    let _ = s.conn.flush();
    Ok(())
}

fn draw(
    st: &mut State,
    qh: &QueueHandle<State>,
    shm: &wl_shm::WlShm,
    surface: &wl_surface::WlSurface,
    viewport: Option<&wp_viewport::WpViewport>,
    cfg: &Config,
    visible: bool,
) -> Result<(), String> {
    st.dirty = false;
    if st.width == 0 || st.height == 0 {
        return Ok(());
    }
    // Buffer size in physical pixels and the scale used to convert config lengths.
    let (bw, bh, scale) = match viewport {
        Some(_) if st.scale120 > 0 => {
            let f = st.scale120 as f64 / 120.0;
            (
                ((st.width as f64) * f).round() as u32,
                ((st.height as f64) * f).round() as u32,
                f,
            )
        }
        _ => {
            let s = st.int_scale.max(1) as u32;
            (st.width * s, st.height * s, s as f64)
        }
    };
    let stride = bw as usize * 4;
    let len = stride * bh as usize;

    let fd =
        memfd_create("roblox-crosshair", MemfdFlags::CLOEXEC).map_err(|e| format!("memfd: {e}"))?;
    ftruncate(&fd, len as u64).map_err(|e| format!("ftruncate: {e}"))?;
    // SAFETY: fresh private memfd of exactly `len` bytes, mapped once and unmapped below.
    unsafe {
        let ptr = mmap(
            std::ptr::null_mut(),
            len,
            ProtFlags::READ | ProtFlags::WRITE,
            MapFlags::SHARED,
            &fd,
            0,
        )
        .map_err(|e| format!("mmap: {e}"))?;
        let buf = std::slice::from_raw_parts_mut(ptr as *mut u8, len);
        render(buf, bw, bh, scale, cfg, visible);
        munmap(ptr, len).map_err(|e| format!("munmap: {e}"))?;
    }
    let pool = shm.create_pool(fd.as_fd(), len as i32, qh, ());
    let buffer = pool.create_buffer(
        0,
        bw as i32,
        bh as i32,
        stride as i32,
        wl_shm::Format::Argb8888,
        qh,
        (),
    );
    pool.destroy();
    drop(fd); // the request carries its own duplicate of the fd

    surface.attach(Some(&buffer), 0, 0);
    match viewport {
        Some(vp) if st.scale120 > 0 => {
            surface.set_buffer_scale(1);
            vp.set_destination(st.width as i32, st.height as i32);
        }
        Some(vp) => {
            vp.set_destination(-1, -1);
            surface.set_buffer_scale(st.int_scale.max(1));
        }
        None => surface.set_buffer_scale(st.int_scale.max(1)),
    }
    surface.damage_buffer(0, 0, bw as i32, bh as i32);
    surface.commit();
    Ok(())
}

// ── Event handling ──────────────────────────────────────────────────────────────

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Hot-plugged monitors are ignored; the overlay stays on the one it started on.
    }
}

impl Dispatch<wl_output::WlOutput, ()> for State {
    fn event(
        st: &mut Self,
        out: &wl_output::WlOutput,
        ev: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(info) = st.outputs.iter_mut().find(|o| &o.output == out) else {
            return;
        };
        match ev {
            wl_output::Event::Name { name } => info.name = name,
            wl_output::Event::Description { description } => info.description = description,
            wl_output::Event::Scale { factor } => info.scale = factor,
            _ => {}
        }
    }
}

impl Dispatch<kde_output_order_v1::KdeOutputOrderV1, ()> for State {
    fn event(
        st: &mut Self,
        _: &kde_output_order_v1::KdeOutputOrderV1,
        ev: kde_output_order_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match ev {
            kde_output_order_v1::Event::Output { output_name } => {
                if st.order_done {
                    st.output_order.clear();
                    st.order_done = false;
                }
                st.output_order.push(output_name);
            }
            kde_output_order_v1::Event::Done => st.order_done = true,
            _ => {}
        }
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for State {
    fn event(
        st: &mut Self,
        _: &wl_surface::WlSurface,
        ev: wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_surface::Event::PreferredBufferScale { factor } = ev {
            if factor != st.int_scale {
                st.int_scale = factor;
                st.dirty = true;
            }
        }
    }
}

impl Dispatch<wp_fractional_scale_v1::WpFractionalScaleV1, ()> for State {
    fn event(
        st: &mut Self,
        _: &wp_fractional_scale_v1::WpFractionalScaleV1,
        ev: wp_fractional_scale_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = ev {
            if scale != st.scale120 {
                st.scale120 = scale;
                st.dirty = true;
            }
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        st: &mut Self,
        layer: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        ev: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match ev {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                layer.ack_configure(serial);
                if (width, height) != (st.width, st.height) || !st.configured {
                    st.width = width;
                    st.height = height;
                    st.dirty = true;
                }
                st.configured = true;
            }
            zwlr_layer_surface_v1::Event::Closed => st.closed = true,
            _ => {}
        }
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for State {
    fn event(
        _: &mut Self,
        buf: &wl_buffer::WlBuffer,
        ev: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = ev {
            buf.destroy();
        }
    }
}

impl Dispatch<wl_shm::WlShm, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_shm::WlShm,
        _: wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_region::WlRegion);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore zwlr_layer_shell_v1::ZwlrLayerShellV1);
delegate_noop!(State: ignore wp_viewporter::WpViewporter);
delegate_noop!(State: ignore wp_viewport::WpViewport);
delegate_noop!(State: ignore wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1);
