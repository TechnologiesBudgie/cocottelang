#![cfg(feature = "gui")]
// Uncomment eframe/egui in Cargo.toml + [features] section to enable GUI

//! charlotte.rs — Native GUI module for Cocotte using egui + eframe
//!
//! Compiled when the `gui` feature is enabled (default).
//! Supports: Linux (Wayland + X11 auto), Windows, macOS.
//! On unsupported platforms or when built without the gui feature,
//! charlotte.window() prints a message and returns immediately.
//!
//! Usage in Cocotte:
//!
//!   module add "charlotte"
//!
//!   var count = 0
//!   var items = ["apple", "banana", "cherry"]
//!
//!   charlotte.window("My App", 800, 600, func(ui)
//!       ui.heading("Hello!")
//!       ui.label("Count: " + count)
//!       if ui.button("Add")
//!           count = count + 1
//!       end
//!       ui.separator()
//!       for item in items
//!           ui.label("- " + item)
//!       end
//!   end)
//!
//! IMPORTANT: Use Value::Map/List for persistent state mutated inside the
//! draw callback — they are reference types (Arc<Mutex<...>>).
//! Plain vars inside the callback are re-evaluated each frame.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::value::{Value, NativeFunction, CocotteFunction};
use crate::error::{CocotteError, Result};

// ── Thread-locals (always compiled) ──────────────────────────────────────────
// Pointers valid only during eframe's update() — eframe is single-threaded.

use std::cell::RefCell;

thread_local! {
    static EGUI_UI:   RefCell<usize> = RefCell::new(0);
    static GUI_STATE: RefCell<usize> = RefCell::new(0);
}

// Interpreter pointer is managed by crate::runtime_ctx.

// ── Persistent state across frames ───────────────────────────────────────────

#[derive(Default)]
pub struct GuiState {
    pub text_fields: HashMap<String, String>,
    pub checkboxes:  HashMap<String, bool>,
    pub sliders:     HashMap<String, f64>,
    pub radio_groups: HashMap<String, String>,
}

// ── Stub implementation (no gui feature) ─────────────────────────────────────

#[cfg(not(feature = "gui"))]
pub fn run_window(
    _title: &str, _width: f32, _height: f32,
    _draw_fn: CocotteFunction,
    _interp: &mut crate::interpreter::Interpreter,
) -> Result<()> {
    eprintln!("charlotte: built without GUI support. Rebuild with: cargo build --features gui");
    Ok(())
}

// ── Real implementation (gui feature enabled) ─────────────────────────────────

#[cfg(feature = "gui")]
pub fn run_window(
    title: &str,
    width: f32,
    height: f32,
    draw_fn: CocotteFunction,
    interp: &mut crate::interpreter::Interpreter,
) -> Result<()> {
    use eframe::egui;

    struct CharlotteApp {
        draw_fn: CocotteFunction,
        state:   GuiState,
        interp:  crate::interpreter::Interpreter,
    }

    impl eframe::App for CharlotteApp {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                let ui_ptr     = ui as *mut egui::Ui as usize;
                let state_ptr  = &mut self.state as *mut GuiState as usize;
                let interp_ptr = &mut self.interp as *mut crate::interpreter::Interpreter as usize;

                EGUI_UI.with(      |p| *p.borrow_mut() = ui_ptr);
                GUI_STATE.with(    |p| *p.borrow_mut() = state_ptr);
                crate::runtime_ctx::set_active_interpreter(interp_ptr);

                let ui_obj = make_ui_object();
                if let Err(e) = call_draw_fn(&self.draw_fn, ui_obj) {
                    if !e.is_signal() {
                        eprintln!("[charlotte] draw error: {}", e);
                        ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
                    }
                }
                ctx.request_repaint();

                EGUI_UI.with(      |p| *p.borrow_mut() = 0);
                GUI_STATE.with(    |p| *p.borrow_mut() = 0);
                crate::runtime_ctx::set_active_interpreter(0);
            });
        }
    }

    let mut app_interp = crate::interpreter::Interpreter::new();
    app_interp.copy_globals_from(interp);

    let app = CharlotteApp {
        draw_fn: draw_fn.clone(),
        state:   GuiState::default(),
        interp:  app_interp,
    };

    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([width, height]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    // First attempt: WGPU (preferred for reliability on Linux)
    match eframe::run_native(title, options.clone(), Box::new(|_cc| Ok(Box::new(app)))) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("[charlotte] WGPU initialization failed: {}. Falling back to OpenGL (Glow)...", e);
            
            // Re-create app for second attempt (previous one was moved)
            let mut app_interp_fallback = crate::interpreter::Interpreter::new();
            app_interp_fallback.copy_globals_from(interp);
            let app_fallback = CharlotteApp {
                draw_fn,
                state:   GuiState::default(),
                interp:  app_interp_fallback,
            };

            options.renderer = eframe::Renderer::Glow;
            eframe::run_native(title, options, Box::new(|_cc| Ok(Box::new(app_fallback))))
                .map_err(|e| CocotteError::runtime(&format!("charlotte fallback error: {}", e)))
        }
    }
}

// ── Thread-local helpers (gui only) ──────────────────────────────────────────

#[cfg(feature = "gui")]
fn with_ui<R, F: FnOnce(&mut eframe::egui::Ui) -> R>(f: F) -> Option<R> {
    // Copy the pointer out before calling f so the RefCell borrow is
    // released first.  If the borrow were held across f(), any nested
    // set_ui_ptr() call (e.g. inside ui.row / ui.column layout closures)
    // would attempt a second borrow_mut on the same RefCell and panic,
    // causing buttons and other interactive widgets to silently return None.
    let ptr = EGUI_UI.with(|p| *p.borrow());
    if ptr == 0 { return None; }
    Some(f(unsafe { &mut *(ptr as *mut eframe::egui::Ui) }))
}

#[cfg(feature = "gui")]
fn get_ui_ptr() -> usize {
    EGUI_UI.with(|p| *p.borrow())
}

#[cfg(feature = "gui")]
fn set_ui_ptr(ptr: usize) {
    EGUI_UI.with(|p| *p.borrow_mut() = ptr);
}

#[cfg(feature = "gui")]
fn call_draw_fn(func: &CocotteFunction, ui_val: Value) -> Result<Value> {
    let ptr = crate::runtime_ctx::get_active_interpreter();
    if ptr == 0 {
        return Err(CocotteError::runtime("charlotte: no active interpreter"));
    }
    let interp = unsafe { &mut *(ptr as *mut crate::interpreter::Interpreter) };
    interp.call_function_pub(func, vec![ui_val], None)
}

#[cfg(feature = "gui")]
fn with_state<R, F: FnOnce(&mut GuiState) -> R>(f: F) -> Option<R> {
    GUI_STATE.with(|p| {
        let ptr = *p.borrow();
        if ptr == 0 { return None; }
        Some(f(unsafe { &mut *(ptr as *mut GuiState) }))
    })
}

// ── Color parsing (gui only) ──────────────────────────────────────────────────

#[cfg(feature = "gui")]
fn parse_color(s: &str) -> eframe::egui::Color32 {
    use eframe::egui::Color32;
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        match hex.len() {
            6 => if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) { return Color32::from_rgb(r, g, b); },
            3 => if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..1].repeat(2), 16),
                u8::from_str_radix(&hex[1..2].repeat(2), 16),
                u8::from_str_radix(&hex[2..3].repeat(2), 16),
            ) { return Color32::from_rgb(r, g, b); },
            _ => {}
        }
    }
    match s.to_lowercase().as_str() {
        "red"           => Color32::RED,
        "green"         => Color32::GREEN,
        "blue"          => Color32::BLUE,
        "yellow"        => Color32::YELLOW,
        "white"         => Color32::WHITE,
        "black"         => Color32::BLACK,
        "gray" | "grey" => Color32::GRAY,
        "orange"        => Color32::from_rgb(255, 165, 0),
        "purple"        => Color32::from_rgb(128, 0, 128),
        "cyan"          => Color32::from_rgb(0, 255, 255),
        "pink"          => Color32::from_rgb(255, 192, 203),
        "brown"         => Color32::from_rgb(139, 69, 19),
        "lime"          => Color32::from_rgb(50, 205, 50),
        _               => Color32::WHITE,
    }
}

// ── UI object (gui only) ──────────────────────────────────────────────────────

/// Build the `ui` object passed to every draw callback.
/// Each entry is a NativeFunction that calls into egui via the thread-local.
#[cfg(feature = "gui")]
fn make_ui_object() -> Value {
    use eframe::egui;

    let mut m: HashMap<String, Value> = HashMap::new();

    // ── Text display ─────────────────────────────────────────────────────

    m.insert("label".into(), nfn("ui.label", None, |args| {
        let text = args.first().map(|v| v.to_display()).unwrap_or_default();
        with_ui(|ui| { ui.label(text); });
        Ok(Value::Nil)
    }));

    m.insert("heading".into(), nfn("ui.heading", None, |args| {
        let text = args.first().map(|v| v.to_display()).unwrap_or_default();
        with_ui(|ui| { ui.heading(text); });
        Ok(Value::Nil)
    }));

    m.insert("subheading".into(), nfn("ui.subheading", None, |args| {
        let text = args.first().map(|v| v.to_display()).unwrap_or_default();
        with_ui(|ui| { ui.strong(text); });
        Ok(Value::Nil)
    }));

    m.insert("monospace".into(), nfn("ui.monospace", None, |args| {
        let text = args.first().map(|v| v.to_display()).unwrap_or_default();
        with_ui(|ui| { ui.monospace(text); });
        Ok(Value::Nil)
    }));

    m.insert("colored_label".into(), nfn("ui.colored_label", None, |args| {
        let color = args.first().map(|v| v.to_display()).unwrap_or_default();
        let text  = args.get(1).map(|v| v.to_display()).unwrap_or_default();
        let c = parse_color(&color);
        with_ui(|ui| { ui.colored_label(c, text); });
        Ok(Value::Nil)
    }));

    // ── Layout spacers ────────────────────────────────────────────────────

    m.insert("separator".into(), nfn("ui.separator", None, |_| {
        with_ui(|ui| { ui.separator(); });
        Ok(Value::Nil)
    }));

    m.insert("space".into(), nfn("ui.space", None, |_| {
        with_ui(|ui| { ui.add_space(8.0); });
        Ok(Value::Nil)
    }));

    m.insert("add_space".into(), nfn("ui.add_space", Some(1), |args| {
        let n = match args.first() { Some(Value::Number(n)) => *n as f32, _ => 8.0 };
        with_ui(|ui| { ui.add_space(n); });
        Ok(Value::Nil)
    }));

    // ── Layout containers ─────────────────────────────────────────────────

    // ui.row(func() ... end)
    m.insert("row".into(), nfn("ui.row", Some(1), |args| {
        let cb = match args.into_iter().next() {
            Some(Value::Function(f)) => f,
            _ => return Err(CocotteError::type_err("ui.row() requires a function")),
        };
        let saved = get_ui_ptr();
        with_ui(|outer| {
            outer.horizontal(|inner| {
                set_ui_ptr(inner as *mut egui::Ui as usize);
                let ui_obj = make_ui_object();
                let _ = call_draw_fn(&cb, ui_obj);
            });
        });
        set_ui_ptr(saved);
        Ok(Value::Nil)
    }));

    // ui.column(func() ... end)
    m.insert("column".into(), nfn("ui.column", Some(1), |args| {
        let cb = match args.into_iter().next() {
            Some(Value::Function(f)) => f,
            _ => return Err(CocotteError::type_err("ui.column() requires a function")),
        };
        let saved = get_ui_ptr();
        with_ui(|outer| {
            outer.vertical(|inner| {
                set_ui_ptr(inner as *mut egui::Ui as usize);
                let ui_obj = make_ui_object();
                let _ = call_draw_fn(&cb, ui_obj);
            });
        });
        set_ui_ptr(saved);
        Ok(Value::Nil)
    }));

    // ui.group(func() ... end)  — framed box
    m.insert("group".into(), nfn("ui.group", Some(1), |args| {
        let cb = match args.into_iter().next() {
            Some(Value::Function(f)) => f,
            _ => return Err(CocotteError::type_err("ui.group() requires a function")),
        };
        let saved = get_ui_ptr();
        with_ui(|outer| {
            outer.group(|inner| {
                set_ui_ptr(inner as *mut egui::Ui as usize);
                let ui_obj = make_ui_object();
                let _ = call_draw_fn(&cb, ui_obj);
            });
        });
        set_ui_ptr(saved);
        Ok(Value::Nil)
    }));

    // ui.scroll(func() ... end) — scrollable region
    m.insert("scroll".into(), nfn("ui.scroll", Some(1), |args| {
        let cb = match args.into_iter().next() {
            Some(Value::Function(f)) => f,
            _ => return Err(CocotteError::type_err("ui.scroll() requires a function")),
        };
        let saved = get_ui_ptr();
        with_ui(|outer| {
            egui::ScrollArea::vertical().show(outer, |inner| {
                set_ui_ptr(inner as *mut egui::Ui as usize);
                let ui_obj = make_ui_object();
                let _ = call_draw_fn(&cb, ui_obj);
            });
        });
        set_ui_ptr(saved);
        Ok(Value::Nil)
    }));

    // ui.collapsible(label, func() ... end)
    m.insert("collapsible".into(), nfn("ui.collapsible", Some(2), |args| {
        let label = args.first().map(|v| v.to_display()).unwrap_or_default();
        let cb = match args.into_iter().nth(1) {
            Some(Value::Function(f)) => f,
            _ => return Err(CocotteError::type_err("ui.collapsible() requires (label, function)")),
        };
        let saved = get_ui_ptr();
        with_ui(|outer| {
            egui::CollapsingHeader::new(&label).show(outer, |inner| {
                set_ui_ptr(inner as *mut egui::Ui as usize);
                let ui_obj = make_ui_object();
                let _ = call_draw_fn(&cb, ui_obj);
            });
        });
        set_ui_ptr(saved);
        Ok(Value::Nil)
    }));

    // ── Buttons ───────────────────────────────────────────────────────────

    // ui.button(label) -> bool
    m.insert("button".into(), nfn("ui.button", Some(1), |args| {
        let label = args.first().map(|v| v.to_display()).unwrap_or_default();
        let clicked = with_ui(|ui| ui.button(label).clicked()).unwrap_or(false);
        Ok(Value::Bool(clicked))
    }));

    // ui.small_button(label) -> bool
    m.insert("small_button".into(), nfn("ui.small_button", Some(1), |args| {
        let label = args.first().map(|v| v.to_display()).unwrap_or_default();
        let clicked = with_ui(|ui| ui.small_button(label).clicked()).unwrap_or(false);
        Ok(Value::Bool(clicked))
    }));

    // ui.link(label) -> bool
    m.insert("link".into(), nfn("ui.link", Some(1), |args| {
        let label = args.first().map(|v| v.to_display()).unwrap_or_default();
        let clicked = with_ui(|ui| ui.link(label).clicked()).unwrap_or(false);
        Ok(Value::Bool(clicked))
    }));

    // ── Text input ────────────────────────────────────────────────────────

    // ui.input(key, placeholder) -> string
    // key is a unique id so state persists across frames
    m.insert("input".into(), nfn("ui.input", Some(2), |args| {
        let key         = args.first().map(|v| v.to_display()).unwrap_or_else(|| "__in".into());
        let placeholder = args.get(1).map(|v| v.to_display()).unwrap_or_default();
        let result = with_state(|s| {
            let val = s.text_fields.entry(key.clone()).or_insert_with(String::new);
            with_ui(|ui| {
                ui.add(egui::TextEdit::singleline(val).hint_text(&placeholder));
            });
            val.clone()
        }).unwrap_or_default();
        Ok(Value::Str(result))
    }));

    // ui.multiline_input(key, placeholder) -> string
    m.insert("multiline_input".into(), nfn("ui.multiline_input", Some(2), |args| {
        let key         = args.first().map(|v| v.to_display()).unwrap_or_else(|| "__ml".into());
        let placeholder = args.get(1).map(|v| v.to_display()).unwrap_or_default();
        let result = with_state(|s| {
            let val = s.text_fields.entry(key.clone()).or_insert_with(String::new);
            with_ui(|ui| {
                ui.add(egui::TextEdit::multiline(val).hint_text(&placeholder));
            });
            val.clone()
        }).unwrap_or_default();
        Ok(Value::Str(result))
    }));

    // ui.set_input(key, value) — programmatically set a text field
    m.insert("set_input".into(), nfn("ui.set_input", Some(2), |args| {
        let key = args.first().map(|v| v.to_display()).unwrap_or_default();
        let val = args.get(1).map(|v| v.to_display()).unwrap_or_default();
        with_state(|s| { s.text_fields.insert(key.clone(), val.clone()); });
        Ok(Value::Nil)
    }));

    // ── Toggles / selectors ───────────────────────────────────────────────

    // ui.checkbox(key, label [, default]) -> bool
    m.insert("checkbox".into(), nfn("ui.checkbox", None, |args| {
        let key     = args.first().map(|v| v.to_display()).unwrap_or_else(|| "__cb".into());
        let label   = args.get(1).map(|v| v.to_display()).unwrap_or_default();
        let default = match args.get(2) { Some(Value::Bool(b)) => *b, _ => false };
        let result = with_state(|s| {
            let val = s.checkboxes.entry(key.clone()).or_insert(default);
            with_ui(|ui| { ui.checkbox(val, &label); });
            *val
        }).unwrap_or(default);
        Ok(Value::Bool(result))
    }));

    // ui.radio(group_key, option_value, label) -> bool (true = this option is selected)
    m.insert("radio".into(), nfn("ui.radio", Some(3), |args| {
        let key   = args.first().map(|v| v.to_display()).unwrap_or_else(|| "__radio".into());
        let opt   = args.get(1).map(|v| v.to_display()).unwrap_or_default();
        let label = args.get(2).map(|v| v.to_display()).unwrap_or_default();
        let selected = with_state(|s| {
            let current  = s.radio_groups.entry(key.clone()).or_insert_with(String::new);
            let is_sel   = *current == opt;
            let clicked  = with_ui(|ui| ui.radio(is_sel, &label).clicked()).unwrap_or(false);
            if clicked { *current = opt.clone(); }
            s.radio_groups[&key] == opt
        }).unwrap_or(false);
        Ok(Value::Bool(selected))
    }));

    // ui.get_radio(group_key) -> string (current value)
    m.insert("get_radio".into(), nfn("ui.get_radio", Some(1), |args| {
        let key = args.first().map(|v| v.to_display()).unwrap_or_default();
        let val = with_state(|s| s.radio_groups.get(&key).cloned().unwrap_or_default())
            .unwrap_or_default();
        Ok(Value::Str(val))
    }));

    // ui.slider(key, label, min, max [, default]) -> number
    m.insert("slider".into(), nfn("ui.slider", None, |args| {
        let key     = args.first().map(|v| v.to_display()).unwrap_or_else(|| "__sl".into());
        let label   = args.get(1).map(|v| v.to_display()).unwrap_or_default();
        let min     = match args.get(2) { Some(Value::Number(n)) => *n, _ => 0.0 };
        let max     = match args.get(3) { Some(Value::Number(n)) => *n, _ => 100.0 };
        let default = match args.get(4) { Some(Value::Number(n)) => *n, _ => min };
        let result = with_state(|s| {
            let val = s.sliders.entry(key.clone()).or_insert(default);
            with_ui(|ui| {
                ui.add(egui::Slider::new(val, min..=max).text(&label));
            });
            *val
        }).unwrap_or(default);
        Ok(Value::Number(result))
    }));

    // ui.progress(value_0_to_1)
    m.insert("progress".into(), nfn("ui.progress", Some(1), |args| {
        let val = match args.first() { Some(Value::Number(n)) => *n as f32, _ => 0.0 };
        with_ui(|ui| { ui.add(egui::ProgressBar::new(val.clamp(0.0, 1.0))); });
        Ok(Value::Nil)
    }));

    // ── Window metrics ────────────────────────────────────────────────────

    m.insert("available_width".into(), nfn("ui.available_width", Some(0), |_| {
        let w = with_ui(|ui| ui.available_width()).unwrap_or(0.0);
        Ok(Value::Number(w as f64))
    }));

    m.insert("available_height".into(), nfn("ui.available_height", Some(0), |_| {
        let h = with_ui(|ui| ui.available_height()).unwrap_or(0.0);
        Ok(Value::Number(h as f64))
    }));

    Value::Module(Arc::new(Mutex::new(m)))
}

// ── Helpers (always compiled) ─────────────────────────────────────────────────

fn nfn(
    name: &'static str,
    arity: Option<usize>,
    f: impl Fn(Vec<Value>) -> Result<Value> + Send + Sync + 'static,
) -> Value {
    Value::NativeFunction(NativeFunction {
        name: name.to_string(),
        arity,
        func: Arc::new(f),
    })
}

// ── Public module factory (always compiled) ───────────────────────────────────

/// Build the `charlotte` module value for Cocotte scripts.
/// Called by modules.rs when `module add "charlotte"` is executed.
pub fn make_charlotte_module() -> Value {
    let mut m: HashMap<String, Value> = HashMap::new();

    // charlotte.window(title, width, height, func(ui) ... end)
    m.insert("window".into(), nfn("charlotte.window", Some(4), |args| {
        let title  = args.first().map(|v| v.to_display()).unwrap_or_else(|| "App".into());
        let width  = match args.get(1) { Some(Value::Number(n)) => *n as f32, _ => 800.0 };
        let height = match args.get(2) { Some(Value::Number(n)) => *n as f32, _ => 600.0 };
        let draw_fn = match args.into_iter().nth(3) {
            Some(Value::Function(f)) => f,
            _ => return Err(CocotteError::type_err(
                "charlotte.window() requires (title, width, height, func(ui) ... end)"
            )),
        };
        let ptr = crate::runtime_ctx::get_active_interpreter();
        if ptr == 0 {
            return Err(CocotteError::runtime(
                "charlotte.window() called outside of an active interpreter"
            ));
        }
        let interp = unsafe { &mut *(ptr as *mut crate::interpreter::Interpreter) };
        run_window(&title, width, height, draw_fn, interp).map(|_| Value::Nil)
    }));

    m.insert("version".into(), nfn("charlotte.version", Some(0), |_| {
        let v = if cfg!(feature = "gui") { "charlotte/egui 0.29" } else { "charlotte/stub" };
        Ok(Value::Str(v.into()))
    }));

    m.insert("has_gui".into(), nfn("charlotte.has_gui", Some(0), |_| {
        Ok(Value::Bool(cfg!(feature = "gui")))
    }));

    Value::Module(Arc::new(Mutex::new(m)))
}

// ── Fallback make_ui_object for non-gui builds ────────────────────────────────
// Never actually called (only used inside run_window which is gated),
// but Rust needs all paths to compile.
#[cfg(not(feature = "gui"))]
fn make_ui_object() -> Value {
    Value::Nil
}

#[cfg(not(feature = "gui"))]
fn call_draw_fn(_func: &CocotteFunction, _ui: Value) -> Result<Value> {
    Ok(Value::Nil)
}

#[cfg(not(feature = "gui"))]
fn with_state<R, F: FnOnce(&mut GuiState) -> R>(_f: F) -> Option<R> {
    None
}

#[cfg(not(feature = "gui"))]
fn with_ui<R, F: FnOnce(&mut ()) -> R>(_f: F) -> Option<R> {
    None
}

#[cfg(not(feature = "gui"))]
fn get_ui_ptr() -> usize { 0 }

#[cfg(not(feature = "gui"))]
fn set_ui_ptr(_ptr: usize) {}
