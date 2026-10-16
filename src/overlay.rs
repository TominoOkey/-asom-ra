//! Local HTTP server and serialized state used by the OBS overlay.

use serde::Serialize;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, OnceLock, RwLock};
use std::thread;
use std::time::Duration;

#[derive(Clone, Serialize)]
pub(super) struct OverlayLane {
    pub(super) lane: u8,
    pub(super) name: String,
    pub(super) club: String,
    pub(super) entry: String,
    pub(super) time: Option<u32>,
    pub(super) rank: Option<usize>,
    pub(super) swimming: bool,
    pub(super) personal_record: bool,
}

#[derive(Clone, Serialize)]
pub(super) struct OverlayData {
    pub(super) event_name: String,
    pub(super) discipline_name: String,
    pub(super) discipline: u16,
    pub(super) heat: u8,
    pub(super) heat_index: usize,
    pub(super) heat_count: usize,
    pub(super) elapsed_ms: Option<u32>,
    pub(super) results_mode: bool,
    pub(super) is_next: bool,
    pub(super) lanes: Vec<OverlayLane>,
}

#[derive(Clone, Serialize)]
pub(super) struct OverlayState {
    pub(super) live_enabled: bool,
    pub(super) live: OverlayData,
    pub(super) logos: Vec<String>,
}

pub(super) fn empty_overlay_state() -> OverlayState {
    OverlayState {
        live_enabled: false,
        logos: Vec::new(),
        live: OverlayData {
            event_name: String::new(),
            discipline_name: String::new(),
            discipline: 0,
            heat: 0,
            heat_index: 0,
            heat_count: 0,
            elapsed_ms: None,
            results_mode: false,
            is_next: false,
            lanes: Vec::new(),
        },
    }
}

pub(super) fn start_overlay_server(state: Arc<RwLock<OverlayState>>) {
    thread::spawn(move || {
        let listener = match TcpListener::bind("127.0.0.1:47831") {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("OBS overlay server se nepodařilo spustit: {error}");
                return;
            }
        };
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let state = Arc::clone(&state);
                    thread::spawn(move || serve_overlay_request(stream, &state));
                }
                Err(error) => eprintln!("OBS overlay server: {error}"),
            }
        }
    });
}

fn serve_overlay_request(mut stream: TcpStream, state: &Arc<RwLock<OverlayState>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut request = [0_u8; 2048];
    let Ok(size) = stream.read(&mut request) else {
        return;
    };
    let request = String::from_utf8_lossy(&request[..size]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let (status, content_type, body) = if path.starts_with("/data") {
        let data = state
            .read()
            .ok()
            .and_then(|state| serde_json::to_string(&*state).ok())
            .unwrap_or_else(|| "{}".into());
        (
            "200 OK",
            "application/json; charset=utf-8",
            data.into_bytes(),
        )
    } else if let Some(filename) = path.strip_prefix("/logos/") {
        let safe_name = filename.rsplit('/').next().unwrap_or("");
        let logo_path = logo_directory().join(safe_name);
        match std::fs::read(&logo_path) {
            Ok(bytes) if safe_name == filename && !safe_name.is_empty() => {
                let mime = match logo_path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "webp" => "image/webp",
                    "gif" => "image/gif",
                    "svg" => "image/svg+xml",
                    _ => "application/octet-stream",
                };
                ("200 OK", mime, bytes)
            }
            _ => (
                "404 Not Found",
                "text/plain; charset=utf-8",
                b"Logo nebylo nalezeno".to_vec(),
            ),
        }
    } else if path.starts_with("/live") {
        (
            "200 OK",
            "text/html; charset=utf-8",
            overlay_document().as_bytes().to_vec(),
        )
    } else {
        (
            "404 Not Found",
            "text/plain; charset=utf-8",
            "OBS stránka nebyla nalezena. Použijte /live."
                .as_bytes()
                .to_vec(),
        )
    };
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(&body);
}

pub(super) fn logo_directory() -> std::path::PathBuf {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Library/Application Support")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir)
                    .join(".local/share")
            })
    };
    base.join("Casomira").join("obs-logos")
}

fn overlay_document() -> &'static str {
    static DOCUMENT: OnceLock<String> = OnceLock::new();
    DOCUMENT.get_or_init(|| {
        let styles = [
            include_str!("../assets/obs_overlay/base.css"),
            include_str!("../assets/obs_overlay/layout.css"),
            include_str!("../assets/obs_overlay/prepare.css"),
            include_str!("../assets/obs_overlay/live.css"),
            include_str!("../assets/obs_overlay/results.css"),
            include_str!("../assets/obs_overlay/animation.css"),
        ]
        .join("\n");
        let script = [
            include_str!("../assets/obs_overlay/prepare.js"),
            include_str!("../assets/obs_overlay/live.js"),
            include_str!("../assets/obs_overlay/results.js"),
            include_str!("../assets/obs_overlay/animation.js"),
        ]
        .join("\n");
        include_str!("../assets/obs_overlay.html")
            .replace("/* OBS_CSS */", &styles)
            .replace("<!-- OBS_SCRIPT -->", &script)
    })
}
