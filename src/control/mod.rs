//! The MCP control channel: lets an AI client work in PresentCraft.
//!
//! - `server` runs inside the app. It listens on 127.0.0.1 with a random
//!   token and hands each request to the UI thread through a queue.
//! - `exec` runs the tools against the open window (`App`).
//! - `shim` is `PresentCraft --mcp`: a stdio MCP server that forwards tool
//!   calls to the running app, or edits a package on disk when the app is
//!   closed.
//! - `script` holds the slide edits shared by both paths; `tools` lists the
//!   tools for MCP clients; `wire` has the small helpers they all use.

pub mod exec;
pub mod script;
pub mod server;
pub mod shim;
pub mod tools;
pub mod wire;

/// Control-channel state kept on the app.
#[derive(Default)]
pub struct Control {
    /// The running server, when File > MCP > Start Server was chosen.
    pub running: Option<server::Running>,
    /// Each AI change with the state before and after it, newest last.
    pub undo: Vec<exec::Entry>,
    /// A slide screenshot waiting for the next rendered frame.
    pub shot: Option<exec::PendingShot>,
}

impl Drop for Control {
    fn drop(&mut self) {
        if let Some(running) = self.running.take() {
            server::stop(running);
        }
    }
}
