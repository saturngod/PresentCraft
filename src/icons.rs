//! The app's icons. macOS shows SF Symbols, rasterised once per size through
//! AppKit and painted as a tinted texture. Windows and Linux show Material
//! Symbols from an embedded icon font. Icons are named by their SF Symbol;
//! `material::glyph` holds the matching Material glyph for each name.
//!
//! Set `PRESENTCRAFT_ICONS=material` to see the Material set on macOS.

use eframe::egui;
use egui::{Color32, Painter, Pos2};

/// Paints the icon `name` (an SF Symbol name) centred on `c` at `pt` points
/// in `color`. Returns false when there is no such icon on this system, so
/// the caller can draw its own shapes instead.
///
/// Where one SF Symbol stands for two different things, a `#hint` after the
/// name picks the Material glyph: `"rectangle.center.inset.filled#contain"`.
pub fn paint(p: &Painter, name: &'static str, c: Pos2, pt: f32, color: Color32) -> bool {
    if use_material() {
        return material::paint(p, name, c, pt, color);
    }
    #[cfg(target_os = "macos")]
    {
        let symbol = name.split_once('#').map_or(name, |(symbol, _)| symbol);
        sf::paint(p, symbol, c, pt, color)
    }
    #[cfg(not(target_os = "macos"))]
    false
}

/// Whether icons come from the Material font instead of the system.
pub fn use_material() -> bool {
    static MATERIAL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *MATERIAL.get_or_init(|| {
        cfg!(not(target_os = "macos"))
            || std::env::var("PRESENTCRAFT_ICONS").is_ok_and(|v| v == "material")
    })
}

/// Adds the icon fonts as their own families. They are never a fallback for
/// text, so they cannot change how any character in the UI looks.
pub fn install_fonts(fonts: &mut egui::FontDefinitions) {
    if !use_material() {
        return;
    }
    for insert in [
        egui_material_icons::font_insert(),
        egui_material_icons::font_insert_outlined(),
    ] {
        fonts
            .font_data
            .insert(insert.name.clone(), std::sync::Arc::new(insert.data));
        fonts
            .families
            .insert(egui::FontFamily::Name(insert.name.clone().into()), vec![insert.name]);
    }
}

mod material {
    use super::*;
    use egui_material_icons::icons::*;
    use egui_material_icons::MaterialIcon;

    /// The font size that makes a glyph as large as the SF Symbol at 1 point.
    const SCALE: f32 = 1.32;

    pub fn paint(p: &Painter, name: &str, c: Pos2, pt: f32, color: Color32) -> bool {
        let Some(icon) = glyph(name) else {
            return false;
        };
        p.text(
            c,
            egui::Align2::CENTER_CENTER,
            icon.codepoint,
            egui::FontId::new(pt * SCALE, icon.font_family()),
            color,
        );
        true
    }

    /// The Material glyph for an SF Symbol name. Outlined unless the symbol
    /// itself is a filled one.
    pub fn glyph(name: &str) -> Option<MaterialIcon> {
        let (icon, filled) = match name {
            "sidebar.left" => (ICON_SIDE_NAVIGATION, false),
            "plus.square" => (ICON_ADD_BOX, false),
            "photo" => (ICON_IMAGE, false),
            "photo.fill" => (ICON_IMAGE, true),
            "pencil" => (ICON_EDIT, false),
            "play.fill" => (ICON_PLAY_ARROW, true),
            "pause.fill" => (ICON_PAUSE, true),
            "line.3.horizontal" => (ICON_MENU, false),
            "paintbrush.pointed.fill" => (ICON_BRUSH, true),
            "chevron.left" => (ICON_CHEVRON_LEFT, false),
            "chevron.right" => (ICON_CHEVRON_RIGHT, false),
            "chevron.down" => (ICON_KEYBOARD_ARROW_DOWN, false),
            "chevron.up" => (ICON_KEYBOARD_ARROW_UP, false),
            "chevron.up.chevron.down" => (ICON_UNFOLD_MORE, false),
            "gearshape" => (ICON_SETTINGS, false),
            "doc.on.doc" => (ICON_CONTENT_COPY, false),
            "play.rectangle" => (ICON_SLIDESHOW, false),
            "play.rectangle.fill" => (ICON_SMART_DISPLAY, true),
            "mic.fill" => (ICON_MIC, true),
            "magnifyingglass" => (ICON_SEARCH, false),
            "clock" => (ICON_SCHEDULE, false),
            "ellipsis" => (ICON_MORE_HORIZ, false),
            // picture placement
            "rectangle.center.inset.filled#contain" => (ICON_FIT_SCREEN, false),
            "rectangle.leadinghalf.inset.filled" => (ICON_ART_TRACK, false),
            "rectangle.inset.filled" => (ICON_WALLPAPER, false),
            // text
            "paragraphsign" => (ICON_FORMAT_PARAGRAPH, false),
            "list.number" => (ICON_FORMAT_LIST_NUMBERED, false),
            "list.bullet" => (ICON_FORMAT_LIST_BULLETED, false),
            "checklist" => (ICON_CHECKLIST, false),
            "text.quote" => (ICON_FORMAT_QUOTE, false),
            "bold" => (ICON_FORMAT_BOLD, false),
            "italic" => (ICON_FORMAT_ITALIC, false),
            "strikethrough" => (ICON_STRIKETHROUGH_S, false),
            "textformat.superscript" => (ICON_SUPERSCRIPT, false),
            "highlighter" => (ICON_INK_HIGHLIGHTER, false),
            "link" => (ICON_LINK, false),
            "curlybraces" => (ICON_DATA_OBJECT, false),
            "chevron.left.forwardslash.chevron.right" => (ICON_CODE, false),
            "text.alignleft" => (ICON_FORMAT_ALIGN_LEFT, false),
            "text.aligncenter" => (ICON_FORMAT_ALIGN_CENTER, false),
            "text.alignright" => (ICON_FORMAT_ALIGN_RIGHT, false),
            "rectangle.topthird.inset.filled" => (ICON_VERTICAL_ALIGN_TOP, false),
            "rectangle.center.inset.filled" => (ICON_VERTICAL_ALIGN_CENTER, false),
            "rectangle.bottomthird.inset.filled" => (ICON_VERTICAL_ALIGN_BOTTOM, false),
            // tables and charts
            "tablecells" => (ICON_TABLE, false),
            "chart.xyaxis.line" => (ICON_SHOW_CHART, false),
            "chart.bar.fill" | "chart.bar.xaxis" => (ICON_BAR_CHART, true),
            "chart.pie" => (ICON_PIE_CHART, false),
            "chart.pie.fill" => (ICON_PIE_CHART, true),
            "square.and.pencil" => (ICON_EDIT_SQUARE, false),
            // settings pages
            "doc.text" => (ICON_DESCRIPTION, false),
            "paintpalette" => (ICON_PALETTE, false),
            // option lines in the editor
            "note.text" => (ICON_NOTES, false),
            "align.horizontal.left" => (ICON_ALIGN_HORIZONTAL_LEFT, false),
            "align.vertical.bottom" => (ICON_ALIGN_VERTICAL_BOTTOM, false),
            "square.grid.3x3.middle.filled" => (ICON_APPS, false),
            "arrow.up.left.and.arrow.down.right" => (ICON_OPEN_IN_FULL, false),
            "arrow.up.left.and.arrow.down.right.square" => (ICON_ASPECT_RATIO, false),
            "circle.lefthalf.filled" => (ICON_OPACITY, false),
            "slider.horizontal.3" => (ICON_TUNE, false),
            "arrow.left.arrow.right" => (ICON_SWAP_HORIZ, false),
            "square.stack.3d.up.fill" => (ICON_LAYERS, true),
            "arrow.clockwise" => (ICON_ROTATE_RIGHT, false),
            "arrow.left.and.right" => (ICON_ARROW_RANGE, false),
            "arrow.up.and.down" => (ICON_HEIGHT, false),
            "number" => (ICON_TAG, false),
            _ => return None,
        };
        Some(if filled { icon.filled() } else { icon.outlined() })
    }
}

#[cfg(target_os = "macos")]
mod sf {
    use super::*;
    use egui::{pos2, vec2, Rect, Vec2};
    use objc2::encode::{Encode, Encoding};
    use objc2::rc::{autoreleasepool, Allocated, Retained};
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::ffi::{c_char, CString};

    /// `NSFontWeightMedium`: a little heavier than regular, so thin strokes
    /// stay clear at toolbar sizes.
    const WEIGHT: f64 = 0.23;
    /// `NSCompositingOperationSourceOver`.
    const SOURCE_OVER: usize = 2;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: f64,
        height: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }
    unsafe impl Encode for CGPoint {
        const ENCODING: Encoding =
            Encoding::Struct("CGPoint", &[Encoding::Double, Encoding::Double]);
    }
    unsafe impl Encode for CGSize {
        const ENCODING: Encoding =
            Encoding::Struct("CGSize", &[Encoding::Double, Encoding::Double]);
    }
    unsafe impl Encode for CGRect {
        const ENCODING: Encoding =
            Encoding::Struct("CGRect", &[CGPoint::ENCODING, CGSize::ENCODING]);
    }

    #[link(name = "AppKit", kind = "framework")]
    extern "C" {
        static NSDeviceRGBColorSpace: *const AnyObject;
    }

    struct Entry {
        tex: egui::TextureHandle,
        /// Size in points.
        size: Vec2,
    }

    thread_local! {
        /// Keyed by name, size in tenths of a point, and pixels per point × 100.
        static CACHE: RefCell<HashMap<(&'static str, u32, u32), Option<Entry>>> =
            RefCell::new(HashMap::new());
    }

    pub fn paint(p: &Painter, name: &'static str, c: Pos2, pt: f32, color: Color32) -> bool {
        let ppp = p.ctx().pixels_per_point();
        let key = (name, (pt * 10.0).round() as u32, (ppp * 100.0).round() as u32);
        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let entry = cache.entry(key).or_insert_with(|| {
                let (w, h, alpha) = render(name, pt, ppp)?;
                let pixels = alpha.into_iter().map(Color32::from_white_alpha).collect();
                let tex = p.ctx().load_texture(
                    format!("sf:{name}:{pt}"),
                    egui::ColorImage::new([w, h], pixels),
                    egui::TextureOptions::LINEAR,
                );
                Some(Entry {
                    tex,
                    size: vec2(w as f32, h as f32) / ppp,
                })
            });
            let Some(e) = entry else {
                return false;
            };
            // Start on a whole pixel so the strokes stay sharp.
            let min = c - e.size / 2.0;
            let min = pos2((min.x * ppp).round() / ppp, (min.y * ppp).round() / ppp);
            p.image(
                e.tex.id(),
                Rect::from_min_size(min, e.size),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                color,
            );
            true
        })
    }

    /// The symbol's coverage as one alpha byte per pixel, with its size in pixels.
    fn render(name: &str, pt: f32, ppp: f32) -> Option<(usize, usize, Vec<u8>)> {
        let cname = CString::new(name).ok()?;
        autoreleasepool(|_| unsafe {
            let ns_name: Option<Retained<AnyObject>> =
                msg_send![class!(NSString), stringWithUTF8String: cname.as_ptr()];
            let ns_name = ns_name?;
            let img: Option<Retained<AnyObject>> = msg_send![
                class!(NSImage),
                imageWithSystemSymbolName: &*ns_name,
                accessibilityDescription: std::ptr::null::<AnyObject>()
            ];
            let img = img?;
            let cfg: Option<Retained<AnyObject>> = msg_send![
                class!(NSImageSymbolConfiguration),
                configurationWithPointSize: pt as f64,
                weight: WEIGHT
            ];
            let cfg = cfg?;
            let img: Option<Retained<AnyObject>> =
                msg_send![&*img, imageWithSymbolConfiguration: &*cfg];
            let img = img?;
            let size: CGSize = msg_send![&*img, size];
            let w = (size.width * ppp as f64).ceil().max(1.0) as isize;
            let h = (size.height * ppp as f64).ceil().max(1.0) as isize;

            let alloc: Allocated<AnyObject> = msg_send![class!(NSBitmapImageRep), alloc];
            let rep: Option<Retained<AnyObject>> = msg_send![
                alloc,
                initWithBitmapDataPlanes: std::ptr::null_mut::<*mut c_char>(),
                pixelsWide: w,
                pixelsHigh: h,
                bitsPerSample: 8isize,
                samplesPerPixel: 4isize,
                hasAlpha: true,
                isPlanar: false,
                colorSpaceName: NSDeviceRGBColorSpace,
                bytesPerRow: 0isize,
                bitsPerPixel: 0isize
            ];
            let rep = rep?;
            let ctx: Option<Retained<AnyObject>> =
                msg_send![class!(NSGraphicsContext), graphicsContextWithBitmapImageRep: &*rep];
            let ctx = ctx?;
            let _: () = msg_send![class!(NSGraphicsContext), saveGraphicsState];
            let _: () = msg_send![class!(NSGraphicsContext), setCurrentContext: &*ctx];
            let zero = CGPoint { x: 0.0, y: 0.0 };
            let dst = CGRect {
                origin: zero,
                size: CGSize {
                    width: w as f64,
                    height: h as f64,
                },
            };
            let whole = CGRect {
                origin: zero,
                size: CGSize {
                    width: 0.0,
                    height: 0.0,
                },
            };
            let _: () = msg_send![
                &*img,
                drawInRect: dst,
                fromRect: whole,
                operation: SOURCE_OVER,
                fraction: 1.0f64
            ];
            let _: () = msg_send![&*ctx, flushGraphics];
            let _: () = msg_send![class!(NSGraphicsContext), restoreGraphicsState];

            let data: *mut c_char = msg_send![&*rep, bitmapData];
            let row: isize = msg_send![&*rep, bytesPerRow];
            if data.is_null() || row < w * 4 {
                return None;
            }
            let (w, h, row) = (w as usize, h as usize, row as usize);
            let bytes = std::slice::from_raw_parts(data as *const u8, row * h);
            let mut alpha = Vec::with_capacity(w * h);
            for y in 0..h {
                for x in 0..w {
                    alpha.push(bytes[y * row + x * 4 + 3]);
                }
            }
            Some((w, h, alpha))
        })
    }
}
