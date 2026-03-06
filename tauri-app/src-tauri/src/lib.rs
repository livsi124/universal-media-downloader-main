use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

// ─── Types ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[allow(unused_mut)]
fn create_command<S: AsRef<std::ffi::OsStr>>(program: S) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    cmd
}

#[allow(unused_mut)]
fn create_tokio_command<S: AsRef<std::ffi::OsStr>>(program: S) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    cmd
}

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub platform: String,
    pub duration: Option<f64>,
    pub uploader: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub task_id: String,
    pub status: String,
    pub percent: f64,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub platform: String,
    pub status: String,
    pub file_path: Option<String>,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    pub language: String,
    pub save_path: Option<String>,
    pub parallel_downloads: u32,
    pub subtitles_enabled: bool,
    pub use_cookies: bool,
    pub cookie_source_type: String,
    pub cookies_path: Option<String>,
    pub cookie_browser: Option<String>,
    pub quality_settings: HashMap<String, String>,
    pub recent_urls: Vec<String>,
    pub skipped_ytdlp_version: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: "dark".to_string(),
            language: "ru".to_string(),
            save_path: None,
            parallel_downloads: 2,
            subtitles_enabled: false,
            use_cookies: false,
            cookie_source_type: "file".to_string(),
            cookies_path: None,
            cookie_browser: None,
            quality_settings: HashMap::new(),
            recent_urls: Vec::new(),
            skipped_ytdlp_version: None,
        }
    }
}

// ─── App State ───────────────────────────────────────────────────────────────

pub struct AppState {
    settings: Mutex<Settings>,
    history_path: Mutex<Option<PathBuf>>,
    settings_path: Mutex<Option<PathBuf>>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            settings: Mutex::new(Settings::default()),
            history_path: Mutex::new(None),
            settings_path: Mutex::new(None),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn get_ytdlp_path(app: &AppHandle) -> Option<String> {
    let mut candidates = vec![];

    // Priority 1: Check bundled assets
    if let Ok(resource_path) = app.path().resource_dir() {
        let bundled = resource_path.join("assets").join("bin").join(if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" });
        if bundled.exists() {
            return Some(bundled.to_string_lossy().to_string());
        }
    }

    // Check common locations
    if let Some(home) = dirs_next::home_dir() {
        candidates.push(home.join(".local/bin/yt-dlp").to_string_lossy().to_string());
        candidates.push(
            home.join("AppData/Local/Programs/Python/Python311/Scripts/yt-dlp.exe")
                .to_string_lossy()
                .to_string(),
        );
    }

    candidates.extend(vec![
        "yt-dlp".to_string(),
        "yt-dlp.exe".to_string(),
        "/usr/local/bin/yt-dlp".to_string(),
        "/usr/bin/yt-dlp".to_string(),
    ]);

    for c in &candidates {
        if create_command(c)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(c.to_string());
        }
    }
    None
}

fn get_ffmpeg_path(app: &AppHandle) -> Option<String> {
    let resource_path = app.path().resource_dir().ok()?;
    let ffmpeg_bin = resource_path.join("assets").join("bin");
    let ffmpeg_exe = if cfg!(windows) {
        ffmpeg_bin.join("ffmpeg.exe")
    } else {
        ffmpeg_bin.join("ffmpeg")
    };
    if ffmpeg_exe.exists() {
        return Some(ffmpeg_exe.to_string_lossy().to_string());
    }
    
    // Also check old location for compatibility
    let old_ffmpeg = resource_path.join("assets").join("ffmpeg").join("bin").join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" });
    if old_ffmpeg.exists() {
        return Some(old_ffmpeg.to_string_lossy().to_string());
    }
    
    // Fallback to system ffmpeg
    if create_command("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some("ffmpeg".to_string());
    }
    None
}

fn get_default_save_path(app: &AppHandle) -> PathBuf {
    if let Ok(dl) = app.path().download_dir() {
        return dl;
    }
    let mut p = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("Downloads");
    std::fs::create_dir_all(&p).ok();
    p
}

fn load_history(path: &PathBuf) -> Vec<HistoryEntry> {
    if let Ok(data) = std::fs::read_to_string(path) {
        if let Ok(entries) = serde_json::from_str::<Vec<HistoryEntry>>(&data) {
            return entries;
        }
    }
    Vec::new()
}

fn save_history(path: &PathBuf, entries: &[HistoryEntry]) {
    if let Ok(data) = serde_json::to_string_pretty(entries) {
        std::fs::write(path, data).ok();
    }
}

fn normalize_url(url: &str) -> String {
    let u = url.trim();
    // Normalize kick.com URLs
    if let Some(m) =
        regex_lite::Regex::new(r"https?://(?:www\.)?kick\.com/[^/]+/videos/([0-9a-fA-F-]{6,})")
            .ok()
            .and_then(|re| re.captures(u))
    {
        return format!("https://kick.com/video/{}", &m[1]);
    }
    u.to_string()
}

// ─── Commands ─────────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
async fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    let mut s = state.settings.lock().map_err(|e| e.to_string())?;
    *s = settings.clone();

    // Persist to disk
    let path_lock = state.settings_path.lock().map_err(|e| e.to_string())?;
    if let Some(ref path) = *path_lock {
        if let Ok(data) = serde_json::to_string_pretty(&settings) {
            std::fs::write(path, data).ok();
        }
    }

    Ok(())
}

#[tauri::command]
async fn fetch_video_info(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    task_id: String,
) -> Result<VideoInfo, String> {
    let normalized = normalize_url(&url);
    let ytdlp =
        get_ytdlp_path(&app).ok_or_else(|| "yt-dlp not found. Please install it.".to_string())?;

    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();

    // Emit fetching status
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            task_id: task_id.clone(),
            status: "fetching_info".to_string(),
            percent: 0.0,
            speed: None,
            eta: None,
            message: None,
        },
    );

    let mut cmd = create_command(&ytdlp);
    cmd.args(["--dump-json", "--no-playlist", "--quiet", "--no-warnings"]);

    // Add cookies if configured
    if settings.use_cookies {
        if settings.cookie_source_type == "file" {
            if let Some(ref cookie_path) = settings.cookies_path {
                if std::path::Path::new(cookie_path).exists() {
                    cmd.args(["--cookies", cookie_path]);
                }
            }
        } else if let Some(ref browser) = settings.cookie_browser {
            if browser != "none" {
                cmd.args(["--cookies-from-browser", browser]);
            }
        }
    }

    cmd.arg(&normalized);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = tokio::task::spawn_blocking(move || cmd.output())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(err);
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

    let info = VideoInfo {
        id: json["id"].as_str().unwrap_or("").to_string(),
        title: json["title"]
            .as_str()
            .unwrap_or("Unknown Title")
            .to_string(),
        thumbnail: json["thumbnail"].as_str().map(|s| s.to_string()),
        platform: json["extractor_key"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string(),
        duration: json["duration"].as_f64(),
        uploader: json["uploader"].as_str().map(|s| s.to_string()),
        url: normalized,
    };

    Ok(info)
}

#[tauri::command]
async fn start_download(
    app: AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    url: String,
    format: String,
) -> Result<(), String> {
    let normalized = normalize_url(&url);
    let ytdlp = get_ytdlp_path(&app).ok_or_else(|| "yt-dlp not found".to_string())?;
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();

    let save_path = match &settings.save_path {
        Some(p) if std::path::Path::new(p).is_dir() => PathBuf::from(p),
        _ => get_default_save_path(&app),
    };

    let save_str = save_path.to_string_lossy().to_string();
    let ytdlp_clone = ytdlp.clone();
    let task_id_clone = task_id.clone();
    let normalized_clone = normalized.clone();
    let format_clone = format.clone();
    let settings_clone = settings.clone();
    let app_clone = app.clone();

    tokio::task::spawn(async move {
        let result = run_download(
            app_clone,
            ytdlp_clone,
            task_id_clone,
            normalized_clone,
            format_clone,
            save_str,
            settings_clone,
        )
        .await;
        if let Err(e) = result {
            eprintln!("Download error: {}", e);
        }
    });

    Ok(())
}

async fn run_download(
    app: AppHandle,
    ytdlp: String,
    task_id: String,
    url: String,
    format: String,
    save_path: String,
    settings: Settings,
) -> Result<(), String> {
    let emit_progress = |status: &str,
                         percent: f64,
                         speed: Option<&str>,
                         eta: Option<&str>,
                         message: Option<&str>| {
        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                task_id: task_id.clone(),
                status: status.to_string(),
                percent,
                speed: speed.map(|s| s.to_string()),
                eta: eta.map(|s| s.to_string()),
                message: message.map(|s| s.to_string()),
            },
        );
    };

    emit_progress("downloading", 0.0, None, None, Some("Starting..."));

    let outtmpl = format!("{}/%(title)s [%(id)s].%(ext)s", save_path);

    let mut args: Vec<String> = vec![
        "--newline".to_string(),
        "--progress".to_string(),
        "-o".to_string(),
        outtmpl,
        "--no-playlist".to_string(),
        "--socket-timeout".to_string(),
        "30".to_string(),
        "--retries".to_string(),
        "10".to_string(),
    ];

    if let Some(ffmpeg) = get_ffmpeg_path(&app) {
        args.extend(["--ffmpeg-location".to_string(), ffmpeg]);
    }

    // Format / postprocessors
    let is_audio_only = format.starts_with("bestaudio");
    let is_video_only = format == "video_only_stripped";

    if is_audio_only {
        args.extend(["--format".to_string(), format.clone()]);
        args.extend([
            "--extract-audio".to_string(),
            "--audio-format".to_string(),
            "mp3".to_string(),
            "--audio-quality".to_string(),
            "192K".to_string(),
        ]);
    } else if is_video_only {
        args.extend([
            "--format".to_string(),
            "bestvideo[ext=mp4]/bestvideo/best".to_string(),
        ]);
        args.extend(["--merge-output-format".to_string(), "mp4".to_string()]);
    } else {
        args.extend(["--format".to_string(), format.clone()]);
        args.extend(["--merge-output-format".to_string(), "mp4".to_string()]);
    }

    if settings.subtitles_enabled {
        args.extend([
            "--write-subs".to_string(),
            "--sub-langs".to_string(),
            "en,ru,uk".to_string(),
        ]);
    }

    // Cookies
    if settings.use_cookies {
        if settings.cookie_source_type == "file" {
            if let Some(ref cp) = settings.cookies_path {
                if std::path::Path::new(cp).exists() {
                    args.extend(["--cookies".to_string(), cp.clone()]);
                }
            }
        } else if let Some(ref browser) = settings.cookie_browser {
            if browser != "none" {
                args.extend(["--cookies-from-browser".to_string(), browser.clone()]);
            }
        }
    }

    args.push(url.clone());

    let mut child = create_tokio_command(&ytdlp)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| e.to_string())?;

    use tokio::io::AsyncBufReadExt;
    if let Some(stdout) = child.stdout.take() {
        let mut reader = tokio::io::BufReader::new(stdout).lines();
        let mut _final_path: Option<String> = None;

        while let Ok(Some(line)) = reader.next_line().await {
            // Parse yt-dlp progress lines like:
            // [download]  45.3% of   10.00MiB at    1.23MiB/s ETA 00:05
            if line.contains("[download]") && line.contains('%') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(pct_str) = parts.iter().find(|p| p.ends_with('%')) {
                    if let Ok(pct) = pct_str.trim_end_matches('%').parse::<f64>() {
                        // Scale to 0-90%
                        let scaled = pct * 0.9;
                        let speed = parts
                            .iter()
                            .position(|&p| p == "at")
                            .and_then(|i| parts.get(i + 1))
                            .map(|s| s.to_string());
                        let eta = parts
                            .iter()
                            .position(|&p| p == "ETA")
                            .and_then(|i| parts.get(i + 1))
                            .map(|s| s.to_string());
                        emit_progress(
                            "downloading",
                            scaled,
                            speed.as_deref(),
                            eta.as_deref(),
                            None,
                        );
                    }
                }
            } else if line.contains("[Merger]") || line.contains("[ffmpeg]") {
                emit_progress("processing", 92.0, None, None, Some("Processing..."));
            } else if line.contains("Destination:") || line.contains("has already been downloaded")
            {
                // Try to extract file path
                if let Some(idx) = line.find("Destination:") {
                    let path = line[idx + 12..].trim().to_string();
                    _final_path = Some(path);
                }
            }
        }
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;

    if status.success() {
        emit_progress("completed", 100.0, None, None, None);
    } else {
        // Read stderr for error
        emit_progress("error", 0.0, None, None, Some("Download failed"));
    }

    Ok(())
}

#[tauri::command]
async fn stop_download(_state: State<'_, AppState>, _task_id: String) -> Result<(), String> {
    // Signal to stop (for future implementation with process tracking)
    Ok(())
}

#[tauri::command]
async fn check_ytdlp(app: AppHandle) -> Result<String, String> {
    let ytdlp = get_ytdlp_path(&app);
    match ytdlp {
        Some(path) => {
            let output = create_command(&path)
                .arg("--version")
                .output()
                .map_err(|e| e.to_string())?;
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        None => Err("yt-dlp not found".to_string()),
    }
}

#[tauri::command]
async fn check_ffmpeg(app: AppHandle) -> Result<String, String> {
    match get_ffmpeg_path(&app) {
        Some(path) => {
            let output = create_command(&path)
                .arg("-version")
                .output()
                .map_err(|e| e.to_string())?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let version_line = stdout.lines().next().unwrap_or("unknown").to_string();
            Ok(version_line)
        }
        None => Err("ffmpeg not found".to_string()),
    }
}

#[tauri::command]
async fn update_ytdlp(app: AppHandle) -> Result<String, String> {
    let ytdlp = get_ytdlp_path(&app).ok_or("yt-dlp not found")?;

    let ytdlp_clone = ytdlp.clone();
    let output = tokio::task::spawn_blocking(move || {
        create_command(&ytdlp_clone)
            .arg("-U")
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        let msg = format!("{}\n{}", stdout, stderr).trim().to_string();
        if !msg.is_empty() {
            return Ok(msg);
        }
        return Ok("yt-dlp updated successfully".to_string());
    }

    let python = if cfg!(windows) { "python" } else { "python3" };

    // Fallback: try with --break-system-packages for newer Linux distributions or global python installs
    let pip_output = tokio::task::spawn_blocking(move || {
        create_command(python)
            .args([
                "-m",
                "pip",
                "install",
                "-U",
                "yt-dlp[curl-cffi]",
                "--user",
                "--break-system-packages",
            ])
            .output()
    })
    .await;

    if let Ok(Ok(out)) = pip_output {
        if out.status.success() {
            return Ok("yt-dlp updated successfully via pip".to_string());
        }
    }

    Err(format!(
        "{}\n(If you are on Windows, try running the app as Administrator to update the bundled version)",
        stderr
    ))
}

#[tauri::command]
async fn check_ytdlp_update(app: AppHandle) -> Result<serde_json::Value, String> {
    // Get current version
    let ytdlp = get_ytdlp_path(&app).ok_or("yt-dlp not found")?;
    let output = create_command(&ytdlp)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;
    let current = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Check PyPI
    let client = reqwest::Client::new();
    let resp = client
        .get("https://pypi.org/pypi/yt-dlp/json")
        .header("User-Agent", "UniversalMediaDownloader/1.1")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let latest = data["info"]["version"].as_str().unwrap_or("").to_string();

    // Parse versions into numbers to compare correctly (e.g. 2026.03.03 == 2026.3.3)
    let current_parsed: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
    let latest_parsed: Vec<u32> = latest.split('.').filter_map(|s| s.parse().ok()).collect();

    let has_update = !latest.is_empty() && latest_parsed > current_parsed;

    Ok(serde_json::json!({
        "current": current,
        "latest": latest,
        "has_update": has_update
    }))
}

#[tauri::command]
async fn check_deno(app: AppHandle) -> Result<bool, String> {
    // Priority 1: Check bundled assets
    if let Ok(resource_path) = app.path().resource_dir() {
        let bundled = resource_path.join("assets").join("bin").join(if cfg!(windows) { "deno.exe" } else { "deno" });
        if bundled.exists() {
            return Ok(true);
        }
    }

    // Check system deno
    if create_command("deno")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Ok(true);
    }

    // Check ~/.deno/bin/deno
    if let Some(home) = dirs_next::home_dir() {
        let deno_path = home.join(".deno/bin/deno");
        if create_command(&deno_path)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(true);
        }
    }

    Ok(false)
}

#[tauri::command]
async fn open_folder(path: String) -> Result<(), String> {
    let cmd = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    create_command(cmd)
        .arg(&path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_file(path: String) -> Result<(), String> {
    let cmd = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    create_command(cmd)
        .arg(&path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_default_download_path(app: AppHandle) -> Result<String, String> {
    Ok(get_default_save_path(&app).to_string_lossy().to_string())
}

// ─── History commands ─────────────────────────────────────────────────────────

#[tauri::command]
async fn get_history(state: State<'_, AppState>) -> Result<Vec<HistoryEntry>, String> {
    let path = state.history_path.lock().map_err(|e| e.to_string())?;
    match path.as_ref() {
        Some(p) => Ok(load_history(p)),
        None => Ok(Vec::new()),
    }
}

#[tauri::command]
async fn add_history_entry(
    state: State<'_, AppState>,
    url: String,
    title: String,
    platform: String,
    status: String,
    file_path: Option<String>,
) -> Result<(), String> {
    let path = state.history_path.lock().map_err(|e| e.to_string())?;
    if let Some(p) = path.as_ref() {
        let mut entries = load_history(p);
        let id = entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        let now = chrono::Local::now().to_rfc3339();
        entries.insert(
            0,
            HistoryEntry {
                id,
                url,
                title,
                platform,
                status,
                file_path,
                date: now,
            },
        );
        if entries.len() > 500 {
            entries.truncate(500);
        }
        save_history(p, &entries);
    }
    Ok(())
}

#[tauri::command]
async fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    let path = state.history_path.lock().map_err(|e| e.to_string())?;
    if let Some(p) = path.as_ref() {
        save_history(p, &[]);
    }
    Ok(())
}

#[tauri::command]
async fn remove_history_entry(state: State<'_, AppState>, id: u64) -> Result<(), String> {
    let path = state.history_path.lock().map_err(|e| e.to_string())?;
    if let Some(p) = path.as_ref() {
        let mut entries = load_history(p);
        entries.retain(|e| e.id != id);
        save_history(p, &entries);
    }
    Ok(())
}

#[tauri::command]
async fn get_logs_path(app: AppHandle) -> Result<String, String> {
    let log_dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&log_dir).ok();
    Ok(log_dir.to_string_lossy().to_string())
}

#[tauri::command]
async fn detect_browsers() -> Result<Vec<String>, String> {
    let mut found = vec!["none".to_string()];
    let browsers = vec![
        (
            "chrome",
            vec!["google-chrome", "google-chrome-stable", "chrome"],
        ),
        ("firefox", vec!["firefox"]),
        ("brave", vec!["brave-browser", "brave"]),
        ("edge", vec!["microsoft-edge", "msedge"]),
        ("chromium", vec!["chromium-browser", "chromium"]),
        ("opera", vec!["opera"]),
        ("vivaldi", vec!["vivaldi"]),
    ];

    for (name, commands) in &browsers {
        for cmd in commands {
            if create_command("which")
                .arg(cmd)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                found.push(name.to_string());
                break;
            }
        }
    }
    Ok(found)
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .setup(|app| {
            // Initialize paths
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            let history_path = data_dir.join("history.json");
            let settings_path = data_dir.join("settings.json");

            let state = app.state::<AppState>();

            *state.history_path.lock().unwrap() = Some(history_path);
            *state.settings_path.lock().unwrap() = Some(settings_path.clone());

            // Load settings if they exist
            if let Ok(data) = std::fs::read_to_string(&settings_path) {
                if let Ok(s) = serde_json::from_str::<Settings>(&data) {
                    *state.settings.lock().unwrap() = s;
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            fetch_video_info,
            start_download,
            stop_download,
            check_ytdlp,
            check_ffmpeg,
            update_ytdlp,
            check_ytdlp_update,
            check_deno,
            open_folder,
            open_file,
            get_default_download_path,
            get_history,
            add_history_entry,
            clear_history,
            remove_history_entry,
            get_logs_path,
            detect_browsers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
