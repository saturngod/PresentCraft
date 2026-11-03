//! Receives documents opened from Finder (double-click, "Open With", drag
//! onto the Dock icon). macOS delivers these to the application delegate
//! rather than on the command line. winit installs no delegate of its own,
//! so a small one is set just before launch finishes, early enough to get
//! the documents the app was launched with.

use std::ffi::{c_char, c_void, CStr};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

type Id = *mut c_void;
type Sel = *const c_void;
type Class = *mut c_void;

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> Class;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_allocateClassPair(superclass: Class, name: *const c_char, extra: usize) -> Class;
    fn objc_registerClassPair(cls: Class);
    fn class_addMethod(cls: Class, name: Sel, imp: *const c_void, types: *const c_char) -> bool;
    fn objc_msgSend();
}

static PENDING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

/// Documents macOS asked us to open since the last call.
pub fn take_pending() -> Vec<PathBuf> {
    std::mem::take(&mut *PENDING.lock().unwrap())
}

/// Lets later open requests wake the window.
pub fn set_context(ctx: &egui::Context) {
    let _ = CONTEXT.set(ctx.clone());
}

fn sel(name: &CStr) -> Sel {
    unsafe { sel_registerName(name.as_ptr()) }
}

unsafe fn send0(obj: Id, s: &CStr) -> Id {
    let f: unsafe extern "C" fn(Id, Sel) -> Id = std::mem::transmute(objc_msgSend as *const c_void);
    f(obj, sel(s))
}

unsafe fn send1(obj: Id, s: &CStr, a: Id) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, Id) -> Id =
        std::mem::transmute(objc_msgSend as *const c_void);
    f(obj, sel(s), a)
}

unsafe fn ns_string(s: &CStr) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, *const c_char) -> Id =
        std::mem::transmute(objc_msgSend as *const c_void);
    let obj = send0(objc_getClass(c"NSString".as_ptr()), c"alloc");
    f(obj, sel(c"initWithUTF8String:"), s.as_ptr())
}

/// `-pcWillFinishLaunching:` — becomes the app delegate.
extern "C" fn will_finish(this: Id, _cmd: Sel, _note: Id) {
    unsafe {
        let app = send0(objc_getClass(c"NSApplication".as_ptr()), c"sharedApplication");
        if !app.is_null() && send0(app, c"delegate").is_null() {
            send1(app, c"setDelegate:", this);
        }
    }
}

/// `-application:openURLs:`
extern "C" fn open_urls(_this: Id, _cmd: Sel, _app: Id, urls: Id) {
    unsafe {
        let count_fn: unsafe extern "C" fn(Id, Sel) -> usize =
            std::mem::transmute(objc_msgSend as *const c_void);
        let at_fn: unsafe extern "C" fn(Id, Sel, usize) -> Id =
            std::mem::transmute(objc_msgSend as *const c_void);
        let n = count_fn(urls, sel(c"count"));
        let mut found = Vec::new();
        for i in 0..n {
            let url = at_fn(urls, sel(c"objectAtIndex:"), i);
            let path = send0(url, c"path");
            if path.is_null() {
                continue;
            }
            let utf8 = send0(path, c"UTF8String") as *const c_char;
            if !utf8.is_null() {
                found.push(PathBuf::from(CStr::from_ptr(utf8).to_string_lossy().into_owned()));
            }
        }
        PENDING.lock().unwrap().extend(found);
    }
    if let Some(ctx) = CONTEXT.get() {
        ctx.request_repaint();
    }
}

/// Call once on the main thread, before the event loop starts.
pub fn install() {
    unsafe {
        let superclass = objc_getClass(c"NSObject".as_ptr());
        let cls = objc_allocateClassPair(superclass, c"PresentCraftOpenDelegate".as_ptr(), 0);
        if cls.is_null() {
            return; // already installed
        }
        class_addMethod(
            cls,
            sel(c"pcWillFinishLaunching:"),
            will_finish as *const c_void,
            c"v@:@".as_ptr(),
        );
        class_addMethod(
            cls,
            sel(c"application:openURLs:"),
            open_urls as *const c_void,
            c"v@:@@".as_ptr(),
        );
        objc_registerClassPair(cls);
        // Kept for the life of the app (the delegate property is weak).
        let delegate = send0(send0(cls, c"alloc"), c"init");
        let center = send0(
            objc_getClass(c"NSNotificationCenter".as_ptr()),
            c"defaultCenter",
        );
        let name = ns_string(c"NSApplicationWillFinishLaunchingNotification");
        let f: unsafe extern "C" fn(Id, Sel, Id, Sel, Id, Id) =
            std::mem::transmute(objc_msgSend as *const c_void);
        f(
            center,
            sel(c"addObserver:selector:name:object:"),
            delegate,
            sel(c"pcWillFinishLaunching:"),
            name,
            std::ptr::null_mut(),
        );
    }
}
