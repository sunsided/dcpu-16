//! Browser entry point: Ratzilla renders the unchanged interface into a WebGL2
//! canvas, keys come from listeners on the document, and the demo programs are
//! built in, since a page cannot open files from a path.
//!
//! The page's query parameters choose what runs: `?program=hello` starts with a
//! demo, and `?source=…` runs percent-encoded assembly source, named by `&name=…`.

use crate::app::{App, Key, KeyCode};
use crate::programs;
use crate::{blink_on, ui};
use ratatui::Terminal;
use ratatui::layout::Constraint;
use ratzilla::backend::webgl2::WebGl2BackendOptions;
use ratzilla::{FontAtlasConfig, WebGl2Backend, WebRenderer};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_time::Instant;

/// Largest interface size in cells; bigger windows get it centered with a margin.
const MAX_COLS: u16 = 110;
/// Largest interface height in cells.
const MAX_ROWS: u16 = 32;

/// Fonts for the glyph atlas, in order; the browser falls back per glyph.
const FONTS: [&str; 5] = [
    "JetBrains Mono",
    "Fira Code",
    "DejaVu Sans Mono",
    "Consolas",
    "monospace",
];
/// Font size of the glyph atlas in pixels.
const FONT_SIZE: f32 = 15.0;

/// Starts the interface with the demo programs and runs it once per animation frame.
pub fn main() {
    console_error_panic_hook::set_once();

    let parameter = query_parameters();
    let selection = programs::select(
        parameter("program").as_deref(),
        parameter("source").as_deref(),
        parameter("name").as_deref(),
    );
    let mut app = App::with_programs(selection.programs);
    app.notice = selection.notice;
    app.can_quit = false;
    let app = Rc::new(RefCell::new(app));

    listen_for_keys(Rc::clone(&app));

    let terminal = terminal();
    let start = Instant::now();
    let mut last_frame = start;
    terminal.draw_web(move |frame| {
        let now = Instant::now();
        let mut app = app.borrow_mut();
        app.advance(now - last_frame);
        last_frame = now;

        let area = frame.area().centered(
            Constraint::Length(MAX_COLS.min(frame.area().width)),
            Constraint::Length(MAX_ROWS.min(frame.area().height)),
        );
        ui::draw(frame, area, &app, blink_on(now - start));
    });
}

/// Reads the page's query parameters, percent-decoded.
///
/// Note that `+` decodes to a space, so source containing `+` must encode it as
/// `%2B`, as `encodeURIComponent` does.
fn query_parameters() -> impl Fn(&str) -> Option<String> {
    let search = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .unwrap_or_default();
    let parameters = web_sys::UrlSearchParams::new_with_str(&search).ok();
    move |name| parameters.as_ref()?.get(name)
}

/// Creates the terminal drawing into a WebGL2 canvas.
///
/// # Panics
///
/// Panics if the page has no WebGL2 support.
fn terminal() -> Terminal<WebGl2Backend> {
    // A dynamic atlas rasterizes glyphs with the page's fonts; the built-in static
    // one lacks glyphs such as the block shades in the header.
    let options = WebGl2BackendOptions::new()
        .disable_auto_css_resize()
        .font_atlas_config(FontAtlasConfig::dynamic(&FONTS, FONT_SIZE));
    let backend =
        WebGl2Backend::new_with_options(options).expect("failed to create WebGL2 backend");
    Terminal::new(backend).expect("failed to create terminal")
}

/// Sends key presses on the document to the app, so the canvas never needs focus.
///
/// Keys the interface uses, including F5, which would otherwise reload the page,
/// stay away from the browser. `Ctrl` and `Cmd` shortcuts, F11 and F12, and keys the
/// interface does not know keep working as usual.
///
/// # Panics
///
/// Panics if the page has no document.
fn listen_for_keys(app: Rc<RefCell<App>>) {
    let listener =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
            if event.ctrl_key() || event.meta_key() || event.alt_key() {
                return;
            }
            let Some(code) = key_code(&event.key()) else {
                return;
            };
            event.prevent_default();
            app.borrow_mut().handle_key(Key::new(code));
        });
    web_sys::window()
        .and_then(|window| window.document())
        .expect("no document")
        .add_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref())
        .expect("failed to listen for keys");
    // The listener lives as long as the page.
    listener.forget();
}

/// Translates the `key` of a browser keyboard event.
fn key_code(key: &str) -> Option<KeyCode> {
    let mut chars = key.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Some(KeyCode::Char(c));
    }
    Some(match key {
        "Enter" => KeyCode::Enter,
        "Backspace" => KeyCode::Backspace,
        "Tab" => KeyCode::Tab,
        "Escape" => KeyCode::Esc,
        "Delete" => KeyCode::Delete,
        "ArrowUp" => KeyCode::Up,
        "ArrowDown" => KeyCode::Down,
        "ArrowLeft" => KeyCode::Left,
        "ArrowRight" => KeyCode::Right,
        _ => match key.strip_prefix('F')?.parse().ok()? {
            // Fullscreen and the developer tools stay with the browser.
            11 | 12 => return None,
            n => KeyCode::F(n),
        },
    })
}
