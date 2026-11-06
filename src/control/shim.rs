//! `PresentCraft --mcp`: the MCP server AI clients start over stdio.
//!
//! Each tool call goes to the running app when it is open. When it is not,
//! the call can still edit a folder-form .prcraft package given in the
//! `package` argument. Nothing but protocol messages may go to stdout.

use super::script::{self, Edit};
use super::tools;
use super::wire::{compact, obj, text};
use crate::json::Json;
use crate::package;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

const PROTOCOL: &str = "2025-06-18";

const INSTRUCTIONS: &str = "Tools for PresentCraft presentations. Slide numbers start at 1. \
Call deck_info first to see what is open. The script is Markdown: a line of three or more dashes \
starts a new slide; lines starting with a tab show on the slide; other lines are speaker notes. \
A slide that starts with a # heading is a cover; place its title with title_align. \
Check your work with slide_screenshot and deck_check. PresentCraft must be running (File > MCP > \
Start Server) for design tools; the script tools also work on a closed .prcraft folder when given \
its path in the package argument.";

const NOT_RUNNING: &str = "PresentCraft is not running with the MCP server started. Open PresentCraft and choose File > MCP > Start Server, then try again.";

/// Runs the server until the client closes stdin. Returns the exit code.
pub fn run() -> i32 {
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Some(reply) = handle(&line) else { continue };
        let mut out = std::io::stdout().lock();
        if writeln!(out, "{}", compact(&reply))
            .and_then(|_| out.flush())
            .is_err()
        {
            break;
        }
    }
    0
}

fn handle(line: &str) -> Option<Json> {
    let Ok(msg) = Json::parse(line) else {
        return Some(rpc_error(Json::Null, -32700, "Parse error"));
    };
    // A message without an id is a notification: nothing to answer.
    let id = msg.get("id").cloned()?;
    let method = msg.str("method").unwrap_or("").to_string();
    let result: Result<Json, String> = match method.as_str() {
        "initialize" => Ok(obj(vec![
            ("protocolVersion", text(PROTOCOL)),
            ("capabilities", obj(vec![("tools", obj(vec![]))])),
            (
                "serverInfo",
                obj(vec![
                    ("name", text("presentcraft")),
                    ("version", text(env!("CARGO_PKG_VERSION"))),
                ]),
            ),
            ("instructions", text(INSTRUCTIONS)),
        ])),
        "ping" => Ok(obj(vec![])),
        "tools/list" => Ok(obj(vec![("tools", tools::list())])),
        "tools/call" => Ok(call(msg.get("params"))),
        other => return Some(rpc_error(id, -32601, &format!("Method not found: {other}"))),
    };
    Some(match result {
        Ok(r) => obj(vec![("jsonrpc", text("2.0")), ("id", id), ("result", r)]),
        Err(e) => rpc_error(id, -32603, &e),
    })
}

fn rpc_error(id: Json, code: i32, message: &str) -> Json {
    obj(vec![
        ("jsonrpc", text("2.0")),
        ("id", id),
        (
            "error",
            obj(vec![
                ("code", Json::Num(code as f64)),
                ("message", text(message)),
            ]),
        ),
    ])
}

fn call(params: Option<&Json>) -> Json {
    let name = params.and_then(|p| p.str("name")).unwrap_or("").to_string();
    let args = params
        .and_then(|p| p.get("arguments"))
        .cloned()
        .unwrap_or(Json::Obj(Vec::new()));
    if !tools::exists(&name) {
        return tool_error(&format!("There is no tool called {name}."));
    }
    match forward(&name, &args) {
        Ok(result) => tool_result(result),
        Err(msg) => tool_error(&msg),
    }
}

fn tool_error(msg: &str) -> Json {
    obj(vec![
        (
            "content",
            Json::Arr(vec![obj(vec![("type", text("text")), ("text", text(msg))])]),
        ),
        ("isError", Json::Bool(true)),
    ])
}

fn tool_result(result: Json) -> Json {
    let mut content = Vec::new();
    let mut rest = result.clone();
    if let (Some(png), Json::Obj(items)) = (result.str("png_base64"), &mut rest) {
        content.push(obj(vec![
            ("type", text("image")),
            ("data", text(png)),
            ("mimeType", text("image/png")),
        ]));
        items.retain(|(k, _)| k != "png_base64");
    }
    content.push(obj(vec![
        ("type", text("text")),
        ("text", text(&rest.to_pretty())),
    ]));
    obj(vec![
        ("content", Json::Arr(content)),
        ("isError", Json::Bool(false)),
    ])
}

/// Sends the call to the running app, or to a package on disk.
fn forward(name: &str, args: &Json) -> Result<Json, String> {
    if args.str("package").is_some() {
        return headless(name, args);
    }
    if let Some((port, token)) = super::wire::read_discovery() {
        if let Ok(answer) = send(port, &token, name, args) {
            return answer;
        }
    }
    Err(NOT_RUNNING.to_string())
}

/// Connects to the app. `Err` means the app could not be reached; `Ok(Err)`
/// is the app's own answer.
fn send(port: u16, token: &str, name: &str, args: &Json) -> Result<Result<Json, String>, String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let stream =
        TcpStream::connect_timeout(&addr, Duration::from_secs(2)).map_err(|e| e.to_string())?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(90)));
    let mut writer = stream.try_clone().map_err(|e| e.to_string())?;
    let request = compact(&obj(vec![
        ("token", text(token)),
        ("tool", text(name)),
        ("arguments", args.clone()),
    ]));
    writeln!(writer, "{request}").map_err(|e| e.to_string())?;
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let answer = Json::parse(line.trim())
        .map_err(|_| "PresentCraft sent an unreadable answer.".to_string())?;
    if answer.bool("ok") == Some(true) {
        Ok(Ok(answer.get("result").cloned().unwrap_or(Json::Null)))
    } else {
        Ok(Err(answer
            .str("error")
            .unwrap_or("PresentCraft refused the request.")
            .to_string()))
    }
}

/// A tool call against a folder-form package, with the app closed.
fn headless(name: &str, args: &Json) -> Result<Json, String> {
    if !tools::HEADLESS.contains(&name) {
        return Err(format!(
            "{name} needs PresentCraft to be open. Only the script tools work on a closed package."
        ));
    }
    let pkg = args.str("package").unwrap_or("");
    let opened = package::open(Path::new(pkg))?;
    if opened.form == package::Form::Zip {
        return Err("A zipped presentation needs PresentCraft open. Unzip it to a folder, or open it in the app.".into());
    }
    let root = opened.root.clone();
    let file = root.join("text.md");
    let deck = opened.text;
    match name {
        "deck_read_markdown" => return Ok(obj(vec![("markdown", text(&deck))])),
        "slide_list" => {
            let items = script::bodies(&deck)
                .iter()
                .enumerate()
                .map(|(i, body)| {
                    obj(vec![
                        ("slide", Json::Num((i + 1) as f64)),
                        ("title", text(&script::title_of(body))),
                    ])
                })
                .collect();
            return Ok(Json::Arr(items));
        }
        "slide_get" => {
            let n = args.num("slide").unwrap_or(0.0) as usize;
            let bodies = script::bodies(&deck);
            let body = bodies
                .get(n.wrapping_sub(1))
                .ok_or_else(|| format!("There is no slide {n}."))?;
            return Ok(obj(vec![
                ("slide", Json::Num(n as f64)),
                ("markdown", text(body)),
            ]));
        }
        _ => {}
    }
    if matches!(name, "deck_write_markdown" | "slide_delete") && args.bool("confirm") != Some(true)
    {
        return Err(format!(
            "{name} changes or removes content. Pass confirm: true to go ahead."
        ));
    }
    let mut edit = script::parse_edit(name, args)?.ok_or("This tool is not a script edit.")?;
    if let Edit::Picture { src, .. } = &mut edit {
        if Path::new(src.as_str()).is_absolute() && Path::new(src.as_str()).is_file() {
            super::wire::check_path(Path::new(src.as_str()), Some(&root))?;
            let local = src.clone();
            *src = package::add_asset(&root, Path::new(&local))?;
        } else if !(src.starts_with("/assets/") || src.starts_with("http")) {
            return Err(format!("There is no file at {src}, and only /assets/ paths or web addresses can be used without PresentCraft open."));
        }
    }
    let new_deck = script::apply(&deck, &edit)?;
    let tmp = root.join("text.md.tmp");
    std::fs::write(&tmp, &new_deck).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &file).map_err(|e| e.to_string())?;
    Ok(obj(vec![
        ("saved", text(&file.display().to_string())),
        ("slides", Json::Num(script::bodies(&new_deck).len() as f64)),
    ]))
}
