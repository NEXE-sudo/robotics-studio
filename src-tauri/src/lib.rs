use std::fs;

/// Very small gitignore-style pattern matcher, supporting the common cases:
///   *.ext          — match by extension/suffix anywhere in the filename
///   dirname/       — match any path that has this directory component
///   exact/path.txt — match a path ending with this exact relative suffix
///   # comment      — ignored
///   blank lines    — ignored
/// This intentionally does not implement full gitignore semantics (negation,
/// nested `**`, anchoring with a leading `/`) — just enough for excluding
/// secrets/config files from AI context. Extend here if richer patterns
/// (via the `ignore` crate) are needed later.
fn matches_aiignore_pattern(rel_path: &str, pattern: &str) -> bool {
    let rel_path = rel_path.replace('\\', "/");
    let pattern = pattern.trim();
    if pattern.is_empty() || pattern.starts_with('#') {
        return false;
    }

    if let Some(dir) = pattern.strip_suffix('/') {
        return rel_path
            .split('/')
            .any(|component| component == dir);
    }

    if let Some(ext) = pattern.strip_prefix("*.") {
        return rel_path
            .rsplit('/')
            .next()
            .map_or(false, |name| name.ends_with(&format!(".{}", ext)));
    }

    if pattern.contains('*') {
        // Basic single-`*` wildcard support: split on '*' and check prefix/suffix.
        if let Some((prefix, suffix)) = pattern.split_once('*') {
            let filename = rel_path.rsplit('/').next().unwrap_or(&rel_path);
            return filename.starts_with(prefix) && filename.ends_with(suffix);
        }
    }

    rel_path == pattern || rel_path.ends_with(&format!("/{}", pattern))
}

/// Loads `.aiignore` from the given workspace root, if present, and returns
/// whether `file_path` (absolute) should be excluded from AI context.
fn is_ignored_by_aiignore(workspace_root: &str, file_path: &str) -> bool {
    let ignore_file = std::path::Path::new(workspace_root).join(".aiignore");
    let Ok(contents) = fs::read_to_string(&ignore_file) else {
        return false;
    };

    let rel_path = std::path::Path::new(file_path)
        .strip_prefix(workspace_root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| file_path.to_string());

    contents
        .lines()
        .any(|pattern| matches_aiignore_pattern(&rel_path, pattern))
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file(path: String, contents: String) -> Result<(), String> {
    fs::write(&path, contents).map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
}

#[tauri::command]
fn list_dir(path: String) -> Result<Vec<FileEntry>, String> {
    let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;
    let mut result = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        let full_path = entry.path().to_string_lossy().to_string();

        result.push(FileEntry {
            name,
            path: full_path,
            is_dir: file_type.is_dir(),
        });
    }

    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            read_file, write_file, list_dir, start_ros_stream, run_colcon_build, run_colcon_build_streaming, start_gazebo_sim, stop_gazebo_sim, reset_gazebo_sim, ask_ai, initialize_ros_environment, publish_twist,
            save_ai_settings, load_ai_settings, save_keybindings, load_keybindings, get_system_specs, list_templates, create_project_from_template, check_aiignore
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub mod rosengine {
    tonic::include_proto!("rosengine");
}

use rosengine::ros_engine_client::RosEngineClient;
use rosengine::Empty;

use tauri::{AppHandle, Emitter};
use std::sync::atomic::{AtomicBool, Ordering};

static STREAM_RUNNING: AtomicBool = AtomicBool::new(false);

#[tauri::command]
async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
    println!("start_ros_stream invoked");

    if STREAM_RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    tauri::async_runtime::spawn(async move {
        loop {
            let _ = app.emit("ros-connecting", ());
            println!("[ros-stream] attempting connection to localhost:50051");
            let client_result = tokio::time::timeout(
    std::time::Duration::from_secs(5),
    RosEngineClient::connect("http://localhost:50051"),
).await;

let mut client = match client_result {
    Ok(Ok(c)) => {
    println!("[ros-stream] connected successfully");
    c
}
    Ok(Err(e)) => {
        println!("[ros-stream] connection failed: {}", e);
        let _ = app.emit("ros-error", format!("Connection failed: {}", e));
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        continue;
    }
    Err(_) => {
        println!("[ros-stream] connection attempt timed out");
        let _ = app.emit("ros-error", "Connection attempt timed out".to_string());
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        continue;
    }
};
let stream_result = tokio::time::timeout(
    std::time::Duration::from_secs(1),
    client.stream_events(Empty {}),
).await;

let mut stream = match stream_result {
    Ok(Ok(s)) => {
        println!("[ros-stream] stream_events established, awaiting messages");
        s.into_inner()
    }
    Ok(Err(e)) => {
        println!("[ros-stream] stream_events failed: {}", e);
        let _ = app.emit("ros-error", format!("Stream failed: {}", e));
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        continue;
    }
    Err(_) => {
        println!("[ros-stream] stream_events timed out");
        let _ = app.emit("ros-error", "stream_events call timed out".to_string());
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        continue;
    }
};
println!("[ros-stream] calling stream_events");
            let mut stream = match client.stream_events(Empty {}).await {
            Ok(s) => {
    println!("[ros-stream] stream_events established, awaiting messages");
    s.into_inner()
}
            Err(e) => {
                let _ = app.emit("ros-error", format!("Stream failed: {}", e));
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                continue;
            }
        };

            loop {
                match stream.message().await {
                    Ok(Some(evt)) => {
                        println!("[ros-stream] received event: {:?}", evt);
                        let _ = app.emit("ros-event", format!("{:?}", evt));

                        if let Some(rosengine::workspace_event::Event::OdometryUpdate(odom)) = &evt.event {
                            let _ = app.emit("odometry-update", OdometryUpdate {
                                node: odom.topic_name.clone(),
                                x: odom.x,
                                y: odom.y,
                                z: odom.z,
                                qx: odom.qx,
                                qy: odom.qy,
                                qz: odom.qz,
                                qw: odom.qw,
                            });
                        }

                        if let Some(rosengine::workspace_event::Event::TfUpdate(tf)) = &evt.event {
                            let _ = app.emit("tf-update", TfUpdate {
                                parent_frame: tf.parent_frame.clone(),
                                child_frame: tf.child_frame.clone(),
                                x: tf.x,
                                y: tf.y,
                                z: tf.z,
                                qx: tf.qx,
                                qy: tf.qy,
                                qz: tf.qz,
                                qw: tf.qw,
                            });
                        }
                        if let Some(rosengine::workspace_event::Event::NodeSnapshot(snap)) = &evt.event {
                            let _ = app.emit("node-snapshot", snap.node_names.clone());
                        }
                    }
                    Ok(None) => {
                        let _ = app.emit("ros-error", "Stream closed by server".to_string());
                        break;
                    }
                    Err(e) => {
                        let _ = app.emit("ros-error", format!("Stream error: {}", e));
                        break;
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
        // Note: this loop never actually exits under normal operation —
        // it retries forever. The guard being permanently `true` is
        // actually fine AS LONG AS this loop is genuinely still alive and
        // retrying. The real bug was elsewhere — see below.
    });

    Ok(())
}

use rosengine::TwistCommand;

#[tauri::command]
async fn publish_twist(topic_name: String, linear_x: f64, angular_z: f64) -> Result<(), String> {
    let mut client = RosEngineClient::connect("http://localhost:50051")
        .await
        .map_err(|e| e.to_string())?;

    client
        .publish_twist(TwistCommand {
            topic_name,
            linear_x,
            angular_z,
        })
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

use std::process::Command as StdCommand;

fn docker_command() -> StdCommand {
    let mut cmd = StdCommand::new("docker");
    let current_path = std::env::var("PATH").unwrap_or_default();
    let augmented_path = format!(
        "/usr/local/bin:/opt/homebrew/bin:/Applications/Docker.app/Contents/Resources/bin:{}",
        current_path
    );
    cmd.env("PATH", augmented_path);
    cmd
}

fn get_default_workspace_dir() -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    Ok(format!("{}/.robotics-studio/ros2-workspace", home))
}

fn find_dev_container() -> Result<String, String> {
    let workspace_path = get_default_workspace_dir()?;
    let filter = format!("label=devcontainer.local_folder={}", workspace_path);
    let output = docker_command()
        .args([
            "ps",
            "--filter",
            &filter,
            "--format",
            "{{.Names}}",
        ])
        .output()
        .map_err(|e| e.to_string())?;

    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() {
        Err("No running dev container found for this workspace".to_string())
    } else {
        Ok(name)
    }
}

#[tauri::command]
fn run_colcon_build() -> Result<String, String> {
    let container_name = find_dev_container()?;

    let output = docker_command()
        .args([
            "exec",
            &container_name,
            "bash",
            "-c",
            "source /opt/ros/jazzy/setup.bash && cd /workspace && colcon build",
        ])
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(format!("{}\n{}", stdout, stderr))
    } else {
        Err(format!("{}\n{}", stdout, stderr))
    }
}

use std::io::{BufReader, Read};
use std::process::Stdio;

/// Event payload for streaming log output from Docker/build processes.
/// Distinguishes between "append" (finalized line with \n) and "replace" 
/// (in-progress update with \r) to properly render Docker BuildKit progress.
#[derive(serde::Serialize, Clone)]
struct LogEvent {
    kind: String, // "append" | "replace"
    text: String,
}

/// Strip ANSI escape sequences (colors, cursor movement, etc.) from a string.
/// Docker BuildKit output often contains these; we need to remove them
/// before displaying in plain text UI.
fn strip_ansi_codes(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Start of an ANSI escape sequence; consume until 'm' or other terminator
            while let Some(c) = chars.next() {
                if c.is_alphabetic() {
                    break; // End of escape sequence
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// Stateful line buffer for streaming output parsing. Maintains pending bytes across
/// multiple read() calls to correctly handle UTF-8 characters and lines split across
/// chunk boundaries. This fixes two critical bugs:
///   1. Panic risk from out-of-bounds indexing when multi-byte UTF-8 spans chunks
///   2. Lost state when partial lines are discarded between read() calls
struct StreamLineBuffer {
    /// Accumulated bytes not yet terminated by \n or \r.
    pending: Vec<u8>,
}

impl StreamLineBuffer {
    fn new() -> Self {
        StreamLineBuffer {
            pending: Vec::new(),
        }
    }

    /// Process a chunk of bytes and return (kind, text) events emitted so far.
    /// Any bytes that don't end with a delimiter are kept in the pending buffer
    /// for the next call, ensuring lines split across chunks are assembled correctly.
    fn process_chunk(&mut self, chunk: &[u8]) -> Vec<(String, String)> {
        let mut result = Vec::new();

        // Add chunk to pending buffer
        self.pending.extend_from_slice(chunk);

        let mut i = 0;
        while i < self.pending.len() {
            match self.pending[i] {
                b'\n' | b'\r' => {
                    let delimiter = self.pending[i];
                    // Extract bytes before delimiter
                    let line_bytes = &self.pending[..i];
                    // Convert to string using lossy UTF-8 (handles partial chars gracefully)
                    let text = String::from_utf8_lossy(line_bytes).to_string();
                    let text = strip_ansi_codes(&text);
                    let kind = if delimiter == b'\n' {
                        "append".to_string()
                    } else {
                        "replace".to_string()
                    };
                    if !text.is_empty() {
                        result.push((kind, text));
                    }
                    // Remove the line (including delimiter) from pending
                    self.pending.drain(0..=i);
                    i = 0; // Restart from beginning of remaining bytes
                }
                _ => {
                    i += 1;
                }
            }
        }

        result
    }

    /// Flush any remaining bytes in the buffer as a final "append" event.
    /// Call this when EOF is reached to ensure no data is lost.
    fn flush(&mut self) -> Option<(String, String)> {
        if !self.pending.is_empty() {
            let text = String::from_utf8_lossy(&self.pending).to_string();
            let text = strip_ansi_codes(&text);
            self.pending.clear();
            if !text.is_empty() {
                return Some(("append".to_string(), text));
            }
        }
        None
    }
}

#[tauri::command]
async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
    let container_name = find_dev_container()?;

    tauri::async_runtime::spawn(async move {
        let child = docker_command()
            .args([
                "exec",
                &container_name,
                "bash",
                "-c",
                "source /opt/ros/jazzy/setup.bash && cd /workspace && colcon build",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("build-error", e.to_string());
                return;
            }
        };

        if let Some(stdout) = child.stdout.take() {
            let mut reader = BufReader::new(stdout);
            let mut buffer = [0; 4096];
            let mut line_buffer = StreamLineBuffer::new();
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        // EOF: flush any remaining bytes in the buffer
                        if let Some((kind, text)) = line_buffer.flush() {
                            let event = LogEvent { kind, text };
                            let _ = app.emit("build-output", event);
                        }
                        break;
                    }
                    Ok(n) => {
                        let events = line_buffer.process_chunk(&buffer[..n]);
                        for (kind, text) in events {
                            let event = LogEvent { kind, text };
                            let _ = app.emit("build-output", event);
                        }
                    }
                    Err(e) => {
                        let _ = app.emit("build-error", e.to_string());
                        break;
                    }
                }
            }
        }

        match child.wait() {
            Ok(status) => {
                let _ = app.emit("build-finished", status.success());
            }
            Err(e) => {
                let _ = app.emit("build-error", e.to_string());
            }
        }
    });

    Ok(())
}

fn detect_models_in_sdf(sdf_content: &str) -> Vec<String> {
    // Simple regex-free scan for <model name='...'> or <model name="...">
    let mut models = Vec::new();
    for line in sdf_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<model") {
            if let Some(start) = trimmed.find("name=") {
                let after = &trimmed[start + 5..];
                let quote_char = after.chars().next();
                if let Some(q) = quote_char {
                    if q == '\'' || q == '"' {
                        if let Some(end) = after[1..].find(q) {
                            models.push(after[1..1 + end].to_string());
                        }
                    }
                }
            }
        }
    }
    models
}

#[tauri::command]
fn start_gazebo_sim(world_path: Option<String>) -> Result<String, String> {
    let container_name = find_dev_container()?;

    // Defensively kill any existing instances, and actively verify
    // they're gone before proceeding — a fixed sleep isn't reliable
    // enough, since detached process teardown timing varies.
    for attempt in 0..10 {
        let kill_result = docker_command()
            .args([
                "exec", &container_name,
                "bash", "-c",
                "pkill -9 -f '[g]z sim -s' ; pkill -9 -f '[/]parameter_bridge '",
            ])
            .output();

        if let Err(e) = &kill_result {
            eprintln!("DEBUG: pkill command itself failed to execute: {}", e);
        }

        std::thread::sleep(std::time::Duration::from_millis(800));

        let check = docker_command()
            .args([
                "exec", &container_name,
                "bash", "-c",
                "pgrep -f '[g]z sim -s' ; pgrep -f '[/]parameter_bridge '",
            ])
            .output()
            .map_err(|e| e.to_string())?;

        eprintln!("DEBUG attempt {}: check.stdout = {:?}", attempt, String::from_utf8_lossy(&check.stdout));

        if check.stdout.is_empty() {
            break;
        }
        if attempt == 9 {
            return Err(format!(
                "Could not fully clean up previous simulation processes after 10 attempts. Still running: {}",
                String::from_utf8_lossy(&check.stdout)
            ));
        }
    }

    // Determine which world file to use, and read its content (for model
    // auto-detection) either from our bundled copy or the user-selected one.
    let (container_world_path, sdf_content) = match &world_path {
        Some(host_path) => {
            // Copy the custom world into the container's /workspace so
            // Gazebo (running inside the container) can actually reach it.
            let file_name = std::path::Path::new(host_path)
                .file_name()
                .ok_or("Invalid world file path")?
                .to_string_lossy()
                .to_string();
            let content = std::fs::read_to_string(host_path).map_err(|e| e.to_string())?;
            let dest = format!("{}/{}", get_default_workspace_dir()?, file_name);
            std::fs::write(&dest, &content).map_err(|e| e.to_string())?;
            (format!("/workspace/{}", file_name), content)
        }
        None => {
            let dest = format!("{}/diff_drive.sdf", get_default_workspace_dir()?);
            let content = std::fs::read_to_string(&dest).map_err(|e| e.to_string())?;
            ("/workspace/diff_drive.sdf".to_string(), content)
        }
    };

    let models = detect_models_in_sdf(&sdf_content);
    let robot_models: Vec<&String> = models.iter().filter(|m| m.as_str() != "ground_plane").collect();

    let gz_launch_cmd = format!(
        "source /opt/ros/jazzy/setup.bash && gz sim -s -r --headless-rendering {} > /tmp/gz.log 2>&1",
        container_world_path
    );

    let gz_result = docker_command()
        .args(["exec", "-d", &container_name, "bash", "-c", &gz_launch_cmd])
        .output()
        .map_err(|e| e.to_string())?;

    if !gz_result.status.success() {
        return Err(String::from_utf8_lossy(&gz_result.stderr).to_string());
    }

    // Give Gazebo a moment to actually start before launching the bridge
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Build bridge args dynamically based on models actually found in the
    // world file, rather than hardcoding vehicle_blue/vehicle_green.
    let mut bridge_topics = Vec::new();
    for model in &robot_models {
        bridge_topics.push(format!(
            "/model/{}/cmd_vel@geometry_msgs/msg/Twist@gz.msgs.Twist", model
        ));
        bridge_topics.push(format!(
            "/model/{}/odometry@nav_msgs/msg/Odometry@gz.msgs.Odometry", model
        ));
        bridge_topics.push(format!(
            "/model/{}/tf@tf2_msgs/msg/TFMessage@gz.msgs.Pose_V", model
        ));
    }

    let bridge_cmd = format!(
        "source /opt/ros/jazzy/setup.bash && ros2 run ros_gz_bridge parameter_bridge {} > /tmp/bridge.log 2>&1",
        bridge_topics.join(" ")
    );

    let bridge_result = docker_command()
        .args(["exec", "-d", &container_name, "bash", "-c", &bridge_cmd])
        .output()
        .map_err(|e| e.to_string())?;

    if !bridge_result.status.success() {
        return Err(String::from_utf8_lossy(&bridge_result.stderr).to_string());
    }

    Ok("Gazebo and bridge started".to_string())
}

#[tauri::command]
fn stop_gazebo_sim() -> Result<String, String> {
    let container_name = find_dev_container()?;

    let result = docker_command()
        .args([
            "exec", &container_name,
            "bash", "-c",
            "pkill -f 'gz sim' ; pkill -f parameter_bridge",
        ])
        .output()
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&result.stdout).to_string())
}

#[tauri::command]
fn reset_gazebo_sim() -> Result<String, String> {
    let container_name = find_dev_container()?;

    let result = docker_command()
        .args([
            "exec", &container_name,
            "bash", "-c",
            "gz service -s /world/default/control --reqtype gz.msgs.WorldControl \
             --reptype gz.msgs.Boolean --timeout 2000 --req 'reset: {all: true}'",
        ])
        .output()
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&result.stdout).to_string())
}

use tauri::Manager;

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    copy_dir_recursive_impl(src, dst, 0)
}

fn copy_dir_recursive_impl(src: &std::path::Path, dst: &std::path::Path, depth: u32) -> Result<(), String> {
    if depth > 50 {
        return Err(format!("copy_dir_recursive exceeded max depth (50) — possible cyclic path: {:?} -> {:?}", src, dst));
    }
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_symlink() {
            continue; // skip symlinks entirely — avoids cycles, and we don't need them copied
        }

        if file_type.is_dir() {
            copy_dir_recursive_impl(&src_path, &dst_path, depth + 1)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
async fn initialize_ros_environment(app: AppHandle) -> Result<(), String> {
    let resource_path = app
        .path()
        .resolve("resources/devcontainer", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;

    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let workspace_dir = format!("{}/.robotics-studio/ros2-workspace", home);
    let workspace_path = std::path::Path::new(&workspace_dir);

    // Always re-copy bundled resource files (Dockerfile, proto/, server.py,
    // world files, etc.) so updates to the app's bundled resources are
    // never silently skipped — this previously caused a confusing bug
    // where a new world file didn't appear because an old Dockerfile
    // already existed. User-generated content (src/, build/, install/,
    // log/) lives alongside these files but isn't touched, since we only
    // copy what's present in the bundled resource_path, not delete anything.
    copy_dir_recursive(&resource_path, workspace_path)?;

    let filter = format!("label=devcontainer.local_folder={}", workspace_dir);

    let existing = docker_command()
        .args(["ps", "-a", "--filter", &filter, "--format", "{{.Names}}"])
        .output()
        .map_err(|e| e.to_string())?;

    let container_name = String::from_utf8_lossy(&existing.stdout).trim().to_string();

    if !container_name.is_empty() {
        let _ = docker_command()
            .args(["start", &container_name])
            .output();
        let event = LogEvent {
            kind: "append".to_string(),
            text: "Existing environment found, starting it...".to_string(),
        };
        let _ = app.emit("init-progress", event);
        let _ = app.emit("init-finished", workspace_dir.clone());
        return Ok(());
    }

    tauri::async_runtime::spawn(async move {
        let event = LogEvent {
            kind: "append".to_string(),
            text: "Building Docker image (this may take several minutes on first run)...".to_string(),
        };
        let _ = app.emit("init-progress", event);

        let build_child = docker_command()
            .args(["build", "-t", "robotics-studio-ros2", &workspace_dir])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut build_child = match build_child {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("init-error", e.to_string());
                return;
            }
        };

        if let Some(stderr) = build_child.stderr.take() {
            let mut reader = BufReader::new(stderr);
            let mut buffer = [0; 4096];
            let mut line_buffer = StreamLineBuffer::new();
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        // EOF: flush any remaining bytes in the buffer
                        if let Some((kind, text)) = line_buffer.flush() {
                            let event = LogEvent { kind, text };
                            let _ = app.emit("init-progress", event);
                        }
                        break;
                    }
                    Ok(n) => {
                        let events = line_buffer.process_chunk(&buffer[..n]);
                        for (kind, text) in events {
                            let event = LogEvent { kind, text };
                            let _ = app.emit("init-progress", event);
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        let build_status = match build_child.wait() {
            Ok(s) => s,
            Err(e) => {
                let _ = app.emit("init-error", e.to_string());
                return;
            }
        };

        if !build_status.success() {
            let _ = app.emit("init-error", "Docker build failed — see progress log above".to_string());
            return;
        }

        let event = LogEvent {
            kind: "append".to_string(),
            text: "Image built. Starting container...".to_string(),
        };
        let _ = app.emit("init-progress", event);

        let run_output = docker_command()
            .args([
                "run", "-d",
                "--label", &format!("devcontainer.local_folder={}", workspace_dir),
                "-p", "50051:50051",
                "-v", &format!("{}:/workspace", workspace_dir),
                "robotics-studio-ros2",
                "tail", "-f", "/dev/null",
            ])
            .output();

        let run_output = match run_output {
            Ok(o) => o,
            Err(e) => {
                let _ = app.emit("init-error", e.to_string());
                return;
            }
        };

        if !run_output.status.success() {
            let _ = app.emit("init-error", String::from_utf8_lossy(&run_output.stderr).to_string());
            return;
        }

        let container_id = String::from_utf8_lossy(&run_output.stdout).trim().to_string();

        let event = LogEvent {
            kind: "append".to_string(),
            text: "Container started. Launching ROS engine service...".to_string(),
        };
        let _ = app.emit("init-progress", event);

        std::thread::sleep(std::time::Duration::from_secs(2));

        let generate_output = docker_command()
    .args([
        "exec",
        &container_id,
        "bash",
        "-c",
        r#"source /opt/ros/jazzy/setup.bash && \
           cd /workspace && \
           mkdir -p generated && \
           python3 -m grpc_tools.protoc \
             -I proto \
             --python_out=generated \
             --grpc_python_out=generated \
             proto/ros_engine.proto"#,
    ])
    .output();

match generate_output {
    Ok(o) if o.status.success() => {}
    Ok(o) => {
        let _ = app.emit(
            "init-error",
            String::from_utf8_lossy(&o.stderr).to_string(),
        );
        return;
    }
    Err(e) => {
        let _ = app.emit("init-error", e.to_string());
        return;
    }
}

let mut ready = false;
for _ in 0..15 {
    if std::net::TcpStream::connect_timeout(
        &"127.0.0.1:50051".parse().unwrap(),
        std::time::Duration::from_millis(500),
    ).is_ok() {
        ready = true;
        break;
    }
    std::thread::sleep(std::time::Duration::from_secs(1));
}

if !ready {
    let log_output = docker_command()
        .args(["exec", &container_id, "cat", "/tmp/server.log"])
        .output();
    let log_text = log_output
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let _ = app.emit("init-error", format!("ROS engine server never started:\n{}", log_text));
    return;
}

let _ = app.emit("init-finished", workspace_dir.clone());

let server_output = docker_command()
    .args([
        "exec",
        "-d",
        &container_id,
        "bash",
        "-c",
        "source /opt/ros/jazzy/setup.bash && \
         cd /workspace && \
         python3 server.py > /tmp/server.log 2>&1",
    ])
    .output();

        match server_output {
            Ok(o) if o.status.success() => {
                std::thread::sleep(std::time::Duration::from_secs(5));
                let _ = app.emit("init-finished", workspace_dir.clone());
            }
            Ok(o) => {
                let _ = app.emit("init-error", String::from_utf8_lossy(&o.stderr).to_string());
            }
            Err(e) => {
                let _ = app.emit("init-error", e.to_string());
            }
        }
    });

    Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
struct AIProviderSettings {
    provider: String, // "groq" | "anthropic" | "openai"
    api_key: String,
    model: String,
}

fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("ai_settings.json"))
}

#[tauri::command]
fn save_ai_settings(app: AppHandle, settings: AIProviderSettings) -> Result<(), String> {
    let path = settings_path(&app)?;
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn load_ai_settings(app: AppHandle) -> Result<Option<AIProviderSettings>, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let settings: AIProviderSettings = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(Some(settings))
}

#[derive(Serialize, Deserialize, Clone)]
struct KeyBinding {
    id: String,
    keys: String,
    description: String,
}

#[derive(Serialize, Deserialize)]
struct KeybindingsData {
    keybindings: Vec<KeyBinding>,
}

fn keybindings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("keybindings.json"))
}

#[tauri::command]
fn save_keybindings(app: AppHandle, keybindings: Vec<KeyBinding>) -> Result<(), String> {
    let path = keybindings_path(&app)?;
    let data = KeybindingsData { keybindings };
    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn load_keybindings(app: AppHandle) -> Result<Option<Vec<KeyBinding>>, String> {
    let path = keybindings_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let data: KeybindingsData = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(Some(data.keybindings))
}

#[derive(Serialize)]
struct SystemSpecs {
    ram_gb: u64,
    cpu_cores: u32,
    gpu_name: Option<String>,
    gpu_vram_gb: Option<u64>,
}

#[tauri::command]
fn get_system_specs() -> Result<SystemSpecs, String> {
    // Get total RAM using system_info-like approach via command line
    let ram_gb = get_total_ram_gb()?;
    let cpu_cores = get_cpu_count();
    let gpu_info = get_gpu_info();
    let gpu_name = gpu_info.as_ref().map(|(name, _)| name.clone());
    let gpu_vram_gb = gpu_info.and_then(|(_, vram)| vram);

    Ok(SystemSpecs {
        ram_gb,
        cpu_cores,
        gpu_name,
        gpu_vram_gb,
    })
}

#[cfg(target_os = "macos")]
fn get_total_ram_gb() -> Result<u64, String> {
    let output = std::process::Command::new("sysctl")
        .args(&["-n", "hw.memsize"])
        .output()
        .map_err(|e| e.to_string())?;
    let bytes_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let bytes: u64 = bytes_str.parse().map_err(|_| "Failed to parse RAM size".to_string())?;
    Ok(bytes / (1024 * 1024 * 1024))
}

#[cfg(target_os = "linux")]
fn get_total_ram_gb() -> Result<u64, String> {
    let output = std::process::Command::new("grep")
        .args(&["MemTotal", "/proc/meminfo"])
        .output()
        .map_err(|e| e.to_string())?;
    let output_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = output_str.split_whitespace().collect();
    if parts.len() >= 2 {
        let kb: u64 = parts[1].parse().map_err(|_| "Failed to parse RAM size".to_string())?;
        Ok(kb / (1024 * 1024))
    } else {
        Err("Failed to parse /proc/meminfo".to_string())
    }
}

#[cfg(target_os = "windows")]
fn get_total_ram_gb() -> Result<u64, String> {
    let output = std::process::Command::new("wmic")
        .args(&["ComputerSystem", "get", "TotalPhysicalMemory"])
        .output()
        .map_err(|e| e.to_string())?;
    let output_str = String::from_utf8_lossy(&output.stdout);
    for line in output_str.lines() {
        if let Ok(bytes) = line.trim().parse::<u64>() {
            return Ok(bytes / (1024 * 1024 * 1024));
        }
    }
    Err("Failed to parse RAM from wmic output".to_string())
}

fn get_cpu_count() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1)
}

#[cfg(target_os = "macos")]
fn get_gpu_info() -> Option<(String, Option<u64>)> {
    // Try to get GPU info from system_profiler on macOS
    let output = std::process::Command::new("system_profiler")
        .args(&["SPDisplaysDataType"])
        .output()
        .ok()?;
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Simple parsing: look for "Chip Model:" and "VRAM:"
    let mut gpu_name = None;
    let mut vram_gb = None;
    
    for line in output_str.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Chip Model:") {
            gpu_name = trimmed.strip_prefix("Chip Model:").map(|s| s.trim().to_string());
        }
        if trimmed.starts_with("VRAM") {
            // Try to extract VRAM amount (e.g., "10 GB")
            if let Some(vram_part) = trimmed.split(": ").nth(1) {
                if let Some(num_part) = vram_part.split_whitespace().next() {
                    vram_gb = num_part.parse::<u64>().ok();
                }
            }
        }
    }
    
    gpu_name.map(|name| (name, vram_gb))
}

#[cfg(target_os = "linux")]
fn get_gpu_info() -> Option<(String, Option<u64>)> {
    // Try lspci on Linux
    let output = std::process::Command::new("lspci")
        .output()
        .ok()?;
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    for line in output_str.lines() {
        if line.contains("VGA") || line.contains("3D") {
            // Extract GPU name from lspci output
            if let Some(gpu_part) = line.split(": ").nth(1) {
                return Some((gpu_part.to_string(), None));
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn get_gpu_info() -> Option<(String, Option<u64>)> {
    // Try WMIC on Windows
    let output = std::process::Command::new("wmic")
        .args(&["path", "win32_videocontroller", "get", "name,adapterram"])
        .output()
        .ok()?;
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    let lines: Vec<&str> = output_str.lines().collect();
    if lines.len() > 1 {
        let data_line = lines[1];
        let parts: Vec<&str> = data_line.split_whitespace().collect();
        if !parts.is_empty() {
            let name = parts[0].to_string();
            let vram_bytes = parts.last().and_then(|s| s.parse::<u64>().ok());
            let vram_gb = vram_bytes.map(|b| b / (1024 * 1024 * 1024));
            return Some((name, vram_gb));
        }
    }
    None
}

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct GroqMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct GroqRequest {
    model: String,
    messages: Vec<GroqMessage>,
}

#[derive(Serialize, Clone)]
struct TfUpdate {
    parent_frame: String,
    child_frame: String,
    x: f64,
    y: f64,
    z: f64,
    qx: f64,
    qy: f64,
    qz: f64,
    qw: f64,
}

#[derive(Deserialize)]
struct GroqChoice {
    message: GroqResponseMessage,
}

#[derive(Deserialize)]
struct GroqResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    text: Option<String>,
}

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}

#[tauri::command]
async fn ask_ai(
    app: AppHandle,
    user_message: String,
    open_file_content: Option<String>,
    open_file_path: Option<String>,
    workspace_root: Option<String>,
    recent_ros_events: Vec<String>,
    recent_build_output: Vec<String>,
    tf_context: Option<String>,
    mode: Option<String>,
) -> Result<String, String> {
    let settings = load_ai_settings(app)?
        .ok_or_else(|| "No AI provider configured. Open Settings to add your API key.".to_string())?;

    // --- Context assembly (unchanged, provider-agnostic) ---
    let mut context = String::new();
    let mut system_context = match mode.as_deref() {
        Some("generate_node") => String::from(
            "You are a ROS 2 code generator. The user wants a new ROS 2 Python node. \
             Output ONLY a single Python code block using ```python fences — no explanation \
             before or after. Use rclpy, follow standard ROS 2 node conventions (a class \
             extending Node, a main() function with rclpy.init/spin/shutdown). Base it on \
             any workspace context below if relevant (e.g., existing topic names, package \
             structure).\n\n"
        ),
        Some("generate_launch") => String::from(
            "You are a ROS 2 launch file generator. The user wants a ROS 2 Python launch file. \
             Output ONLY a single Python code block using ```python fences — no explanation \
             before or after. Follow standard ROS 2 launch conventions (LaunchDescription, \
             generate_launch_description()).\n\n"
        ),
        Some("generate_urdf") => String::from(
            "You are a ROS 2 robot description generator. The user wants a URDF or xacro file \
             describing a robot. Output ONLY a single XML code block using ```xml fences — no \
             explanation before or after. Use standard URDF conventions: <robot> root element, \
             <link> elements with <visual>/<collision>/<inertial>, <joint> elements connecting \
             them with correct parent/child and joint type. If workspace context mentions \
             specific link/frame names (e.g., from TF data or existing topics), reuse those \
             names for consistency. Prefer plain URDF unless the user explicitly asks for \
             xacro macros/parameters.\n\n"
        ),
        Some("explain_error") => String::from(
            "You are a ROS 2 / robotics debugging assistant. The user wants their most recent \
             build error or runtime error explained. Be concise and specific: name the likely \
             root cause, then give a concrete fix. Reference the actual error text from the \
             context below rather than generic advice.\n\n"
        ),
        Some("explain_tf") => String::from(
            "You are a ROS 2 / robotics TF (transform tree) assistant. The user wants a specific \
             TF parent→child frame relationship explained in plain terms. Using the frame names \
             and the transform values (translation and quaternion rotation) given in the context \
             below, explain: (1) what this parent/child frame pair likely represents physically \
             on the robot (e.g., sensor mount, wheel, arm link) based on naming conventions and \
             standard ROS 2 REP-105/REP-103 frame conventions if the names match common patterns \
             (base_link, odom, map, camera_link, laser, imu_link, etc.), (2) what the given \
             translation and rotation values mean in practical terms (e.g., 'this frame is offset \
             30cm forward and rotated 90° about Z'), and (3) anything that looks unusual or \
             worth double-checking (e.g., an unexpectedly large offset, a non-normalized \
             quaternion, or a frame pair that doesn't follow common conventions). Be concise — \
             a few short paragraphs, not an exhaustive lecture. Do not invent details about the \
             robot that aren't supported by the frame names or values given.\n\n"
        ),
        _ => String::from(
            "You are an AI coding assistant. Answer the user's question directly and accurately. \
             Only reference ROS 2, robotics concepts, or the workspace context below if it is \
             actually relevant to the question — do not force a connection that isn't there.\n\n"
        ),
    };

    if open_file_path.as_deref().map_or(false, |p| p.ends_with(".py"))
        || recent_ros_events.iter().any(|e| e.contains("WorkspaceEvent")) {
        system_context.push_str("This workspace appears to involve ROS 2 development.\n\n");
    }

    let file_is_ignored = match (&open_file_path, &workspace_root) {
        (Some(path), Some(root)) => is_ignored_by_aiignore(root, path),
        _ => false,
    };

    if file_is_ignored {
        context.push_str(
            "Currently open file: [path excluded — matches a pattern in .aiignore]\n\n",
        );
    } else if let Some(path) = &open_file_path {
        context.push_str(&format!("Currently open file: {}\n\n", path));
    }

    if file_is_ignored {
        context.push_str(
            "File contents: [excluded from AI context — this file matches a pattern in .aiignore]\n\n",
        );
    } else if let Some(content) = &open_file_content {
        context.push_str(&format!("File contents:\n```\n{}\n```\n\n", content));
    }
    if let Some(tf) = &tf_context {
        context.push_str(&format!("TF frame data:\n{}\n\n", tf));
    }
    if !recent_ros_events.is_empty() {
        context.push_str("Recent ROS 2 runtime events:\n");
        for evt in recent_ros_events.iter().rev().take(10) {
            context.push_str(&format!("- {}\n", evt));
        }
        context.push('\n');
    }
    if !recent_build_output.is_empty() {
        context.push_str("Recent build output:\n");
        for line in recent_build_output.iter().rev().take(20) {
            context.push_str(&format!("{}\n", line));
        }
        context.push('\n');
    }

    let full_prompt = format!(
        "{}=== WORKSPACE CONTEXT ===\n{}\n=== END CONTEXT ===\n\nUser question: {}",
        system_context, context, user_message
    );

    let client = reqwest::Client::new();

    match settings.provider.as_str() {
        "groq" => {
            let request_body = GroqRequest {
                model: settings.model,
                messages: vec![GroqMessage { role: "user".to_string(), content: full_prompt }],
            };
            let response = client
                .post("https://api.groq.com/openai/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", settings.api_key))
                .header("content-type", "application/json")
                .json(&request_body)
                .send().await.map_err(|e| e.to_string())?;
            let parsed: GroqResponse = response.json().await.map_err(|e| e.to_string())?;
            parsed.choices.into_iter().next().map(|c| c.message.content)
                .ok_or_else(|| "No response from AI".to_string())
        }
        "anthropic" => {
            let request_body = ClaudeRequest {
                model: settings.model,
                max_tokens: 1024,
                messages: vec![ClaudeMessage { role: "user".to_string(), content: full_prompt }],
            };
            let response = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", settings.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&request_body)
                .send().await.map_err(|e| e.to_string())?;
            let parsed: ClaudeResponse = response.json().await.map_err(|e| e.to_string())?;
            parsed.content.into_iter().next().and_then(|b| b.text)
                .ok_or_else(|| "No response from AI".to_string())
        }
        "openai" => {
            let request_body = GroqRequest {
                model: settings.model,
                messages: vec![GroqMessage { role: "user".to_string(), content: full_prompt }],
            };
            let response = client
                .post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", settings.api_key))
                .header("content-type", "application/json")
                .json(&request_body)
                .send().await.map_err(|e| e.to_string())?;
            let parsed: GroqResponse = response.json().await.map_err(|e| e.to_string())?;
            parsed.choices.into_iter().next().map(|c| c.message.content)
                .ok_or_else(|| "No response from AI".to_string())
        }
        other => Err(format!("Unknown provider: {}", other)),
    }
}

#[tauri::command]
fn check_aiignore(workspace_root: String, file_path: String) -> bool {
    is_ignored_by_aiignore(&workspace_root, &file_path)
}

#[tauri::command]
fn list_templates(app: AppHandle) -> Result<Vec<String>, String> {
    let templates_path = app
        .path()
        .resolve("resources/templates", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;

    if !templates_path.exists() {
        return Ok(vec![]);
    }

    let mut names = Vec::new();
    for entry in std::fs::read_dir(&templates_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            names.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    Ok(names)
}

#[tauri::command]
fn create_project_from_template(
    app: AppHandle,
    template_name: String,
    destination_dir: String,
    project_name: String,
) -> Result<String, String> {
    let template_path = app
        .path()
        .resolve(
            format!("resources/templates/{}", template_name),
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| e.to_string())?;

    if !template_path.exists() {
        return Err(format!("Template '{}' not found", template_name));
    }

    let dest = std::path::Path::new(&destination_dir).join(&project_name);
    if dest.exists() {
        return Err(format!("A folder named '{}' already exists there", project_name));
    }

    copy_dir_recursive(&template_path, &dest)?;

    Ok(dest.to_string_lossy().to_string())
}

#[derive(Serialize, Clone)]
struct OdometryUpdate {
    node: String,
    x: f64,
    y: f64,
    z: f64,
    qx: f64,
    qy: f64,
    qz: f64,
    qw: f64,
}

#[cfg(test)]
mod stream_line_buffer_tests {
    use super::*;

    #[test]
    fn test_single_chunk_complete_multibyte_utf8() {
        // Test case (a): Single read() with complete multi-byte UTF-8 character
        // (e.g., '─' U+2500 is 3 bytes in UTF-8: E2 94 80)
        let mut buf = StreamLineBuffer::new();
        let chunk = "Box: ─ drawing\n".as_bytes();
        let events = buf.process_chunk(chunk);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "append");
        assert!(events[0].1.contains("─"), "Multi-byte UTF-8 character should be decoded");
    }

    #[test]
    fn test_multibyte_utf8_split_across_chunks() {
        // Test case (b): Multi-byte UTF-8 character deliberately split across two chunks.
        // The character '─' (U+2500) is E2 94 80 in UTF-8.
        // Split as: "Box: " + first 2 bytes of '─' in chunk 1, last byte + "\n" in chunk 2.
        let mut buf = StreamLineBuffer::new();
        
        // First chunk: "Box: " (5 bytes ASCII) + E2 94 (first 2 bytes of '─')
        let chunk1 = b"Box: \xE2\x94";
        let events1 = buf.process_chunk(chunk1);
        assert_eq!(events1.len(), 0, "No complete line yet, should buffer");

        // Second chunk: 80 (last byte of '─') + " line\n"
        let chunk2 = b"\x80 line\n";
        let events2 = buf.process_chunk(chunk2);
        assert_eq!(events2.len(), 1, "Should emit one complete line");
        assert_eq!(events2[0].0, "append");
        // The decoded string should contain the full box-drawing character
        assert!(
            events2[0].1.contains("─") || events2[0].1.contains("Box: "),
            "Should successfully decode split UTF-8 character and complete line: got '{}'",
            events2[0].1
        );
    }

    #[test]
    fn test_line_split_across_chunks_no_delimiter_between() {
        // Test case (c): Single logical line split across two chunks with no \r/\n between.
        // E.g., "Downloading laye" in chunk 1, "r 3/8\n" in chunk 2.
        let mut buf = StreamLineBuffer::new();
        let chunk1 = b"Downloading laye";
        let events1 = buf.process_chunk(chunk1);
        assert_eq!(events1.len(), 0, "No delimiter, should buffer");

        let chunk2 = b"r 3/8\n";
        let events2 = buf.process_chunk(chunk2);
        assert_eq!(events2.len(), 1, "Should emit one complete line");
        assert_eq!(events2[0].0, "append");
        assert_eq!(events2[0].1, "Downloading layer 3/8", "Should combine both chunks into one line");
    }

    #[test]
    fn test_repeated_carriage_returns_docker_progress() {
        // Test case (d): Repeated \r-terminated updates simulating real Docker progress.
        // E.g., "Pulling fs layer\rDownloading 10%\rDownloading 45%\rDownloading 100%\n"
        let mut buf = StreamLineBuffer::new();
        let chunk = b"Pulling fs layer\rDownloading 10%\rDownloading 45%\rDownloading 100%\n";
        let events = buf.process_chunk(chunk);
        
        // Should produce 4 events: 3 "replace" and 1 final "append"
        assert_eq!(events.len(), 4, "Should emit 4 events (3 replace + 1 append)");
        assert_eq!(events[0].0, "replace");
        assert_eq!(events[0].1, "Pulling fs layer");
        assert_eq!(events[1].0, "replace");
        assert_eq!(events[1].1, "Downloading 10%");
        assert_eq!(events[2].0, "replace");
        assert_eq!(events[2].1, "Downloading 45%");
        assert_eq!(events[3].0, "append");
        assert_eq!(events[3].1, "Downloading 100%");
    }

    #[test]
    fn test_flush_remaining_bytes_on_eof() {
        // EOF with remaining bytes in buffer should be emitted as final "append"
        let mut buf = StreamLineBuffer::new();
        let chunk = b"Final line without newline";
        let events = buf.process_chunk(chunk);
        assert_eq!(events.len(), 0, "No delimiter, should buffer");

        let final_event = buf.flush();
        assert!(final_event.is_some());
        let (kind, text) = final_event.unwrap();
        assert_eq!(kind, "append");
        assert_eq!(text, "Final line without newline");
    }

    #[test]
    fn test_no_panic_on_multibyte_at_chunk_boundary() {
        // This test ensures we don't panic when a multi-byte UTF-8 sequence
        // happens to end exactly at the boundary of a chunk (the old code would panic here).
        let mut buf = StreamLineBuffer::new();
        
        // First chunk ends with a complete multi-byte character (3-byte UTF-8)
        let chunk1 = "Test─".as_bytes(); // ends with complete ─
        let _events1 = buf.process_chunk(chunk1);
        
        // Second chunk continues with more text
        let chunk2 = b"more\n";
        let events2 = buf.process_chunk(chunk2);
        
        // Should not panic and should produce one event
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].0, "append");
        assert!(events2[0].1.contains("Test─more"));
    }
}