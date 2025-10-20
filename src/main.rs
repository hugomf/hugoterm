//! HugoTerm – clean, fast, single-binary terminal emulator
#![allow(unexpected_cfgs)]

use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, EventControllerKey, gdk,
};
use once_cell::sync::Lazy; // ➊ add once_cell = "1.19" to Cargo.toml
use pango::FontDescription;
use phf::phf_map;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    sync::mpsc,
    thread,
};
use vte4::{Terminal, prelude::*};

#[cfg(target_os = "macos")]
mod macos_blur;

// ---------- Config (immutable after start-up) ------------------------------

static CONFIG: Lazy<Config> = Lazy::new(Config::load);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Config {
    appearance: Appearance,
    cursor: Cursor,
    terminal: Term,
    colors: Colors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Appearance {
    font_family: String,
    font_size: i32,
    foreground_color: String,
    background_color: String,
    opacity: f64,
    blur: bool,
    blur_strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cursor {
    shape: String,
    blink: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Term {
    scrollback_lines: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Colors {
    #[serde(deserialize_with = "de_palette", serialize_with = "ser_palette")]
    palette: [String; 16],
    #[serde(skip)]
    cached: Vec<gdk::RGBA>,
}

// ---------- palette (de)serialisers (keep old TOML layout) -----------------

fn de_palette<'de, D>(d: D) -> Result<[String; 16], D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{MapAccess, Visitor};
    use std::collections::HashMap;

    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = [String; 16];
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a map of 16 color strings with keys color0 to color15")
        }
        
        // This is the main change: We visit a map (the [colors.palette] table)
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut temp_map: HashMap<u8, String> = HashMap::with_capacity(16);

            while let Some((key_str, value)) = map.next_entry::<String, String>()? {
                // Parse the key (e.g., "color5") to get the index (5)
                if let Some(index_str) = key_str.strip_prefix("color") {
                    if let Ok(index) = index_str.parse::<u8>() {
                        if index < 16 {
                            temp_map.insert(index, value);
                            continue;
                        }
                    }
                }
                // Ignore unknown keys, or you can fail with an error
            }

            if temp_map.len() != 16 {
                return Err(serde::de::Error::custom(format!(
                    "expected 16 colors (color0..color15), found {}",
                    temp_map.len()
                )));
            }
            
            // Reconstruct the fixed-size array from the map
            let mut out = core::array::from_fn(|_| String::new());
            for i in 0..16 {
                let index = i as u8;
                // Since we checked for 16 elements, .remove() should always succeed
                // We use remove to satisfy the borrow checker and extract the value
                out[i] = temp_map.remove(&index).ok_or_else(|| {
                    serde::de::Error::custom(format!("Missing key 'color{}'", i))
                })?;
            }
            Ok(out)
        }
    }
    
    // Deserialize as a map (TOML Table)
    d.deserialize_map(V)
}

fn ser_palette<S>(p: &[String; 16], s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeMap;
    let mut map = s.serialize_map(Some(16))?;
    for (i, c) in p.iter().enumerate() {
        map.serialize_entry(&format!("color{i}"), c)?;
    }
    map.end()
}

// ---------- Default implementations ----------------------------------------

impl Default for Appearance {
    fn default() -> Self {
        Self {
            font_family: "Monospace".into(),
            font_size: 12,
            foreground_color: "#d4d4d4".into(),
            background_color: "#1e1e1e".into(),
            opacity: 0.95,
            blur: false,
            blur_strength: 0.0,
        }
    }
}
impl Default for Cursor {
    fn default() -> Self {
        Self {
            shape: "block".into(),
            blink: true,
        }
    }
}
impl Default for Term {
    fn default() -> Self {
        Self {
            scrollback_lines: 10_000,
        }
    }
}
impl Default for Colors {
    fn default() -> Self {
        let palette = [
            "#000000", "#cd3131", "#0dbc79", "#e5e510", "#2472c8", "#bc3fbc", "#11a8cd", "#e5e5e5",
            "#666666", "#f14c4c", "#23d18b", "#f5f543", "#3b8eea", "#d670d6", "#29b8db", "#e5e5e5",
        ]
        .map(String::from);
        let cached = palette.iter().filter_map(|s| hex_to_rgba(s)).collect();
        Self { palette, cached }
    }
}

// ---------- Config loading -------------------------------------------------

impl Config {
    fn path() -> std::path::PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("hugoterm/config.toml")
    }

    fn load() -> Self {
        let path = Self::path();

        //println!("PATH: {:?}", path);
        if !path.exists() {
            let def = Self::default();
            if let Some(p) = path.parent() {
                let _ = std::fs::create_dir_all(p);
                let _ = std::fs::write(&path, toml::to_string_pretty(&def).unwrap());
            }
            return def;
        }
       
        println!("Loading configuration from file...");
        let config = config::Config::builder()
            .add_source(config::File::from(path))
            .build()
            .expect("Failed to load configuration file");
        let cfg: Config = config.try_deserialize().expect("Failed to deserialize configuration");
        println!("Loaded configuration: {:?}", cfg);
        cfg
    }
}

// ---------- Helpers --------------------------------------------------------

fn hex_to_rgba(hex: &str) -> Option<gdk::RGBA> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let rgb = u32::from_str_radix(hex, 16).ok()?;
    Some(gdk::RGBA::new(
        ((rgb >> 16) & 0xff) as f32 / 255.0,
        ((rgb >> 8) & 0xff) as f32 / 255.0,
        (rgb & 0xff) as f32 / 255.0,
        1.0,
    ))
}

// ---------- PTY starter with resize support -------------------------------

fn start_pty(
    tx_out: mpsc::Sender<Vec<u8>>,
    rx_in: mpsc::Receiver<Vec<u8>>,
) -> mpsc::Sender<(u16, u16)> {
    let (tx_resize, rx_resize) = mpsc::channel();
    
    thread::spawn(move || {
        let pty = NativePtySystem::default()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("openpty");

        let shell = if cfg!(windows) {
            "powershell.exe".into()
        } else {
            std::env::var("SHELL").unwrap_or("/bin/sh".into())
        };

        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        let _child = pty.slave.spawn_command(cmd).expect("spawn");
        drop(pty.slave);

        let mut reader = pty.master.try_clone_reader().expect("clone reader");
        let mut writer = pty.master.take_writer().expect("take writer");

        // Keep a handle to the master for resizing
        let master = std::sync::Arc::new(std::sync::Mutex::new(pty.master));
        let master_resize = master.clone();

        // reader thread
        let tx = tx_out.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 || tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });

        // writer thread
        thread::spawn(move || {
            while let Ok(data) = rx_in.recv() {
                if writer.write_all(&data).is_err() {
                    break;
                }
                let _ = writer.flush();
            }
        });

        // resize handler thread
        thread::spawn(move || {
            while let Ok((cols, rows)) = rx_resize.recv() {
                if let Ok(master) = master_resize.lock() {
                    let _ = master.resize(PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
            }
        });
    });
    
    tx_resize
}

// ---------- UI -------------------------------------------------------------

static KEY_MAP: phf::Map<&'static str, &'static [u8]> = phf_map! {
    "BackSpace" => &[0x7f],
    "Return"    => b"\r",
    "Tab"       => b"\t",
    "Up"        => b"\x1b[A",
    "Down"      => b"\x1b[B",
    "Left"      => b"\x1b[D",
    "Right"     => b"\x1b[C",
    "Home"      => b"\x1b[H",
    "End"       => b"\x1b[F",
};

fn build_ui(app: &Application) {
    let win = ApplicationWindow::new(app);
    win.set_title(Some("HugoTerm"));
    win.set_default_size(800, 600);

    let term = Terminal::new();

    println!("BLUR: {}", CONFIG.appearance.blur);
    println!(" font_family: {}",  CONFIG.appearance.font_family);

    // font
    term.set_font(Some(&FontDescription::from_string(&format!(
        "{} {}",
        CONFIG.appearance.font_family, CONFIG.appearance.font_size
    ))));

    // colours (use cached palette and set proper background)
    let fg = hex_to_rgba(&CONFIG.appearance.foreground_color).unwrap_or(gdk::RGBA::WHITE);
    let bg = hex_to_rgba(&CONFIG.appearance.background_color).unwrap_or(gdk::RGBA::BLACK);
    
    // Set the background color on the terminal directly
    term.set_colors(
        Some(&fg),
        Some(&bg),
        &CONFIG.colors.cached.iter().collect::<Vec<_>>(),
    );
    term.set_scrollback_lines(CONFIG.terminal.scrollback_lines);

    // cursor
    term.set_cursor_blink_mode(if CONFIG.cursor.blink {
        vte4::CursorBlinkMode::On
    } else {
        vte4::CursorBlinkMode::Off
    });
    term.set_cursor_shape(match CONFIG.cursor.shape.to_lowercase().as_str() {
        "ibeam" | "line" => vte4::CursorShape::Ibeam,
        "underline" => vte4::CursorShape::Underline,
        _ => vte4::CursorShape::Block,
    });

    // Make sure the terminal expands to fill the entire window
    term.set_vexpand(true);
    term.set_hexpand(true);
    term.set_valign(gtk4::Align::Fill);
    term.set_halign(gtk4::Align::Fill);
    term.set_scroll_on_output(false);
    term.set_scroll_on_keystroke(true);
    
    win.set_child(Some(&term));

    // pty channels
    let (tx_out, rx_out) = mpsc::channel();
    let (tx_in, rx_in) = mpsc::channel();
    let tx_resize = start_pty(tx_out, rx_in);

    // io
    let term2 = term.clone();
    glib::idle_add_local(
        move || match rx_out.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(data) => {
                term2.feed(&data);
                glib::ControlFlow::Continue
            }
            Err(mpsc::RecvTimeoutError::Timeout) => glib::ControlFlow::Continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => glib::ControlFlow::Break,
        },
    );

    let tx_in_commit = tx_in.clone();
    term.connect_commit(move |_, txt, _| {
        let _ = tx_in_commit.send(txt.as_bytes().to_vec());
    });

    // title
    let win2 = win.clone();
    term.connect_window_title_changed(move |t| {
        win2.set_title(Some(&format!(
            "HugoTerm - {}",
            t.window_title().unwrap_or_default()
        )));
    });

    // keyboard
    let tx2 = tx_in.clone();
    let term_kb = term.clone();  // Clone for keyboard handler
    let keyctl = EventControllerKey::new();

    const KEY_C: u32 = 0x63;
    const KEY_V: u32 = 0x76;

    keyctl.connect_key_pressed(move |_, key, _, state| {
        use gdk::ModifierType;

        let keyval = gdk::Key::to_unicode(&key).unwrap_or('\0') as u32;

        let ctrl   = state.contains(ModifierType::CONTROL_MASK);
        let shift  = state.contains(ModifierType::SHIFT_MASK);
        let super_ = state.contains(ModifierType::SUPER_MASK);
        let meta   = state.contains(ModifierType::META_MASK);  // ⌘ on macOS with GTK4
        let cmd    = super_ || meta;  // Command key can be either

        eprintln!(
            "KEY  keyval=0x{:x}  name={:?}  ctrl={}  shift={}  super={}  meta={}",
            keyval, key.name(), ctrl, shift, super_, meta
        );

        /* 0.  NEVER intercept platform-native copy/paste
            Linux: Ctrl-Shift-C/V
            macOS: ⌘-C/⌘-V (META_MASK with GTK4)
            Windows: Ctrl-C / Ctrl-V  (no Shift!)  */
        let copy = (ctrl && shift && keyval == KEY_C)   // Linux
                || (cmd && keyval == KEY_C)             // macOS
                || (ctrl && !shift && keyval == KEY_C); // Windows
        let paste = (ctrl && shift && keyval == KEY_V)  // Linux
                || (cmd && keyval == KEY_V)            // macOS
                || (ctrl && !shift && keyval == KEY_V); // Windows

        if copy {
            eprintln!("COPY detected  keyval={:#x}  ctrl={} shift={} cmd={}", keyval, ctrl, shift, cmd);
            term_kb.emit_by_name::<()>("copy-clipboard", &[]);
            return glib::Propagation::Stop;
        }
        
        if paste {
            eprintln!("PASTE detected  keyval={:#x}  ctrl={} shift={} cmd={}", keyval, ctrl, shift, cmd);
            term_kb.emit_by_name::<()>("paste-clipboard", &[]);
            return glib::Propagation::Stop;
        }

        /* 1.  Terminal control sequences (Ctrl+letter, no Shift, no Command)  */
        if ctrl && !shift && !cmd {
            if let Some(name) = key.name() {
                if let Some(seq) = KEY_MAP.get(name.as_str()) {
                    let _ = tx2.send(seq.to_vec());
                    return glib::Propagation::Stop;
                }
            }
        }

        /* 2.  Plain editing keys (arrows, BackSpace, …)  */
        if !ctrl && !cmd {
            if let Some(name) = key.name() {
                if let Some(seq) = KEY_MAP.get(name.as_str()) {
                    let _ = tx2.send(seq.to_vec());
                    return glib::Propagation::Stop;
                }
            }
        }

        glib::Propagation::Proceed
    });

    term.add_controller(keyctl);

    // resize handling - watch for size changes
    let term_resize = term.clone();
    let last_size = std::rc::Rc::new(std::cell::RefCell::new((0u16, 0u16)));
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        let cols = term_resize.column_count() as u16;
        let rows = term_resize.row_count() as u16;
        let mut last = last_size.borrow_mut();
        
        // Only send if size actually changed
        if *last != (cols, rows) {
            eprintln!("Terminal resized to {}x{}", cols, rows);
            *last = (cols, rows);
            let _ = tx_resize.send((cols, rows));
        }
        glib::ControlFlow::Continue
    });

    // macOS transparency - no CSS needed
    // The opacity is already set above with win.set_opacity()

    win.present();

    // Apply native macOS transparency/blur without affecting GTK layout
    #[cfg(target_os = "macos")]
    {
        let win_clone = win.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
            if CONFIG.appearance.blur {
                println!("IS blurred now !!!!");
                macos_blur::apply_native_blur(&win_clone, CONFIG.appearance.opacity, CONFIG.appearance.blur_strength);
            } else {
                println!("Not blurred now !!!!");
                macos_blur::apply_native_transparency(&win_clone, CONFIG.appearance.opacity);
            }
            glib::ControlFlow::Break
        });
    }
}

fn main() {
    let app = Application::builder()
        .application_id("com.aztekoders.hugoterm")
        .build();
    app.connect_activate(build_ui);
    app.run();
}