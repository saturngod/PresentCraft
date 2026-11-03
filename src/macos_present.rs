//! Instant full screen for the presentation window on macOS.
//!
//! The native full screen (`toggleFullScreen:`) opens the window at its
//! normal size first and then animates into a new Space, which looks like
//! a jump. Instead the borderless presentation window is resized to cover
//! the editor's screen and the menu bar and Dock are hidden, so the slide
//! appears in one step.

use std::ffi::{c_char, c_void, CStr};

type Id = *mut c_void;
type Sel = *const c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct NsRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_msgSend();
    #[cfg(target_arch = "x86_64")]
    fn objc_msgSend_stret();
}

/// NSApplicationPresentationHideDock | NSApplicationPresentationHideMenuBar
const HIDE_DOCK_AND_MENU: usize = (1 << 1) | (1 << 3);

fn sel(name: &CStr) -> Sel {
    unsafe { sel_registerName(name.as_ptr()) }
}

unsafe fn send0(obj: Id, s: &CStr) -> Id {
    let f: unsafe extern "C" fn(Id, Sel) -> Id = std::mem::transmute(objc_msgSend as *const c_void);
    f(obj, sel(s))
}

unsafe fn send_usize(obj: Id, s: &CStr) -> usize {
    let f: unsafe extern "C" fn(Id, Sel) -> usize =
        std::mem::transmute(objc_msgSend as *const c_void);
    f(obj, sel(s))
}

unsafe fn send_at(obj: Id, s: &CStr, i: usize) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, usize) -> Id =
        std::mem::transmute(objc_msgSend as *const c_void);
    f(obj, sel(s), i)
}

unsafe fn frame(obj: Id) -> NsRect {
    #[cfg(target_arch = "x86_64")]
    {
        let f: unsafe extern "C" fn(*mut NsRect, Id, Sel) =
            std::mem::transmute(objc_msgSend_stret as *const c_void);
        let mut r = NsRect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };
        f(&mut r, obj, sel(c"frame"));
        r
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let f: unsafe extern "C" fn(Id, Sel) -> NsRect =
            std::mem::transmute(objc_msgSend as *const c_void);
        f(obj, sel(c"frame"))
    }
}

unsafe fn app() -> Id {
    send0(objc_getClass(c"NSApplication".as_ptr()), c"sharedApplication")
}

unsafe fn set_options(opts: usize) {
    let f: unsafe extern "C" fn(Id, Sel, usize) =
        std::mem::transmute(objc_msgSend as *const c_void);
    f(app(), sel(c"setPresentationOptions:"), opts);
}

/// The window whose title is `title`, if it exists.
unsafe fn window_titled(title: &str) -> Id {
    let windows = send0(app(), c"windows");
    if windows.is_null() {
        return std::ptr::null_mut();
    }
    for i in 0..send_usize(windows, c"count") {
        let w = send_at(windows, c"objectAtIndex:", i);
        let t = send0(w, c"title");
        if t.is_null() {
            continue;
        }
        let utf8: unsafe extern "C" fn(Id, Sel) -> *const c_char =
            std::mem::transmute(objc_msgSend as *const c_void);
        let p = utf8(t, sel(c"UTF8String"));
        if !p.is_null() && CStr::from_ptr(p).to_bytes() == title.as_bytes() {
            return w;
        }
    }
    std::ptr::null_mut()
}

/// Covers the editor's screen with the window titled `title` and hides
/// the menu bar and Dock. Returns false if the window isn't there yet.
pub fn enter(title: &str) -> bool {
    unsafe {
        let win = window_titled(title);
        if win.is_null() {
            return false;
        }
        // The editor is still the main window, since the presentation
        // window is hidden until now.
        let main = send0(app(), c"mainWindow");
        let mut screen = if main.is_null() {
            std::ptr::null_mut()
        } else {
            send0(main, c"screen")
        };
        if screen.is_null() {
            screen = send0(objc_getClass(c"NSScreen".as_ptr()), c"mainScreen");
        }
        if screen.is_null() {
            return false;
        }
        let r = frame(screen);
        set_options(HIDE_DOCK_AND_MENU);
        let set_frame: unsafe extern "C" fn(Id, Sel, NsRect, bool) =
            std::mem::transmute(objc_msgSend as *const c_void);
        set_frame(win, sel(c"setFrame:display:"), r, true);
        true
    }
}

/// Brings the menu bar and Dock back.
pub fn exit() {
    unsafe { set_options(0) }
}
