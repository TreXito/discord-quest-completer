// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use once_cell::sync::OnceCell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{path::BaseDirectory, AppHandle, Emitter, Listener, Manager};

mod rpc;
mod runner;

// Global static instance of the Discord client
static DISCORD_CLIENT: OnceCell<Mutex<Option<rpc::Client>>> = OnceCell::new();

fn get_discord_client() -> &'static Mutex<Option<rpc::Client>> {
    DISCORD_CLIENT.get_or_init(|| Mutex::new(None))
}

fn runner_resource_name() -> &'static str {
    #[cfg(target_os = "windows")]
    let runner_name = "data/src-win.exe";

    #[cfg(target_os = "linux")]
    let runner_name = "data/src-linux";

    #[cfg(target_os = "macos")]
    let runner_name = "data/src-darwin";

    runner_name
}

fn is_app_bundle(executable_name: &str) -> bool {
    cfg!(target_os = "macos") && executable_name.ends_with(".app")
}

fn bundle_binary_name(app_name: &str) -> String {
    app_name
        .strip_suffix(".app")
        .unwrap_or(app_name)
        .to_string()
}

fn game_folder_path(exe_dir: &Path, path: &str, app_id: i64) -> PathBuf {
    let normalized_path = Path::new(path).to_string_lossy().to_string();

    exe_dir
        .join("games")
        .join(app_id.to_string())
        .join(normalized_path)
}

/// Where fake games are written.
///
/// Windows keeps them next to the executable (that is how the original app
/// works, and it keeps everything in one portable folder). Linux moves them to
/// the XDG data dir instead: an AppImage is a read-only squashfs mount and
/// `/usr/bin` is not writable either, so "next to the executable" simply fails
/// there. The CLI (`cli/dqc`) uses exactly this path, so both front ends share
/// the same fake games, pid files and manifests.
fn fake_games_base_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = env::var("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("discord-quest-completer");
            }
        }
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(".local/share/discord-quest-completer");
        }
        PathBuf::from("/tmp/discord-quest-completer")
    }

    #[cfg(not(target_os = "linux"))]
    {
        let exe_path = env::current_exe().unwrap_or_default();
        exe_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf()
    }
}

/// Directory holding one fake game, always `<base dir>/games/<app id>`.
/// The pid file and the manifest live here, the executable may be nested.
fn game_root_path(exe_dir: &Path, app_id: i64) -> PathBuf {
    exe_dir.join("games").join(app_id.to_string())
}

fn pid_file_path(exe_dir: &Path, app_id: i64) -> PathBuf {
    game_root_path(exe_dir, app_id).join(".dqc.pid")
}

fn manifest_file_path(exe_dir: &Path, app_id: i64) -> PathBuf {
    game_root_path(exe_dir, app_id).join(".dqc.json")
}

/// The detectable list is attacker-adjacent data: it is fetched over the
/// network and its names become file paths. Refuse anything that could escape
/// the games directory instead of silently "fixing" it.
fn validate_executable_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("empty executable name".into());
    }
    if name.contains('\0') {
        return Err("executable name contains a NUL byte".into());
    }
    let normalized = name.replace('\\', "/");
    if normalized.starts_with('/') || normalized.starts_with("~") {
        return Err(format!("executable name must be relative: {name}"));
    }
    for segment in normalized.split('/') {
        if segment == ".." {
            return Err(format!("executable name escapes the games folder: {name}"));
        }
    }
    Ok(())
}

/// `arguments` in the detectable list is a single string like `-game tf`.
/// Whitespace splitting is enough for the handful of entries that use it and
/// keeps us from pulling in a shell-word parser.
fn split_arguments(arguments: &str) -> Vec<String> {
    arguments
        .split_whitespace()
        .map(|part| part.to_string())
        .collect()
}

#[cfg(target_os = "linux")]
fn pids_under(prefix: &Path) -> Vec<i32> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return found;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if let Ok(exe) = fs::read_link(format!("/proc/{}/exe", name)) {
            if exe.starts_with(prefix) {
                if let Ok(pid) = name.parse::<i32>() {
                    found.push(pid);
                }
            }
        }
    }
    found
}

#[cfg(target_os = "linux")]
fn pid_is_alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(target_os = "linux")]
fn terminate_pids(pids: &[i32]) {
    use std::thread::sleep;
    use std::time::Duration;

    for pid in pids {
        unsafe { libc::kill(*pid, libc::SIGTERM) };
    }
    for _ in 0..15 {
        if pids.iter().all(|pid| !pid_is_alive(*pid)) {
            return;
        }
        sleep(Duration::from_millis(100));
    }
    for pid in pids {
        if pid_is_alive(*pid) {
            unsafe { libc::kill(*pid, libc::SIGKILL) };
        }
    }
}

fn resolve_runner_template(handle: &AppHandle) -> Result<PathBuf, String> {
    handle
        .path()
        .resolve(runner_resource_name(), BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve runner template: {}", e))
}

fn make_macos_app_bundle(
    app_bundle_path: &Path,
    app_name: &str,
    display_name: &str,
    runner_template: &Path,
) -> Result<PathBuf, String> {
    let binary_name = bundle_binary_name(app_name);
    let macos_dir = app_bundle_path.join("Contents/MacOS");
    fs::create_dir_all(&macos_dir)
        .map_err(|e| format!("Failed to create app bundle directories: {}", e))?;

    let target_executable_path = macos_dir.join(&binary_name);
    fs::copy(runner_template, &target_executable_path)
        .map_err(|e| format!("Failed to copy dummy executable: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&target_executable_path)
            .map_err(|e| format!("Failed to read executable permissions: {}", e))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&target_executable_path, permissions)
            .map_err(|e| format!("Failed to set executable permissions: {}", e))?;
    }

    let info_plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDisplayName</key>
    <string>{display_name}</string>
    <key>CFBundleExecutable</key>
    <string>{binary_name}</string>
    <key>CFBundleIdentifier</key>
    <string>me.markterence.discordquestcompleter.dummy</string>
    <key>CFBundleName</key>
    <string>{display_name}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
</dict>
</plist>
"#
    );

    fs::write(app_bundle_path.join("Contents/Info.plist"), info_plist)
        .map_err(|e| format!("Failed to write Info.plist: {}", e))?;

    Ok(target_executable_path)
}

fn launch_executable_path(game_folder_path: &Path, executable_name: &str) -> PathBuf {
    if is_app_bundle(executable_name) {
        game_folder_path
            .join(executable_name)
            .join("Contents/MacOS")
            .join(bundle_binary_name(executable_name))
    } else {
        game_folder_path.join(executable_name)
    }
}

fn stop_process_name(exec_name: &str) -> String {
    if is_app_bundle(exec_name) {
        bundle_binary_name(exec_name)
    } else {
        exec_name.to_string()
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command(rename_all = "snake_case")]
async fn create_fake_game(
    handle: tauri::AppHandle,
    path: &str,
    executable_name: &str,
    path_len: i64,
    app_id: i64,
    display_name: Option<String>,
    arguments: Option<String>,
) -> Result<String, String> {
    let _ = path_len;
    let exe_dir = fake_games_base_dir();

    validate_executable_name(executable_name)?;
    let game_folder_path = game_folder_path(&exe_dir, path, app_id);

    println!("Game folder path: {:?}", game_folder_path);
    println!(
        "Game full path: {:?}",
        game_folder_path.join(executable_name)
    );

    match fs::create_dir_all(&game_folder_path) {
        Ok(_) => {
            println!("Successfully created directory: {:?}", game_folder_path);
        }
        Err(e) => return Err(format!("Failed to create game folder: {}", e)),
    };

    let resource_path = resolve_runner_template(&handle)?;
    println!("Creating dummy game executable from: {:?}", resource_path);

    if is_app_bundle(executable_name) {
        let app_bundle_path = game_folder_path.join(executable_name);
        let bundle_display_name = display_name
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| bundle_binary_name(executable_name));
        let target_executable_path = make_macos_app_bundle(
            &app_bundle_path,
            executable_name,
            &bundle_display_name,
            &resource_path,
        )?;
        return Ok(format!(
            "Dummy app bundle created at: {:?}",
            target_executable_path
        ));
    }

    let target_executable_path = game_folder_path.join(executable_name);
    match fs::copy(&resource_path, &target_executable_path) {
        Ok(_) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&target_executable_path)
                    .map_err(|e| format!("Failed to get file metadata: {}", e))?
                    .permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&target_executable_path, perms)
                    .map_err(|e| format!("Failed to set executable permissions: {}", e))?;
            }

            // Manifest + pid file live at games/<app id>/ so the CLI and this
            // app agree on where a fake game is, even when the executable sits
            // in a nested folder ("bin/x64/game.exe").
            let manifest = serde_json::json!({
                "app_id": app_id,
                "app_name": display_name.clone().unwrap_or_default(),
                "executable": executable_name,
                "arguments": arguments.clone().unwrap_or_default(),
                "path": target_executable_path.to_string_lossy(),
            });
            let _ = fs::write(
                manifest_file_path(&exe_dir, app_id),
                serde_json::to_vec_pretty(&manifest).unwrap_or_default(),
            );

            Ok(format!(
                "Dummy executable copied to: {:?}",
                target_executable_path
            ))
        }
        Err(e) => Err(format!("Failed to copy dummy executable: {}", e)),
    }
}

#[cfg(target_os = "macos")]
fn launch_macos_app_bundle(app_bundle_path: &Path, title: &str) -> Result<(), String> {
    let mut command = std::process::Command::new("open");
    command
        .arg("-n")
        .arg("-g")
        .arg("-a")
        .arg(app_bundle_path)
        .arg("--args")
        .arg("--title")
        .arg(title);

    command
        .spawn()
        .map_err(|e| format!("Failed to launch app bundle with open: {}", e))?;

    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
async fn run_background_process(
    name: &str,
    path: &str,
    executable_name: &str,
    path_len: i64,
    app_id: i64,
    arguments: Option<String>,
) -> Result<String, String> {
    let _ = path_len;
    validate_executable_name(executable_name)?;

    let exe_dir = fake_games_base_dir();

    let game_folder_path = game_folder_path(&exe_dir, path, app_id);

    if is_app_bundle(executable_name) {
        #[cfg(target_os = "macos")]
        {
            let bundle_path = game_folder_path.join(executable_name);
            launch_macos_app_bundle(&bundle_path, name)?;
            return Ok("App bundle launched successfully".to_string());
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = name;
            return Err("App bundle launches are only supported on macOS".to_string());
        }
    }

    let executable_path = launch_executable_path(&game_folder_path, executable_name);

    if !executable_path.exists() {
        return Err(format!(
            "fake game executable is missing, create it first: {:?}",
            executable_path
        ));
    }

    let mut cmd = std::process::Command::new(&executable_path);
    cmd.arg("--title")
        .arg(name)
        .args(split_arguments(arguments.as_deref().unwrap_or("")))
        .current_dir(&game_folder_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    // Platform-specific process spawning
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0); // Create new process group on Unix
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start process: {}", e))?;
    let pid = child.id();

    // Reap it when it exits, otherwise stopping a fake game leaves a zombie
    // behind for as long as this app keeps running.
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    // Let the child outlive this app, and leave a pid file behind so a later
    // run (or `dqc status`) can see and stop it.
    let _ = fs::write(pid_file_path(&exe_dir, app_id), pid.to_string());

    Ok(format!("Process started successfully (pid {})", pid))
}

#[tauri::command(rename_all = "snake_case")]
async fn stop_process(
    exec_name: String,
    app_id: Option<i64>,
    path: Option<String>,
) -> Result<(), String> {
    let process_name = stop_process_name(&exec_name);

    #[cfg(target_os = "windows")]
    {
        let _ = (app_id, path);
        let output = std::process::Command::new("taskkill")
            .arg("/F")
            .arg("/IM")
            .arg(&process_name)
            .output()
            .map_err(|e| format!("Failed to execute taskkill: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "Failed to stop process: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    #[cfg(target_os = "macos")]
    {
        let _ = (app_id, path);
        if is_app_bundle(&exec_name) {
            let bundle_pattern = format!("{}/Contents/MacOS", exec_name);
            let output = std::process::Command::new("pkill")
                .arg("-f")
                .arg(&bundle_pattern)
                .output()
                .map_err(|e| format!("Failed to execute pkill: {}", e))?;

            if output.status.success() {
                return Ok(());
            }
        }

        let output = std::process::Command::new("pkill")
            .arg("-x")
            .arg(&process_name)
            .output()
            .map_err(|e| format!("Failed to execute pkill: {}", e))?;

        if output.status.success() || output.status.code() == Some(1) {
            Ok(())
        } else {
            Err(format!(
                "Failed to stop process: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    // Linux: never `pkill -f <name>`. That pattern also matches this app, the
    // terminal that launched it and anything else with the string in its
    // command line. Kill the exact processes whose /proc/<pid>/exe lives in
    // this app's games folder instead.
    #[cfg(target_os = "linux")]
    {
        let _ = process_name;
        let exe_dir = fake_games_base_dir();
        let mut pids: Vec<i32> = Vec::new();

        let root = app_id.map(|id| game_root_path(&exe_dir, id)).or_else(|| {
            path.as_deref()
                .filter(|p| !p.is_empty())
                .map(|p| game_folder_path(&exe_dir, p, app_id.unwrap_or(0)))
        });

        if let Some(root) = root.as_ref() {
            // the recorded pid is only trusted while its executable still lives
            // in this app's games folder, so a recycled pid is never signalled
            if app_id.is_some() {
                let recorded = pid_file_path(&exe_dir, app_id.unwrap_or(0));
                if let Ok(raw) = fs::read_to_string(&recorded) {
                    if let Ok(pid) = raw.trim().parse::<i32>() {
                        let owned = fs::read_link(format!("/proc/{}/exe", pid))
                            .map(|exe| exe.starts_with(root))
                            .unwrap_or(false);
                        if owned && pid_is_alive(pid) {
                            pids.push(pid);
                        }
                    }
                }
            }
            for pid in pids_under(root) {
                if !pids.contains(&pid) {
                    pids.push(pid);
                }
            }
        }

        if pids.is_empty() {
            return Ok(());
        }

        terminate_pids(&pids);

        if let Some(id) = app_id {
            let _ = fs::remove_file(pid_file_path(&exe_dir, id));
        }

        Ok(())
    }
}

/// Fake games that already exist on disk, so the UI can restore its state
/// after a restart instead of claiming nothing is installed.
#[tauri::command(rename_all = "snake_case")]
async fn list_fake_games() -> Vec<serde_json::Value> {
    let exe_dir = fake_games_base_dir();
    let games_dir = exe_dir.join("games");
    let mut out = Vec::new();

    let Ok(entries) = fs::read_dir(&games_dir) else {
        return out;
    };

    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let folder_name = entry.file_name().to_string_lossy().to_string();
        let Ok(app_id) = folder_name.parse::<i64>() else {
            continue;
        };

        let mut manifest: serde_json::Value = fs::read_to_string(manifest_file_path(&exe_dir, app_id))
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        #[cfg(target_os = "linux")]
        let pids = {
            let mut pids = pids_under(&game_root_path(&exe_dir, app_id));
            if let Ok(raw) = fs::read_to_string(pid_file_path(&exe_dir, app_id)) {
                if let Ok(pid) = raw.trim().parse::<i32>() {
                    if pid_is_alive(pid) && !pids.contains(&pid) {
                        pids.push(pid);
                    }
                }
            }
            pids
        };
        #[cfg(not(target_os = "linux"))]
        let pids: Vec<i32> = Vec::new();

        if let Some(object) = manifest.as_object_mut() {
            object.insert("app_id".into(), serde_json::json!(app_id));
            object.insert("running".into(), serde_json::json!(!pids.is_empty()));
            object.insert("pids".into(), serde_json::json!(pids));
        }
        out.push(manifest);
    }

    out
}

/// Usage: Calling from JS:
/// ```javascript
/// await invoke('connect_to_discord_rpc_3', json, 'connect' | 'disconnect');
#[tauri::command(rename_all = "snake_case")]
fn connect_to_discord_rpc_3(handle: AppHandle, activity_json: String, action: String) {
    let app = handle.clone();

    let event_connecting = "client_connecting";
    let event_connected = "client_connected";
    let event_disconnect = "event_disconnect";
    let event_connect = "event_connect";

    let activity = runner::parse_activity_json(&activity_json).unwrap();

    let connecting_payload = serde_json::json!({
        "app_id": activity.app_id,
    });

    let client_option = {
        let mut client_guard = get_discord_client().lock().unwrap();
        // Take the client out, leaving None in its place
        client_guard.take()
        // MutexGuard is dropped here at the end of scope
    };

    let task = tauri::async_runtime::spawn(async move {
        handle
            .emit(event_connecting, connecting_payload)
            .unwrap_or_else(|e| eprintln!("Failed to emit event: {}", e));

        let client = runner::set_activity(activity_json)
            .await
            .map_err(|e| {
                println!("Failed to set activity: {}", e);
            })
            .unwrap();

        let connected_payload = serde_json::json!({
            "app_id": activity.app_id,
        });

        {
            let mut client_guard = get_discord_client().lock().unwrap();
            *client_guard = Some(client);
        }

        handle
            .emit(event_connected, connected_payload)
            .unwrap_or_else(|e| {
                eprintln!("Failed to emit event: {}", e);
            });

        handle.listen(event_disconnect, move |_| {
            println!("Disconnecting from Discord RPC inner");
            let disconnect_task = tauri::async_runtime::spawn(async move {
                let client_option = {
                    let mut client_guard = get_discord_client().lock().unwrap();
                    // Take the client out, leaving None in its place
                    client_guard.take()
                    // MutexGuard is dropped here at the end of scope
                };
                if let Some(client) = client_option {
                    client.discord.disconnect().await;
                    println!("Disconnected from Discord RPC inner");
                }
            });
            // disconnect_task.abort();
        });
    });

    app.listen(event_disconnect, move |_| {
        println!("Disconnecting from Discord RPC...");
        task.abort();
    });
}

#[tauri::command(rename_all = "snake_case")]
async fn fetch_gamelist_gh_mirror() -> tauri::ipc::Response {
    let res = tauri_plugin_http::reqwest::get("https://markterence.github.io/discord-quest-completer/detectable.json").await;
    tauri::ipc::Response::new(res.unwrap().text().await.unwrap())
}

#[tauri::command(rename_all = "snake_case")]
async fn fetch_gamelist_from_discord() -> tauri::ipc::Response {
    let res = tauri_plugin_http::reqwest::get("https://discord.com/api/applications/detectable").await;
    tauri::ipc::Response::new(res.unwrap().text().await.unwrap())
}

/// The list the Discord client itself uses (24883 applications, all platforms).
/// The public `/applications/detectable` endpoint is a smaller, older mirror of
/// it, so prefer this one: it is what both the Linux and Windows detection paths
/// match against.
#[tauri::command(rename_all = "snake_case")]
async fn fetch_gamelist_from_cdn() -> tauri::ipc::Response {
    let res = tauri_plugin_http::reqwest::get("https://cdn.discordapp.com/detectables/games-v1.json").await;
    tauri::ipc::Response::new(res.unwrap().text().await.unwrap())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            create_fake_game,
            stop_process,
            connect_to_discord_rpc_3,
            run_background_process,
            list_fake_games,
            fetch_gamelist_gh_mirror,
            fetch_gamelist_from_cdn,
            fetch_gamelist_from_discord
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}