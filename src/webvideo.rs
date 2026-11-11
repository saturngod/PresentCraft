//! YouTube videos while presenting.
//!
//! They play in YouTube's own embedded player, shown in the system's web
//! view (WebKit on macOS, WebView2 on Windows) placed over the video's spot
//! on the slide. Nothing is downloaded or decoded by the app. On other
//! systems `WebVideo::open` returns `None` and the caller opens the video
//! in the browser.

use std::sync::{Arc, Mutex};

/// What the app tells YouTube about itself (its bundle identifier as an
/// address, which YouTube asks embedding apps to send).
#[cfg(any(target_os = "macos", target_os = "windows"))]
const REFERER: &str = "https://app.myanmars.presentcraft";

/// Something pressed while the player had the keyboard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Next,
    Prev,
    /// Close the player, back to the still picture (V).
    Close,
    /// Stop presenting (Escape).
    Stop,
}

fn parse_event(msg: &str) -> Option<Event> {
    match msg {
        "next" => Some(Event::Next),
        "prev" => Some(Event::Prev),
        "close" => Some(Event::Close),
        "stop" => Some(Event::Stop),
        _ => None,
    }
}

/// The address of YouTube's embedded player for a video. It starts playing
/// at once: the player is only opened when the video should play.
pub fn embed_url(id: &str, start: u32, controls: bool) -> String {
    format!(
        "https://www.youtube.com/embed/{id}?autoplay=1&controls={}&start={start}&playsinline=1&rel=0&fs=0&iv_load_policy=3",
        controls as u8
    )
}

/// Runs in the player's page: keys that move through the slides go to the
/// app instead of the player (which would seek or do nothing with them).
#[cfg(any(target_os = "macos", target_os = "windows"))]
const KEYS_SCRIPT: &str = r#"document.addEventListener('keydown', function (e) {
  var k = e.key, m = null;
  if (['ArrowRight', 'ArrowDown', 'PageDown', 'Enter'].indexOf(k) >= 0) m = 'next';
  else if (['ArrowLeft', 'ArrowUp', 'PageUp', 'Backspace'].indexOf(k) >= 0) m = 'prev';
  else if (k === 'Escape') m = 'stop';
  else if (k === 'v' || k === 'V') m = 'close';
  if (m) { e.preventDefault(); e.stopImmediatePropagation(); window.ipc.postMessage(m); }
}, true);"#;

/// A YouTube player shown over a slide.
pub struct WebVideo {
    pub src: String,
    rect: egui::Rect,
    events: Arc<Mutex<Vec<Event>>>,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    view: wry::WebView,
}

impl WebVideo {
    /// Opens the player for the YouTube video `src` over `rect` (in points)
    /// of the window titled `window_title` (else the active window). `None`
    /// when `src` isn't a YouTube video or no web view can be shown here.
    pub fn open(
        ctx: &egui::Context,
        window_title: &str,
        src: &str,
        rect: egui::Rect,
        controls: bool,
    ) -> Option<WebVideo> {
        let id = crate::images::youtube_id(src)?;
        let url = embed_url(&id, crate::images::youtube_start(src) as u32, controls);
        let events: Arc<Mutex<Vec<Event>>> = Arc::default();
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            use wry::http::{header::REFERER as REFERER_HEADER, HeaderMap, HeaderValue};
            let parent = native::Parent::find(window_title)?;
            let mut headers = HeaderMap::new();
            headers.insert(REFERER_HEADER, HeaderValue::from_static(REFERER));
            let (sink, ctx, viewport) = (events.clone(), ctx.clone(), ctx.viewport_id());
            let view = wry::WebViewBuilder::new()
                .with_url_and_headers(url, headers)
                .with_bounds(bounds(rect))
                .with_autoplay(true)
                .with_focused(false)
                .with_background_color((0, 0, 0, 255))
                .with_initialization_script(KEYS_SCRIPT)
                .with_ipc_handler(move |req| {
                    if let Some(e) = parse_event(req.body()) {
                        if let Ok(mut list) = sink.lock() {
                            list.push(e);
                        }
                        ctx.request_repaint_of(viewport);
                    }
                })
                .build_as_child(&parent)
                .ok()?;
            Some(WebVideo {
                src: src.to_string(),
                rect,
                events,
                view,
            })
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = (ctx, window_title, rect, url, events);
            None
        }
    }

    /// Keeps the player over its video when the slide moves or resizes.
    pub fn set_rect(&mut self, rect: egui::Rect) {
        if rect == self.rect {
            return;
        }
        self.rect = rect;
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let _ = self.view.set_bounds(bounds(rect));
        }
    }

    /// Keys pressed in the player since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        self.events
            .lock()
            .map(|mut e| std::mem::take(&mut *e))
            .unwrap_or_default()
    }
}

impl Drop for WebVideo {
    fn drop(&mut self) {
        // Give the keyboard back to the slides before the player goes.
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let _ = self.view.focus_parent();
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn bounds(r: egui::Rect) -> wry::Rect {
    wry::Rect {
        position: wry::dpi::LogicalPosition::new(r.left() as f64, r.top() as f64).into(),
        size: wry::dpi::LogicalSize::new(r.width().max(1.0) as f64, r.height().max(1.0) as f64)
            .into(),
    }
}

/// Finding the window the player goes into. egui doesn't hand out the
/// presentation window, so the system is asked for it by title.
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod native {
    use wry::raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle};

    pub struct Parent(RawWindowHandle);

    impl HasWindowHandle for Parent {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            // The window outlives the player: it is dropped when presenting
            // stops, before the window closes.
            Ok(unsafe { WindowHandle::borrow_raw(self.0) })
        }
    }

    #[cfg(target_os = "macos")]
    impl Parent {
        pub fn find(title: &str) -> Option<Parent> {
            use std::ffi::{c_char, c_void, CStr};
            use wry::raw_window_handle::AppKitWindowHandle;
            type Id = *mut c_void;
            #[link(name = "objc")]
            extern "C" {
                fn objc_getClass(name: *const c_char) -> Id;
                fn sel_registerName(name: *const c_char) -> *const c_void;
                fn objc_msgSend();
            }
            unsafe fn send(obj: Id, sel: &CStr) -> Id {
                let f: unsafe extern "C" fn(Id, *const c_void) -> Id =
                    std::mem::transmute(objc_msgSend as *const c_void);
                f(obj, sel_registerName(sel.as_ptr()))
            }
            unsafe fn send_index(obj: Id, sel: &CStr, i: usize) -> Id {
                let f: unsafe extern "C" fn(Id, *const c_void, usize) -> Id =
                    std::mem::transmute(objc_msgSend as *const c_void);
                f(obj, sel_registerName(sel.as_ptr()), i)
            }
            unsafe {
                let app = send(objc_getClass(c"NSApplication".as_ptr()), c"sharedApplication");
                if app.is_null() {
                    return None;
                }
                let windows = send(app, c"windows");
                let count = if windows.is_null() {
                    0
                } else {
                    send(windows, c"count") as usize
                };
                let mut window: Id = std::ptr::null_mut();
                for i in 0..count {
                    let w = send_index(windows, c"objectAtIndex:", i);
                    let name = send(w, c"title");
                    if name.is_null() {
                        continue;
                    }
                    let text = send(name, c"UTF8String") as *const c_char;
                    if !text.is_null() && CStr::from_ptr(text).to_bytes() == title.as_bytes() {
                        window = w;
                        break;
                    }
                }
                if window.is_null() {
                    window = send(app, c"keyWindow");
                }
                if window.is_null() {
                    window = send(app, c"mainWindow");
                }
                if window.is_null() {
                    return None;
                }
                let view = std::ptr::NonNull::new(send(window, c"contentView"))?;
                Some(Parent(RawWindowHandle::AppKit(AppKitWindowHandle::new(
                    view,
                ))))
            }
        }
    }

    #[cfg(target_os = "windows")]
    impl Parent {
        pub fn find(title: &str) -> Option<Parent> {
            use std::ffi::c_void;
            use wry::raw_window_handle::Win32WindowHandle;
            #[link(name = "user32")]
            extern "system" {
                fn FindWindowW(class: *const u16, title: *const u16) -> *mut c_void;
                fn GetActiveWindow() -> *mut c_void;
            }
            let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
            let mut hwnd = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
            if hwnd.is_null() {
                hwnd = unsafe { GetActiveWindow() };
            }
            let hwnd = std::num::NonZeroIsize::new(hwnd as isize)?;
            Some(Parent(RawWindowHandle::Win32(Win32WindowHandle::new(hwnd))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_player_address() {
        assert_eq!(
            embed_url("Eu4vnYbKvCI", 65, false),
            "https://www.youtube.com/embed/Eu4vnYbKvCI?autoplay=1&controls=0&start=65&playsinline=1&rel=0&fs=0&iv_load_policy=3"
        );
        assert!(embed_url("Eu4vnYbKvCI", 0, true).contains("controls=1"));
    }

    #[test]
    fn reads_player_keys() {
        assert_eq!(parse_event("next"), Some(Event::Next));
        assert_eq!(parse_event("stop"), Some(Event::Stop));
        assert_eq!(parse_event("anything else"), None);
    }
}
