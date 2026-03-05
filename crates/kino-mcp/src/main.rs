use serde_json::{json, Value};
use std::io::BufRead;
use std::process::Command;

fn resolve_kino_cli() -> String {
    if let Ok(p) = std::env::var("KINO_CLI_PATH") {
        return p;
    }
    for candidate in &[
        "/usr/local/bin/kino-cli",
        "/opt/homebrew/bin/kino-cli",
    ] {
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let cargo_bin = format!("{home}/.cargo/bin/kino-cli");
        if std::path::Path::new(&cargo_bin).exists() {
            return cargo_bin;
        }
    }
    "kino-cli".into()
}

fn run_kino(args: &[&str]) -> Value {
    let cli = resolve_kino_cli();
    match Command::new(&cli).args(args).output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let text = stdout.trim();
            if out.status.success() {
                if let Ok(v) = serde_json::from_str::<Value>(text) {
                    v
                } else {
                    json!({"output": text})
                }
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                json!({"error": stderr.trim()})
            }
        }
        Err(e) => json!({"error": format!("exec kino-cli: {e}")}),
    }
}

fn tool_definitions() -> Value {
    json!([
        {"name": "analyze_stream", "description": "Parse HLS/DASH manifest — renditions, codecs, duration, live status",
         "inputSchema": {"type": "object", "properties": {"url": {"type": "string", "description": "HLS or DASH manifest URL"}}, "required": ["url"]}},
        {"name": "validate_stream", "description": "Check segment accessibility and bitrate conformance",
         "inputSchema": {"type": "object", "properties": {"url": {"type": "string"}, "segments": {"type": "integer", "description": "Segments to validate (default: all)"}}, "required": ["url"]}},
        {"name": "quality_check", "description": "Full QC report — DRM, captions, bitrate ladder",
         "inputSchema": {"type": "object", "properties": {"url": {"type": "string"}, "output_format": {"type": "string", "enum": ["json", "text"], "default": "json"}}, "required": ["url"]}},
        {"name": "monitor_stream", "description": "Live stream health — latency, segment freshness, error rate",
         "inputSchema": {"type": "object", "properties": {"url": {"type": "string"}, "interval": {"type": "integer", "default": 5}}, "required": ["url"]}},
        {"name": "fingerprint_audio", "description": "Audio fingerprint for content ID and dedup",
         "inputSchema": {"type": "object", "properties": {"file_path": {"type": "string"}}, "required": ["file_path"]}},
        {"name": "autotag_content", "description": "Auto-detect genre, mood, BPM from audio",
         "inputSchema": {"type": "object", "properties": {"file_path": {"type": "string"}}, "required": ["file_path"]}},
        {"name": "compare_streams", "description": "Compare two streams — resolution, bitrate, codec differences",
         "inputSchema": {"type": "object", "properties": {"url_a": {"type": "string"}, "url_b": {"type": "string"}}, "required": ["url_a", "url_b"]}},
        {"name": "encode_video", "description": "Encoding presets for target platform — ffmpeg flags and HLS/DASH config",
         "inputSchema": {"type": "object", "properties": {"preset": {"type": "string", "enum": ["mobile", "desktop", "4k", "low-bandwidth"]}}, "required": ["preset"]}}
    ])
}

fn call_tool(name: &str, args: &Value) -> Value {
    match name {
        "analyze_stream" => {
            let url = args["url"].as_str().unwrap_or_default();
            run_kino(&["analyze", url])
        }
        "validate_stream" => {
            let url = args["url"].as_str().unwrap_or_default();
            if let Some(n) = args["segments"].as_i64() {
                run_kino(&["validate", url, "--segments", &n.to_string()])
            } else {
                run_kino(&["validate", url])
            }
        }
        "quality_check" => {
            let url = args["url"].as_str().unwrap_or_default();
            let fmt = args["output_format"].as_str().unwrap_or("json");
            run_kino(&["qc", url, "--output-format", fmt])
        }
        "monitor_stream" => {
            let url = args["url"].as_str().unwrap_or_default();
            let interval = args["interval"].as_i64().unwrap_or(5);
            run_kino(&["monitor", url, "--interval", &interval.to_string(), "--count", "1"])
        }
        "fingerprint_audio" => {
            let path = args["file_path"].as_str().unwrap_or_default();
            run_kino(&["fingerprint", path])
        }
        "autotag_content" => {
            let path = args["file_path"].as_str().unwrap_or_default();
            run_kino(&["autotag", path])
        }
        "compare_streams" => {
            let a = args["url_a"].as_str().unwrap_or_default();
            let b = args["url_b"].as_str().unwrap_or_default();
            run_kino(&["compare", a, b])
        }
        "encode_video" => {
            let preset = args["preset"].as_str().unwrap_or("desktop");
            run_kino(&["preset", preset])
        }
        _ => json!({"error": format!("unknown tool: {name}")}),
    }
}

fn handle(req: &Value) -> Value {
    let id = &req["id"];
    let method = req["method"].as_str().unwrap_or_default();
    match method {
        "initialize" => json!({"jsonrpc": "2.0", "id": id,
            "result": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}},
                "serverInfo": {"name": "kino-mcp", "version": "0.1.0"}}}),
        "notifications/initialized" | "initialized" => return Value::Null,
        "tools/list" => json!({"jsonrpc": "2.0", "id": id, "result": {"tools": tool_definitions()}}),
        "tools/call" => {
            let name = req["params"]["name"].as_str().unwrap_or_default();
            let args = req["params"].get("arguments").cloned().unwrap_or(json!({}));
            let result = call_tool(name, &args);
            let text = if result.get("error").is_some() {
                serde_json::to_string_pretty(&result).unwrap_or_default()
            } else {
                serde_json::to_string_pretty(&result).unwrap_or_default()
            };
            let is_err = result.get("error").is_some();
            json!({"jsonrpc": "2.0", "id": id, "result": {
                "content": [{"type": "text", "text": text}],
                "isError": is_err
            }})
        }
        _ => json!({"jsonrpc": "2.0", "id": id,
            "error": {"code": -32601, "message": format!("unknown method: {method}")}}),
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut line = String::new();
    while stdin.lock().read_line(&mut line).unwrap_or(0) > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }
        if let Ok(req) = serde_json::from_str::<Value>(trimmed) {
            let resp = handle(&req);
            if !resp.is_null() {
                println!("{}", serde_json::to_string(&resp).unwrap());
            }
        }
        line.clear();
    }
}
