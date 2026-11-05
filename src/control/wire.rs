//! Helpers shared by the server, the shim and the tools: compact JSON,
//! base64, the access token and the discovery file.

use crate::json::Json;
use std::path::PathBuf;

/// One-line JSON (the protocol is newline-delimited, so no raw newlines).
pub fn compact(v: &Json) -> String {
    let mut out = String::new();
    write(&mut out, v);
    out
}

fn write(out: &mut String, v: &Json) {
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Num(n) => {
            if !n.is_finite() {
                out.push_str("null");
            } else if n.fract() == 0.0 && n.abs() < 1e15 {
                out.push_str(&format!("{}", *n as i64));
            } else {
                out.push_str(&format!("{n}"));
            }
        }
        Json::Str(s) => escape(out, s),
        Json::Arr(items) => {
            out.push('[');
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(out, x);
            }
            out.push(']');
        }
        Json::Obj(items) => {
            out.push('{');
            for (i, (k, x)) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                escape(out, k);
                out.push(':');
                write(out, x);
            }
            out.push('}');
        }
    }
}

fn escape(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A JSON object from (key, value) pairs.
pub fn obj(items: Vec<(&str, Json)>) -> Json {
    Json::Obj(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

pub fn text(v: &str) -> Json {
    Json::Str(v.to_string())
}

pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// A 128-bit random token, as 32 hex digits.
pub fn token() -> String {
    use std::hash::{BuildHasher, Hasher};
    use std::io::Read;
    let mut buf = [0u8; 16];
    let from_os = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut buf))
        .is_ok();
    if !from_os {
        // Each RandomState is seeded at random by the standard library.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        for (i, chunk) in buf.chunks_mut(8).enumerate() {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u64(nanos ^ i as u64);
            let v = h.finish().to_le_bytes();
            chunk.copy_from_slice(&v[..chunk.len()]);
        }
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// Paths the tools may read from or write to: absolute, with no `..`, and
/// inside the home folder, the temporary folder, or `extra` (the presentation's
/// folder). Keeps an AI from copying or writing files elsewhere on the disk.
pub fn check_path(path: &std::path::Path, extra: Option<&std::path::Path>) -> Result<(), String> {
    use std::path::Component;
    if !path.is_absolute() {
        return Err("Use an absolute path, such as /Users/you/Documents/talk.pdf.".into());
    }
    if path.components().any(|c| c == Component::ParentDir) {
        return Err("The path may not contain \"..\".".into());
    }
    let mut roots = vec![std::env::temp_dir()];
    if let Some(h) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        roots.push(PathBuf::from(h));
    }
    roots.extend(extra.map(|e| e.to_path_buf()));
    if roots.iter().any(|r| path.starts_with(r)) {
        Ok(())
    } else {
        Err("The path must be inside your home folder or the presentation's folder.".into())
    }
}

/// Where the running server writes its port and token. Only the person's
/// own account can read it (the folder is the app's preferences folder).
pub fn discovery_path() -> Option<PathBuf> {
    crate::settings::Settings::app_dir().map(|d| d.join("mcp.json"))
}

pub fn write_discovery(port: u16, token: &str) -> std::io::Result<()> {
    let path = discovery_path().ok_or_else(|| std::io::Error::other("no settings folder"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let body = compact(&obj(vec![
        ("port", Json::Num(port as f64)),
        ("token", text(token)),
        ("pid", Json::Num(std::process::id() as f64)),
    ]));
    std::fs::write(&path, body)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn remove_discovery() {
    if let Some(path) = discovery_path() {
        let _ = std::fs::remove_file(path);
    }
}

/// Reads the port and token the running app wrote.
pub fn read_discovery() -> Option<(u16, String)> {
    let text = std::fs::read_to_string(discovery_path()?).ok()?;
    let v = Json::parse(&text).ok()?;
    let port = v.num("port")? as u16;
    let token = v.str("token")?.to_string();
    Some((port, token))
}
