//! The app's side of the control channel: a localhost listener.
//!
//! Protocol: one JSON object per line.
//!   request:  {"token": "...", "tool": "slide_get", "arguments": {...}}
//!   response: {"ok": true, "result": ...}  or  {"ok": false, "error": "..."}
//! Each request is passed to the UI thread, which runs it on the next frame.

use super::wire;
use crate::json::Json;
use eframe::egui;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

/// A tool call waiting for the UI thread.
pub struct Req {
    pub tool: String,
    pub args: Json,
    pub reply: Sender<Result<Json, String>>,
}

pub struct Running {
    pub port: u16,
    stop: Arc<AtomicBool>,
    pub rx: Receiver<Req>,
}

/// Starts listening on 127.0.0.1 with a new token and writes the discovery
/// file the shim reads.
pub fn start(ctx: egui::Context) -> Result<Running, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let token = wire::token();
    wire::write_discovery(port, &token)
        .map_err(|e| format!("Couldn't write the server file: {e}"))?;
    let (tx, rx) = channel::<Req>();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = stop.clone();
    std::thread::spawn(move || {
        while !stop_thread.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    // Accepted sockets inherit non-blocking mode on macOS, which
                    // would cut large replies short.
                    let _ = stream.set_nonblocking(false);
                    let (token, tx, ctx, stop) =
                        (token.clone(), tx.clone(), ctx.clone(), stop_thread.clone());
                    std::thread::spawn(move || serve(stream, &token, tx, ctx, stop));
                }
                Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        }
    });
    Ok(Running { port, stop, rx })
}

/// Stops listening and removes the discovery file.
pub fn stop(running: Running) {
    running.stop.store(true, Ordering::Relaxed);
    wire::remove_discovery();
}

fn serve(
    stream: TcpStream,
    token: &str,
    tx: Sender<Req>,
    ctx: egui::Context,
    stop: Arc<AtomicBool>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if !line.ends_with('\n') {
                    continue;
                }
                let request = std::mem::take(&mut line);
                let response = handle(request.trim(), token, &tx, &ctx);
                if writeln!(writer, "{}", wire::compact(&response)).is_err() {
                    break;
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue
            }
            Err(_) => break,
        }
    }
}

fn handle(line: &str, token: &str, tx: &Sender<Req>, ctx: &egui::Context) -> Json {
    let fail = |msg: &str| wire::obj(vec![("ok", Json::Bool(false)), ("error", wire::text(msg))]);
    let Ok(req) = Json::parse(line) else {
        return fail("The request is not valid JSON.");
    };
    if req.str("token") != Some(token) {
        return fail("The token is wrong. Start the server again from File > MCP.");
    }
    let Some(tool) = req.str("tool") else {
        return fail("The request has no tool name.");
    };
    if tool == "ping" {
        return wire::obj(vec![
            ("ok", Json::Bool(true)),
            ("result", wire::text("pong")),
        ]);
    }
    let args = req
        .get("arguments")
        .cloned()
        .unwrap_or(Json::Obj(Vec::new()));
    let (reply_tx, reply_rx) = channel();
    if tx
        .send(Req {
            tool: tool.to_string(),
            args,
            reply: reply_tx,
        })
        .is_err()
    {
        return fail("PresentCraft is closing.");
    }
    ctx.request_repaint();
    match reply_rx.recv_timeout(Duration::from_secs(60)) {
        Ok(Ok(result)) => wire::obj(vec![("ok", Json::Bool(true)), ("result", result)]),
        Ok(Err(msg)) => fail(&msg),
        Err(_) => fail("PresentCraft did not answer in time."),
    }
}
