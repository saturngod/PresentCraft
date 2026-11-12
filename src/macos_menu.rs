//! The menu bar at the top of the screen. On macOS this replaces the menu
//! button in the toolbar. Choosing an item queues its id, which the app
//! handles on its next frame (see `App::menu_action`).

use std::path::PathBuf;
use std::sync::Mutex;

use muda::accelerator::Accelerator;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

static PENDING: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Ids of the items chosen since the last call.
pub fn take_actions() -> Vec<String> {
    std::mem::take(&mut *PENDING.lock().unwrap())
}

/// What the menu shows that depends on the document.
#[derive(Clone, PartialEq, Default)]
pub struct State {
    pub recent: Vec<PathBuf>,
    pub has_path: bool,
    pub sidebar_shown: bool,
    pub presenting: bool,
    pub mcp_port: Option<u16>,
}

pub struct MacMenu {
    // The menu bar goes away when this is dropped.
    _menu: Menu,
    recent: Submenu,
    revert: MenuItem,
    sidebar: MenuItem,
    present: MenuItem,
    mcp_status: MenuItem,
    mcp_start: MenuItem,
    mcp_stop: MenuItem,
    shown: Option<State>,
}

fn item(id: &str, text: &str, shortcut: &str) -> MenuItem {
    let accel = shortcut.parse::<Accelerator>().ok();
    MenuItem::with_id(id, text, true, accel)
}

fn note(text: &str) -> MenuItem {
    MenuItem::new(text, false, None)
}

fn sep() -> PredefinedMenuItem {
    PredefinedMenuItem::separator()
}

impl MacMenu {
    /// Builds the menu bar. Call on the main thread once the app has
    /// launched; `templates` are the Insert menu's groups of slide names.
    pub fn install(ctx: &egui::Context, templates: &[(&str, Vec<&str>)]) -> Option<Self> {
        let ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            PENDING.lock().unwrap().push(e.id.0);
            ctx.request_repaint();
        }));

        let app = Submenu::new("PresentCraft", true);
        app.append_items(&[
            &PredefinedMenuItem::about(Some("About PresentCraft"), None),
            &item("updates", "Check for Updates…", ""),
            &sep(),
            &item("settings", "Settings…", "CmdOrCtrl+,"),
            &sep(),
            &PredefinedMenuItem::services(None),
            &sep(),
            &PredefinedMenuItem::hide(Some("Hide PresentCraft")),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &sep(),
            // Not the predefined Quit, which would skip the unsaved-changes check.
            &item("quit", "Quit PresentCraft", "CmdOrCtrl+Q"),
        ])
        .ok()?;

        let recent = Submenu::new("Open Recent", true);
        let revert = item("revert", "Revert to Saved…", "");
        let import = Submenu::new("Import", true);
        import.append(&item("import", "Markdown or Text…", "")).ok()?;
        let export = Submenu::new("Export", true);
        export
            .append_items(&[
                &item("export:pdf", "PDF…", ""),
                &item("export:pptx", "PowerPoint…", ""),
                &item("export:images", "Images…", ""),
                &item("export:html", "HTML…", ""),
                &item("export:markdown", "Markdown…", ""),
            ])
            .ok()?;
        let mcp = Submenu::new("MCP", true);
        let mcp_status = note("Server stopped");
        let mcp_start = item("mcp_start", "Start Server", "");
        let mcp_stop = item("mcp_stop", "Stop Server", "");
        mcp.append_items(&[&mcp_status, &mcp_start, &mcp_stop]).ok()?;
        let file = Submenu::new("File", true);
        file.append_items(&[
            &item("new", "New", "CmdOrCtrl+N"),
            &item("open", "Open…", "CmdOrCtrl+O"),
            &recent,
            &sep(),
            &item("close", "Close Window", "CmdOrCtrl+W"),
            &item("save", "Save", "CmdOrCtrl+S"),
            &item("save_as", "Save As…", "CmdOrCtrl+Shift+S"),
            &item("duplicate", "Duplicate…", ""),
            &revert,
            &sep(),
            &import,
            &export,
            &sep(),
            &mcp,
        ])
        .ok()?;

        let insert = Submenu::new("Insert", true);
        insert
            .append(&item("blank", "Blank Slide", "CmdOrCtrl+Shift+N"))
            .ok()?;
        for (g, (group, names)) in templates.iter().enumerate() {
            insert.append(&sep()).ok()?;
            insert.append(&note(group)).ok()?;
            for (i, name) in names.iter().enumerate() {
                insert
                    .append(&item(&format!("template:{g}:{i}"), name, ""))
                    .ok()?;
            }
        }

        let sidebar = item("sidebar", "Hide Slide List", "");
        let present = item("present", "Start Presentation", "CmdOrCtrl+Enter");
        let view = Submenu::new("View", true);
        view.append_items(&[
            &sidebar,
            &sep(),
            &present,
            &sep(),
            &PredefinedMenuItem::fullscreen(None),
        ])
        .ok()?;

        let window = Submenu::new("Window", true);
        window
            .append_items(&[
                &PredefinedMenuItem::minimize(None),
                &PredefinedMenuItem::maximize(Some("Zoom")),
                &sep(),
                &PredefinedMenuItem::bring_all_to_front(None),
            ])
            .ok()?;

        let menu = Menu::new();
        menu.append_items(&[&app, &file, &insert, &view, &window])
            .ok()?;
        menu.init_for_nsapp();
        window.set_as_windows_menu_for_nsapp();

        Some(Self {
            _menu: menu,
            recent,
            revert,
            sidebar,
            present,
            mcp_status,
            mcp_start,
            mcp_stop,
            shown: None,
        })
    }

    /// Brings the items that depend on the document up to date.
    pub fn sync(&mut self, state: State) {
        if self.shown.as_ref() == Some(&state) {
            return;
        }
        let old = self.shown.take();
        if old.as_ref().map(|o| &o.recent) != Some(&state.recent) {
            while self.recent.remove_at(0).is_some() {}
            if state.recent.is_empty() {
                let _ = self.recent.append(&note("No recent documents"));
            }
            for (i, path) in state.recent.iter().enumerate() {
                let _ = self.recent.append(&item(
                    &format!("recent:{i}"),
                    &path.display().to_string(),
                    "",
                ));
            }
        }
        self.revert.set_enabled(state.has_path);
        self.sidebar.set_text(if state.sidebar_shown {
            "Hide Slide List"
        } else {
            "Show Slide List"
        });
        self.present.set_text(if state.presenting {
            "Stop Presentation"
        } else {
            "Start Presentation"
        });
        self.mcp_status.set_text(match state.mcp_port {
            Some(p) => format!("Server running on port {p}"),
            None => "Server stopped".to_string(),
        });
        self.mcp_start.set_enabled(state.mcp_port.is_none());
        self.mcp_stop.set_enabled(state.mcp_port.is_some());
        self.shown = Some(state);
    }
}
