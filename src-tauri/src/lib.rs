use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;
const MAX_WORLD_SIZE: u64 = 10 * 1024 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 10_000;

#[derive(Default)]
struct FsAccess {
    approved_roots: Mutex<HashSet<PathBuf>>,
}

impl FsAccess {
    fn approve_directory(&self, path: &Path) -> Result<PathBuf, String> {
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("Could not resolve selected directory: {error}"))?;
        if !canonical.is_dir() {
            return Err("Selected path is not a directory".to_string());
        }

        self.approved_roots
            .lock()
            .map_err(|_| "Filesystem access state is unavailable".to_string())?
            .insert(canonical.clone());
        Ok(canonical)
    }

    fn resolve_existing(&self, path: &Path) -> Result<PathBuf, String> {
        self.resolve(path, false)
    }

    fn resolve_for_write(&self, path: &Path) -> Result<PathBuf, String> {
        self.resolve(path, true)
    }

    fn resolve(&self, path: &Path, allow_missing_leaf: bool) -> Result<PathBuf, String> {
        if !path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
        {
            return Err(
                "Path must be absolute and cannot contain traversal components".to_string(),
            );
        }

        let candidate = if allow_missing_leaf {
            let parent = path
                .parent()
                .ok_or_else(|| "Invalid destination path".to_string())?
                .canonicalize()
                .map_err(|error| format!("Invalid destination directory: {error}"))?;
            parent.join(
                path.file_name()
                    .ok_or_else(|| "Invalid destination filename".to_string())?,
            )
        } else {
            path.canonicalize()
                .map_err(|error| format!("Could not resolve path: {error}"))?
        };

        let roots = self
            .approved_roots
            .lock()
            .map_err(|_| "Filesystem access state is unavailable".to_string())?;
        for root in roots.iter() {
            let current_root = root
                .canonicalize()
                .map_err(|_| "An approved directory is no longer available".to_string())?;
            if current_root == *root && candidate.starts_with(root) {
                if allow_missing_leaf && candidate.exists() {
                    let metadata =
                        fs::symlink_metadata(&candidate).map_err(|error| error.to_string())?;
                    if metadata.file_type().is_symlink() {
                        return Err(
                            "Symbolic links are not allowed as file destinations".to_string()
                        );
                    }
                }
                return Ok(candidate);
            }
        }

        Err("Path is outside approved directories".to_string())
    }
}

fn read_approved_file(access: &FsAccess, path: &Path) -> Result<String, String> {
    let path = access.resolve_existing(path)?;
    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("Path is not a regular file".to_string());
    }
    if metadata.len() > MAX_FILE_SIZE {
        return Err(format!("File exceeds the {MAX_FILE_SIZE}-byte limit"));
    }
    read_text_file_limited(&path, MAX_FILE_SIZE)
}

fn read_text_file_limited(path: &Path, max_bytes: u64) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut contents = String::new();
    file.take(max_bytes + 1)
        .read_to_string(&mut contents)
        .map_err(|error| error.to_string())?;
    if contents.len() as u64 > max_bytes {
        return Err(format!("File exceeds the {max_bytes}-byte limit"));
    }
    Ok(contents)
}

fn write_approved_file(access: &FsAccess, path: &Path, contents: &str) -> Result<(), String> {
    if contents.len() as u64 > MAX_FILE_SIZE {
        return Err(format!("File exceeds the {MAX_FILE_SIZE}-byte limit"));
    }
    let path = access.resolve_for_write(path)?;
    if path.exists() {
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("Destination must be a regular file, not a symbolic link".to_string());
        }
    }
    let parent = path
        .parent()
        .ok_or_else(|| "Destination has no parent directory".to_string())?;
    let file_name = path
        .file_name()
        .ok_or_else(|| "Destination has no filename".to_string())?
        .to_string_lossy();
    let counter = std::sync::atomic::AtomicU64::fetch_add(
        &FILE_WRITE_COUNTER,
        1,
        std::sync::atomic::Ordering::Relaxed,
    );
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        counter
    ));
    let write_result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        std::io::Write::write_all(&mut file, contents.as_bytes())
            .map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        #[cfg(windows)]
        if path.exists() {
            fs::remove_file(&path).map_err(|error| error.to_string())?;
        }
        fs::rename(&temporary, &path).map_err(|error| error.to_string())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

static FILE_WRITE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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
        return rel_path.split('/').any(|component| component == dir);
    }

    if let Some(ext) = pattern.strip_prefix("*.") {
        return rel_path
            .rsplit('/')
            .next()
            .is_some_and(|name| name.ends_with(&format!(".{}", ext)));
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
fn is_ignored_by_aiignore(
    access: &FsAccess,
    workspace_root: &Path,
    file_path: &Path,
) -> Result<bool, String> {
    let workspace_root = access.resolve_existing(workspace_root)?;
    if !workspace_root.is_dir() {
        return Err("Workspace path is not a directory".to_string());
    }
    let file_path = access.resolve_existing(file_path)?;
    if !file_path.starts_with(&workspace_root) {
        return Err("File is outside the selected workspace".to_string());
    }
    let ignore_file = workspace_root.join(".aiignore");
    let contents = match access.resolve_existing(&ignore_file) {
        Ok(path) => read_approved_file(access, &path)?,
        Err(_) if !ignore_file.exists() => return Ok(false),
        Err(error) => return Err(error),
    };

    let rel_path = file_path
        .strip_prefix(&workspace_root)
        .map_err(|_| "File is outside the selected workspace".to_string())?
        .to_string_lossy()
        .to_string();

    Ok(contents
        .lines()
        .any(|pattern| matches_aiignore_pattern(&rel_path, pattern)))
}

#[tauri::command]
fn read_file(access: tauri::State<'_, FsAccess>, path: String) -> Result<String, String> {
    read_approved_file(&access, Path::new(&path))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct RecentProject {
    path: String,
    name: String,
    last_opened: String,
}

fn normalize_recent_projects(mut projects: Vec<RecentProject>) -> Vec<RecentProject> {
    let mut by_path: std::collections::HashMap<String, RecentProject> =
        std::collections::HashMap::new();

    for project in projects.drain(..) {
        let path = project.path.trim().to_string();
        if path.is_empty() {
            continue;
        }

        match by_path.get_mut(&path) {
            Some(existing) => {
                let incoming_ts = project.last_opened.parse::<u64>().unwrap_or(0);
                let current_ts = existing.last_opened.parse::<u64>().unwrap_or(0);
                if incoming_ts >= current_ts {
                    *existing = RecentProject {
                        path: path.clone(),
                        name: if project.name.trim().is_empty() {
                            existing.name.clone()
                        } else {
                            project.name
                        },
                        last_opened: project.last_opened,
                    };
                }
            }
            None => {
                by_path.insert(path.clone(), project);
            }
        }
    }

    let mut latest: Vec<RecentProject> = by_path.into_values().collect();
    latest.sort_by(|a, b| {
        let a_ts = a.last_opened.parse::<u64>().unwrap_or(0);
        let b_ts = b.last_opened.parse::<u64>().unwrap_or(0);
        b_ts.cmp(&a_ts)
    });
    latest.truncate(8);
    latest
}

#[tauri::command]
fn write_file(
    access: tauri::State<'_, FsAccess>,
    path: String,
    contents: String,
) -> Result<(), String> {
    write_approved_file(&access, Path::new(&path), &contents)
}

#[derive(serde::Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
}

#[tauri::command]
fn list_dir(access: tauri::State<'_, FsAccess>, path: String) -> Result<Vec<FileEntry>, String> {
    let path = access.resolve_existing(Path::new(&path))?;
    if !path.is_dir() {
        return Err("Path is not a directory".to_string());
    }
    let entries = fs::read_dir(&path).map_err(|e| e.to_string())?;
    let mut result = Vec::new();

    for (index, entry) in entries.enumerate() {
        if index >= MAX_DIRECTORY_ENTRIES {
            return Err(format!(
                "Directory contains more than {MAX_DIRECTORY_ENTRIES} entries"
            ));
        }
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

#[tauri::command]
async fn pick_directory(
    app: AppHandle,
    access: tauri::State<'_, FsAccess>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let selected =
        tauri::async_runtime::spawn_blocking(move || app.dialog().file().blocking_pick_folder())
            .await
            .map_err(|error| format!("Directory picker failed: {error}"))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let selected = selected
        .into_path()
        .map_err(|error| format!("Invalid selected directory: {error}"))?;
    let approved = access.approve_directory(&selected)?;
    Ok(Some(approved.to_string_lossy().into_owned()))
}

#[tauri::command]
async fn pick_save_path(
    app: AppHandle,
    is_xml: bool,
    access: tauri::State<'_, FsAccess>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let selected = tauri::async_runtime::spawn_blocking(move || {
        let dialog = app.dialog().file().set_title("Save generated file");
        if is_xml {
            dialog
                .add_filter("URDF/XML", &["urdf", "xacro", "xml"])
                .blocking_save_file()
        } else {
            dialog.add_filter("Python", &["py"]).blocking_save_file()
        }
    })
    .await
    .map_err(|error| format!("Save dialog failed: {error}"))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let selected = selected
        .into_path()
        .map_err(|error| format!("Invalid selected file: {error}"))?;
    let parent = selected
        .parent()
        .ok_or_else(|| "Selected file has no parent directory".to_string())?;
    access.approve_directory(parent)?;
    let safe_path = access.resolve_for_write(&selected)?;
    Ok(Some(safe_path.to_string_lossy().into_owned()))
}

fn validate_sdf_file(path: &Path, content: &str) -> Result<(), String> {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_none_or(|extension| !extension.eq_ignore_ascii_case("sdf"))
    {
        return Err("World files must use the .sdf extension".to_string());
    }
    if content.len() as u64 > MAX_WORLD_SIZE {
        return Err(format!(
            "World file exceeds the {MAX_WORLD_SIZE}-byte limit"
        ));
    }
    if content.contains('\0') {
        return Err("World file contains a NUL character".to_string());
    }

    fn local_name(name: &[u8]) -> &[u8] {
        name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
    }

    fn version_from_attributes(
        element: &quick_xml::events::BytesStart<'_>,
        decoder: quick_xml::encoding::Decoder,
    ) -> Result<String, String> {
        for attribute in element.attributes().with_checks(true) {
            let attribute =
                attribute.map_err(|error| format!("Malformed SDF XML attribute: {error}"))?;
            if attribute.key.as_ref() == b"version" {
                return attribute
                    .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, decoder)
                    .map(|value| value.into_owned())
                    .map_err(|error| format!("Malformed SDF version attribute: {error}"));
            }
        }
        Err("SDF root element is missing its version attribute".to_string())
    }

    let mut reader = quick_xml::Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut root_seen = false;
    let mut root_closed = false;
    let mut world_seen = false;
    let mut depth = 0usize;
    let mut version = None;
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(element)) => {
                if !root_seen {
                    if local_name(element.name().as_ref()) != b"sdf" {
                        return Err("World file root element must be <sdf>".to_string());
                    }
                    version = Some(version_from_attributes(&element, reader.decoder())?);
                    root_seen = true;
                } else if root_closed {
                    return Err("SDF document contains content after its root element".to_string());
                } else if depth == 1 && local_name(element.name().as_ref()) == b"world" {
                    world_seen = true;
                }
                depth += 1;
            }
            Ok(quick_xml::events::Event::Empty(element)) => {
                if !root_seen {
                    if local_name(element.name().as_ref()) != b"sdf" {
                        return Err("World file root element must be <sdf>".to_string());
                    }
                    version = Some(version_from_attributes(&element, reader.decoder())?);
                    root_seen = true;
                    root_closed = true;
                } else if root_closed {
                    return Err("SDF document contains content after its root element".to_string());
                } else if depth == 1 && local_name(element.name().as_ref()) == b"world" {
                    world_seen = true;
                }
            }
            Ok(quick_xml::events::Event::End(_)) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    root_closed = true;
                }
            }
            Ok(quick_xml::events::Event::DocType(_)) => {
                return Err("SDF documents must not contain a DTD".to_string());
            }
            Ok(quick_xml::events::Event::Text(text)) if depth == 0 => {
                let text_bytes: &[u8] = text.as_ref();
                if !text_bytes.iter().all(u8::is_ascii_whitespace) {
                    return Err("SDF document contains text outside its root element".to_string());
                }
            }
            Ok(quick_xml::events::Event::CData(_)) if depth == 0 => {
                return Err("SDF document contains data outside its root element".to_string());
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(format!("Malformed SDF XML: {error}")),
        }
    }

    if !root_seen || !root_closed || depth != 0 {
        return Err("SDF document is incomplete".to_string());
    }
    let version = version.ok_or_else(|| "SDF version is missing".to_string())?;
    if !world_seen {
        return Err("SDF world document must contain a <world> element".to_string());
    }
    if !matches!(
        version.as_str(),
        "1.4" | "1.5" | "1.6" | "1.7" | "1.8" | "1.9" | "1.10"
    ) {
        return Err(format!("Unsupported SDF version: {version}"));
    }
    Ok(())
}

fn import_world_contents(
    workspace: &Path,
    source: &Path,
    content: &str,
) -> Result<PathBuf, String> {
    validate_sdf_file(source, content)?;
    let workspace = workspace
        .canonicalize()
        .map_err(|error| format!("ROS workspace is unavailable: {error}"))?;
    let worlds_dir = workspace.join("worlds");
    match fs::symlink_metadata(&worlds_dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err("World import directory must not be a symbolic link".to_string());
        }
        Ok(metadata) if !metadata.is_dir() => {
            return Err("World import path is not a directory".to_string());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(&worlds_dir).map_err(|error| error.to_string())?;
        }
        Err(error) => return Err(error.to_string()),
    }
    let canonical_worlds = worlds_dir
        .canonicalize()
        .map_err(|error| format!("Could not resolve world import directory: {error}"))?;
    if !canonical_worlds.starts_with(&workspace) {
        return Err("World import directory escapes the ROS workspace".to_string());
    }

    let source_stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("world");
    let safe_stem = source_stem
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        .take(48)
        .collect::<String>();
    let safe_stem = if safe_stem.is_empty() {
        "world"
    } else {
        &safe_stem
    };
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();

    for attempt in 0..100u32 {
        let name = format!(
            "{safe_stem}-{timestamp}-{}-{attempt}.sdf",
            std::process::id()
        );
        let destination = canonical_worlds.join(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
        {
            Ok(mut file) => {
                if let Err(error) = std::io::Write::write_all(&mut file, content.as_bytes()) {
                    let _ = fs::remove_file(&destination);
                    return Err(format!("Could not import world file: {error}"));
                }
                return Ok(destination);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Could not import world file: {error}")),
        }
    }
    Err("Could not allocate a unique world filename".to_string())
}

#[tauri::command]
async fn import_world(
    app: AppHandle,
    access: tauri::State<'_, FsAccess>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("SDF World", &["sdf"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| format!("World picker failed: {error}"))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let source = selected
        .into_path()
        .map_err(|error| format!("Invalid selected world file: {error}"))?;
    let metadata = fs::metadata(&source).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_WORLD_SIZE {
        return Err(format!(
            "World file must be a regular file no larger than {MAX_WORLD_SIZE} bytes"
        ));
    }
    let content = read_text_file_limited(&source, MAX_WORLD_SIZE)
        .map_err(|error| format!("Could not read selected world file: {error}"))?;
    validate_sdf_file(&source, &content)?;

    let workspace = PathBuf::from(get_default_workspace_dir()?);
    fs::create_dir_all(&workspace).map_err(|error| error.to_string())?;
    access.approve_directory(&workspace)?;
    let imported = import_world_contents(&workspace, &source, &content)?;
    Ok(Some(imported.to_string_lossy().into_owned()))
}

#[tauri::command]
async fn pick_keybindings_file(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("JSON", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| format!("File picker failed: {error}"))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|error| format!("Invalid selected file: {error}"))?;
    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_SIZE {
        return Err("Selected file is not a regular file or exceeds the size limit".to_string());
    }
    read_text_file_limited(&path, MAX_FILE_SIZE)
        .map(Some)
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(FsAccess::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            read_file,
            write_file,
            list_dir,
            pick_directory,
            pick_save_path,
            pick_keybindings_file,
            import_world,
            start_ros_stream,
            run_colcon_build,
            run_colcon_build_streaming,
            start_gazebo_sim,
            stop_gazebo_sim,
            reset_gazebo_sim,
            ask_ai,
            initialize_ros_environment,
            publish_twist,
            save_ai_settings,
            load_ai_settings,
            delete_ai_credential,
            has_ai_credential,
            save_keybindings,
            load_keybindings,
            save_recent_projects,
            load_recent_projects,
            get_system_specs,
            list_templates,
            create_project_from_template,
            check_aiignore
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub mod rosengine {
    tonic::include_proto!("rosengine");
}

use rosengine::ros_engine_client::RosEngineClient;
use rosengine::Empty;

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

static STREAM_RUNNING: AtomicBool = AtomicBool::new(false);

fn stop_stream_for_auth_recovery(app: &AppHandle, message: &str) {
    let _ = app.emit("ros-auth-error", message.to_string());
    STREAM_RUNNING.store(false, Ordering::SeqCst);
}

#[tauri::command]
async fn start_ros_stream(app: AppHandle) -> Result<(), String> {
    println!("start_ros_stream invoked");

    if STREAM_RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    tauri::async_runtime::spawn(async move {
        loop {
            let token = match read_grpc_auth_token(&app) {
                Ok(token) => token,
                Err(error) => {
                    stop_stream_for_auth_recovery(&app, &error);
                    return;
                }
            };
            let _ = app.emit("ros-connecting", ());
            println!("[ros-stream] attempting connection to localhost:50051");
            let client_result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                RosEngineClient::connect("http://localhost:50051"),
            )
            .await;

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
            let request = match authenticated_request(Empty {}, &token) {
                Ok(request) => request,
                Err(error) => {
                    let _ = app.emit("ros-error", error);
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    continue;
                }
            };
            let mut stream = match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                client.stream_events(request),
            )
            .await
            {
                Ok(Ok(s)) => {
                    println!("[ros-stream] stream_events established, awaiting messages");
                    s.into_inner()
                }
                Ok(Err(e)) if e.code() == tonic::Code::Unauthenticated => {
                    stop_stream_for_auth_recovery(
                        &app,
                        "ROS authentication was rejected. The session will be renewed.",
                    );
                    return;
                }
                Ok(Err(e)) => {
                    let _ = app.emit("ros-error", format!("Stream failed: {}", e));
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    continue;
                }
                Err(_) => {
                    let _ = app.emit("ros-error", "stream_events timed out".to_string());
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    continue;
                }
            };

            loop {
                match stream.message().await {
                    Ok(Some(evt)) => {
                        println!("[ros-stream] received event: {:?}", evt);
                        let _ = app.emit("ros-event", format!("{:?}", evt));

                        if let Some(rosengine::workspace_event::Event::OdometryUpdate(odom)) =
                            &evt.event
                        {
                            let _ = app.emit(
                                "odometry-update",
                                OdometryUpdate {
                                    node: odom.topic_name.clone(),
                                    x: odom.x,
                                    y: odom.y,
                                    z: odom.z,
                                    qx: odom.qx,
                                    qy: odom.qy,
                                    qz: odom.qz,
                                    qw: odom.qw,
                                },
                            );
                        }

                        if let Some(rosengine::workspace_event::Event::TfUpdate(tf)) = &evt.event {
                            let _ = app.emit(
                                "tf-update",
                                TfUpdate {
                                    parent_frame: tf.parent_frame.clone(),
                                    child_frame: tf.child_frame.clone(),
                                    x: tf.x,
                                    y: tf.y,
                                    z: tf.z,
                                    qx: tf.qx,
                                    qy: tf.qy,
                                    qz: tf.qz,
                                    qw: tf.qw,
                                },
                            );
                        }
                        if let Some(rosengine::workspace_event::Event::NodeSnapshot(snap)) =
                            &evt.event
                        {
                            let _ = app.emit("node-snapshot", snap.node_names.clone());
                        }
                    }
                    Ok(None) => {
                        let _ = app.emit("ros-error", "Stream closed by server".to_string());
                        break;
                    }
                    Err(e) if e.code() == tonic::Code::Unauthenticated => {
                        stop_stream_for_auth_recovery(
                            &app,
                            "ROS authentication expired. The session will be renewed.",
                        );
                        return;
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

const MAX_LINEAR_VELOCITY: f64 = 0.5;
const MAX_ANGULAR_VELOCITY: f64 = 1.0;

fn is_valid_ros_topic(topic_name: &str) -> bool {
    topic_name.len() <= 200
        && topic_name.starts_with('/')
        && topic_name.split('/').skip(1).all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                && segment
                    .chars()
                    .next()
                    .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        })
}

fn is_safe_project_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        && value
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic())
}

fn validate_velocity(value: f64, maximum: f64) -> Result<(), String> {
    if !value.is_finite() || value.abs() > maximum {
        return Err(format!(
            "Velocity must be finite and between -{maximum} and {maximum}"
        ));
    }
    Ok(())
}

fn validate_velocity_vector(values: [f64; 3], maximum: f64, axis: &str) -> Result<(), String> {
    for value in values {
        validate_velocity(value, maximum)?;
    }
    let magnitude = values[0].hypot(values[1]).hypot(values[2]);
    if magnitude > maximum {
        return Err(format!(
            "{axis} velocity magnitude must not exceed {maximum}"
        ));
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TwistInput {
    topic_name: String,
    linear_x: Option<f64>,
    linear_y: Option<f64>,
    linear_z: Option<f64>,
    angular_x: Option<f64>,
    angular_y: Option<f64>,
    angular_z: Option<f64>,
}

#[tauri::command]
async fn publish_twist(app: AppHandle, command: TwistInput) -> Result<(), String> {
    let TwistInput {
        topic_name,
        linear_x,
        linear_y,
        linear_z,
        angular_x,
        angular_y,
        angular_z,
    } = command;
    if !is_valid_ros_topic(&topic_name) {
        return Err("Invalid ROS topic name".to_string());
    }
    validate_velocity_vector(
        [
            linear_x.unwrap_or(0.0),
            linear_y.unwrap_or(0.0),
            linear_z.unwrap_or(0.0),
        ],
        MAX_LINEAR_VELOCITY,
        "Linear",
    )?;
    validate_velocity_vector(
        [
            angular_x.unwrap_or(0.0),
            angular_y.unwrap_or(0.0),
            angular_z.unwrap_or(0.0),
        ],
        MAX_ANGULAR_VELOCITY,
        "Angular",
    )?;

    let mut client = RosEngineClient::connect("http://localhost:50051")
        .await
        .map_err(|_| docker_lost_connection_message())?;

    let token = read_grpc_auth_token(&app)?;
    let timestamp_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos()
        .try_into()
        .map_err(|_| "Command timestamp is out of range".to_string())?;
    client
        .publish_twist(authenticated_request(
            TwistCommand {
                topic_name,
                linear_x: linear_x.unwrap_or(0.0),
                linear_y: linear_y.unwrap_or(0.0),
                linear_z: linear_z.unwrap_or(0.0),
                angular_x: angular_x.unwrap_or(0.0),
                angular_y: angular_y.unwrap_or(0.0),
                angular_z: angular_z.unwrap_or(0.0),
                timestamp_ns,
            },
            &token,
        )?)
        .await
        .map_err(|_| docker_lost_connection_message())?;

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
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| "Could not determine the current user's home directory".to_string())?;
    Ok(std::path::PathBuf::from(home)
        .join(".robotics-studio")
        .join("ros2-workspace")
        .to_string_lossy()
        .into_owned())
}

fn docker_daemon_unavailable_message() -> String {
    "Docker Desktop doesn't appear to be running. Please start Docker Desktop and restart Robotics Studio.".to_string()
}

fn docker_lost_connection_message() -> String {
    "Lost connection to the ROS environment. Docker may have stopped — check Docker Desktop is running and try again.".to_string()
}

fn ensure_docker_available() -> Result<(), String> {
    let output = docker_command()
        .args(["info"])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(docker_daemon_unavailable_message())
    }
}

fn ensure_loopback_grpc_binding(container_id: &str) -> Result<(), String> {
    let output = docker_command()
        .args([
            "inspect",
            "--format",
            "{{json .HostConfig.PortBindings}}",
            container_id,
        ])
        .output()
        .map_err(|error| format!("Could not verify ROS container port binding: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Could not verify ROS container port binding: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let bindings: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Could not parse ROS container port binding: {error}"))?;
    let is_loopback_only = bindings
        .get("50051/tcp")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|entries| {
            !entries.is_empty()
                && entries.iter().all(|entry| {
                    entry.get("HostIp").and_then(serde_json::Value::as_str) == Some("127.0.0.1")
                })
        });
    if !is_loopback_only {
        let _ = docker_command()
            .args([
                "exec",
                container_id,
                "bash",
                "-c",
                "pkill -f '[p]ython3 server.py'",
            ])
            .output();
        return Err(
            "This ROS container publishes gRPC beyond loopback. Its port binding cannot be changed in place; stop it and recreate the container with its existing workspace bind-mounted to preserve files."
                .to_string(),
        );
    }
    Ok(())
}

fn find_dev_container() -> Result<String, String> {
    if ensure_docker_available().is_err() {
        return Err(docker_lost_connection_message());
    }

    let workspace_path = get_default_workspace_dir()?;
    let filter = format!("label=devcontainer.local_folder={}", workspace_path);
    let output = docker_command()
        .args(["ps", "--filter", &filter, "--format", "{{.Names}}"])
        .output()
        .map_err(|_| docker_lost_connection_message())?;

    if !output.status.success() {
        return Err(docker_lost_connection_message());
    }

    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() {
        Err("No running dev container found for this workspace".to_string())
    } else {
        ensure_loopback_grpc_binding(&name)?;
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
        .map_err(|_| docker_lost_connection_message())?;

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
    let mut skipping_escape = false;

    for ch in input.chars() {
        if skipping_escape {
            if ch.is_alphabetic() {
                skipping_escape = false;
            }
        } else if ch == '\x1b' {
            skipping_escape = true;
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
    discarding_long_line: bool,
}

impl StreamLineBuffer {
    fn new() -> Self {
        StreamLineBuffer {
            pending: Vec::new(),
            discarding_long_line: false,
        }
    }

    /// Process a chunk of bytes and return (kind, text) events emitted so far.
    /// Any bytes that don't end with a delimiter are kept in the pending buffer
    /// for the next call, ensuring lines split across chunks are assembled correctly.
    fn process_chunk(&mut self, chunk: &[u8]) -> Vec<(String, String)> {
        if self.discarding_long_line {
            let Some(delimiter) = chunk.iter().position(|byte| matches!(byte, b'\n' | b'\r'))
            else {
                return Vec::new();
            };
            self.discarding_long_line = false;
            return self.process_chunk(&chunk[delimiter + 1..]);
        }

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

        if self.pending.len() > MAX_PENDING_LINE_BYTES {
            let text = String::from_utf8_lossy(&self.pending[..MAX_PENDING_LINE_BYTES]);
            result.push((
                "append".to_string(),
                format!("{} [line truncated]", strip_ansi_codes(&text)),
            ));
            self.pending.clear();
            self.discarding_long_line = true;
        }

        result
    }

    /// Flush any remaining bytes in the buffer as a final "append" event.
    /// Call this when EOF is reached to ensure no data is lost.
    fn flush(&mut self) -> Option<(String, String)> {
        if self.discarding_long_line {
            self.discarding_long_line = false;
            return None;
        }
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

const MAX_PENDING_LINE_BYTES: usize = 64 * 1024;

fn stream_process_output<R: Read + Send + 'static>(
    reader: R,
    app: AppHandle,
    output_event: &'static str,
    error_event: &'static str,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut buffer = [0; 4096];
        let mut line_buffer = StreamLineBuffer::new();
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    if let Some((kind, text)) = line_buffer.flush() {
                        let _ = app.emit(output_event, LogEvent { kind, text });
                    }
                    break;
                }
                Ok(size) => {
                    for (kind, text) in line_buffer.process_chunk(&buffer[..size]) {
                        let _ = app.emit(output_event, LogEvent { kind, text });
                    }
                }
                Err(error) => {
                    let _ = app.emit(error_event, error.to_string());
                    break;
                }
            }
        }
    })
}

static BUILD_RUNNING: AtomicBool = AtomicBool::new(false);
static INITIALIZATION_RUNNING: AtomicBool = AtomicBool::new(false);

struct BuildRunGuard;
struct InitializationGuard;

impl Drop for BuildRunGuard {
    fn drop(&mut self) {
        BUILD_RUNNING.store(false, Ordering::SeqCst);
    }
}

impl Drop for InitializationGuard {
    fn drop(&mut self) {
        INITIALIZATION_RUNNING.store(false, Ordering::SeqCst);
    }
}

#[tauri::command]
async fn run_colcon_build_streaming(app: AppHandle) -> Result<(), String> {
    let container_name = find_dev_container()?;
    if BUILD_RUNNING.swap(true, Ordering::SeqCst) {
        return Err("A build is already running".to_string());
    }
    let build_guard = BuildRunGuard;

    tauri::async_runtime::spawn(async move {
        let _build_guard = build_guard;
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
            Err(_) => {
                let _ = app.emit("build-error", docker_lost_connection_message());
                let _ = app.emit("build-finished", false);
                return;
            }
        };

        let stdout_reader = child.stdout.take().map(|stdout| {
            stream_process_output(stdout, app.clone(), "build-output", "build-error")
        });
        let stderr_reader = child.stderr.take().map(|stderr| {
            stream_process_output(stderr, app.clone(), "build-output", "build-error")
        });

        match child.wait() {
            Ok(status) => {
                if let Some(reader) = stdout_reader {
                    let _ = reader.join();
                }
                if let Some(reader) = stderr_reader {
                    let _ = reader.join();
                }
                let _ = app.emit("build-finished", status.success());
            }
            Err(e) => {
                let _ = app.emit("build-error", e.to_string());
                let _ = app.emit("build-finished", false);
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
fn start_gazebo_sim(
    world_path: Option<String>,
    access: tauri::State<'_, FsAccess>,
) -> Result<String, String> {
    let container_name = find_dev_container()?;

    // Defensively kill any existing instances, and actively verify
    // they're gone before proceeding — a fixed sleep isn't reliable
    // enough, since detached process teardown timing varies.
    for attempt in 0..10 {
        let kill_result = docker_command()
            .args([
                "exec",
                &container_name,
                "bash",
                "-c",
                "pkill -9 -f '[g]z sim -s' ; pkill -9 -f '[/]parameter_bridge '",
            ])
            .output();

        if kill_result.is_err() {
            return Err(docker_lost_connection_message());
        }

        std::thread::sleep(std::time::Duration::from_millis(800));

        let check = docker_command()
            .args([
                "exec",
                &container_name,
                "bash",
                "-c",
                "pgrep -f '[g]z sim -s' ; pgrep -f '[/]parameter_bridge '",
            ])
            .output()
            .map_err(|_| docker_lost_connection_message())?;

        eprintln!(
            "DEBUG attempt {}: check.stdout = {:?}",
            attempt,
            String::from_utf8_lossy(&check.stdout)
        );

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
    let workspace = PathBuf::from(get_default_workspace_dir()?)
        .canonicalize()
        .map_err(|error| format!("ROS workspace is unavailable: {error}"))?;
    let (container_world_path, sdf_content) = match &world_path {
        Some(host_path) => {
            let host_path = access.resolve_existing(Path::new(host_path))?;
            let worlds_dir = workspace
                .join("worlds")
                .canonicalize()
                .map_err(|error| format!("Imported worlds are unavailable: {error}"))?;
            if !host_path.starts_with(&worlds_dir) {
                return Err(
                    "Custom worlds must be imported into the managed worlds directory".to_string(),
                );
            }
            let content = read_approved_file(&access, &host_path)?;
            validate_sdf_file(&host_path, &content)?;
            let relative = host_path
                .strip_prefix(&workspace)
                .map_err(|_| "World file is outside the ROS workspace".to_string())?;
            let relative = relative.to_string_lossy().replace('\\', "/");
            (format!("/workspace/{relative}"), content)
        }
        None => {
            let bundled_path = workspace.join("diff_drive.sdf");
            let content = read_approved_file(&access, &bundled_path)?;
            ("/workspace/diff_drive.sdf".to_string(), content)
        }
    };

    let models = detect_models_in_sdf(&sdf_content);
    let robot_models: Vec<&String> = models
        .iter()
        .filter(|m| m.as_str() != "ground_plane")
        .collect();

    let gz_result = docker_command()
        .args([
            "exec",
            "-d",
            &container_name,
            "bash",
            "-c",
            "source /opt/ros/jazzy/setup.bash && exec gz sim -s -r --headless-rendering \"$1\" > /tmp/gz.log 2>&1",
            "robotics-studio",
            &container_world_path,
        ])
        .output()
        .map_err(|_| docker_lost_connection_message())?;

    if !gz_result.status.success() {
        if ensure_docker_available().is_err() {
            return Err(docker_lost_connection_message());
        }
        return Err(String::from_utf8_lossy(&gz_result.stderr).to_string());
    }

    // Give Gazebo a moment to actually start before launching the bridge
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Build bridge args dynamically based on models actually found in the
    // world file, rather than hardcoding vehicle_blue/vehicle_green.
    let mut bridge_topics = Vec::new();
    for model in &robot_models {
        bridge_topics.push(format!(
            "/model/{}/cmd_vel@geometry_msgs/msg/Twist@gz.msgs.Twist",
            model
        ));
        bridge_topics.push(format!(
            "/model/{}/odometry@nav_msgs/msg/Odometry@gz.msgs.Odometry",
            model
        ));
        bridge_topics.push(format!(
            "/model/{}/tf@tf2_msgs/msg/TFMessage@gz.msgs.Pose_V",
            model
        ));
    }

    let mut bridge_args = vec![
        "exec",
        "-d",
        &container_name,
        "bash",
        "-c",
        "source /opt/ros/jazzy/setup.bash && exec ros2 run ros_gz_bridge parameter_bridge \"$@\" > /tmp/bridge.log 2>&1",
        "robotics-studio",
    ];
    bridge_args.extend(bridge_topics.iter().map(String::as_str));
    let bridge_result = docker_command()
        .args(bridge_args)
        .output()
        .map_err(|_| docker_lost_connection_message())?;

    if !bridge_result.status.success() {
        if ensure_docker_available().is_err() {
            return Err(docker_lost_connection_message());
        }
        return Err(String::from_utf8_lossy(&bridge_result.stderr).to_string());
    }

    Ok("Gazebo and bridge started".to_string())
}

#[tauri::command]
fn stop_gazebo_sim() -> Result<String, String> {
    let container_name = find_dev_container()?;

    let result = docker_command()
        .args([
            "exec",
            &container_name,
            "bash",
            "-c",
            "pkill -f 'gz sim' ; pkill -f parameter_bridge",
        ])
        .output()
        .map_err(|_| docker_lost_connection_message())?;

    Ok(String::from_utf8_lossy(&result.stdout).to_string())
}

#[tauri::command]
fn reset_gazebo_sim() -> Result<String, String> {
    let container_name = find_dev_container()?;

    let result = docker_command()
        .args([
            "exec",
            &container_name,
            "bash",
            "-c",
            "gz service -s /world/default/control --reqtype gz.msgs.WorldControl \
             --reptype gz.msgs.Boolean --timeout 2000 --req 'reset: {all: true}'",
        ])
        .output()
        .map_err(|_| docker_lost_connection_message())?;

    Ok(String::from_utf8_lossy(&result.stdout).to_string())
}

use tauri::Manager;

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    copy_dir_recursive_impl(src, dst, 0)
}

fn copy_dir_recursive_impl(
    src: &std::path::Path,
    dst: &std::path::Path,
    depth: u32,
) -> Result<(), String> {
    if depth > 50 {
        return Err(format!(
            "copy_dir_recursive exceeded max depth (50) — possible cyclic path: {:?} -> {:?}",
            src, dst
        ));
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
        } else if !dst_path.exists() {
            std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

const GRPC_AUTH_CONTAINER_PATH: &str = "/run/robotics-studio/grpc-auth.json";
const GRPC_AUTH_LIFETIME_SECONDS: u64 = 24 * 60 * 60;
const AUTH_PROTOCOL_MARKER: &str = "AUTH_PROTOCOL_VERSION = 1";

#[derive(Serialize, Deserialize)]
struct GrpcAuthCredentials {
    token: String,
    expires_at: u64,
}

fn create_grpc_auth_file(app: &AppHandle) -> Result<(PathBuf, String), String> {
    let mut random = [0u8; 32];
    getrandom::fill(&mut random)
        .map_err(|error| format!("Could not generate gRPC credentials: {error}"))?;
    let token = random
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs()
        .saturating_add(GRPC_AUTH_LIFETIME_SECONDS);
    let credentials = GrpcAuthCredentials {
        token: token.clone(),
        expires_at,
    };

    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join("grpc-auth.json");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let temporary = directory.join(format!(".grpc-auth-{}-{timestamp}.tmp", std::process::id()));
    let serialized = serde_json::to_vec(&credentials).map_err(|error| error.to_string())?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("Could not create gRPC credential file: {error}"))?;
    if let Err(error) =
        std::io::Write::write_all(&mut file, &serialized).and_then(|_| file.sync_all())
    {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Could not write gRPC credential file: {error}"));
    }
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|error| format!("Could not rotate gRPC credentials: {error}"))?;
    }
    fs::rename(&temporary, &path)
        .map_err(|error| format!("Could not rotate gRPC credentials: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("Could not protect gRPC credentials: {error}"))?;
    }
    Ok((path, token))
}

fn read_grpc_auth_token(app: &AppHandle) -> Result<String, String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("grpc-auth.json");
    let credentials: GrpcAuthCredentials = serde_json::from_slice(
        &fs::read(path).map_err(|_| "ROS authentication is not initialized".to_string())?,
    )
    .map_err(|_| "ROS authentication credentials are invalid".to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    if credentials.token.len() != 64 || now >= credentials.expires_at {
        return Err(
            "ROS authentication credentials have expired; restart the ROS environment".to_string(),
        );
    }
    Ok(credentials.token)
}

fn install_grpc_auth_file(container_id: &str, host_path: &Path) -> Result<(), String> {
    let directory = docker_command()
        .args([
            "exec",
            container_id,
            "bash",
            "-c",
            "mkdir -p /run/robotics-studio && chmod 700 /run/robotics-studio",
        ])
        .output()
        .map_err(|error| format!("Could not prepare ROS authentication: {error}"))?;
    if !directory.status.success() {
        return Err(format!(
            "Could not prepare ROS authentication: {}",
            String::from_utf8_lossy(&directory.stderr)
        ));
    }
    let destination = format!("{container_id}:{GRPC_AUTH_CONTAINER_PATH}");
    let copied = docker_command()
        .args(["cp", &host_path.to_string_lossy(), &destination])
        .output()
        .map_err(|error| format!("Could not transfer ROS authentication: {error}"))?;
    if !copied.status.success() {
        return Err(format!(
            "Could not transfer ROS authentication: {}",
            String::from_utf8_lossy(&copied.stderr)
        ));
    }
    let protected = docker_command()
        .args([
            "exec",
            container_id,
            "chmod",
            "600",
            GRPC_AUTH_CONTAINER_PATH,
        ])
        .output()
        .map_err(|error| format!("Could not protect ROS authentication: {error}"))?;
    if !protected.status.success() {
        return Err(format!(
            "Could not protect ROS authentication: {}",
            String::from_utf8_lossy(&protected.stderr)
        ));
    }
    Ok(())
}

fn install_authenticated_server_files(
    resource_dir: &Path,
    workspace_dir: &Path,
    backup_dir: &Path,
) -> Result<(), String> {
    fs::create_dir_all(backup_dir)
        .map_err(|error| format!("Could not prepare server migration backup: {error}"))?;
    for file_name in ["server.py", "server_core.py", "proto/ros_engine.proto"] {
        let bundled_path = resource_dir.join(file_name);
        let target_path = workspace_dir.join(file_name);
        let bundled = fs::read(&bundled_path)
            .map_err(|error| format!("Could not read bundled {file_name}: {error}"))?;
        let installed = match fs::symlink_metadata(&target_path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!("Workspace {file_name} must not be a symbolic link"));
            }
            Ok(_) => Some(
                fs::read(&target_path)
                    .map_err(|error| format!("Could not inspect workspace {file_name}: {error}"))?,
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("Could not inspect workspace {file_name}: {error}")),
        };
        if installed.as_ref().is_some_and(|contents| {
            String::from_utf8_lossy(contents).contains(AUTH_PROTOCOL_MARKER)
        }) {
            continue;
        }

        if installed.is_some() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| error.to_string())?
                .as_nanos();
            let backup_path = backup_dir.join(format!(
                "{}.pre-auth-{}-{timestamp}",
                file_name.replace('/', "_"),
                std::process::id()
            ));
            fs::copy(&target_path, &backup_path)
                .map_err(|error| format!("Could not preserve existing {file_name}: {error}"))?;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let target_parent = target_path
            .parent()
            .ok_or_else(|| format!("Invalid workspace path for {file_name}"))?;
        let target_name = target_path
            .file_name()
            .ok_or_else(|| format!("Invalid workspace filename for {file_name}"))?
            .to_string_lossy();
        let temporary = target_parent.join(format!(
            ".{target_name}.auth-{}-{timestamp}.tmp",
            std::process::id()
        ));
        fs::write(&temporary, bundled)
            .map_err(|error| format!("Could not stage secure {file_name}: {error}"))?;
        #[cfg(windows)]
        if target_path.exists() {
            fs::remove_file(&target_path)
                .map_err(|error| format!("Could not update secure {file_name}: {error}"))?;
        }
        if let Err(error) = fs::rename(&temporary, &target_path) {
            let _ = fs::remove_file(&temporary);
            return Err(format!("Could not install secure {file_name}: {error}"));
        }
    }
    Ok(())
}

fn start_ros_engine_server(container_id: &str, auth_file: &Path) -> Result<(), String> {
    let generate = docker_command()
        .args([
            "exec",
            container_id,
            "bash",
            "-c",
            "source /opt/ros/jazzy/setup.bash && cd /workspace && mkdir -p generated && python3 -m grpc_tools.protoc -I proto --python_out=generated --grpc_python_out=generated proto/ros_engine.proto",
        ])
        .output()
        .map_err(|error| format!("Failed to generate gRPC modules: {error}"))?;

    if !generate.status.success() {
        return Err(format!(
            "Failed to generate gRPC modules: {}",
            String::from_utf8_lossy(&generate.stderr)
        ));
    }

    install_grpc_auth_file(container_id, auth_file)?;

    let _ = docker_command()
        .args([
            "exec",
            container_id,
            "bash",
            "-c",
            "pkill -f '[p]ython3 server.py'",
        ])
        .output()
        .map_err(|error| format!("Could not rotate the ROS engine session: {error}"))?;

    let started = docker_command()
        .args([
            "exec",
            "-d",
            container_id,
            "bash",
            "-c",
            "source /opt/ros/jazzy/setup.bash && cd /workspace && ROBOTICS_STUDIO_AUTH_FILE=/run/robotics-studio/grpc-auth.json exec python3 server.py > /tmp/server.log 2>&1",
        ])
        .output()
        .map_err(|error| format!("Failed to start ROS engine server: {error}"))?;

    if !started.status.success() {
        return Err(format!(
            "Failed to start ROS engine server: {}",
            String::from_utf8_lossy(&started.stderr)
        ));
    }
    Ok(())
}

fn authenticated_request<T>(message: T, token: &str) -> Result<tonic::Request<T>, String> {
    let mut request = tonic::Request::new(message);
    let token = tonic::metadata::MetadataValue::try_from(token)
        .map_err(|error| format!("Could not encode ROS authentication metadata: {error}"))?;
    request
        .metadata_mut()
        .insert("x-robotics-studio-token", token);
    Ok(request)
}

async fn wait_for_ros_engine(container_id: &str, token: &str) -> Result<(), String> {
    for _ in 0..30 {
        if let Ok(Ok(mut client)) = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            RosEngineClient::connect("http://127.0.0.1:50051"),
        )
        .await
        {
            if tokio::time::timeout(
                std::time::Duration::from_secs(2),
                client.stream_events(authenticated_request(Empty {}, token)?),
            )
            .await
            .is_ok_and(|result| result.is_ok())
            {
                return Ok(());
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    let logs = docker_command()
        .args(["exec", container_id, "cat", "/tmp/server.log"])
        .output()
        .map_err(|error| {
            format!("gRPC service did not become ready; could not read logs: {error}")
        })?;
    Err(format!(
        "gRPC service did not become ready: {}",
        String::from_utf8_lossy(&logs.stdout)
    ))
}

#[tauri::command]
async fn initialize_ros_environment(
    app: AppHandle,
    access: tauri::State<'_, FsAccess>,
) -> Result<(), String> {
    if INITIALIZATION_RUNNING.swap(true, Ordering::SeqCst) {
        return Err("ROS environment initialization is already in progress".to_string());
    }
    let initialization_guard = InitializationGuard;

    if let Err(err) = ensure_docker_available() {
        let _ = app.emit("init-error", err.clone());
        return Err(err);
    }
    let (auth_file, auth_token) = create_grpc_auth_file(&app)?;

    let resource_path = app
        .path()
        .resolve(
            "resources/devcontainer",
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| e.to_string())?;

    let workspace_dir = get_default_workspace_dir()?;
    let workspace_path = Path::new(&workspace_dir);
    fs::create_dir_all(workspace_path).map_err(|error| error.to_string())?;
    let workspace_dir = access
        .approve_directory(workspace_path)?
        .to_string_lossy()
        .into_owned();
    let workspace_path = Path::new(&workspace_dir);

    // Install missing resources and preserve legacy server files before updating
    // the authentication-critical ROS service scripts.
    copy_dir_recursive(&resource_path, workspace_path)?;
    let backup_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("server-migration-backups");
    install_authenticated_server_files(&resource_path, workspace_path, &backup_dir)?;

    let filter = format!("label=devcontainer.local_folder={}", workspace_dir);

    let existing = docker_command()
        .args(["ps", "-a", "--filter", &filter, "--format", "{{.Names}}"])
        .output()
        .map_err(|_| docker_lost_connection_message())?;
    if !existing.status.success() {
        return Err(format!(
            "Could not inspect existing ROS containers: {}",
            String::from_utf8_lossy(&existing.stderr)
        ));
    }

    let container_names = String::from_utf8_lossy(&existing.stdout)
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if container_names.len() > 1 {
        return Err(format!(
            "Multiple ROS containers are registered for workspace '{}': {}",
            workspace_dir,
            container_names.join(", ")
        ));
    }
    let container_name = container_names.first().cloned().unwrap_or_default();

    if !container_name.is_empty() {
        let started = docker_command()
            .args(["start", &container_name])
            .output()
            .map_err(|error| format!("Could not start existing ROS container: {error}"))?;
        if !started.status.success() {
            return Err(format!(
                "Could not start existing ROS container: {}",
                String::from_utf8_lossy(&started.stderr)
            ));
        }
        ensure_loopback_grpc_binding(&container_name)?;

        let app = app.clone();
        let workspace_dir = workspace_dir.clone();
        tauri::async_runtime::spawn(async move {
            let _initialization_guard = initialization_guard;
            let _ = app.emit(
                "init-progress",
                LogEvent {
                    kind: "append".to_string(),
                    text: "Existing environment found. Checking ROS engine...".to_string(),
                },
            );
            let result = match start_ros_engine_server(&container_name, &auth_file) {
                Ok(()) => wait_for_ros_engine(&container_name, &auth_token).await,
                Err(error) => Err(error),
            };
            match result {
                Ok(()) => {
                    let _ = app.emit("init-finished", workspace_dir);
                }
                Err(error) => {
                    let _ = app.emit("init-error", error);
                }
            }
        });
        return Ok(());
    }

    tauri::async_runtime::spawn(async move {
        let _initialization_guard = initialization_guard;
        let event = LogEvent {
            kind: "append".to_string(),
            text: "Building Docker image (this may take several minutes on first run)..."
                .to_string(),
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

        let stdout_reader = build_child.stdout.take().map(|stdout| {
            stream_process_output(stdout, app.clone(), "init-progress", "init-error")
        });
        let stderr_reader = build_child.stderr.take().map(|stderr| {
            stream_process_output(stderr, app.clone(), "init-progress", "init-error")
        });

        let build_status = match build_child.wait() {
            Ok(status) => {
                if let Some(reader) = stdout_reader {
                    let _ = reader.join();
                }
                if let Some(reader) = stderr_reader {
                    let _ = reader.join();
                }
                status
            }
            Err(e) => {
                let _ = app.emit("init-error", e.to_string());
                return;
            }
        };

        if !build_status.success() {
            let _ = app.emit(
                "init-error",
                "Docker build failed — see progress log above".to_string(),
            );
            return;
        }

        let event = LogEvent {
            kind: "append".to_string(),
            text: "Image built. Starting container...".to_string(),
        };
        let _ = app.emit("init-progress", event);

        let run_output = docker_command()
            .args([
                "run",
                "-d",
                "--label",
                &format!("devcontainer.local_folder={}", workspace_dir),
                "-p",
                "127.0.0.1:50051:50051",
                "-v",
                &format!("{}:/workspace", workspace_dir),
                "robotics-studio-ros2",
                "tail",
                "-f",
                "/dev/null",
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
            let _ = app.emit(
                "init-error",
                String::from_utf8_lossy(&run_output.stderr).to_string(),
            );
            return;
        }

        let container_id = String::from_utf8_lossy(&run_output.stdout)
            .trim()
            .to_string();
        if let Err(error) = ensure_loopback_grpc_binding(&container_id) {
            let _ = app.emit("init-error", error.clone());
            return;
        }

        let event = LogEvent {
            kind: "append".to_string(),
            text: "Container started. Launching ROS engine service...".to_string(),
        };
        let _ = app.emit("init-progress", event);

        match start_ros_engine_server(&container_id, &auth_file) {
            Ok(()) => match wait_for_ros_engine(&container_id, &auth_token).await {
                Ok(()) => {
                    let _ = app.emit("init-finished", workspace_dir.clone());
                }
                Err(error) => {
                    let _ = app.emit("init-error", error);
                }
            },
            Err(error) => {
                let _ = app.emit("init-error", error);
            }
        }
    });

    Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
struct AIProviderSettings {
    provider: String, // "groq" | "anthropic" | "openai"
    model: String,
}

#[derive(Deserialize)]
struct SaveAIProviderSettings {
    provider: String,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct PublicAIProviderSettings {
    provider: String,
    model: String,
    has_api_key: bool,
}

trait CredentialProvider {
    fn get(&self, provider: &str) -> Result<Option<String>, String>;
    fn set(&self, provider: &str, secret: &str) -> Result<(), String>;
    fn delete(&self, provider: &str) -> Result<(), String>;
}

struct SystemCredentialProvider;

impl CredentialProvider for SystemCredentialProvider {
    fn get(&self, provider: &str) -> Result<Option<String>, String> {
        let entry = credential_entry(provider)?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(format!("Secure credential storage is unavailable: {error}")),
        }
    }

    fn set(&self, provider: &str, secret: &str) -> Result<(), String> {
        credential_entry(provider)?
            .set_password(secret)
            .map_err(|error| format!("Could not store the AI credential securely: {error}"))
    }

    fn delete(&self, provider: &str) -> Result<(), String> {
        match credential_entry(provider)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(format!("Could not delete the AI credential: {error}")),
        }
    }
}

fn validate_ai_provider(provider: &str) -> Result<(), String> {
    if matches!(provider, "groq" | "anthropic" | "openai") {
        Ok(())
    } else {
        Err("Unsupported AI provider".to_string())
    }
}

fn credential_entry(provider: &str) -> Result<keyring::Entry, String> {
    validate_ai_provider(provider)?;
    keyring::Entry::new("com.kartik.robotics-studio.ai", provider)
        .map_err(|error| format!("Could not access secure credential storage: {error}"))
}

fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("ai_settings.json"))
}

fn write_ai_settings_file(path: &Path, settings: &AIProviderSettings) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!(
        "json.{}.{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos()
    ));
    fs::write(&temporary, json).map_err(|error| error.to_string())?;
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
fn save_ai_settings(app: AppHandle, settings: SaveAIProviderSettings) -> Result<(), String> {
    validate_ai_provider(&settings.provider)?;
    let credentials = SystemCredentialProvider;
    if !settings.api_key.trim().is_empty() {
        credentials.set(&settings.provider, settings.api_key.trim())?;
    }
    let stored = AIProviderSettings {
        provider: settings.provider,
        model: settings.model,
    };
    let path = settings_path(&app)?;
    write_ai_settings_file(&path, &stored)
}

#[tauri::command]
fn load_ai_settings(app: AppHandle) -> Result<Option<PublicAIProviderSettings>, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut value: serde_json::Value =
        serde_json::from_str(&json).map_err(|error| error.to_string())?;
    if let Some(api_key) = value
        .get("api_key")
        .and_then(serde_json::Value::as_str)
        .filter(|secret| !secret.trim().is_empty())
    {
        let provider = value
            .get("provider")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "Legacy AI settings have no provider".to_string())?;
        SystemCredentialProvider.set(provider, api_key.trim())?;
        value
            .as_object_mut()
            .ok_or_else(|| "Invalid AI settings format".to_string())?
            .remove("api_key");
        let migrated: AIProviderSettings =
            serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
        write_ai_settings_file(&path, &migrated)?;
    }
    let settings: AIProviderSettings =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    validate_ai_provider(&settings.provider)?;
    let has_api_key = SystemCredentialProvider.get(&settings.provider)?.is_some();
    Ok(Some(PublicAIProviderSettings {
        provider: settings.provider,
        model: settings.model,
        has_api_key,
    }))
}

#[tauri::command]
fn delete_ai_credential(provider: String) -> Result<(), String> {
    SystemCredentialProvider.delete(&provider)
}

#[tauri::command]
fn has_ai_credential(provider: String) -> Result<bool, String> {
    Ok(SystemCredentialProvider.get(&provider)?.is_some())
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

fn recent_projects_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("recent_projects.json"))
}

#[tauri::command]
fn save_recent_projects(
    app: AppHandle,
    access: tauri::State<'_, FsAccess>,
    projects: Vec<RecentProject>,
) -> Result<(), String> {
    let path = recent_projects_path(&app)?;
    let normalized = normalize_recent_projects(projects);
    for project in &normalized {
        access.resolve_existing(Path::new(&project.path))?;
    }
    let json = serde_json::to_string_pretty(&normalized).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn load_recent_projects(
    app: AppHandle,
    access: tauri::State<'_, FsAccess>,
) -> Result<Vec<RecentProject>, String> {
    let path = recent_projects_path(&app)?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let projects: Vec<RecentProject> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let projects = normalize_recent_projects(projects);
    for project in &projects {
        if let Ok(path) = Path::new(&project.path).canonicalize() {
            if path.is_dir() {
                access.approve_directory(&path)?;
            }
        }
    }
    Ok(projects)
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
        .args(["-n", "hw.memsize"])
        .output()
        .map_err(|e| e.to_string())?;
    let bytes_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let bytes: u64 = bytes_str
        .parse()
        .map_err(|_| "Failed to parse RAM size".to_string())?;
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
        let kb: u64 = parts[1]
            .parse()
            .map_err(|_| "Failed to parse RAM size".to_string())?;
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
        .args(["SPDisplaysDataType"])
        .output()
        .ok()?;
    let output_str = String::from_utf8_lossy(&output.stdout);

    // Simple parsing: look for "Chip Model:" and "VRAM:"
    let mut gpu_name = None;
    let mut vram_gb = None;

    for line in output_str.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Chip Model:") {
            gpu_name = trimmed
                .strip_prefix("Chip Model:")
                .map(|s| s.trim().to_string());
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
    let output = std::process::Command::new("lspci").output().ok()?;
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

fn format_provider_http_error(
    provider: &str,
    model: &str,
    status: reqwest::StatusCode,
    body: &str,
) -> String {
    let snippet = body.trim().chars().take(200).collect::<String>();
    let snippet = if snippet.len() < body.trim().len() {
        format!("{}...", snippet)
    } else {
        snippet
    };

    match status {
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
            format!(
                "Invalid API key for {}. Check your key in Settings.",
                provider
            )
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            format!("Rate limited by {}. Wait a moment and try again.", provider)
        }
        reqwest::StatusCode::NOT_FOUND => {
            format!(
                "Model '{}' not found for {}. It may have been deprecated — check available models in Settings.",
                model, provider
            )
        }
        _ => {
            format!(
                "AI request failed for {} with status {}: {}",
                provider,
                status.as_u16(),
                snippet
            )
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AskAiInput {
    user_message: String,
    open_file_content: Option<String>,
    open_file_path: Option<String>,
    workspace_root: Option<String>,
    recent_ros_events: Vec<String>,
    recent_build_output: Vec<String>,
    tf_context: Option<String>,
    mode: Option<String>,
}

#[tauri::command]
async fn ask_ai(
    app: AppHandle,
    access: tauri::State<'_, FsAccess>,
    input: AskAiInput,
) -> Result<String, String> {
    let AskAiInput {
        user_message,
        open_file_content,
        open_file_path,
        workspace_root,
        recent_ros_events,
        recent_build_output,
        tf_context,
        mode,
    } = input;
    let settings = load_ai_settings(app.clone())?.ok_or_else(|| {
        "No AI provider configured. Open Settings to add your API key.".to_string()
    })?;
    let api_key = SystemCredentialProvider
        .get(&settings.provider)?
        .ok_or_else(|| "No API key stored for the selected AI provider".to_string())?;

    // --- Context assembly (unchanged, provider-agnostic) ---
    let mut context = String::new();
    let mut system_context = match mode.as_deref() {
        Some("generate_node") => String::from(
            "You are a ROS 2 code generator. The user wants a new ROS 2 Python node. \
             Output ONLY a single Python code block using ```python fences — no explanation \
             before or after. Use rclpy, follow standard ROS 2 node conventions (a class \
             extending Node, a main() function with rclpy.init/spin/shutdown). Base it on \
             any workspace context below if relevant (e.g., existing topic names, package \
             structure).\n\n",
        ),
        Some("generate_launch") => String::from(
            "You are a ROS 2 launch file generator. The user wants a ROS 2 Python launch file. \
             Output ONLY a single Python code block using ```python fences — no explanation \
             before or after. Follow standard ROS 2 launch conventions (LaunchDescription, \
             generate_launch_description()).\n\n",
        ),
        Some("generate_urdf") => String::from(
            "You are a ROS 2 robot description generator. The user wants a URDF or xacro file \
             describing a robot. Output ONLY a single XML code block using ```xml fences — no \
             explanation before or after. Use standard URDF conventions: <robot> root element, \
             <link> elements with <visual>/<collision>/<inertial>, <joint> elements connecting \
             them with correct parent/child and joint type. If workspace context mentions \
             specific link/frame names (e.g., from TF data or existing topics), reuse those \
             names for consistency. Prefer plain URDF unless the user explicitly asks for \
             xacro macros/parameters.\n\n",
        ),
        Some("explain_error") => String::from(
            "You are a ROS 2 / robotics debugging assistant. The user wants their most recent \
             build error or runtime error explained. Be concise and specific: name the likely \
             root cause, then give a concrete fix. Reference the actual error text from the \
             context below rather than generic advice.\n\n",
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
             robot that aren't supported by the frame names or values given.\n\n",
        ),
        _ => String::from(
            "You are an AI coding assistant. Answer the user's question directly and accurately. \
             Only reference ROS 2, robotics concepts, or the workspace context below if it is \
             actually relevant to the question — do not force a connection that isn't there.\n\n",
        ),
    };

    if open_file_path
        .as_deref()
        .is_some_and(|p| p.ends_with(".py"))
        || recent_ros_events
            .iter()
            .any(|e| e.contains("WorkspaceEvent"))
    {
        system_context.push_str("This workspace appears to involve ROS 2 development.\n\n");
    }

    let file_is_ignored = match (&open_file_path, &workspace_root) {
        (Some(path), Some(root)) => {
            is_ignored_by_aiignore(&access, Path::new(root), Path::new(path))?
        }
        _ => false,
    };

    if file_is_ignored {
        context
            .push_str("Currently open file: [path excluded — matches a pattern in .aiignore]\n\n");
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

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|error| format!("Could not configure AI request: {error}"))?;

    match settings.provider.as_str() {
        "groq" => {
            let request_body = GroqRequest {
                model: settings.model.clone(),
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

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.map_err(|e| e.to_string())?;
                return Err(format_provider_http_error(
                    "groq",
                    &settings.model,
                    status,
                    &body,
                ));
            }

            let parsed: GroqResponse = response.json().await.map_err(|e| e.to_string())?;
            parsed
                .choices
                .into_iter()
                .next()
                .map(|c| c.message.content)
                .ok_or_else(|| "No response from AI".to_string())
        }
        "anthropic" => {
            let request_body = ClaudeRequest {
                model: settings.model.clone(),
                max_tokens: 1024,
                messages: vec![ClaudeMessage {
                    role: "user".to_string(),
                    content: full_prompt,
                }],
            };
            let response = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&request_body)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.map_err(|e| e.to_string())?;
                return Err(format_provider_http_error(
                    "anthropic",
                    &settings.model,
                    status,
                    &body,
                ));
            }

            let parsed: ClaudeResponse = response.json().await.map_err(|e| e.to_string())?;
            parsed
                .content
                .into_iter()
                .next()
                .and_then(|b| b.text)
                .ok_or_else(|| "No response from AI".to_string())
        }
        "openai" => {
            let request_body = GroqRequest {
                model: settings.model.clone(),
                messages: vec![GroqMessage {
                    role: "user".to_string(),
                    content: full_prompt,
                }],
            };
            let response = client
                .post("https://api.openai.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .json(&request_body)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.map_err(|e| e.to_string())?;
                return Err(format_provider_http_error(
                    "openai",
                    &settings.model,
                    status,
                    &body,
                ));
            }

            let parsed: GroqResponse = response.json().await.map_err(|e| e.to_string())?;
            parsed
                .choices
                .into_iter()
                .next()
                .map(|c| c.message.content)
                .ok_or_else(|| "No response from AI".to_string())
        }
        other => Err(format!("Unknown provider: {}", other)),
    }
}

#[tauri::command]
fn check_aiignore(
    access: tauri::State<'_, FsAccess>,
    workspace_root: String,
    file_path: String,
) -> Result<bool, String> {
    is_ignored_by_aiignore(&access, Path::new(&workspace_root), Path::new(&file_path))
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
    access: tauri::State<'_, FsAccess>,
    template_name: String,
    destination_dir: String,
    project_name: String,
) -> Result<String, String> {
    if !is_safe_project_component(&template_name) {
        return Err("Invalid template name".to_string());
    }
    if !is_safe_project_component(&project_name) {
        return Err(
            "Project name must start with a letter and contain only letters, numbers, '_' or '-'"
                .to_string(),
        );
    }

    let template_path = app
        .path()
        .resolve(
            format!("resources/templates/{}", template_name),
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| e.to_string())?;

    let templates_root = app
        .path()
        .resolve("resources/templates", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| format!("Template directory is unavailable: {e}"))?;
    let template_path = template_path
        .canonicalize()
        .map_err(|_| format!("Template '{}' not found", template_name))?;
    if !template_path.starts_with(&templates_root) || !template_path.is_dir() {
        return Err(format!("Template '{}' not found", template_name));
    }

    let destination_root = access
        .resolve_existing(Path::new(&destination_dir))
        .map_err(|e| format!("Invalid project destination: {e}"))?;
    if !destination_root.is_dir() {
        return Err("Project destination must be a directory".to_string());
    }
    let dest = destination_root.join(&project_name);
    if dest.exists() {
        return Err(format!(
            "A folder named '{}' already exists there",
            project_name
        ));
    }

    std::fs::create_dir(&dest).map_err(|e| format!("Could not create project directory: {e}"))?;
    if let Err(error) = copy_dir_recursive(&template_path, &dest) {
        if let Err(cleanup_error) = std::fs::remove_dir_all(&dest) {
            return Err(format!(
                "Project creation failed: {error}. Incomplete project could not be removed: {cleanup_error}"
            ));
        }
        return Err(format!("Project creation failed: {error}"));
    }

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
mod credential_store_tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MockCredentialProvider {
        entries: Mutex<HashMap<String, String>>,
    }

    impl CredentialProvider for MockCredentialProvider {
        fn get(&self, provider: &str) -> Result<Option<String>, String> {
            Ok(self
                .entries
                .lock()
                .map_err(|_| "mock store lock failed".to_string())?
                .get(provider)
                .cloned())
        }

        fn set(&self, provider: &str, secret: &str) -> Result<(), String> {
            self.entries
                .lock()
                .map_err(|_| "mock store lock failed".to_string())?
                .insert(provider.to_string(), secret.to_string());
            Ok(())
        }

        fn delete(&self, provider: &str) -> Result<(), String> {
            self.entries
                .lock()
                .map_err(|_| "mock store lock failed".to_string())?
                .remove(provider);
            Ok(())
        }
    }

    #[test]
    fn mock_provider_supports_replacement_retrieval_and_deletion() {
        let provider = MockCredentialProvider::default();
        assert_eq!(provider.get("groq").unwrap(), None);
        provider.set("groq", "old").unwrap();
        assert_eq!(provider.get("groq").unwrap(), Some("old".to_string()));
        provider.set("groq", "new").unwrap();
        assert_eq!(provider.get("groq").unwrap(), Some("new".to_string()));
        provider.delete("groq").unwrap();
        assert_eq!(provider.get("groq").unwrap(), None);
    }

    #[test]
    fn rejects_unknown_credential_providers() {
        assert!(validate_ai_provider("groq").is_ok());
        assert!(validate_ai_provider("unknown").is_err());
    }
}

#[cfg(test)]
mod recent_project_tests {
    use super::*;

    #[test]
    fn test_recent_projects_reorder_and_dedupe() {
        let projects = vec![
            RecentProject {
                path: "/tmp/old".to_string(),
                name: "old".to_string(),
                last_opened: "100".to_string(),
            },
            RecentProject {
                path: "/tmp/alpha".to_string(),
                name: "alpha".to_string(),
                last_opened: "200".to_string(),
            },
            RecentProject {
                path: "/tmp/alpha".to_string(),
                name: "alpha".to_string(),
                last_opened: "300".to_string(),
            },
            RecentProject {
                path: "/tmp/beta".to_string(),
                name: "beta".to_string(),
                last_opened: "150".to_string(),
            },
        ];

        let normalized = normalize_recent_projects(projects);
        assert_eq!(normalized.len(), 3);
        assert_eq!(normalized[0].path, "/tmp/alpha");
        assert_eq!(normalized[0].last_opened, "300");
        assert_eq!(normalized[1].path, "/tmp/beta");
        assert_eq!(normalized[2].path, "/tmp/old");
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn validates_absolute_ros_topics() {
        assert!(is_valid_ros_topic("/robot/cmd_vel"));
        assert!(is_valid_ros_topic("/_robot/cmd_vel2"));
        assert!(!is_valid_ros_topic("robot/cmd_vel"));
        assert!(!is_valid_ros_topic("/robot/../cmd_vel"));
        assert!(!is_valid_ros_topic("/robot//cmd_vel"));
    }

    #[test]
    fn rejects_non_finite_and_unbounded_velocity() {
        assert!(validate_velocity(0.5, MAX_LINEAR_VELOCITY).is_ok());
        assert!(validate_velocity(0.6, MAX_LINEAR_VELOCITY).is_err());
        assert!(validate_velocity(f64::NAN, MAX_LINEAR_VELOCITY).is_err());
        assert!(validate_velocity(f64::INFINITY, MAX_LINEAR_VELOCITY).is_err());
        assert!(validate_velocity(5.1, MAX_LINEAR_VELOCITY).is_err());
    }

    #[test]
    fn validates_combined_linear_and_angular_velocity_magnitudes() {
        assert!(validate_velocity_vector([0.3, 0.3, 0.0], 0.5, "Linear").is_ok());
        assert!(validate_velocity_vector([0.4, 0.4, 0.0], 0.5, "Linear").is_err());
        assert!(validate_velocity_vector([0.8, 0.8, 0.0], 1.0, "Angular").is_err());
        assert!(validate_velocity_vector([f64::NAN, 0.0, 0.0], 0.5, "Linear").is_err());
    }

    #[test]
    fn validates_project_path_components() {
        assert!(is_safe_project_component("ros2_robot"));
        assert!(!is_safe_project_component("../escape"));
        assert!(!is_safe_project_component("2robot"));
        assert!(!is_safe_project_component(""));
    }
}

#[cfg(test)]
mod filesystem_security_tests {
    use super::*;

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            static NEXT_DIRECTORY: std::sync::atomic::AtomicU64 =
                std::sync::atomic::AtomicU64::new(0);
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "robotics-studio-test-{}-{unique}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn approved_root_allows_normal_reads_and_writes() {
        let directory = TempDirectory::new();
        let access = FsAccess::default();
        access.approve_directory(&directory.0).unwrap();
        let file = directory.0.join("robot.py");

        write_approved_file(&access, &file, "print('ok')").unwrap();

        assert_eq!(read_approved_file(&access, &file).unwrap(), "print('ok')");
    }

    #[test]
    fn rejects_paths_outside_root_and_parent_traversal() {
        let directory = TempDirectory::new();
        let outside = TempDirectory::new();
        let access = FsAccess::default();
        access.approve_directory(&directory.0).unwrap();

        assert!(access.resolve_existing(&outside.0).is_err());
        assert!(access
            .resolve_existing(&directory.0.join("../outside"))
            .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape_from_approved_root() {
        use std::os::unix::fs::symlink;

        let directory = TempDirectory::new();
        let outside = TempDirectory::new();
        let access = FsAccess::default();
        access.approve_directory(&directory.0).unwrap();
        symlink(&outside.0, directory.0.join("escape")).unwrap();

        assert!(access
            .resolve_existing(&directory.0.join("escape"))
            .is_err());
    }

    #[test]
    fn imports_worlds_to_unique_safe_paths_without_overwriting() {
        let directory = TempDirectory::new();
        let source = Path::new("../../Dockerfile.sdf");
        let sdf = "<sdf version=\"1.9\"><world name=\"test\"/></sdf>";
        let first = import_world_contents(&directory.0, source, sdf).unwrap();
        let second = import_world_contents(&directory.0, source, sdf).unwrap();

        assert_ne!(first, second);
        assert!(first.starts_with(directory.0.canonicalize().unwrap().join("worlds")));
        assert!(first
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("Dockerfile-"));
        assert_eq!(fs::read_to_string(first).unwrap(), sdf);
        assert_eq!(fs::read_to_string(second).unwrap(), sdf);
    }

    #[test]
    fn rejects_invalid_world_document_and_extension() {
        let directory = TempDirectory::new();
        assert!(
            import_world_contents(&directory.0, Path::new("world.txt"), "<sdf></sdf>").is_err()
        );
        assert!(
            import_world_contents(&directory.0, Path::new("world.sdf"), "<world></world>").is_err()
        );
    }

    #[test]
    fn validates_sdf_xml_root_and_supported_version() {
        let path = Path::new("world.sdf");
        assert!(validate_sdf_file(
            path,
            "<?xml version=\"1.0\"?><sdf version=\"1.10\"><world name=\"demo\"/></sdf>"
        )
        .is_ok());
        assert!(validate_sdf_file(path, "<sdf version=\"1.9\"><world></sdf>").is_err());
        assert!(validate_sdf_file(path, "<sdf version=\"1.11\"/>").is_err());
        assert!(validate_sdf_file(path, "<sdf version=\"1.9\"/>").is_err());
        assert!(validate_sdf_file(path, "<world version=\"1.9\"/>").is_err());
        assert!(validate_sdf_file(path, "<!DOCTYPE sdf><sdf version=\"1.9\"/>").is_err());
        assert!(validate_sdf_file(
            path,
            "<sdf version=\"1.9\"><world/></sdf><sdf version=\"1.9\"><world/></sdf>"
        )
        .is_err());
    }

    #[test]
    fn accepts_namespaced_sdf_root() {
        assert!(validate_sdf_file(
            Path::new("world.sdf"),
            "<sd:sdf xmlns:sd=\"urn:sdformat\" version=\"1.9\"><sd:world name=\"demo\"/></sd:sdf>"
        )
        .is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_world_destination() {
        use std::os::unix::fs::symlink;

        let directory = TempDirectory::new();
        let outside = TempDirectory::new();
        symlink(&outside.0, directory.0.join("worlds")).unwrap();

        assert!(
            import_world_contents(&directory.0, Path::new("world.sdf"), "<sdf></sdf>").is_err()
        );
        assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0);
    }
}

#[cfg(test)]
mod stream_line_buffer_tests {
    use super::*;

    #[test]
    fn test_unterminated_line_is_truncated_and_bounded() {
        let mut buffer = StreamLineBuffer::new();
        let oversized = vec![b'x'; MAX_PENDING_LINE_BYTES + 4096];

        let events = buffer.process_chunk(&oversized);

        assert_eq!(events.len(), 1);
        assert!(events[0].1.ends_with("[line truncated]"));
        assert!(buffer.pending.len() <= MAX_PENDING_LINE_BYTES);
        assert!(buffer.discarding_long_line);
        assert!(buffer
            .process_chunk(&vec![b'y'; MAX_PENDING_LINE_BYTES])
            .is_empty());
        assert_eq!(buffer.process_chunk(b"\nshort line\n")[0].1, "short line");
    }

    #[test]
    fn test_single_chunk_complete_multibyte_utf8() {
        // Test case (a): Single read() with complete multi-byte UTF-8 character
        // (e.g., '─' U+2500 is 3 bytes in UTF-8: E2 94 80)
        let mut buf = StreamLineBuffer::new();
        let chunk = "Box: ─ drawing\n".as_bytes();
        let events = buf.process_chunk(chunk);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "append");
        assert!(
            events[0].1.contains("─"),
            "Multi-byte UTF-8 character should be decoded"
        );
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
        assert_eq!(
            events2[0].1, "Box: ─ line",
            "Split UTF-8 character should reassemble exactly, got '{}'",
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
        assert_eq!(
            events2[0].1, "Downloading layer 3/8",
            "Should combine both chunks into one line"
        );
    }

    #[test]
    fn test_repeated_carriage_returns_docker_progress() {
        // Test case (d): Repeated \r-terminated updates simulating real Docker progress.
        // E.g., "Pulling fs layer\rDownloading 10%\rDownloading 45%\rDownloading 100%\n"
        let mut buf = StreamLineBuffer::new();
        let chunk = b"Pulling fs layer\rDownloading 10%\rDownloading 45%\rDownloading 100%\n";
        let events = buf.process_chunk(chunk);

        // Should produce 4 events: 3 "replace" and 1 final "append"
        assert_eq!(
            events.len(),
            4,
            "Should emit 4 events (3 replace + 1 append)"
        );
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
