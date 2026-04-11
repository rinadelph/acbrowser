use std::sync::OnceLock;

use crate::color;
use crate::connection::Response;

static BOUNDARY_NONCE: OnceLock<String> = OnceLock::new();

/// Per-process nonce for content boundary markers. Uses a CSPRNG (getrandom) so
/// that untrusted page content cannot predict or spoof the boundary delimiter.
/// Process ID or timestamps would be insufficient since pages can read those.
fn get_boundary_nonce() -> &'static str {
    BOUNDARY_NONCE.get_or_init(|| {
        let mut buf = [0u8; 16];
        getrandom::getrandom(&mut buf).expect("failed to generate random nonce");
        buf.iter().map(|b| format!("{:02x}", b)).collect()
    })
}

#[derive(Default)]
pub struct OutputOptions {
    pub json: bool,
    pub content_boundaries: bool,
    pub max_output: Option<usize>,
}

impl OutputOptions {
    pub fn from_flags(flags: &crate::flags::Flags) -> Self {
        Self {
            json: flags.json,
            content_boundaries: flags.content_boundaries,
            max_output: flags.max_output,
        }
    }
}

fn truncate_if_needed(content: &str, max: Option<usize>) -> String {
    let Some(limit) = max else {
        return content.to_string();
    };
    // Fast path: byte length is a lower bound on char count, so if the
    // byte length is within the limit the char count must be too.
    if content.len() <= limit {
        return content.to_string();
    }
    // Find the byte offset of the limit-th character.
    match content.char_indices().nth(limit).map(|(i, _)| i) {
        Some(byte_offset) => {
            let total_chars = content.chars().count();
            format!(
                "{}\n[truncated: showing {} of {} chars. Use --max-output to adjust]",
                &content[..byte_offset],
                limit,
                total_chars
            )
        }
        // Content has fewer than `limit` chars despite more bytes
        None => content.to_string(),
    }
}

fn print_with_boundaries(content: &str, origin: Option<&str>, opts: &OutputOptions) {
    let content = truncate_if_needed(content, opts.max_output);
    if opts.content_boundaries {
        let origin_str = origin.unwrap_or("unknown");
        let nonce = get_boundary_nonce();
        println!(
            "--- AGENT_BROWSER_PAGE_CONTENT nonce={} origin={} ---",
            nonce, origin_str
        );
        println!("{}", content);
        println!("--- END_AGENT_BROWSER_PAGE_CONTENT nonce={} ---", nonce);
    } else {
        println!("{}", content);
    }
}

fn format_storage_value(value: &serde_json::Value) -> String {
    value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| serde_json::to_string(value).unwrap_or_default())
}

fn format_storage_text(data: &serde_json::Value) -> Option<String> {
    if let Some(entries) = data.get("data").and_then(|v| v.as_object()) {
        if entries.is_empty() {
            return Some("No storage entries".to_string());
        }

        let lines = entries
            .iter()
            .map(|(key, value)| format!("{}: {}", key, format_storage_value(value)))
            .collect::<Vec<_>>();
        return Some(lines.join("\n"));
    }

    let key = data.get("key").and_then(|v| v.as_str())?;
    let value = data.get("value")?;
    Some(format!("{}: {}", key, format_storage_value(value)))
}

fn format_stream_status_text(action: Option<&str>, data: &serde_json::Value) -> Option<String> {
    match action {
        Some("stream_disable") => data
            .get("disabled")
            .and_then(|v| v.as_bool())
            .filter(|disabled| *disabled)
            .map(|_| "Streaming disabled".to_string()),
        Some("stream_enable") | Some("stream_status") => {
            let enabled = data.get("enabled").and_then(|v| v.as_bool())?;
            if !enabled {
                return Some("Streaming disabled".to_string());
            }

            let port = data.get("port").and_then(|v| v.as_u64())?;
            let connected = data
                .get("connected")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let screencasting = data
                .get("screencasting")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            Some(format!(
                "Streaming enabled on ws://127.0.0.1:{port}\nConnected: {connected}\nScreencasting: {screencasting}"
            ))
        }
        _ => None,
    }
}

pub fn print_response_with_opts(resp: &Response, action: Option<&str>, opts: &OutputOptions) {
    if opts.json {
        if opts.content_boundaries {
            let mut json_val = serde_json::to_value(resp).unwrap_or_default();
            if let Some(obj) = json_val.as_object_mut() {
                let nonce = get_boundary_nonce();
                let origin = obj
                    .get("data")
                    .and_then(|d| d.get("origin"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                obj.insert(
                    "_boundary".to_string(),
                    serde_json::json!({
                        "nonce": nonce,
                        "origin": origin,
                    }),
                );
            }
            println!("{}", serde_json::to_string(&json_val).unwrap_or_default());
        } else {
            println!("{}", serde_json::to_string(resp).unwrap_or_default());
        }
        // JSON mode includes the warning field in the JSON payload already
        return;
    }

    if !resp.success {
        eprintln!(
            "{} {}",
            color::error_indicator(),
            resp.error.as_deref().unwrap_or("Unknown error")
        );
        // Still print dialog warning after errors, since a pending dialog
        // is the most common cause of commands timing out
        if let Some(ref warning) = resp.warning {
            eprintln!("{} {}", color::warning_indicator(), warning);
        }
        return;
    }

    if let Some(data) = &resp.data {
        // Dialog status response
        if action == Some("dialog") {
            if let Some(has_dialog) = data.get("hasDialog").and_then(|v| v.as_bool()) {
                if has_dialog {
                    let dtype = data
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let message = data.get("message").and_then(|v| v.as_str()).unwrap_or("");
                    println!(
                        "{} JavaScript {} dialog is open: \"{}\"",
                        color::warning_indicator(),
                        dtype,
                        message
                    );
                    if let Some(default_prompt) = data.get("defaultPrompt").and_then(|v| v.as_str())
                    {
                        println!("  Default prompt text: \"{}\"", default_prompt);
                    }
                    println!("  Use `dialog accept [text]` or `dialog dismiss` to resolve it");
                } else {
                    println!("{} No dialog is currently open", color::success_indicator());
                }
                print_warning(resp);
                return;
            }
        }
        if let Some(output) = format_stream_status_text(action, data) {
            println!("{}", output);
            return;
        }
        if action == Some("storage_get") {
            if let Some(output) = format_storage_text(data) {
                println!("{}", output);
                return;
            }
        }
        // Inspect response (check before generic URL handler since it also has a "url" field)
        if action == Some("inspect") {
            let opened = data
                .get("opened")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if opened {
                if let Some(url) = data.get("url").and_then(|v| v.as_str()) {
                    println!("{} Opened DevTools: {}", color::success_indicator(), url);
                } else {
                    println!("{} Opened DevTools", color::success_indicator());
                }
            } else if let Some(err) = data.get("error").and_then(|v| v.as_str()) {
                eprintln!("Could not open DevTools: {}", err);
            }
            return;
        }
        // Navigation response
        if let Some(url) = data.get("url").and_then(|v| v.as_str()) {
            if let Some(title) = data.get("title").and_then(|v| v.as_str()) {
                println!("{} {}", color::success_indicator(), color::bold(title));
                println!("  {}", color::dim(url));
                return;
            }
            println!("{}", url);
            return;
        }
        if let Some(cdp_url) = data.get("cdpUrl").and_then(|v| v.as_str()) {
            println!("{}", cdp_url);
            return;
        }
        // Diff responses -- route by action to avoid fragile shape probing
        if let Some(obj) = data.as_object() {
            match action {
                Some("diff_snapshot") => {
                    print_snapshot_diff(obj);
                    return;
                }
                Some("diff_screenshot") => {
                    print_screenshot_diff(obj);
                    return;
                }
                Some("diff_url") => {
                    if let Some(snap_data) = obj.get("snapshot").and_then(|v| v.as_object()) {
                        println!("{}", color::bold("Snapshot diff:"));
                        print_snapshot_diff(snap_data);
                    }
                    if let Some(ss_data) = obj.get("screenshot").and_then(|v| v.as_object()) {
                        println!("\n{}", color::bold("Screenshot diff:"));
                        print_screenshot_diff(ss_data);
                    }
                    return;
                }
                _ => {}
            }
        }
        let origin = data.get("origin").and_then(|v| v.as_str());
        // Snapshot
        if let Some(snapshot) = data.get("snapshot").and_then(|v| v.as_str()) {
            print_with_boundaries(snapshot, origin, opts);
            return;
        }
        // Title
        if let Some(title) = data.get("title").and_then(|v| v.as_str()) {
            println!("{}", title);
            return;
        }
        // Text
        if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
            print_with_boundaries(text, origin, opts);
            return;
        }
        // HTML
        if let Some(html) = data.get("html").and_then(|v| v.as_str()) {
            print_with_boundaries(html, origin, opts);
            return;
        }
        // Value
        if let Some(value) = data.get("value").and_then(|v| v.as_str()) {
            println!("{}", value);
            return;
        }
        // Count
        if let Some(count) = data.get("count").and_then(|v| v.as_i64()) {
            println!("{}", count);
            return;
        }
        // Boolean results
        if let Some(visible) = data.get("visible").and_then(|v| v.as_bool()) {
            println!("{}", visible);
            return;
        }
        if let Some(enabled) = data.get("enabled").and_then(|v| v.as_bool()) {
            println!("{}", enabled);
            return;
        }
        if let Some(checked) = data.get("checked").and_then(|v| v.as_bool()) {
            println!("{}", checked);
            return;
        }
        // Eval result
        if let Some(result) = data.get("result") {
            let formatted = serde_json::to_string_pretty(result).unwrap_or_default();
            print_with_boundaries(&formatted, origin, opts);
            return;
        }
        // iOS Devices
        if let Some(devices) = data.get("devices").and_then(|v| v.as_array()) {
            if devices.is_empty() {
                println!("No iOS devices available. Open Xcode to download simulator runtimes.");
                return;
            }

            // Separate real devices from simulators
            let real_devices: Vec<_> = devices
                .iter()
                .filter(|d| {
                    d.get("isRealDevice")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                })
                .collect();
            let simulators: Vec<_> = devices
                .iter()
                .filter(|d| {
                    !d.get("isRealDevice")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                })
                .collect();

            if !real_devices.is_empty() {
                println!("Connected Devices:\n");
                for device in real_devices.iter() {
                    let name = device
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let runtime = device.get("runtime").and_then(|v| v.as_str()).unwrap_or("");
                    let udid = device.get("udid").and_then(|v| v.as_str()).unwrap_or("");
                    println!("  {} {} ({})", color::green("●"), name, runtime);
                    println!("    {}", color::dim(udid));
                }
                println!();
            }

            if !simulators.is_empty() {
                println!("Simulators:\n");
                for device in simulators.iter() {
                    let name = device
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let runtime = device.get("runtime").and_then(|v| v.as_str()).unwrap_or("");
                    let state = device
                        .get("state")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let udid = device.get("udid").and_then(|v| v.as_str()).unwrap_or("");
                    let state_indicator = if state == "Booted" {
                        color::green("●")
                    } else {
                        color::dim("○")
                    };
                    println!("  {} {} ({})", state_indicator, name, runtime);
                    println!("    {}", color::dim(udid));
                }
            }
            return;
        }
        // Tabs
        if let Some(tabs) = data.get("tabs").and_then(|v| v.as_array()) {
            for (i, tab) in tabs.iter().enumerate() {
                let title = tab
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled");
                let url = tab.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let active = tab.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
                let marker = if active {
                    color::cyan("→")
                } else {
                    " ".to_string()
                };
                println!("{} [{}] {} - {}", marker, i, title, url);
            }
            return;
        }
        // Console logs
        if let Some(logs) = data.get("messages").and_then(|v| v.as_array()) {
            if opts.content_boundaries {
                let mut console_output = String::new();
                for log in logs {
                    let level = log.get("type").and_then(|v| v.as_str()).unwrap_or("log");
                    let text = log.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    console_output.push_str(&format!(
                        "{} {}\n",
                        color::console_level_prefix(level),
                        text
                    ));
                }
                if console_output.ends_with('\n') {
                    console_output.pop();
                }
                print_with_boundaries(&console_output, origin, opts);
            } else {
                for log in logs {
                    let level = log.get("type").and_then(|v| v.as_str()).unwrap_or("log");
                    let text = log.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    println!("{} {}", color::console_level_prefix(level), text);
                }
            }
            return;
        }
        // Errors
        if let Some(errors) = data.get("errors").and_then(|v| v.as_array()) {
            for err in errors {
                let msg = err.get("message").and_then(|v| v.as_str()).unwrap_or("");
                println!("{} {}", color::error_indicator(), msg);
            }
            return;
        }
        // Cookies
        if let Some(cookies) = data.get("cookies").and_then(|v| v.as_array()) {
            for cookie in cookies {
                let name = cookie.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let value = cookie.get("value").and_then(|v| v.as_str()).unwrap_or("");
                println!("{}={}", name, value);
            }
            return;
        }
        // Network requests
        if let Some(requests) = data.get("requests").and_then(|v| v.as_array()) {
            if requests.is_empty() {
                println!("No requests captured");
            } else {
                for req in requests {
                    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
                    let url = req.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let resource_type = req
                        .get("resourceType")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let request_id = req.get("requestId").and_then(|v| v.as_str()).unwrap_or("");
                    let status = req.get("status").and_then(|v| v.as_i64());
                    match status {
                        Some(s) => println!(
                            "[{}] {} {} ({}) {}",
                            request_id, method, url, resource_type, s
                        ),
                        None => println!("[{}] {} {} ({})", request_id, method, url, resource_type),
                    }
                }
            }
            return;
        }
        // Cleared (cookies, console, or request log)
        if let Some(cleared) = data.get("cleared").and_then(|v| v.as_bool()) {
            if cleared {
                let label = match action {
                    Some("cookies_clear") => "Cookies cleared",
                    Some("console") => "Console log cleared",
                    _ => "Request log cleared",
                };
                println!("{} {}", color::success_indicator(), label);
                return;
            }
        }
        // Bounding box
        if let Some(box_data) = data.get("box") {
            println!(
                "{}",
                serde_json::to_string_pretty(box_data).unwrap_or_default()
            );
            return;
        }
        // Element styles
        if let Some(elements) = data.get("elements").and_then(|v| v.as_array()) {
            for (i, el) in elements.iter().enumerate() {
                let tag = el.get("tag").and_then(|v| v.as_str()).unwrap_or("?");
                let text = el.get("text").and_then(|v| v.as_str()).unwrap_or("");
                println!("[{}] {} \"{}\"", i, tag, text);

                if let Some(box_data) = el.get("box") {
                    let w = box_data.get("width").and_then(|v| v.as_i64()).unwrap_or(0);
                    let h = box_data.get("height").and_then(|v| v.as_i64()).unwrap_or(0);
                    let x = box_data.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
                    let y = box_data.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
                    println!("    box: {}x{} at ({}, {})", w, h, x, y);
                }

                if let Some(styles) = el.get("styles") {
                    let font_size = styles
                        .get("fontSize")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let font_weight = styles
                        .get("fontWeight")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let font_family = styles
                        .get("fontFamily")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let color = styles.get("color").and_then(|v| v.as_str()).unwrap_or("");
                    let bg = styles
                        .get("backgroundColor")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let radius = styles
                        .get("borderRadius")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    println!("    font: {} {} {}", font_size, font_weight, font_family);
                    println!("    color: {}", color);
                    println!("    background: {}", bg);
                    if radius != "0px" {
                        println!("    border-radius: {}", radius);
                    }
                }
                println!();
            }
            return;
        }
        // Closed (browser or tab)
        if data.get("closed").is_some() {
            let label = match action {
                Some("tab_close") => "Tab closed",
                _ => "Browser closed",
            };
            println!("{} {}", color::success_indicator(), label);
            return;
        }
        // Started actions (profiling, HAR, recording)
        if let Some(started) = data.get("started").and_then(|v| v.as_bool()) {
            if started {
                match action {
                    Some("profiler_start") => {
                        println!("{} Profiling started", color::success_indicator());
                    }
                    Some("har_start") => {
                        println!("{} HAR recording started", color::success_indicator());
                    }
                    _ => {
                        if let Some(path) = data.get("path").and_then(|v| v.as_str()) {
                            println!("{} Recording started: {}", color::success_indicator(), path);
                        } else {
                            println!("{} Recording started", color::success_indicator());
                        }
                    }
                }
                return;
            }
        }
        // Recording restart (has "stopped" field - from recording_restart action)
        if data.get("stopped").is_some() {
            let path = data
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            if let Some(prev_path) = data.get("previousPath").and_then(|v| v.as_str()) {
                println!(
                    "{} Recording restarted: {} (previous saved to {})",
                    color::success_indicator(),
                    path,
                    prev_path
                );
            } else {
                println!("{} Recording started: {}", color::success_indicator(), path);
            }
            return;
        }
        // Recording stop (has "frames" field - from recording_stop action)
        if data.get("frames").is_some() {
            if let Some(path) = data.get("path").and_then(|v| v.as_str()) {
                if let Some(error) = data.get("error").and_then(|v| v.as_str()) {
                    println!(
                        "{} Recording saved to {} - {}",
                        color::warning_indicator(),
                        path,
                        error
                    );
                } else {
                    println!("{} Recording saved to {}", color::success_indicator(), path);
                }
            } else {
                println!("{} Recording stopped", color::success_indicator());
            }
            return;
        }
        // Download response (has "suggestedFilename" or "filename" field)
        if data.get("suggestedFilename").is_some() || data.get("filename").is_some() {
            if let Some(path) = data.get("path").and_then(|v| v.as_str()) {
                let filename = data
                    .get("suggestedFilename")
                    .or_else(|| data.get("filename"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if filename.is_empty() {
                    println!(
                        "{} Downloaded to {}",
                        color::success_indicator(),
                        color::green(path)
                    );
                } else {
                    println!(
                        "{} Downloaded to {} ({})",
                        color::success_indicator(),
                        color::green(path),
                        filename
                    );
                }
                return;
            }
        }
        // Trace stop without path
        if data.get("traceStopped").is_some() {
            println!("{} Trace stopped", color::success_indicator());
            return;
        }
        // Path-based operations (screenshot/pdf/trace/har/download/state/video)
        if let Some(path) = data.get("path").and_then(|v| v.as_str()) {
            match action.unwrap_or("") {
                "screenshot" => {
                    println!(
                        "{} Screenshot saved to {}",
                        color::success_indicator(),
                        color::green(path)
                    );
                    if let Some(annotations) = data.get("annotations").and_then(|v| v.as_array()) {
                        for ann in annotations {
                            let num = ann.get("number").and_then(|n| n.as_u64()).unwrap_or(0);
                            let ref_id = ann.get("ref").and_then(|r| r.as_str()).unwrap_or("");
                            let role = ann.get("role").and_then(|r| r.as_str()).unwrap_or("");
                            let name = ann.get("name").and_then(|n| n.as_str()).unwrap_or("");
                            if name.is_empty() {
                                println!(
                                    "   {} @{} {}",
                                    color::dim(&format!("[{}]", num)),
                                    ref_id,
                                    role,
                                );
                            } else {
                                println!(
                                    "   {} @{} {} {:?}",
                                    color::dim(&format!("[{}]", num)),
                                    ref_id,
                                    role,
                                    name,
                                );
                            }
                        }
                    }
                }
                "pdf" => println!(
                    "{} PDF saved to {}",
                    color::success_indicator(),
                    color::green(path)
                ),
                "trace_stop" => println!(
                    "{} Trace saved to {}",
                    color::success_indicator(),
                    color::green(path)
                ),
                "profiler_stop" => println!(
                    "{} Profile saved to {} ({} events)",
                    color::success_indicator(),
                    color::green(path),
                    data.get("eventCount").and_then(|c| c.as_u64()).unwrap_or(0)
                ),
                "har_stop" => println!(
                    "{} HAR saved to {} ({} requests)",
                    color::success_indicator(),
                    color::green(path),
                    data.get("requestCount")
                        .and_then(|c| c.as_u64())
                        .unwrap_or(0)
                ),
                "download" | "waitfordownload" => println!(
                    "{} Download saved to {}",
                    color::success_indicator(),
                    color::green(path)
                ),
                "video_stop" => println!(
                    "{} Video saved to {}",
                    color::success_indicator(),
                    color::green(path)
                ),
                "state_save" => println!(
                    "{} State saved to {}",
                    color::success_indicator(),
                    color::green(path)
                ),
                "state_load" => {
                    if let Some(note) = data.get("note").and_then(|v| v.as_str()) {
                        println!("{}", note);
                    }
                    println!(
                        "{} State path set to {}",
                        color::success_indicator(),
                        color::green(path)
                    );
                }
                // video_start and other commands that provide a path with a note
                "video_start" => {
                    if let Some(note) = data.get("note").and_then(|v| v.as_str()) {
                        println!("{}", note);
                    }
                    println!("Path: {}", path);
                }
                _ => println!(
                    "{} Saved to {}",
                    color::success_indicator(),
                    color::green(path)
                ),
            }
            return;
        }

        // State list
        if let Some(files) = data.get("files").and_then(|v| v.as_array()) {
            if let Some(dir) = data.get("directory").and_then(|v| v.as_str()) {
                println!("{}", color::bold(&format!("Saved states in {}", dir)));
            }
            if files.is_empty() {
                println!("{}", color::dim("  No state files found"));
            } else {
                for file in files {
                    let filename = file.get("filename").and_then(|v| v.as_str()).unwrap_or("");
                    let size = file.get("size").and_then(|v| v.as_i64()).unwrap_or(0);
                    let modified = file.get("modified").and_then(|v| v.as_str()).unwrap_or("");
                    let encrypted = file
                        .get("encrypted")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let size_str = if size > 1024 {
                        format!("{:.1}KB", size as f64 / 1024.0)
                    } else {
                        format!("{}B", size)
                    };
                    let date_str = modified.split('T').next().unwrap_or(modified);
                    let enc_str = if encrypted { " [encrypted]" } else { "" };
                    println!(
                        "  {} {}",
                        filename,
                        color::dim(&format!("({}, {}){}", size_str, date_str, enc_str))
                    );
                }
            }
            return;
        }

        // State rename
        if let Some(true) = data.get("renamed").and_then(|v| v.as_bool()) {
            let old_name = data.get("oldName").and_then(|v| v.as_str()).unwrap_or("");
            let new_name = data.get("newName").and_then(|v| v.as_str()).unwrap_or("");
            println!(
                "{} Renamed {} -> {}",
                color::success_indicator(),
                old_name,
                new_name
            );
            return;
        }

        // State clear
        if let Some(cleared) = data.get("cleared").and_then(|v| v.as_i64()) {
            println!(
                "{} Cleared {} state file(s)",
                color::success_indicator(),
                cleared
            );
            return;
        }

        // State show summary
        if let Some(summary) = data.get("summary") {
            let cookies = summary.get("cookies").and_then(|v| v.as_i64()).unwrap_or(0);
            let origins = summary.get("origins").and_then(|v| v.as_i64()).unwrap_or(0);
            let encrypted = data
                .get("encrypted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let enc_str = if encrypted { " (encrypted)" } else { "" };
            println!("State file summary{}:", enc_str);
            println!("  Cookies: {}", cookies);
            println!("  Origins with localStorage: {}", origins);
            return;
        }

        // State clean
        if let Some(cleaned) = data.get("cleaned").and_then(|v| v.as_i64()) {
            println!(
                "{} Cleaned {} old state file(s)",
                color::success_indicator(),
                cleaned
            );
            return;
        }

        // Informational note
        if let Some(note) = data.get("note").and_then(|v| v.as_str()) {
            println!("{}", note);
            return;
        }
        // Auth list
        if let Some(profiles) = data.get("profiles").and_then(|v| v.as_array()) {
            if profiles.is_empty() {
                println!("{}", color::dim("No auth profiles saved"));
            } else {
                println!("{}", color::bold("Auth profiles:"));
                for p in profiles {
                    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let url = p.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let user = p.get("username").and_then(|v| v.as_str()).unwrap_or("");
                    println!(
                        "  {} {} {}",
                        color::green(name),
                        color::dim(user),
                        color::dim(url)
                    );
                }
            }
            return;
        }

        // Auth show
        if let Some(profile) = data.get("profile").and_then(|v| v.as_object()) {
            let name = profile.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let url = profile.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let user = profile
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let created = profile
                .get("createdAt")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let last_login = profile.get("lastLoginAt").and_then(|v| v.as_str());
            println!("Name: {}", name);
            println!("URL: {}", url);
            println!("Username: {}", user);
            println!("Created: {}", created);
            if let Some(ll) = last_login {
                println!("Last login: {}", ll);
            }
            return;
        }

        // Auth save/update/login/delete
        if data.get("saved").and_then(|v| v.as_bool()).unwrap_or(false) {
            let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("");
            println!(
                "{} Auth profile '{}' saved",
                color::success_indicator(),
                name
            );
            return;
        }
        if data
            .get("updated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            && !data.get("saved").and_then(|v| v.as_bool()).unwrap_or(false)
        {
            let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("");
            println!(
                "{} Auth profile '{}' updated",
                color::success_indicator(),
                name
            );
            return;
        }
        if data
            .get("loggedIn")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(title) = data.get("title").and_then(|v| v.as_str()) {
                println!(
                    "{} Logged in as '{}' - {}",
                    color::success_indicator(),
                    name,
                    title
                );
            } else {
                println!("{} Logged in as '{}'", color::success_indicator(), name);
            }
            return;
        }
        if data
            .get("deleted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
                println!(
                    "{} Auth profile '{}' deleted",
                    color::success_indicator(),
                    name
                );
                return;
            }
        }

        // Confirmation required (for orchestrator use)
        if data
            .get("confirmation_required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let category = data.get("category").and_then(|v| v.as_str()).unwrap_or("");
            let description = data
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let cid = data
                .get("confirmation_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!("Confirmation required:");
            println!("  {}: {}", category, description);
            println!("  Run: acbrowser confirm {}", cid);
            println!("  Or:  acbrowser deny {}", cid);
            return;
        }
        if data
            .get("confirmed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            println!("{} Action confirmed", color::success_indicator());
            return;
        }
        if data
            .get("denied")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            println!("{} Action denied", color::success_indicator());
            return;
        }

        // Default success
        println!("{} Done", color::success_indicator());
    }

    print_warning(resp);
}

fn print_warning(resp: &Response) {
    if let Some(ref warning) = resp.warning {
        eprintln!("{} {}", color::warning_indicator(), warning);
    }
}

/// Print command-specific help. Returns true if help was printed, false if command unknown.
pub fn print_command_help(command: &str) -> bool {
    let help = match command {
        // === Navigation ===
        "open" | "goto" | "navigate" => {
            r##"
acbrowser open - Navigate to a URL

Usage: acbrowser open <url>

Navigates the browser to the specified URL. If no protocol is provided,
https:// is automatically prepended.

Aliases: goto, navigate

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session
  --headers <json>     Set HTTP headers (scoped to this origin)
  --headed             Show browser window

Examples:
  acbrowser open example.com
  acbrowser open https://github.com
  acbrowser open localhost:3000
  acbrowser open api.example.com --headers '{"Authorization": "Bearer token"}'
    # ^ Headers only sent to api.example.com, not other domains
"##
        }
        "back" => {
            r##"
acbrowser back - Navigate back in history

Usage: acbrowser back

Goes back one page in the browser history, equivalent to clicking
the browser's back button.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser back
"##
        }
        "forward" => {
            r##"
acbrowser forward - Navigate forward in history

Usage: acbrowser forward

Goes forward one page in the browser history, equivalent to clicking
the browser's forward button.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser forward
"##
        }
        "reload" => {
            r##"
acbrowser reload - Reload the current page

Usage: acbrowser reload

Reloads the current page, equivalent to pressing F5 or clicking
the browser's reload button.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser reload
"##
        }

        // === Core Actions ===
        "click" => {
            r##"
acbrowser click - Click an element

Usage: acbrowser click <selector> [--new-tab]

Clicks on the specified element. The selector can be a CSS selector,
XPath, or an element reference from snapshot (e.g., @e1).

Options:
  --new-tab            Open link in a new tab instead of navigating current tab
                       (only works on elements with href attribute)

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser click "#submit-button"
  acbrowser click @e1
  acbrowser click "button.primary"
  acbrowser click "//button[@type='submit']"
  acbrowser click @e3 --new-tab
"##
        }
        "dblclick" => {
            r##"
acbrowser dblclick - Double-click an element

Usage: acbrowser dblclick <selector>

Double-clicks on the specified element. Useful for text selection
or triggering double-click handlers.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser dblclick "#editable-text"
  acbrowser dblclick @e5
"##
        }
        "fill" => {
            r##"
acbrowser fill - Clear and fill an input field

Usage: acbrowser fill <selector> <text>

Clears the input field and fills it with the specified text.
This replaces any existing content in the field.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser fill "#email" "user@example.com"
  acbrowser fill @e3 "Hello World"
  acbrowser fill "input[name='search']" "query"
"##
        }
        "type" => {
            r##"
acbrowser type - Type text into an element

Usage: acbrowser type <selector> <text>

Types text into the specified element character by character.
Unlike fill, this does not clear existing content first.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser type "#search" "hello"
  acbrowser type @e2 "additional text"

See Also:
  For typing into contenteditable editors (Lexical, ProseMirror, etc.)
  without a selector, use 'keyboard type' instead:
    acbrowser keyboard type "# My Heading"
"##
        }
        "hover" => {
            r##"
acbrowser hover - Hover over an element

Usage: acbrowser hover <selector>

Moves the mouse to hover over the specified element. Useful for
triggering hover states or dropdown menus.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser hover "#dropdown-trigger"
  acbrowser hover @e4
"##
        }
        "focus" => {
            r##"
acbrowser focus - Focus an element

Usage: acbrowser focus <selector>

Sets keyboard focus to the specified element.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser focus "#input-field"
  acbrowser focus @e2
"##
        }
        "check" => {
            r##"
acbrowser check - Check a checkbox

Usage: acbrowser check <selector>

Checks a checkbox element. If already checked, no action is taken.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser check "#terms-checkbox"
  acbrowser check @e7
"##
        }
        "uncheck" => {
            r##"
acbrowser uncheck - Uncheck a checkbox

Usage: acbrowser uncheck <selector>

Unchecks a checkbox element. If already unchecked, no action is taken.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser uncheck "#newsletter-opt-in"
  acbrowser uncheck @e8
"##
        }
        "select" => {
            r##"
acbrowser select - Select a dropdown option

Usage: acbrowser select <selector> <value...>

Selects one or more options in a <select> dropdown by value.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser select "#country" "US"
  acbrowser select @e5 "option2"
  acbrowser select "#menu" "opt1" "opt2" "opt3"
"##
        }
        "drag" => {
            r##"
acbrowser drag - Drag and drop

Usage: acbrowser drag <source> <target>

Drags an element from source to target location.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser drag "#draggable" "#drop-zone"
  acbrowser drag @e1 @e2
"##
        }
        "upload" => {
            r##"
acbrowser upload - Upload files

Usage: acbrowser upload <selector> <files...>

Uploads one or more files to a file input element.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser upload "#file-input" ./document.pdf
  acbrowser upload @e3 ./image1.png ./image2.png
"##
        }
        "download" => {
            r##"
acbrowser download - Download a file by clicking an element

Usage: acbrowser download <selector> <path>

Clicks an element that triggers a download and saves the file to the specified path.

Arguments:
  selector             Element to click (CSS selector or @ref)
  path                 Path where the downloaded file will be saved

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser download "#download-btn" ./file.pdf
  acbrowser download @e5 ./report.xlsx
  acbrowser download "a[href$='.zip']" ./archive.zip
"##
        }

        // === Keyboard ===
        "press" | "key" => {
            r##"
acbrowser press - Press a key or key combination

Usage: acbrowser press <key>

Presses a key or key combination. Supports special keys and modifiers.

Aliases: key

Special Keys:
  Enter, Tab, Escape, Backspace, Delete, Space
  ArrowUp, ArrowDown, ArrowLeft, ArrowRight
  Home, End, PageUp, PageDown
  F1-F12

Modifiers (combine with +):
  Control, Alt, Shift, Meta

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser press Enter
  acbrowser press Tab
  acbrowser press Control+a
  acbrowser press Control+Shift+s
  acbrowser press Escape
"##
        }
        "keydown" => {
            r##"
acbrowser keydown - Press a key down (without release)

Usage: acbrowser keydown <key>

Presses a key down without releasing it. Use keyup to release.
Useful for holding modifier keys.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser keydown Shift
  acbrowser keydown Control
"##
        }
        "keyup" => {
            r##"
acbrowser keyup - Release a key

Usage: acbrowser keyup <key>

Releases a key that was pressed with keydown.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser keyup Shift
  acbrowser keyup Control
"##
        }
        "keyboard" => {
            r##"
acbrowser keyboard - Raw keyboard input (no selector needed)

Usage: acbrowser keyboard <subcommand> <text>

Sends keyboard input to whatever element currently has focus.
Unlike 'type' which requires a selector, 'keyboard' operates on
the current focus — essential for contenteditable editors like
Lexical, ProseMirror, CodeMirror, and Monaco.

Subcommands:
  type <text>          Type text character-by-character with real
                       key events (keydown, keypress, keyup per char)
  inserttext <text>    Insert text without key events (like paste)

Note: For key combos (Enter, Control+a), use the 'press' command
directly — it already operates on the current focus.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser keyboard type "Hello, World!"
  acbrowser keyboard type "# My Heading"
  acbrowser keyboard inserttext "pasted content"

Use Cases:
  # Type into a Lexical/ProseMirror contenteditable editor:
  acbrowser click "[contenteditable]"
  acbrowser keyboard type "# My Heading"
  acbrowser press Enter
  acbrowser keyboard type "Some paragraph text"
"##
        }

        // === Scroll ===
        "scroll" => {
            r##"
acbrowser scroll - Scroll the page

Usage: acbrowser scroll [direction] [amount] [options]

Scrolls the page or a specific element in the specified direction.

Arguments:
  direction            up, down, left, right (default: down)
  amount               Pixels to scroll (default: 300)

Options:
  -s, --selector <sel> CSS selector for a scrollable container

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser scroll
  acbrowser scroll down 500
  acbrowser scroll up 200
  acbrowser scroll left 100
  acbrowser scroll down 500 --selector "div.scroll-container"
"##
        }
        "scrollintoview" | "scrollinto" => {
            r##"
acbrowser scrollintoview - Scroll element into view

Usage: acbrowser scrollintoview <selector>

Scrolls the page until the specified element is visible in the viewport.

Aliases: scrollinto

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser scrollintoview "#footer"
  acbrowser scrollintoview @e15
"##
        }

        // === Wait ===
        "wait" => {
            r##"
acbrowser wait - Wait for condition

Usage: acbrowser wait <selector|ms|option>

Waits for an element to appear, a timeout, or other conditions.

Modes:
  <selector>           Wait for element to appear
  <ms>                 Wait for specified milliseconds
  --url <pattern>      Wait for URL to match pattern
  --load <state>       Wait for load state (load, domcontentloaded, networkidle)
  --fn <expression>    Wait for JavaScript expression to be truthy
  --text <text>        Wait for text to appear on page (substring match)
  --download [path]    Wait for a download to complete (optionally save to path)

Download Options (with --download):
  --timeout <ms>       Timeout in milliseconds for download to start

Wait for text to disappear:
  Use --fn or --state hidden to wait for text or elements to go away:
  wait --fn "!document.body.innerText.includes('Loading...')"
  wait "#spinner" --state hidden
  wait @e5 --state detached

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser wait "#loading-spinner"
  acbrowser wait 2000
  acbrowser wait --url "**/dashboard"
  acbrowser wait --load networkidle
  acbrowser wait --fn "window.appReady === true"
  acbrowser wait --text "Welcome back"
  acbrowser wait --download ./file.pdf
  acbrowser wait --download ./report.xlsx --timeout 30000
  acbrowser wait --fn "!document.body.innerText.includes('Loading...')"
"##
        }

        // === Screenshot/PDF ===
        "screenshot" => {
            r##"
acbrowser screenshot - Take a screenshot

Usage: acbrowser screenshot [selector] [path]

Captures a screenshot of the current page. If no path is provided,
saves to a temporary directory with a generated filename.

Options:
  --full, -f           Capture full page (not just viewport)
  --annotate           Overlay numbered labels on interactive elements.
                       Each label [N] corresponds to ref @eN from snapshot.
                       Prints a legend mapping labels to element roles/names.
                       With --json, annotations are included in the response.
                       Supported on Chromium and Lightpanda.
  --screenshot-dir <path>  Default output directory for screenshots
                       (or AGENT_BROWSER_SCREENSHOT_DIR env)
  --screenshot-quality <0-100>  JPEG quality (0-100, only applies to jpeg format)
                       (or AGENT_BROWSER_SCREENSHOT_QUALITY env)
  --screenshot-format <fmt>  Image format: png (default) or jpeg
                       (or AGENT_BROWSER_SCREENSHOT_FORMAT env)

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser screenshot
  acbrowser screenshot ./screenshot.png
  acbrowser screenshot --full ./full-page.png
  acbrowser screenshot --annotate              # Labeled screenshot + legend
  acbrowser screenshot --annotate ./page.png   # Save annotated screenshot
  acbrowser screenshot --annotate --json       # JSON output with annotations
  acbrowser screenshot --screenshot-dir ./shots # Save to custom directory
  acbrowser screenshot --screenshot-format jpeg --screenshot-quality 80
"##
        }
        "pdf" => {
            r##"
acbrowser pdf - Save page as PDF

Usage: acbrowser pdf <path>

Saves the current page as a PDF file.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser pdf ./page.pdf
  acbrowser pdf ~/Documents/report.pdf
"##
        }

        // === Snapshot ===
        "snapshot" => {
            r##"
acbrowser snapshot - Get accessibility tree snapshot

Usage: acbrowser snapshot [options]

Returns an accessibility tree representation of the page with element
references (like @e1, @e2) that can be used in subsequent commands.
Designed for AI agents to understand page structure.

Options:
  -i, --interactive    Only include interactive elements
  -u, --urls           Include href URLs for link elements
  -c, --compact        Remove empty structural elements
  -d, --depth <n>      Limit tree depth
  -s, --selector <sel> Scope snapshot to CSS selector

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser snapshot
  acbrowser snapshot -i
  acbrowser snapshot -i --urls
  acbrowser snapshot --compact --depth 5
  acbrowser snapshot -s "#main-content"
"##
        }

        // === Eval ===
        "eval" => {
            r##"
acbrowser eval - Execute JavaScript

Usage: acbrowser eval [options] <script>

Executes JavaScript code in the browser context and returns the result.

Options:
  -b, --base64         Decode script from base64 (avoids shell escaping issues)
  --stdin              Read script from stdin (useful for heredocs/multiline)

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser eval "document.title"
  acbrowser eval "window.location.href"
  acbrowser eval "document.querySelectorAll('a').length"
  acbrowser eval -b "ZG9jdW1lbnQudGl0bGU="

  # Read from stdin with heredoc
  cat <<'EOF' | acbrowser eval --stdin
  const links = document.querySelectorAll('a');
  links.length;
  EOF
"##
        }

        // === Close ===
        "close" | "quit" | "exit" => {
            r##"
acbrowser close - Close the browser

Usage: acbrowser close [options]

Closes the browser instance for the current session.

Aliases: quit, exit

Options:
  --all                Close all active sessions

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser close
  acbrowser close --session mysession
  acbrowser close --all
"##
        }

        // === Inspect ===
        "inspect" => {
            r##"
acbrowser inspect - Open Chrome DevTools for the active page

Starts a local WebSocket proxy and opens Chrome's DevTools frontend in your
default browser. The proxy routes DevTools traffic through the daemon's
existing CDP connection, so both DevTools and acbrowser commands work
simultaneously.

Usage: acbrowser inspect

Examples:
  acbrowser open example.com
  acbrowser inspect          # opens DevTools in your browser
  acbrowser click "Submit"   # commands still work while DevTools is open
"##
        }

        // === Get ===
        "get" => {
            r##"
acbrowser get - Retrieve information from elements or page

Usage: acbrowser get <subcommand> [args]

Retrieves various types of information from elements or the page.

Subcommands:
  text <selector>            Get text content of element
  html <selector>            Get inner HTML of element
  value <selector>           Get value of input element
  attr <selector> <name>     Get attribute value
  title                      Get page title
  url                        Get current URL
  count <selector>           Count matching elements
  box <selector>             Get bounding box (x, y, width, height)
  styles <selector>          Get computed styles of elements
  cdp-url                    Get Chrome DevTools Protocol WebSocket URL

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser get text @e1
  acbrowser get html "#content"
  acbrowser get value "#email-input"
  acbrowser get attr "#link" href
  acbrowser get title
  acbrowser get url
  acbrowser get count "li.item"
  acbrowser get box "#header"
  acbrowser get styles "button"
  acbrowser get styles @e1
"##
        }

        // === Is ===
        "is" => {
            r##"
acbrowser is - Check element state

Usage: acbrowser is <subcommand> <selector>

Checks the state of an element and returns true/false.

Subcommands:
  visible <selector>   Check if element is visible
  enabled <selector>   Check if element is enabled (not disabled)
  checked <selector>   Check if checkbox/radio is checked

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser is visible "#modal"
  acbrowser is enabled "#submit-btn"
  acbrowser is checked "#agree-checkbox"
"##
        }

        // === Find ===
        "find" => {
            r##"
acbrowser find - Find and interact with elements by locator

Usage: acbrowser find <locator> <value> [action] [text]

Finds elements using semantic locators and optionally performs an action.

Locators:
  role <role>              Find by ARIA role (--name <n>, --exact)
  text <text>              Find by text content (--exact)
  label <label>            Find by associated label (--exact)
  placeholder <text>       Find by placeholder text (--exact)
  alt <text>               Find by alt text (--exact)
  title <text>             Find by title attribute (--exact)
  testid <id>              Find by data-testid attribute
  first <selector>         First matching element
  last <selector>          Last matching element
  nth <index> <selector>   Nth matching element (0-based)

Actions (default: click):
  click, fill, type, hover, focus, check, uncheck

Options:
  --name <name>        Filter role by accessible name
  --exact              Require exact text match

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser find role button click --name Submit
  acbrowser find text "Sign In" click
  acbrowser find label "Email" fill "user@example.com"
  acbrowser find placeholder "Search..." type "query"
  acbrowser find testid "login-form" click
  acbrowser find first "li.item" click
  acbrowser find nth 2 ".card" hover
"##
        }

        // === Mouse ===
        "mouse" => {
            r##"
acbrowser mouse - Low-level mouse operations

Usage: acbrowser mouse <subcommand> [args]

Performs low-level mouse operations for precise control.

Subcommands:
  move <x> <y>         Move mouse to coordinates
  down [button]        Press mouse button (left, right, middle)
  up [button]          Release mouse button
  wheel <dy> [dx]      Scroll mouse wheel

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser mouse move 100 200
  acbrowser mouse down
  acbrowser mouse up
  acbrowser mouse down right
  acbrowser mouse wheel 100
  acbrowser mouse wheel -50 0
"##
        }

        // === Set ===
        "set" => {
            r##"
acbrowser set - Configure browser settings

Usage: acbrowser set <setting> [args]

Configures various browser settings and emulation options.

Settings:
  viewport <w> <h> [scale]   Set viewport size (scale = deviceScaleFactor, e.g. 2 for retina)
  device <name>              Emulate device (e.g., "iPhone 12")
  geo <lat> <lng>            Set geolocation
  offline [on|off]           Toggle offline mode
  headers <json>             Set extra HTTP headers
  credentials <user> <pass>  Set HTTP authentication
  media [dark|light]         Set color scheme preference
        [reduced-motion]     Enable reduced motion

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser set viewport 1920 1080
  acbrowser set viewport 1920 1080 2    # 2x retina
  acbrowser set device "iPhone 12"
  acbrowser set geo 37.7749 -122.4194
  acbrowser set offline on
  acbrowser set headers '{"X-Custom": "value"}'
  acbrowser set credentials admin secret123
  acbrowser set media dark
  acbrowser set media light reduced-motion
"##
        }

        // === Network ===
        "network" => {
            r##"
acbrowser network - Network interception and monitoring

Usage: acbrowser network <subcommand> [args]

Intercept, mock, or monitor network requests.

Subcommands:
  route <url> [options]      Intercept requests matching URL pattern
    --abort                  Abort matching requests
    --body <json>            Respond with custom body
  unroute [url]              Remove route (all if no URL)
  requests [options]         List captured requests
    --clear                  Clear request log
    --filter <pattern>       Filter by URL pattern
    --type <types>           Filter by resource type (comma-separated: xhr,fetch,document)
    --method <method>        Filter by HTTP method (GET, POST, etc.)
    --status <code>          Filter by status (200, 2xx, 400-499)
  request <requestId>        View full request/response detail (including body)
  har <start|stop> [path]    Record and export a HAR file

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser network route "**/api/*" --abort
  acbrowser network route "**/data.json" --body '{"mock": true}'
  acbrowser network unroute
  acbrowser network requests
  acbrowser network requests --filter "api"
  acbrowser network requests --type xhr,fetch
  acbrowser network requests --method POST --status 2xx
  acbrowser network requests --clear
  acbrowser network request 1234.5
  acbrowser network har start
  acbrowser network har stop ./capture.har
"##
        }

        // === Storage ===
        "storage" => {
            r##"
acbrowser storage - Manage web storage

Usage: acbrowser storage <type> [operation] [key] [value]

Manage localStorage and sessionStorage.

Types:
  local                localStorage
  session              sessionStorage

Operations:
  get [key]            Get all storage or specific key
  set <key> <value>    Set a key-value pair
  clear                Clear all storage

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser storage local
  acbrowser storage local get authToken
  acbrowser storage local set theme "dark"
  acbrowser storage local clear
  acbrowser storage session get userId
"##
        }

        // === Cookies ===
        "cookies" => {
            r##"
acbrowser cookies - Manage browser cookies

Usage: acbrowser cookies [operation] [args]

Manage browser cookies for the current context.

Operations:
  get                                Get all cookies (default)
  set <name> <value> [options]       Set a cookie with optional properties
  clear                              Clear all cookies

Cookie Set Options:
  --url <url>                        URL for the cookie (allows setting before page load)
  --domain <domain>                  Cookie domain (e.g., ".example.com")
  --path <path>                      Cookie path (e.g., "/api")
  --httpOnly                         Set HttpOnly flag (prevents JavaScript access)
  --secure                           Set Secure flag (HTTPS only)
  --sameSite <Strict|Lax|None>       SameSite policy
  --expires <timestamp>              Expiration time (Unix timestamp in seconds)

Note: If --url, --domain, and --path are all omitted, the cookie will be set
for the current page URL.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  # Simple cookie for current page
  acbrowser cookies set session_id "abc123"

  # Set cookie for a URL before loading it (useful for authentication)
  acbrowser cookies set session_id "abc123" --url https://app.example.com

  # Set secure, httpOnly cookie with domain and path
  acbrowser cookies set auth_token "xyz789" --domain example.com --path /api --httpOnly --secure

  # Set cookie with SameSite policy
  acbrowser cookies set tracking_consent "yes" --sameSite Strict

  # Set cookie with expiration (Unix timestamp)
  acbrowser cookies set temp_token "temp123" --expires 1735689600

  # Get all cookies
  acbrowser cookies

  # Clear all cookies
  acbrowser cookies clear
"##
        }

        // === Tabs ===
        "tab" => {
            r##"
acbrowser tab - Manage browser tabs

Usage: acbrowser tab [operation] [args]

Manage browser tabs in the current window.

Operations:
  list                 List all tabs (default)
  new [url]            Open new tab
  close [index]        Close tab (current if no index)
  <index>              Switch to tab by index

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser tab
  acbrowser tab list
  acbrowser tab new
  acbrowser tab new https://example.com
  acbrowser tab 2
  acbrowser tab close
  acbrowser tab close 1
"##
        }

        // === Window ===
        "window" => {
            r##"
acbrowser window - Manage browser windows

Usage: acbrowser window <operation>

Manage browser windows.

Operations:
  new                  Open new browser window

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser window new
"##
        }

        // === Frame ===
        "frame" => {
            r##"
acbrowser frame - Switch frame context

Usage: acbrowser frame <selector|main>

Switch to an iframe or back to the main frame.

Arguments:
  <selector>           CSS selector for iframe
  main                 Switch back to main frame

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser frame "#embed-iframe"
  acbrowser frame "iframe[name='content']"
  acbrowser frame main
"##
        }

        // === Auth ===
        "auth" => {
            r##"
acbrowser auth - Manage authentication profiles

Usage: acbrowser auth <subcommand> [args]

Subcommands:
  save <name>              Save credentials for a login profile
  login <name>             Login using saved credentials (waits for form fields)
  list                     List saved profiles (names and URLs only)
  show <name>              Show profile metadata (no passwords)
  delete <name>            Delete a saved profile

Save Options:
  --url <url>              Login page URL (required)
  --username <user>        Username (required)
  --password <pass>        Password (required unless --password-stdin)
  --password-stdin          Read password from stdin (recommended)
  --username-selector <s>  Custom CSS selector for username field
  --password-selector <s>  Custom CSS selector for password field
  --submit-selector <s>    Custom CSS selector for submit button

Login behavior:
  auth login waits for form selectors to appear before filling/clicking.
  Selector wait timeout follows the default action timeout.

Global Options:
  --json                   Output as JSON
  --session <name>         Use specific session

Examples:
  echo "pass" | acbrowser auth save github --url https://github.com/login --username user --password-stdin
  acbrowser auth save github --url https://github.com/login --username user --password pass
  acbrowser auth login github
  acbrowser auth list
  acbrowser auth show github
  acbrowser auth delete github
"##
        }

        // === Confirm/Deny ===
        "confirm" | "deny" => {
            r##"
acbrowser confirm/deny - Approve or deny pending actions

Usage:
  acbrowser confirm <confirmation-id>
  acbrowser deny <confirmation-id>

When --confirm-actions is set, certain action categories return a
confirmation_required response with a confirmation ID. Use confirm/deny
to approve or reject the action.

Pending confirmations auto-deny after 60 seconds.

Examples:
  acbrowser confirm c_8f3a1234
  acbrowser deny c_8f3a1234
"##
        }

        // === Dialog ===
        "dialog" => {
            r##"
acbrowser dialog - Handle browser dialogs

Usage: acbrowser dialog <accept|dismiss|status> [text]

Respond to or check for browser dialogs (alert, confirm, prompt).

Operations:
  accept [text]        Accept dialog, optionally with prompt text
  dismiss              Dismiss/cancel dialog
  status               Check if a dialog is currently open

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser dialog accept
  acbrowser dialog accept "my input"
  acbrowser dialog dismiss
  acbrowser dialog status
"##
        }

        // === Trace ===
        "trace" => {
            r##"
acbrowser trace - Record execution trace

Usage: acbrowser trace <operation> [path]

Record a Chrome DevTools trace for debugging.

Operations:
  start [path]         Start recording trace
  stop [path]          Stop recording and save trace

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser trace start
  acbrowser trace start ./my-trace
  acbrowser trace stop
  acbrowser trace stop ./debug-trace.zip
"##
        }

        // === Profile (CDP Tracing) ===
        "profiler" => {
            r##"
acbrowser profiler - Record Chrome DevTools performance profile

Usage: acbrowser profiler <operation> [options]

Record a performance profile using Chrome DevTools Protocol (CDP) Tracing.
The output JSON file can be loaded into Chrome DevTools Performance panel,
Perfetto UI (https://ui.perfetto.dev/), or other trace analysis tools.

Operations:
  start                Start profiling
  stop [path]          Stop profiling and save to file

Start Options:
  --categories <list>  Comma-separated trace categories (default includes
                       devtools.timeline, v8.execute, blink, and others)

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  # Basic profiling
  acbrowser profiler start
  acbrowser navigate https://example.com
  acbrowser click "#button"
  acbrowser profiler stop ./trace.json

  # With custom categories
  acbrowser profiler start --categories "devtools.timeline,v8.execute,blink.user_timing"
  acbrowser profiler stop ./custom-trace.json

The output file can be viewed in:
  - Chrome DevTools: Performance panel > Load profile
  - Perfetto: https://ui.perfetto.dev/
"##
        }

        // === Record (video) ===
        "record" => {
            r##"
acbrowser record - Record browser session to video

Usage: acbrowser record start <path.webm> [url]
       acbrowser record stop
       acbrowser record restart <path.webm> [url]

Record the browser to a WebM video file.
Creates a fresh browser context but preserves cookies and localStorage.
If no URL is provided, automatically navigates to your current page.

Operations:
  start <path> [url]     Start recording (defaults to current URL if omitted)
  stop                   Stop recording and save video
  restart <path> [url]   Stop current recording (if any) and start a new one

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  # Record from current page (preserves login state)
  acbrowser open https://app.example.com/dashboard
  acbrowser snapshot -i            # Explore and plan
  acbrowser record start ./demo.webm
  acbrowser click @e3              # Execute planned actions
  acbrowser record stop

  # Or specify a different URL
  acbrowser record start ./demo.webm https://example.com

  # Restart recording with a new file (stops previous, starts new)
  acbrowser record restart ./take2.webm
"##
        }

        // === Console/Errors ===
        "console" => {
            r##"
acbrowser console - View console logs

Usage: acbrowser console [--clear]

View browser console output (log, warn, error, info).

Options:
  --clear              Clear console log buffer

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser console
  acbrowser console --clear
"##
        }
        "errors" => {
            r##"
acbrowser errors - View page errors

Usage: acbrowser errors [--clear]

View JavaScript errors and uncaught exceptions.

Options:
  --clear              Clear error buffer

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser errors
  acbrowser errors --clear
"##
        }

        // === Highlight ===
        "highlight" => {
            r##"
acbrowser highlight - Highlight an element

Usage: acbrowser highlight <selector>

Visually highlights an element on the page for debugging.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser highlight "#target-element"
  acbrowser highlight @e5
"##
        }

        // === Clipboard ===
        "clipboard" => {
            r##"
acbrowser clipboard - Read and write clipboard

Usage: acbrowser clipboard <operation> [text]

Read from or write to the browser clipboard.

Operations:
  read                 Read text from clipboard
  write <text>         Write text to clipboard
  copy                 Copy current selection (simulates Ctrl+C)
  paste                Paste from clipboard (simulates Ctrl+V)

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser clipboard read
  acbrowser clipboard write "Hello, World!"
  acbrowser clipboard copy
  acbrowser clipboard paste
"##
        }

        // === State ===
        "state" => {
            r##"
acbrowser state - Manage browser state

Usage: acbrowser state <operation> [args]

Save, restore, list, and manage browser state (cookies, localStorage, sessionStorage).

Operations:
  save <path>                        Save current state to file
  load <path>                        Load state from file
  list                               List saved state files
  show <filename>                    Show state summary
  rename <old-name> <new-name>       Rename state file
  clear [session-name] [--all]       Clear saved states
  clean --older-than <days>          Delete expired state files

Automatic State Persistence:
  Use --session-name to auto-save/restore state across restarts:
  acbrowser --session-name myapp open https://example.com
  Or set AGENT_BROWSER_SESSION_NAME environment variable.

State Encryption:
  Set AGENT_BROWSER_ENCRYPTION_KEY (64-char hex) for AES-256-GCM encryption.
  Generate a key: openssl rand -hex 32

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser state save ./auth-state.json
  acbrowser state load ./auth-state.json
  acbrowser state list
  acbrowser state show myapp-default.json
  acbrowser state rename old-name new-name
  acbrowser state clear --all
  acbrowser state clean --older-than 7
"##
        }

        // === Session ===
        "session" => {
            r##"
acbrowser session - Manage sessions

Usage: acbrowser session [operation]

Manage isolated browser sessions. Each session has its own browser
instance with separate cookies, storage, and state.

Operations:
  (none)               Show current session name
  list                 List all active sessions

Environment:
  AGENT_BROWSER_SESSION    Default session name

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser session
  acbrowser session list
  acbrowser --session test open example.com
"##
        }

        // === Install ===
        "install" => {
            r##"
acbrowser install - Install browser binaries

Usage: acbrowser install [--with-deps]

Downloads and installs browser binaries required for automation.

Options:
  -d, --with-deps      Also install system dependencies (Linux only)

Examples:
  acbrowser install
  acbrowser install --with-deps
"##
        }

        // === Upgrade ===
        "upgrade" => {
            r##"
acbrowser upgrade - Upgrade to the latest version

Usage: acbrowser upgrade

Detects the current installation method (npm, Homebrew, or Cargo) and runs
the appropriate update command. Displays the version change on success, or
informs you if you are already on the latest version.

Examples:
  acbrowser upgrade
"##
        }

        // === Dashboard ===
        "dashboard" => {
            r##"
acbrowser dashboard - Observability dashboard

Usage: acbrowser dashboard [start|stop] [options]

Manage the observability dashboard, a local web UI that shows live
browser viewports and command activity feeds for all sessions.
The dashboard is bundled into the binary and requires no separate install.

Subcommands:
  start [--port <n>]   Start the dashboard server (default port: 4848)
  stop                 Stop the dashboard server

Running 'acbrowser dashboard' with no subcommand is equivalent to 'dashboard start'.

The dashboard runs as a standalone background process, independent of
browser sessions. All sessions automatically stream to the dashboard.

Options:
  --port <n>           Port for the dashboard server (default: 4848)

Global Options:
  --json               Output as JSON

Examples:
  acbrowser dashboard start
  acbrowser dashboard start --port 8080
  acbrowser dashboard stop
"##
        }

        // === Connect ===
        "connect" => {
            r##"
acbrowser connect - Connect to browser via CDP

Usage: acbrowser connect <port|url>

Connects to a running browser instance via Chrome DevTools Protocol (CDP).
This allows controlling browsers, Electron apps, or remote browser services.

Arguments:
  <port>               Local port number (e.g., 9222)
  <url>                Full WebSocket URL (ws://, wss://, http://, https://)

Supported URL formats:
  - Port number: 9222 (connects to http://localhost:9222)
  - WebSocket URL: ws://localhost:9222/devtools/browser/...
  - Remote service: wss://remote-browser.example.com/cdp?token=...

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  # Connect to local Chrome with remote debugging
  # Start Chrome: google-chrome --remote-debugging-port=9222
  acbrowser connect 9222

  # Connect using WebSocket URL from /json/version endpoint
  acbrowser connect "ws://localhost:9222/devtools/browser/abc123"

  # Connect to remote browser service
  acbrowser connect "wss://browser-service.example.com/cdp?token=xyz"

  # After connecting, run commands normally
  acbrowser snapshot
  acbrowser click @e1
"##
        }

        // === Runtime streaming ===
        "stream" => {
            r##"
acbrowser stream - Manage live WebSocket browser streaming

Usage:
  acbrowser stream enable [--port <port>]
  acbrowser stream disable
  acbrowser stream status

Enables or disables the session-scoped WebSocket stream server without restarting
an already-running daemon. If --port is omitted, acbrowser binds an
available localhost port automatically and reports it back.

Notes:
  - 'stream enable' creates the WebSocket server.
  - WebSocket clients trigger frame streaming automatically.
  - 'screencast_start' and 'screencast_stop' still control explicit CDP screencasts.
  - Streaming is always enabled. Set AGENT_BROWSER_STREAM_PORT to bind to a
    specific port instead of the default OS-assigned port.

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser stream status
  acbrowser stream enable
  acbrowser stream enable --port 9223
  acbrowser stream disable
"##
        }

        // === iOS Commands ===
        "tap" => {
            r##"
acbrowser tap - Tap an element (touch gesture)

Usage: acbrowser tap <selector>

Taps an element. This is an alias for 'click' that provides semantic clarity
for touch-based interfaces like iOS Safari.

Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser tap "#submit-button"
  acbrowser tap @e1
  acbrowser -p ios tap "button:has-text('Sign In')"
"##
        }
        "swipe" => {
            r##"
acbrowser swipe - Swipe gesture (iOS)

Usage: acbrowser swipe <direction> [distance]

Performs a swipe gesture on iOS Safari. The direction determines
which way the content moves (swipe up scrolls down, etc.).

Arguments:
  direction    up, down, left, or right
  distance     Optional distance in pixels (default: 300)

Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser -p ios swipe up
  acbrowser -p ios swipe down 500
  acbrowser -p ios swipe left
"##
        }
        "device" => {
            r##"
acbrowser device - Manage iOS simulators

Usage: acbrowser device <subcommand>

Subcommands:
  list    List available iOS simulators

Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser device list
  acbrowser -p ios device list
"##
        }

        "diff" => {
            r##"
acbrowser diff - Compare page states

Subcommands:

  diff snapshot                   Compare current snapshot to last snapshot in session
  diff screenshot --baseline <f>  Visual pixel diff against a baseline image
  diff url <url1> <url2>          Compare two pages

Snapshot Diff:

  Usage: acbrowser diff snapshot [options]

  Options:
    -b, --baseline <file>    Compare against a saved snapshot file
    -s, --selector <sel>     Scope snapshot to a CSS selector or @ref
    -c, --compact            Use compact snapshot format
    -d, --depth <n>          Limit snapshot tree depth

  Without --baseline, compares against the last snapshot taken in this session.

Screenshot Diff:

  Usage: acbrowser diff screenshot --baseline <file> [options]

  Options:
    -b, --baseline <file>    Baseline image to compare against (required)
    -o, --output <file>      Path for the diff image (default: temp dir)
    -t, --threshold <0-1>    Color distance threshold (default: 0.1)
    -s, --selector <sel>     Scope screenshot to element
        --full               Full page screenshot

URL Diff:

  Usage: acbrowser diff url <url1> <url2> [options]

  Options:
    --screenshot             Also compare screenshots (default: snapshot only)
    --full                   Full page screenshots
    --wait-until <strategy>  Navigation wait strategy: load, domcontentloaded, networkidle (default: load)
    -s, --selector <sel>     Scope snapshots to a CSS selector or @ref
    -c, --compact            Use compact snapshot format
    -d, --depth <n>          Limit snapshot tree depth

Global Options:
  --json               Output as JSON
  --session <name>     Use specific session

Examples:
  acbrowser diff snapshot
  acbrowser diff snapshot --baseline before.txt
  acbrowser diff screenshot --baseline before.png
  acbrowser diff screenshot --baseline before.png --output diff.png --threshold 0.2
  acbrowser diff url https://staging.example.com https://prod.example.com
  acbrowser diff url https://v1.example.com https://v2.example.com --screenshot
"##
        }

        "batch" => {
            r##"
acbrowser batch - Execute multiple commands sequentially

Usage: acbrowser batch [options] "<cmd1>" "<cmd2>" ...
       echo '<json>' | acbrowser batch [options]

Runs multiple commands in sequence. Commands can be passed as quoted
arguments or piped as JSON via stdin. Results are printed in order,
separated by blank lines (or as a JSON array with --json).

Options:
  --bail               Stop on first error (default: continue all commands)
  --json               Output results as a JSON array

Argument Mode:
  Each quoted argument is a full command string:
  acbrowser batch "open https://example.com" "snapshot -i" "screenshot"

Stdin Mode (JSON):
  A JSON array of string arrays. Each inner array is one command:
  [
    ["open", "https://example.com"],
    ["snapshot", "-i"],
    ["click", "@e1"],
    ["fill", "@e2", "test@example.com"],
    ["screenshot", "result.png"]
  ]

Examples:
  acbrowser batch "open https://example.com" "screenshot"
  acbrowser batch --bail "open https://example.com" "click @e1" "screenshot"
  echo '[["open", "https://example.com"], ["snapshot"]]' | acbrowser batch
  acbrowser batch --bail < commands.json
"##
        }

        "profiles" => {
            r##"
acbrowser profiles - List available Chrome profiles

Usage: acbrowser profiles

Lists all Chrome profiles found in your Chrome user data directory, showing
the directory name and display name for each profile. Use the directory name
with --profile to launch Chrome with that profile's login state.

Global Options:
  --json               Output as JSON

Examples:
  acbrowser profiles
  acbrowser profiles --json
  acbrowser --profile Default open https://gmail.com
"##
        }

        "chat" => {
            r##"
acbrowser chat - Natural language browser control via AI

Usage:
  acbrowser chat <message>         Single-shot: execute instruction and exit
  acbrowser chat                   Interactive REPL (when stdin is a TTY)
  echo "instruction" | acbrowser chat   Piped input

Sends natural language instructions to an AI model that translates them
into acbrowser commands and executes them against the active session.
Requires AI_GATEWAY_API_KEY to be set.

In interactive mode, type "quit", "exit", or "q" to leave the REPL.

Chat Options:
  --model <name>         AI model (or AI_GATEWAY_MODEL env, default: anthropic/claude-sonnet-4.6)
  -v, --verbose          Show tool commands and their raw output
  -q, --quiet            Show only the AI text response (hide tool calls)

Global Options:
  --json                 Structured JSON output per turn
  --session <name>       Target session for commands

Examples:
  acbrowser chat "open google.com and search for cats"
  acbrowser chat "take a screenshot of the current page"
  acbrowser -q chat "summarize this page"
  acbrowser -v chat "fill in the login form with test@example.com"
  acbrowser --model openai/gpt-4o chat "navigate to hacker news"
  acbrowser chat
"##
        }

        _ => return false,
    };
    println!("{}", help.trim());
    true
}

pub fn print_help() {
    println!(
        r#"
acbrowser - fast browser automation CLI for AI agents

Usage: acbrowser <command> [args] [options]

Core Commands:
  open <url>                 Navigate to URL
  click <sel>                Click element (or @ref)
  dblclick <sel>             Double-click element
  type <sel> <text>          Type into element
  fill <sel> <text>          Clear and fill
  press <key>                Press key (Enter, Tab, Control+a)
  keyboard type <text>       Type text with real keystrokes (no selector)
  keyboard inserttext <text> Insert text without key events
  hover <sel>                Hover element
  focus <sel>                Focus element
  check <sel>                Check checkbox
  uncheck <sel>              Uncheck checkbox
  select <sel> <val...>      Select dropdown option
  drag <src> <dst>           Drag and drop
  upload <sel> <files...>    Upload files
  download <sel> <path>      Download file by clicking element
  scroll <dir> [px]          Scroll (up/down/left/right)
  scrollintoview <sel>       Scroll element into view
  wait <sel|ms>              Wait for element or time
  screenshot [path]          Take screenshot
  pdf <path>                 Save as PDF
  snapshot                   Accessibility tree with refs (for AI)
  eval <js>                  Run JavaScript
  connect <port|url>         Connect to browser via CDP
  close [--all]              Close browser (--all closes every session)

Navigation:
  back                       Go back
  forward                    Go forward
  reload                     Reload page

Get Info:  acbrowser get <what> [selector]
  text, html, value, attr <name>, title, url, count, box, styles, cdp-url

Check State:  acbrowser is <what> <selector>
  visible, enabled, checked

Find Elements:  acbrowser find <locator> <value> <action> [text]
  role, text, label, placeholder, alt, title, testid, first, last, nth

Mouse:  acbrowser mouse <action> [args]
  move <x> <y>, down [btn], up [btn], wheel <dy> [dx]

Browser Settings:  acbrowser set <setting> [value]
  viewport <w> <h>, device <name>, geo <lat> <lng>
  offline [on|off], headers <json>, credentials <user> <pass>
  media [dark|light] [reduced-motion]

Network:  acbrowser network <action>
  route <url> [--abort|--body <json>]
  unroute [url]
  requests [--clear] [--filter <pattern>]
  har <start|stop> [path]

Storage:
  cookies [get|set|clear]    Manage cookies (set supports --url, --domain, --path, --httpOnly, --secure, --sameSite, --expires)
  storage <local|session>    Manage web storage

Tabs:
  tab [new|list|close|<n>]   Manage tabs

Diff:
  diff snapshot              Compare current vs last snapshot
  diff screenshot --baseline Compare current vs baseline image
  diff url <u1> <u2>         Compare two pages

Debug:
  trace start|stop [path]    Record Chrome DevTools trace
  profiler start|stop [path] Record Chrome DevTools profile
  record start <path> [url]  Start video recording (WebM)
  record stop                Stop and save video
  console [--clear]          View console logs
  errors [--clear]           View page errors
  highlight <sel>            Highlight element
  inspect                    Open Chrome DevTools for the active page
  clipboard <op> [text]      Read/write clipboard (read, write, copy, paste)

Streaming:
  stream enable [--port <n>] Start runtime WebSocket streaming for this session
  stream disable             Stop runtime WebSocket streaming
  stream status              Show streaming status and active port

Batch:
  batch [--bail] ["cmd" ...]  Execute multiple commands sequentially (args or stdin)
                              --bail stops on first error (default: continue all)

Auth Vault:
  auth save <name> [opts]    Save auth profile (--url, --username, --password/--password-stdin)
  auth login <name>          Login using saved credentials (waits for form fields)
  auth list                  List saved auth profiles
  auth show <name>           Show auth profile metadata
  auth delete <name>         Delete auth profile

Confirmation:
  confirm <id>               Approve a pending action
  deny <id>                  Deny a pending action

Sessions:
  session                    Show current session name
  session list               List active sessions

Chat (AI):
  chat <message>             Send a natural language instruction (single-shot)
  chat                       Start interactive chat (REPL mode when stdin is a TTY)
  Options: --model <name>, -v/--verbose, -q/--quiet

Dashboard:
  dashboard [start]          Start the dashboard server (default port: 4848)
  dashboard start --port <n> Start on a specific port
  dashboard stop             Stop the dashboard server

Setup:
  install                    Install browser binaries
  install --with-deps        Also install system dependencies (Linux)
  upgrade                    Upgrade to the latest version
  dashboard start            Start the observability dashboard
  profiles                   List available Chrome profiles

Snapshot Options:
  -i, --interactive          Only interactive elements
  -c, --compact              Remove empty structural elements
  -d, --depth <n>            Limit tree depth
  -s, --selector <sel>       Scope to CSS selector

Authentication:
  --profile <name|path>      Chrome profile name (e.g., Default) to reuse login state,
                             or a directory path for a persistent custom profile
                             (or AGENT_BROWSER_PROFILE env)
  --session-name <name>      Auto-save/restore cookies and localStorage by name
                             (or AGENT_BROWSER_SESSION_NAME env)
  --state <path>             Load saved auth state (cookies + storage) from JSON file
                             (or AGENT_BROWSER_STATE env)
  --auto-connect             Connect to a running Chrome to reuse its auth state
                             Tip: acbrowser --auto-connect state save ./auth.json
  --headers <json>           HTTP headers scoped to URL's origin (e.g., Authorization bearer token)

Options:
  --session <name>           Isolated session (or AGENT_BROWSER_SESSION env)
  --executable-path <path>   Custom browser executable (or AGENT_BROWSER_EXECUTABLE_PATH)
  --extension <path>         Load browser extensions (repeatable)
  --args <args>              Browser launch args, comma or newline separated (or AGENT_BROWSER_ARGS)
                             e.g., --args "--no-sandbox,--disable-blink-features=AutomationControlled"
  --user-agent <ua>          Custom User-Agent (or AGENT_BROWSER_USER_AGENT)
  --proxy <server>           Proxy server URL (or AGENT_BROWSER_PROXY, HTTP_PROXY, HTTPS_PROXY, ALL_PROXY)
                             Supports authenticated proxies: --proxy "http://user:pass@127.0.0.1:7890"
  --proxy-bypass <hosts>     Bypass proxy for these hosts (or AGENT_BROWSER_PROXY_BYPASS, NO_PROXY)
                             e.g., --proxy-bypass "localhost,*.internal.com"
  --ignore-https-errors      Ignore HTTPS certificate errors
  --allow-file-access        Allow file:// URLs to access local files (Chromium only)
  -p, --provider <name>      Browser provider: ios, browserbase, kernel, browseruse, browserless, agentcore
  --device <name>            iOS device name (e.g., "iPhone 15 Pro")
  --json                     JSON output
  --annotate                 Annotated screenshot with numbered labels and legend
  --screenshot-dir <path>    Default screenshot output directory (or AGENT_BROWSER_SCREENSHOT_DIR)
  --screenshot-quality <n>   JPEG quality 0-100; ignored for PNG (or AGENT_BROWSER_SCREENSHOT_QUALITY)
  --screenshot-format <fmt>  Screenshot format: png, jpeg (or AGENT_BROWSER_SCREENSHOT_FORMAT)
  --headed                   Show browser window (not headless) (or AGENT_BROWSER_HEADED env)
  --cdp <port>               Connect via CDP (Chrome DevTools Protocol)
  --color-scheme <scheme>    Color scheme: dark, light, no-preference (or AGENT_BROWSER_COLOR_SCHEME)
  --download-path <path>     Default download directory (or AGENT_BROWSER_DOWNLOAD_PATH)
  --content-boundaries       Wrap page output in boundary markers (or AGENT_BROWSER_CONTENT_BOUNDARIES)
  --max-output <chars>       Truncate page output to N chars (or AGENT_BROWSER_MAX_OUTPUT)
  --allowed-domains <list>   Restrict navigation domains (or AGENT_BROWSER_ALLOWED_DOMAINS)
  --action-policy <path>     Action policy JSON file (or AGENT_BROWSER_ACTION_POLICY)
  --confirm-actions <list>   Categories requiring confirmation (or AGENT_BROWSER_CONFIRM_ACTIONS)
  --confirm-interactive      Interactive confirmation prompts; auto-denies if stdin is not a TTY (or AGENT_BROWSER_CONFIRM_INTERACTIVE)
  --engine <name>            Browser engine: chrome (default), lightpanda (or AGENT_BROWSER_ENGINE)
  --no-auto-dialog           Disable automatic dismissal of alert/beforeunload dialogs (or AGENT_BROWSER_NO_AUTO_DIALOG)
  --model <name>             AI model for chat (or AI_GATEWAY_MODEL env)
  -v, --verbose              Show tool commands and their raw output
  -q, --quiet                Show only AI text responses (hide tool calls)
  --config <path>            Use a custom config file (or AGENT_BROWSER_CONFIG env)
  --debug                    Debug output
  --version, -V              Show version

Configuration:
  acbrowser looks for acbrowser.json in these locations (lowest to highest priority):
    1. ~/.acbrowser/config.json      User-level defaults
    2. ./acbrowser.json              Project-level overrides
    3. Environment variables             Override config file values
    4. CLI flags                         Override everything

  Use --config <path> to load a specific config file instead of the defaults.
  If --config points to a missing or invalid file, acbrowser exits with an error.

  Boolean flags accept an optional true/false value to override config:
    --headed           (same as --headed true)
    --headed false     (disables "headed": true from config)

  Extensions from user and project configs are merged (not replaced).

  Example acbrowser.json:
    {{"headed": true, "proxy": "http://localhost:8080", "profile": "./browser-data"}}

Environment:
  AGENT_BROWSER_CONFIG           Path to config file (or use --config)
  AGENT_BROWSER_SESSION          Session name (default: "default")
  AGENT_BROWSER_SESSION_NAME     Auto-save/restore state persistence name
  AGENT_BROWSER_ENCRYPTION_KEY   64-char hex key for AES-256-GCM state encryption
  AGENT_BROWSER_STATE_EXPIRE_DAYS Auto-delete states older than N days (default: 30)
  AGENT_BROWSER_EXECUTABLE_PATH  Custom browser executable path
  AGENT_BROWSER_EXTENSIONS       Comma-separated browser extension paths
  AGENT_BROWSER_HEADED           Show browser window (not headless)
  AGENT_BROWSER_JSON             JSON output
  AGENT_BROWSER_ANNOTATE         Annotated screenshot with numbered labels and legend
  AGENT_BROWSER_DEBUG            Debug output
  AGENT_BROWSER_IGNORE_HTTPS_ERRORS Ignore HTTPS certificate errors
  AGENT_BROWSER_PROVIDER         Browser provider (ios, browserbase, kernel, browseruse, browserless, agentcore)
  AGENT_BROWSER_AUTO_CONNECT     Auto-discover and connect to running Chrome
  AGENT_BROWSER_ALLOW_FILE_ACCESS Allow file:// URLs to access local files
  AGENT_BROWSER_COLOR_SCHEME     Color scheme preference (dark, light, no-preference)
  AGENT_BROWSER_DOWNLOAD_PATH    Default download directory for browser downloads
  AGENT_BROWSER_DEFAULT_TIMEOUT  Default action timeout in ms (default: 25000)
  AGENT_BROWSER_SESSION_NAME     Auto-save/load state persistence name
  AGENT_BROWSER_STATE_EXPIRE_DAYS Auto-delete saved states older than N days (default: 30)
  AGENT_BROWSER_ENCRYPTION_KEY   64-char hex key for AES-256-GCM session encryption
  AGENT_BROWSER_STREAM_PORT      Override WebSocket streaming port (default: OS-assigned)
  AGENT_BROWSER_IDLE_TIMEOUT_MS  Auto-shutdown daemon after N ms of inactivity (disabled by default)
  AGENT_BROWSER_IOS_DEVICE       Default iOS device name
  AGENT_BROWSER_IOS_UDID         Default iOS device UDID
  AGENT_BROWSER_CONTENT_BOUNDARIES Wrap page output in boundary markers
  AGENT_BROWSER_MAX_OUTPUT       Max characters for page output
  AGENT_BROWSER_ALLOWED_DOMAINS  Comma-separated allowed domain patterns
  AGENT_BROWSER_ACTION_POLICY    Path to action policy JSON file
  AGENT_BROWSER_CONFIRM_ACTIONS  Action categories requiring confirmation
  AGENT_BROWSER_CONFIRM_INTERACTIVE Enable interactive confirmation prompts
  AGENT_BROWSER_NO_AUTO_DIALOG   Disable automatic dismissal of alert/beforeunload dialogs
  AGENT_BROWSER_ENGINE           Browser engine: chrome (default), lightpanda
  HTTP_PROXY / HTTPS_PROXY       Standard proxy env vars (fallback if AGENT_BROWSER_PROXY not set)
  ALL_PROXY                      SOCKS proxy (fallback for proxy)
  NO_PROXY                       Bypass proxy for hosts (fallback for proxy-bypass)
  AGENT_BROWSER_SCREENSHOT_DIR   Default screenshot output directory
  AGENT_BROWSER_SCREENSHOT_QUALITY JPEG quality 0-100
  AGENT_BROWSER_SCREENSHOT_FORMAT Screenshot format: png, jpeg
  AI_GATEWAY_URL                 Vercel AI Gateway base URL (default: https://ai-gateway.vercel.sh)
  AI_GATEWAY_API_KEY             API key for the AI Gateway (enables chat command and dashboard AI chat)
  AI_GATEWAY_MODEL               Default AI model (default: anthropic/claude-sonnet-4.6, or --model flag)

Install:
  npm install -g acbrowser           # npm
  brew install acbrowser             # Homebrew
  cargo install acbrowser            # Cargo
  acbrowser install                  # Download Chrome (first time)

Examples:
  acbrowser open example.com
  acbrowser snapshot -i              # Interactive elements only
  acbrowser click @e2                # Click by ref from snapshot
  acbrowser fill @e3 "test@example.com"
  acbrowser find role button click --name Submit
  acbrowser get text @e1
  acbrowser screenshot --full
  acbrowser screenshot --annotate    # Labeled screenshot for vision models
  acbrowser wait 2000               # Wait for slow pages to settle
  acbrowser --cdp 9222 snapshot      # Connect via CDP port
  acbrowser --auto-connect snapshot  # Auto-discover running Chrome
  acbrowser stream enable            # Start runtime streaming on an auto-selected port
  acbrowser stream status            # Inspect runtime streaming state
  acbrowser --color-scheme dark open example.com  # Dark mode
  acbrowser --profile Default open gmail.com        # Reuse Chrome login state
  acbrowser --profile ~/.myapp open example.com    # Persistent custom profile
  acbrowser profiles                               # List available Chrome profiles
  acbrowser --session-name myapp open example.com  # Auto-save/restore state
  acbrowser chat "open google.com and search for cats"  # AI chat (single-shot)
  acbrowser chat                                        # AI chat (interactive REPL)
  acbrowser -q chat "summarize this page"               # Quiet mode (text only)

Command Chaining:
  Chain commands with && in a single shell call (browser persists via daemon):

  acbrowser open example.com && acbrowser snapshot -i
  acbrowser fill @e1 "user@example.com" && acbrowser fill @e2 "pass" && acbrowser click @e3
  acbrowser open example.com && acbrowser screenshot

iOS Simulator (requires Xcode and Appium):
  acbrowser -p ios open example.com                    # Use default iPhone
  acbrowser -p ios --device "iPhone 15 Pro" open url   # Specific device
  acbrowser -p ios device list                         # List simulators
  acbrowser -p ios swipe up                            # Swipe gesture
  acbrowser -p ios tap @e1                             # Touch element
"#
    );
}

fn print_snapshot_diff(data: &serde_json::Map<String, serde_json::Value>) {
    let changed = data
        .get("changed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !changed {
        println!("{} No changes detected", color::success_indicator());
        return;
    }
    if let Some(diff) = data.get("diff").and_then(|v| v.as_str()) {
        for line in diff.lines() {
            if line.starts_with("+ ") {
                println!("{}", color::green(line));
            } else if line.starts_with("- ") {
                println!("{}", color::red(line));
            } else {
                println!("{}", color::dim(line));
            }
        }
        let additions = data.get("additions").and_then(|v| v.as_i64()).unwrap_or(0);
        let removals = data.get("removals").and_then(|v| v.as_i64()).unwrap_or(0);
        let unchanged = data.get("unchanged").and_then(|v| v.as_i64()).unwrap_or(0);
        println!(
            "\n{} additions, {} removals, {} unchanged",
            color::green(&additions.to_string()),
            color::red(&removals.to_string()),
            unchanged
        );
    }
}

fn print_screenshot_diff(data: &serde_json::Map<String, serde_json::Value>) {
    let mismatch = data
        .get("mismatchPercentage")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let is_match = data.get("match").and_then(|v| v.as_bool()).unwrap_or(false);
    let dim_mismatch = data
        .get("dimensionMismatch")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if dim_mismatch {
        println!(
            "{} Images have different dimensions",
            color::error_indicator()
        );
    } else if is_match {
        println!(
            "{} Images match (0% difference)",
            color::success_indicator()
        );
    } else {
        println!(
            "{} {:.2}% pixels differ",
            color::error_indicator(),
            mismatch
        );
    }
    if let Some(diff_path) = data.get("diffPath").and_then(|v| v.as_str()) {
        println!("  Diff image: {}", color::green(diff_path));
    }
    let total = data
        .get("totalPixels")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let different = data
        .get("differentPixels")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    println!(
        "  {} different / {} total pixels",
        color::red(&different.to_string()),
        total
    );
}

pub fn print_version() {
    println!("acbrowser {}", env!("CARGO_PKG_VERSION"));
}

#[cfg(test)]
mod tests {
    use super::format_storage_text;
    use serde_json::json;

    #[test]
    fn test_format_stream_status_text_for_enabled_stream() {
        let data = json!({
            "enabled": true,
            "port": 9223,
            "connected": true,
            "screencasting": false
        });

        let rendered = super::format_stream_status_text(Some("stream_status"), &data).unwrap();

        assert_eq!(
            rendered,
            "Streaming enabled on ws://127.0.0.1:9223\nConnected: true\nScreencasting: false"
        );
    }

    #[test]
    fn test_format_stream_status_text_for_disabled_stream() {
        let data =
            json!({ "enabled": false, "port": null, "connected": false, "screencasting": false });

        let rendered = super::format_stream_status_text(Some("stream_status"), &data).unwrap();

        assert_eq!(rendered, "Streaming disabled");
    }

    #[test]
    fn test_format_storage_text_for_all_entries() {
        let data = json!({
            "data": {
                "token": "abc123",
                "user": "alice"
            }
        });

        let rendered = format_storage_text(&data).unwrap();

        assert_eq!(rendered, "token: abc123\nuser: alice");
    }

    #[test]
    fn test_format_storage_text_for_key_lookup() {
        let data = json!({
            "key": "token",
            "value": "abc123"
        });

        let rendered = format_storage_text(&data).unwrap();

        assert_eq!(rendered, "token: abc123");
    }

    #[test]
    fn test_format_storage_text_for_empty_store() {
        let data = json!({
            "data": {}
        });

        let rendered = format_storage_text(&data).unwrap();

        assert_eq!(rendered, "No storage entries");
    }
}
