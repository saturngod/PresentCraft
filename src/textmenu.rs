//! The menu for selected text in the editor, on macOS. It is the system's own
//! text menu, with Look Up, Services, Share, Cut, Copy, Paste and Select All,
//! as in any Mac text field.
//!
//! egui draws the editor, so a hidden `NSTextView` stands in for it. The view
//! holds the whole document with the editor's selection, and the menu acts on
//! that view. Afterwards its text and selection are read back, so Cut, Paste,
//! Select All and Services that rewrite text take effect in the editor. Copy
//! needs no read-back: the text view puts the selection on the pasteboard.

use std::ffi::{c_char, CStr, CString};
use std::sync::atomic::{AtomicUsize, Ordering};

use objc2::encode::{Encode, Encoding};
use objc2::rc::{autoreleasepool, Allocated, Retained};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use wry::raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// The window's content view, an `NSView *`. Set on the first frame.
static VIEW: AtomicUsize = AtomicUsize::new(0);

/// `NSEventTypeRightMouseDown`.
const RIGHT_MOUSE_DOWN: usize = 3;
/// Size of the hidden text view. It only has to cover the right-click.
const PROBE_W: f64 = 400.0;
const PROBE_H: f64 = 200.0;

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Size {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    origin: Point,
    size: Size,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct NsRange {
    location: usize,
    length: usize,
}

unsafe impl Encode for Point {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[Encoding::Double, Encoding::Double]);
}
unsafe impl Encode for Size {
    const ENCODING: Encoding = Encoding::Struct("CGSize", &[Encoding::Double, Encoding::Double]);
}
unsafe impl Encode for Rect {
    const ENCODING: Encoding = Encoding::Struct("CGRect", &[Point::ENCODING, Size::ENCODING]);
}
unsafe impl Encode for NsRange {
    const ENCODING: Encoding = Encoding::Struct("_NSRange", &[usize::ENCODING, usize::ENCODING]);
}

/// What the menu left behind.
pub struct Menu {
    pub text: String,
    /// Selection as char indices, start first.
    pub selection: (usize, usize),
}

/// Remembers the window's view for the menu. Call every frame; it does
/// nothing after the first time it finds the view.
pub fn remember(frame: &eframe::Frame) {
    if VIEW.load(Ordering::Relaxed) != 0 {
        return;
    }
    if let Ok(handle) = frame.window_handle() {
        if let RawWindowHandle::AppKit(h) = handle.as_raw() {
            VIEW.store(h.ns_view.as_ptr() as usize, Ordering::Relaxed);
        }
    }
}

/// Shows the text menu for `sel` (char indices) of `text`. `at` is the
/// right-click in points from the top-left of the window's content. Returns
/// what the menu left, or `None` when no menu was shown.
pub fn show(text: &str, sel: (usize, usize), at: egui::Pos2) -> Option<Menu> {
    let view = VIEW.load(Ordering::Relaxed);
    if view == 0 {
        return None;
    }
    // A document with a NUL byte cannot cross into Objective-C as a C string.
    let cstr = CString::new(text).ok()?;
    let utf16 = (char_to_utf16(text, sel.0), char_to_utf16(text, sel.1));
    let (text_out, start, end) =
        autoreleasepool(|_| unsafe { run(view, &cstr, utf16, at) })?;
    Some(Menu {
        selection: (utf16_to_char(&text_out, start), utf16_to_char(&text_out, end)),
        text: text_out,
    })
}

/// Offset in UTF-16 units of char index `c`, as `NSTextView` counts.
fn char_to_utf16(text: &str, c: usize) -> usize {
    text.chars().take(c).map(char::len_utf16).sum()
}

/// Char index of UTF-16 offset `u`.
fn utf16_to_char(text: &str, u: usize) -> usize {
    let mut units = 0;
    for (i, ch) in text.chars().enumerate() {
        if units >= u {
            return i;
        }
        units += ch.len_utf16();
    }
    text.chars().count()
}

/// Runs the menu on a probe view and returns the text with the selection in
/// UTF-16 units. The window's first responder is put back afterwards.
unsafe fn run(
    view_ptr: usize,
    cstr: &CString,
    sel: (usize, usize),
    at: egui::Pos2,
) -> Option<(String, usize, usize)> {
    let view: &AnyObject = &*(view_ptr as *const AnyObject);
    let window: Option<Retained<AnyObject>> = msg_send![view, window];
    let window = window?;

    // The probe's top-left corner sits on the click. Its text starts there,
    // so the click lands on the first line.
    let flipped: bool = msg_send![view, isFlipped];
    let bounds: Rect = msg_send![view, bounds];
    let x = at.x as f64;
    let top = if flipped {
        at.y as f64
    } else {
        bounds.size.height - at.y as f64
    };
    let frame = Rect {
        origin: Point {
            x,
            y: if flipped { top } else { top - PROBE_H },
        },
        size: Size {
            width: PROBE_W,
            height: PROBE_H,
        },
    };
    let inside = Point {
        x: x + 2.0,
        y: if flipped { top + 2.0 } else { top - 2.0 },
    };
    let inside_window: Point = msg_send![view, convertPoint: inside, toView: None::<&AnyObject>];
    let window_number: isize = msg_send![&*window, windowNumber];

    let alloc: Allocated<AnyObject> = msg_send![class!(NSTextView), alloc];
    let probe: Option<Retained<AnyObject>> = msg_send![alloc, initWithFrame: frame];
    let probe = probe?;
    let ns_text: Option<Retained<AnyObject>> =
        msg_send![class!(NSString), stringWithUTF8String: cstr.as_ptr()];
    let ns_text = ns_text?;
    let _: () = msg_send![&*probe, setRichText: false];
    let _: () = msg_send![&*probe, setString: &*ns_text];
    let _: () = msg_send![&*probe, setAlphaValue: 0.0f64];
    let _: () = msg_send![view, addSubview: &*probe];
    let _: bool = msg_send![&*window, makeFirstResponder: &*probe];

    let outcome = popup(&probe, sel, inside_window, window_number);

    let _: () = msg_send![&*probe, removeFromSuperview];
    let _: bool = msg_send![&*window, makeFirstResponder: view];
    outcome
}

/// Opens the menu on the probe and reads back what it did.
unsafe fn popup(
    probe: &AnyObject,
    sel: (usize, usize),
    at: Point,
    window_number: isize,
) -> Option<(String, usize, usize)> {
    let event: Option<Retained<AnyObject>> = msg_send![
        class!(NSEvent),
        mouseEventWithType: RIGHT_MOUSE_DOWN,
        location: at,
        modifierFlags: 0usize,
        timestamp: 0.0f64,
        windowNumber: window_number,
        context: None::<&AnyObject>,
        eventNumber: 0isize,
        clickCount: 1isize,
        pressure: 1.0f32
    ];
    let event = event?;
    let selection = NsRange {
        location: sel.0,
        length: sel.1 - sel.0,
    };
    let menu: Option<Retained<AnyObject>> = msg_send![probe, menuForEvent: &*event];
    // A right-click outside the selection moves the caret. Put the selection
    // back, so the menu acts on the text the editor had selected.
    let _: () = msg_send![probe, setSelectedRange: selection];
    let menu = menu?;

    // The event gives the menu its Look Up, Siri and Services items.
    let _: () = msg_send![
        class!(NSMenu),
        popUpContextMenu: &*menu,
        withEvent: &*event,
        forView: probe
    ];

    let text: Option<Retained<AnyObject>> = msg_send![probe, string];
    let text = text?;
    let utf8: *const c_char = msg_send![&*text, UTF8String];
    if utf8.is_null() {
        return None;
    }
    let range: NsRange = msg_send![probe, selectedRange];
    let text = CStr::from_ptr(utf8).to_string_lossy().into_owned();
    Some((text, range.location, range.location + range.length))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_convert_between_chars_and_utf16() {
        // "😀" is one char but two UTF-16 units.
        let text = "a😀b";
        assert_eq!(char_to_utf16(text, 0), 0);
        assert_eq!(char_to_utf16(text, 1), 1);
        assert_eq!(char_to_utf16(text, 2), 3);
        assert_eq!(char_to_utf16(text, 3), 4);
        assert_eq!(utf16_to_char(text, 0), 0);
        assert_eq!(utf16_to_char(text, 1), 1);
        assert_eq!(utf16_to_char(text, 3), 2);
        assert_eq!(utf16_to_char(text, 4), 3);
        assert_eq!(utf16_to_char(text, 99), 3);
    }
}
