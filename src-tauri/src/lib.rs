use std::fs;

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
        .invoke_handler(tauri::generate_handler![
            read_file, write_file, list_dir, start_ros_stream, run_colcon_build, run_colcon_build_streaming, start_gazebo_sim, stop_gazebo_sim, reset_gazebo_sim, ask_ai, initialize_ros_environment, publish_twist
            
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

use std::io::{BufRead, BufReader};
use std::process::Stdio;

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
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let _ = app.emit("build-output", line);
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

#[tauri::command]
fn start_gazebo_sim() -> Result<String, String> {
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

    // Launch Gazebo headless, detached, inside the container
    let gz_result = docker_command()
    .args([
        "exec", "-d", &container_name,
        "bash", "-c",
        "source /opt/ros/jazzy/setup.bash && gz sim -s -r --headless-rendering diff_drive.sdf > /tmp/gz.log 2>&1",
    ])
    .output()
    .map_err(|e| e.to_string())?;

    if !gz_result.status.success() {
        return Err(String::from_utf8_lossy(&gz_result.stderr).to_string());
    }

    // Give Gazebo a moment to actually start before launching the bridge
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Launch the bridge, also detached
    let bridge_result = docker_command()
        .args([
            "exec", "-d", &container_name,
            "bash", "-c",
            "source /opt/ros/jazzy/setup.bash && ros2 run ros_gz_bridge parameter_bridge \
             /model/vehicle_blue/cmd_vel@geometry_msgs/msg/Twist@gz.msgs.Twist \
             /model/vehicle_blue/odometry@nav_msgs/msg/Odometry@gz.msgs.Odometry \
             /model/vehicle_green/cmd_vel@geometry_msgs/msg/Twist@gz.msgs.Twist \
             /model/vehicle_green/odometry@nav_msgs/msg/Odometry@gz.msgs.Odometry \
             > /tmp/bridge.log 2>&1",
        ])
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

    if !workspace_path.join("Dockerfile").exists() {
        copy_dir_recursive(&resource_path, workspace_path)?;
    }

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
        let _ = app.emit("init-progress", "Existing environment found, starting it...".to_string());
        let _ = app.emit("init-finished", workspace_dir.clone());
        return Ok(());
    }

    tauri::async_runtime::spawn(async move {
        let _ = app.emit("init-progress", "Building Docker image (this may take several minutes on first run)...".to_string());

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
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let _ = app.emit("init-progress", line);
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

        let _ = app.emit("init-progress", "Image built. Starting container...".to_string());

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

        let _ = app.emit("init-progress", "Container started. Launching ROS engine service...".to_string());

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

#[tauri::command]
async fn ask_ai(
    user_message: String,
    open_file_content: Option<String>,
    open_file_path: Option<String>,
    recent_ros_events: Vec<String>,
    recent_build_output: Vec<String>,
) -> Result<String, String> {
    let api_key = std::env::var("GROQ_API_KEY")
        .map_err(|_| "GROQ_API_KEY not set".to_string())?;

    // --- Context assembly (unchanged, provider-agnostic) ---
    let mut context = String::new();

    let mut system_context = String::from(
    "You are an AI coding assistant. Answer the user's question directly and accurately. \
     Only reference ROS 2, robotics concepts, or the workspace context below if it is \
     actually relevant to the question — do not force a connection that isn't there.\n\n"
);

if open_file_path.as_deref().map_or(false, |p| p.ends_with(".py")) 
    || recent_ros_events.iter().any(|e| e.contains("WorkspaceEvent")) {
    system_context.push_str("This workspace appears to involve ROS 2 development.\n\n");
}

    if let Some(path) = &open_file_path {
        context.push_str(&format!("Currently open file: {}\n\n", path));
    }
    if let Some(content) = &open_file_content {
        context.push_str(&format!("File contents:\n```\n{}\n```\n\n", content));
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
        "{}=== WORKSPACE CONTEXT ===\n{}\n=== END CONTEXT ===\n\n\
         User question: {}",
        system_context, context, user_message
    );

    let client = reqwest::Client::new();
    let request_body = GroqRequest {
        model: "llama-3.3-70b-versatile".to_string(),
        messages: vec![GroqMessage {
            role: "user".to_string(),
            content: full_prompt,
        }],
    };

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let parsed: GroqResponse = response.json().await.map_err(|e| e.to_string())?;

    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| "No response from AI".to_string())
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