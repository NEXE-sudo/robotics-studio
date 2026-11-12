use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

mod urdf;
mod xacro;

const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;
const MAX_WORLD_SIZE: u64 = 10 * 1024 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 10_000;
static ACTIVE_ROS_WORKSPACE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

#[derive(Default)]
struct FsAccess {
    approved_roots: Mutex<HashSet<PathBuf>>,
    #[cfg(unix)]
    root_descriptors: Mutex<HashMap<PathBuf, Arc<rustix::fd::OwnedFd>>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SafeServiceResponse {
    service_name: String,
    service_type: String,
    response_json: String,
    truncated: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SafeActionGoalUpdateResponse {
    goal_id: String,
    action_name: String,
    action_type: String,
    status: String,
    feedback_json: String,
    result_json: String,
    error: String,
    terminal: bool,
    truncated: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SafeActionGoalResponse {
    goal_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SafeActionCancelResponse {
    requested: bool,
}

impl FsAccess {
    fn approve_directory(&self, path: &Path) -> Result<PathBuf, String> {
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("Could not resolve selected directory: {error}"))?;
        if !canonical.is_dir() {
            return Err("Selected path is not a directory".to_string());
        }

        #[cfg(unix)]
        let descriptor = rustix::fs::open(
            &canonical,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(|error| format!("Could not pin selected directory: {error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let path_metadata = fs::metadata(&canonical)
                .map_err(|error| format!("Could not verify selected directory: {error}"))?;
            let descriptor_metadata = rustix::fs::fstat(&descriptor)
                .map_err(|error| format!("Could not verify selected directory: {error}"))?;
            if path_metadata.dev() != descriptor_metadata.st_dev as u64
                || path_metadata.ino() != descriptor_metadata.st_ino
            {
                return Err("Selected directory changed while it was being approved".to_string());
            }
        }

        self.approved_roots
            .lock()
            .map_err(|_| "Filesystem access state is unavailable".to_string())?
            .insert(canonical.clone());
        #[cfg(unix)]
        self.root_descriptors
            .lock()
            .map_err(|_| "Filesystem access state is unavailable".to_string())?
            .insert(canonical.clone(), Arc::new(descriptor));
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

    #[cfg(unix)]
    fn descriptor_and_relative_path(
        &self,
        path: &Path,
    ) -> Result<(rustix::fd::OwnedFd, PathBuf), String> {
        if !path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
        {
            return Err(
                "Path must be absolute and cannot contain traversal components".to_string(),
            );
        }

        let roots = self
            .approved_roots
            .lock()
            .map_err(|_| "Filesystem access state is unavailable".to_string())?;
        let candidate: PathBuf = if roots.iter().any(|root| path.starts_with(root)) {
            path.to_path_buf()
        } else {
            let normalized: std::io::Result<PathBuf> = path.canonicalize().or_else(|_| {
                let parent = path
                    .parent()
                    .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?
                    .canonicalize()?;
                let name = path
                    .file_name()
                    .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
                Ok(parent.join(name))
            });
            normalized.map_err(|_| "Path is outside approved directories".to_string())?
        };
        let root = roots
            .iter()
            .filter(|root| candidate.starts_with(root))
            .max_by_key(|root| root.components().count())
            .ok_or_else(|| "Path is outside approved directories".to_string())?;
        let relative = candidate
            .strip_prefix(root)
            .map_err(|_| "Path is outside approved directories".to_string())?;
        if relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err("Path contains an unsupported component".to_string());
        }

        let descriptors = self
            .root_descriptors
            .lock()
            .map_err(|_| "Filesystem access state is unavailable".to_string())?;
        let descriptor = descriptors
            .get(root)
            .ok_or_else(|| "Approved directory handle is unavailable".to_string())?;
        let path_metadata = fs::metadata(root)
            .map_err(|_| "An approved directory is no longer available".to_string())?;
        let descriptor_metadata = rustix::fs::fstat(&**descriptor)
            .map_err(|error| format!("Could not verify approved directory: {error}"))?;
        use std::os::unix::fs::MetadataExt;
        if path_metadata.dev() != descriptor_metadata.st_dev as u64
            || path_metadata.ino() != descriptor_metadata.st_ino
        {
            return Err("An approved directory has been moved or replaced".to_string());
        }
        let duplicated = rustix::io::dup(&**descriptor)
            .map_err(|error| format!("Could not access approved directory: {error}"))?;
        Ok((duplicated, relative.to_path_buf()))
    }

    #[cfg(unix)]
    fn open_rooted_path(&self, path: &Path, flags: rustix::fs::OFlags) -> Result<fs::File, String> {
        use rustix::fs::{openat, Mode, OFlags};

        let (mut current, relative) = self.descriptor_and_relative_path(path)?;
        let components = relative
            .components()
            .map(|component| match component {
                Component::Normal(name) => Ok(name.to_os_string()),
                _ => Err("Path contains an unsupported component".to_string()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        if components.is_empty() {
            return Ok(fs::File::from(current));
        }
        for component in &components[..components.len() - 1] {
            current = openat(
                &current,
                component,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|error| format!("Could not safely open directory: {error}"))?;
        }
        let descriptor = openat(
            &current,
            &components[components.len() - 1],
            flags | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|error| format!("Could not safely open file: {error}"))?;
        Ok(fs::File::from(descriptor))
    }

    #[cfg(unix)]
    fn rooted_parent_and_name(
        &self,
        path: &Path,
    ) -> Result<(rustix::fd::OwnedFd, std::ffi::OsString), String> {
        use rustix::fs::{openat, Mode, OFlags};

        let (mut current, relative) = self.descriptor_and_relative_path(path)?;
        let components = relative
            .components()
            .map(|component| match component {
                Component::Normal(name) => Ok(name.to_os_string()),
                _ => Err("Path contains an unsupported component".to_string()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let Some(name) = components.last().cloned() else {
            return Err("Destination must name a file inside an approved directory".to_string());
        };
        for component in &components[..components.len() - 1] {
            current = openat(
                &current,
                component,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|error| format!("Could not safely open destination directory: {error}"))?;
        }
        Ok((current, name))
    }

    #[cfg(unix)]
    fn create_directory(&self, path: &Path) -> Result<(), String> {
        let (parent, name) = self.rooted_parent_and_name(path)?;
        rustix::fs::mkdirat(&parent, name, rustix::fs::Mode::RWXU)
            .map_err(|error| format!("Could not safely create directory: {error}"))
    }

    #[cfg(not(unix))]
    fn create_directory(&self, path: &Path) -> Result<(), String> {
        let path = self.resolve_for_write(path)?;
        fs::create_dir(path).map_err(|error| format!("Could not create directory: {error}"))
    }

    fn ensure_directory(&self, path: &Path) -> Result<(), String> {
        if self.open_approved_directory(path).is_ok() {
            return Ok(());
        }
        self.create_directory(path)?;
        self.open_approved_directory(path)
    }

    #[cfg(unix)]
    fn open_approved_directory(&self, path: &Path) -> Result<(), String> {
        self.open_rooted_path(
            path,
            rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY,
        )
        .map(|_| ())
    }

    #[cfg(not(unix))]
    fn open_approved_directory(&self, path: &Path) -> Result<(), String> {
        let path = self.resolve_existing(path)?;
        if path.is_dir() {
            Ok(())
        } else {
            Err("Path is not a directory".to_string())
        }
    }

    #[cfg(unix)]
    fn create_new_file(&self, path: &Path, contents: &[u8]) -> Result<(), String> {
        use rustix::fs::{openat, unlinkat, AtFlags, Mode, OFlags};

        if contents.len() as u64 > MAX_WORLD_SIZE {
            return Err(format!("File exceeds the {MAX_WORLD_SIZE}-byte limit"));
        }
        let (parent, name) = self.rooted_parent_and_name(path)?;
        let descriptor = openat(
            &parent,
            &name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(|error| format!("Could not safely create file: {error}"))?;
        let mut file = fs::File::from(descriptor);
        let result = (|| {
            std::io::Write::write_all(&mut file, contents).map_err(|error| error.to_string())?;
            file.sync_all().map_err(|error| error.to_string())?;
            rustix::fs::fsync(&parent).map_err(|error| error.to_string())
        })();
        if result.is_err() {
            let _ = unlinkat(&parent, &name, AtFlags::empty());
        }
        result
    }

    #[cfg(unix)]
    fn copy_file_if_missing(&self, source: &Path, destination: &Path) -> Result<(), String> {
        use rustix::fs::{openat, statat, unlinkat, AtFlags, FileType, Mode, OFlags};

        let (parent, name) = self.rooted_parent_and_name(destination)?;
        match statat(&parent, &name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(metadata) if FileType::from_raw_mode(metadata.st_mode).is_file() => return Ok(()),
            Ok(_) => return Err("Copy destination must be a regular file".to_string()),
            Err(error) if error == rustix::io::Errno::NOENT => {}
            Err(error) => return Err(format!("Could not inspect copy destination: {error}")),
        }

        let descriptor = openat(
            &parent,
            &name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(|error| format!("Could not safely create copy destination: {error}"))?;
        let mut output = fs::File::from(descriptor);
        let result = (|| {
            let mut input = fs::File::open(source).map_err(|error| error.to_string())?;
            let permissions = input
                .metadata()
                .map_err(|error| error.to_string())?
                .permissions();
            std::io::copy(&mut input, &mut output).map_err(|error| error.to_string())?;
            output
                .set_permissions(permissions)
                .map_err(|error| error.to_string())?;
            output.sync_all().map_err(|error| error.to_string())?;
            rustix::fs::fsync(&parent).map_err(|error| error.to_string())
        })();
        if result.is_err() {
            let _ = unlinkat(&parent, &name, AtFlags::empty());
        }
        result
    }

    #[cfg(unix)]
    fn read_file_if_present(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
        use rustix::fs::{openat, statat, AtFlags, FileType, OFlags};

        let (parent, name) = self.rooted_parent_and_name(path)?;
        match statat(&parent, &name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(metadata) if FileType::from_raw_mode(metadata.st_mode).is_file() => {}
            Ok(_) => return Err("Workspace file must be a regular file".to_string()),
            Err(error) if error == rustix::io::Errno::NOENT => return Ok(None),
            Err(error) => return Err(format!("Could not inspect workspace file safely: {error}")),
        }
        let descriptor = openat(
            &parent,
            &name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(|error| format!("Could not safely read workspace file: {error}"))?;
        let mut file = fs::File::from(descriptor);
        let mut contents = Vec::new();
        file.by_ref()
            .take(MAX_FILE_SIZE + 1)
            .read_to_end(&mut contents)
            .map_err(|error| format!("Could not read workspace file: {error}"))?;
        if contents.len() as u64 > MAX_FILE_SIZE {
            return Err(format!(
                "Workspace file exceeds the {MAX_FILE_SIZE}-byte limit"
            ));
        }
        Ok(Some(contents))
    }

    #[cfg(not(unix))]
    fn read_file_if_present(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
        let path = self.resolve_for_write(path)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("Could not inspect workspace file: {error}")),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("Workspace file must be a regular file".to_string());
        }
        let mut file = fs::File::open(path)
            .map_err(|error| format!("Could not read workspace file: {error}"))?;
        let mut contents = Vec::new();
        file.by_ref()
            .take(MAX_FILE_SIZE + 1)
            .read_to_end(&mut contents)
            .map_err(|error| format!("Could not read workspace file: {error}"))?;
        if contents.len() as u64 > MAX_FILE_SIZE {
            return Err(format!(
                "Workspace file exceeds the {MAX_FILE_SIZE}-byte limit"
            ));
        }
        Ok(Some(contents))
    }

    #[cfg(not(unix))]
    fn copy_file_if_missing(&self, source: &Path, destination: &Path) -> Result<(), String> {
        let destination = self.resolve_for_write(destination)?;
        match fs::symlink_metadata(&destination) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                return Ok(());
            }
            Ok(_) => return Err("Copy destination must be a regular file".to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("Could not inspect copy destination: {error}")),
        }

        let mut input = fs::File::open(source).map_err(|error| error.to_string())?;
        let permissions = input
            .metadata()
            .map_err(|error| error.to_string())?
            .permissions();
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(|error| error.to_string())?;
        std::io::copy(&mut input, &mut output).map_err(|error| error.to_string())?;
        output
            .set_permissions(permissions)
            .map_err(|error| error.to_string())?;
        output.sync_all().map_err(|error| error.to_string())
    }

    #[cfg(not(unix))]
    fn create_new_file(&self, path: &Path, contents: &[u8]) -> Result<(), String> {
        if contents.len() as u64 > MAX_WORLD_SIZE {
            return Err(format!("File exceeds the {MAX_WORLD_SIZE}-byte limit"));
        }
        let path = self.resolve_for_write(path)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| error.to_string())?;
        std::io::Write::write_all(&mut file, contents).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())
    }
}

#[cfg(unix)]
fn read_approved_file(access: &FsAccess, path: &Path) -> Result<String, String> {
    let file = access.open_rooted_path(path, rustix::fs::OFlags::RDONLY)?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("Path is not a regular file".to_string());
    }
    if metadata.len() > MAX_FILE_SIZE {
        return Err(format!("File exceeds the {MAX_FILE_SIZE}-byte limit"));
    }
    read_file_limited(file, MAX_FILE_SIZE)
}

#[cfg(not(unix))]
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

fn read_file_limited(mut file: fs::File, max_bytes: u64) -> Result<String, String> {
    let mut contents = String::new();
    file.by_ref()
        .take(max_bytes + 1)
        .read_to_string(&mut contents)
        .map_err(|error| error.to_string())?;
    if contents.len() as u64 > max_bytes {
        return Err(format!("File exceeds the {max_bytes}-byte limit"));
    }
    Ok(contents)
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

#[cfg(unix)]
fn write_approved_file(access: &FsAccess, path: &Path, contents: &str) -> Result<(), String> {
    if contents.len() as u64 > MAX_FILE_SIZE {
        return Err(format!("File exceeds the {MAX_FILE_SIZE}-byte limit"));
    }
    use rustix::fs::{openat, renameat, statat, unlinkat, AtFlags, FileType, Mode, OFlags};

    let (parent, name) = access.rooted_parent_and_name(path)?;
    match statat(&parent, &name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(metadata) => {
            if !FileType::from_raw_mode(metadata.st_mode).is_file() {
                return Err("Destination must be a regular file, not a symbolic link".to_string());
            }
        }
        Err(error) if error == rustix::io::Errno::NOENT => {}
        Err(error) => return Err(format!("Could not inspect destination safely: {error}")),
    }
    let counter = std::sync::atomic::AtomicU64::fetch_add(
        &FILE_WRITE_COUNTER,
        1,
        std::sync::atomic::Ordering::Relaxed,
    );
    let temporary = format!(".write-{}.{}.tmp", std::process::id(), counter);
    let write_result = (|| {
        let descriptor = openat(
            &parent,
            &temporary,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(|error| format!("Could not create temporary file safely: {error}"))?;
        let mut file = fs::File::from(descriptor);
        std::io::Write::write_all(&mut file, contents.as_bytes())
            .map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        renameat(&parent, &temporary, &parent, &name)
            .map_err(|error| format!("Could not atomically replace destination: {error}"))?;
        rustix::fs::fsync(&parent).map_err(|error| error.to_string())
    })();
    if write_result.is_err() {
        let _ = unlinkat(&parent, &temporary, AtFlags::empty());
    }
    write_result
}

#[cfg(not(unix))]
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

#[cfg(unix)]
fn list_approved_directory(access: &FsAccess, requested: &Path) -> Result<Vec<FileEntry>, String> {
    let path = access.open_rooted_path(
        requested,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY,
    )?;
    let mut directory = rustix::fs::Dir::read_from(&path).map_err(|error| error.to_string())?;
    let mut result = Vec::new();
    while let Some(entry) = directory.read() {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "." || name == ".." {
            continue;
        }
        if result.len() >= MAX_DIRECTORY_ENTRIES {
            return Err(format!(
                "Directory contains more than {MAX_DIRECTORY_ENTRIES} entries"
            ));
        }
        result.push(FileEntry {
            path: requested.join(&name).to_string_lossy().into_owned(),
            name,
            is_dir: entry.file_type().is_dir(),
        });
    }
    Ok(result)
}

#[cfg(not(unix))]
fn list_approved_directory(access: &FsAccess, requested: &Path) -> Result<Vec<FileEntry>, String> {
    let path = access.resolve_existing(requested)?;
    if !path.is_dir() {
        return Err("Path is not a directory".to_string());
    }
    let entries = fs::read_dir(&path).map_err(|error| error.to_string())?;
    let mut result = Vec::new();
    for (index, entry) in entries.enumerate() {
        if index >= MAX_DIRECTORY_ENTRIES {
            return Err(format!(
                "Directory contains more than {MAX_DIRECTORY_ENTRIES} entries"
            ));
        }
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        result.push(FileEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path().to_string_lossy().into_owned(),
            is_dir: file_type.is_dir(),
        });
    }
    Ok(result)
}

#[tauri::command]
fn list_dir(access: tauri::State<'_, FsAccess>, path: String) -> Result<Vec<FileEntry>, String> {
    list_approved_directory(&access, Path::new(&path))
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

const MAX_XACRO_TOTAL_BYTES: usize = 8 * 1024 * 1024;
const MAX_MESH_COUNT: usize = 128;
const MAX_TOTAL_MESH_BYTES: usize = 16 * 1024 * 1024;

fn package_name_from_manifest(content: &str) -> Result<String, String> {
    let mut reader = quick_xml::Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut package_root = false;
    let mut in_name = false;
    let mut name = None;
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(element)) => {
                let local = element.local_name();
                if !package_root {
                    if local.as_ref() != b"package" {
                        return Err("ROS package manifest root must be <package>".to_string());
                    }
                    package_root = true;
                } else if local.as_ref() == b"name" && name.is_none() {
                    in_name = true;
                }
            }
            Ok(quick_xml::events::Event::Text(text)) if in_name => {
                name = Some(
                    text.decode()
                        .map_err(|error| format!("Malformed ROS package name: {error}"))?
                        .trim()
                        .to_string(),
                );
            }
            Ok(quick_xml::events::Event::End(element))
                if element.local_name().as_ref() == b"name" =>
            {
                in_name = false;
            }
            Ok(quick_xml::events::Event::DocType(_)) => {
                return Err("ROS package manifests must not contain a DTD".to_string());
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(format!("Malformed ROS package manifest: {error}")),
        }
    }
    let name = name.ok_or_else(|| "ROS package manifest is missing its name".to_string())?;
    if name.is_empty()
        || !name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
        || name.starts_with('_')
    {
        return Err(format!("Invalid ROS package name '{name}'"));
    }
    Ok(name)
}

fn find_ros_packages(
    access: &FsAccess,
    workspace: &Path,
) -> Result<HashMap<String, PathBuf>, String> {
    const MAX_SCANNED_ENTRIES: usize = 20_000;
    const MAX_PACKAGE_DEPTH: usize = 12;
    let mut packages = HashMap::new();
    let mut pending = vec![(workspace.to_path_buf(), 0usize)];
    let mut scanned = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        let manifest = directory.join("package.xml");
        if fs::symlink_metadata(&manifest)
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        {
            if fs::metadata(&manifest)
                .map_err(|error| format!("Could not inspect ROS package manifest: {error}"))?
                .len()
                > 64 * 1024
            {
                return Err("ROS package manifest exceeds the 64 KiB limit".to_string());
            }
            let content = read_approved_file(access, &manifest)?;
            if content.len() > 64 * 1024 {
                return Err("ROS package manifest exceeds the 64 KiB limit".to_string());
            }
            let name = package_name_from_manifest(&content)?;
            if packages.insert(name.clone(), directory.clone()).is_some() {
                return Err(format!(
                    "ROS package name '{name}' is ambiguous in the selected workspace"
                ));
            }
            continue;
        }
        if depth >= MAX_PACKAGE_DEPTH {
            continue;
        }
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries {
            scanned += 1;
            if scanned > MAX_SCANNED_ENTRIES {
                return Err(format!(
                    "ROS package discovery exceeds the {MAX_SCANNED_ENTRIES}-entry limit"
                ));
            }
            let entry = entry.map_err(|error| format!("Could not scan workspace: {error}"))?;
            let file_type = entry
                .file_type()
                .map_err(|error| format!("Could not inspect workspace entry: {error}"))?;
            let name = entry.file_name();
            if file_type.is_dir()
                && !file_type.is_symlink()
                && !matches!(
                    name.to_str(),
                    Some(".git" | "node_modules" | "build" | "install" | "log" | "target")
                )
            {
                pending.push((entry.path(), depth + 1));
            }
        }
    }
    Ok(packages)
}

fn package_boundary_for(
    source: &Path,
    packages: &HashMap<String, PathBuf>,
    workspace: &Path,
) -> PathBuf {
    packages
        .values()
        .filter(|root| source.starts_with(root))
        .max_by_key(|root| root.components().count())
        .cloned()
        .unwrap_or_else(|| workspace.to_path_buf())
}

fn resolve_resource_file(base: &Path, boundary: &Path, filename: &str) -> Result<PathBuf, String> {
    if filename.is_empty()
        || filename.contains("://")
        || filename.contains('\\')
        || filename.contains('\0')
    {
        return Err("Robot resource URI must be a local relative path".to_string());
    }
    let relative = Path::new(filename);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::RootDir | Component::Prefix(_)))
    {
        return Err("Robot resource path must be relative".to_string());
    }
    let resolved = base
        .join(relative)
        .canonicalize()
        .map_err(|error| format!("Could not resolve robot resource '{filename}': {error}"))?;
    if !resolved.starts_with(boundary) {
        return Err("Robot resource resolves outside its approved workspace/package root".into());
    }
    let metadata = fs::symlink_metadata(&resolved)
        .map_err(|error| format!("Could not inspect robot resource: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("Robot resource must be a regular file".to_string());
    }
    Ok(resolved)
}

fn resolve_mesh_path(
    filename: &str,
    description_path: &Path,
    workspace: &Path,
    packages: &HashMap<String, PathBuf>,
) -> Result<PathBuf, String> {
    if let Some(package_uri) = filename.strip_prefix("package://") {
        let (package_name, relative) = package_uri.split_once('/').ok_or_else(|| {
            "package:// mesh URI must include a package and file path".to_string()
        })?;
        if package_name.is_empty()
            || !package_name.chars().all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
            })
        {
            return Err("package:// URI contains an invalid package name".to_string());
        }
        let package_root = packages.get(package_name).ok_or_else(|| {
            format!("ROS package '{package_name}' is not in the selected workspace")
        })?;
        if Path::new(relative)
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
        {
            return Err(
                "package:// mesh path cannot traverse outside its package root".to_string(),
            );
        }
        resolve_resource_file(package_root, package_root, relative)
    } else {
        let boundary = package_boundary_for(description_path, packages, workspace);
        let base = description_path
            .parent()
            .ok_or_else(|| "Robot description has no parent directory".to_string())?;
        resolve_resource_file(base, &boundary, filename)
    }
}

fn render_robot_description(
    access: &FsAccess,
    workspace_root: &str,
    path: &str,
    content: &str,
) -> Result<urdf::RobotDescription, String> {
    let workspace = Path::new(workspace_root)
        .canonicalize()
        .map_err(|error| format!("Could not resolve selected workspace: {error}"))?;
    if !workspace.is_dir()
        || !access
            .approved_roots
            .lock()
            .map_err(|_| "Filesystem access state is unavailable".to_string())?
            .contains(&workspace)
    {
        return Err("Robot resources are restricted to the approved selected workspace".into());
    }
    let source_path = access.resolve_existing(Path::new(path))?;
    if !source_path.starts_with(&workspace) {
        return Err("Robot description is outside the selected workspace".to_string());
    }
    if content.len() > 2 * 1024 * 1024 {
        return Err("Robot description exceeds the 2 MiB source limit".to_string());
    }
    let packages = find_ros_packages(access, &workspace)?;
    let is_xacro = source_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xacro"));
    let expanded = if is_xacro {
        let mut total_include_bytes = content.len();
        let mut include_loader = |including_file: &Path,
                                  filename: &str|
         -> Result<(PathBuf, String), String> {
            if filename.contains('\\') {
                return Err("Xacro includes must use workspace-local relative paths".to_string());
            }
            let include_path = if let Some(package_uri) = filename.strip_prefix("package://") {
                let (package_name, relative) = package_uri.split_once('/').ok_or_else(|| {
                    "Xacro package include must name a package and file".to_string()
                })?;
                let package_root = packages.get(package_name).ok_or_else(|| {
                    format!("ROS package '{package_name}' is not in the selected workspace")
                })?;
                if Path::new(relative)
                    .components()
                    .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
                {
                    return Err(
                        "Xacro package include cannot traverse its package root".to_string()
                    );
                }
                resolve_resource_file(package_root, package_root, relative)?
            } else {
                if Path::new(filename).is_absolute() || filename.contains("://") {
                    return Err(
                        "Xacro includes must use workspace-local relative paths".to_string()
                    );
                }
                let include_boundary = package_boundary_for(including_file, &packages, &workspace);
                let include_base = including_file
                    .parent()
                    .ok_or_else(|| "Xacro source has no parent directory".to_string())?;
                resolve_resource_file(include_base, &include_boundary, filename)?
            };
            if fs::metadata(&include_path)
                .map_err(|error| format!("Could not inspect Xacro include: {error}"))?
                .len()
                > 2 * 1024 * 1024
            {
                return Err("Xacro include exceeds the 2 MiB source limit".to_string());
            }
            let include_content = read_approved_file(access, &include_path)?;
            if include_content.len() > 2 * 1024 * 1024 {
                return Err("Xacro include exceeds the 2 MiB source limit".to_string());
            }
            total_include_bytes = total_include_bytes.saturating_add(include_content.len());
            if total_include_bytes > MAX_XACRO_TOTAL_BYTES {
                return Err("Xacro source and includes exceed the 8 MiB total limit".into());
            }
            Ok((include_path, include_content))
        };
        xacro::expand(content, &source_path, &mut include_loader)?
    } else {
        content.to_string()
    };

    let mut robot = urdf::parse_robot_description(&expanded)?;
    let mut mesh_count = 0usize;
    let mut mesh_bytes = 0usize;
    for link in &mut robot.links {
        for visual in link.visuals.iter_mut().chain(link.collisions.iter_mut()) {
            let urdf::RobotGeometry::Mesh {
                filename,
                format,
                data,
                ..
            } = &mut visual.geometry
            else {
                continue;
            };
            mesh_count += 1;
            if mesh_count > MAX_MESH_COUNT {
                return Err(format!("Robot mesh count exceeds {MAX_MESH_COUNT}"));
            }
            let mesh_path = resolve_mesh_path(filename, &source_path, &workspace, &packages)?;
            let mesh_content = access
                .read_file_if_present(&mesh_path)?
                .ok_or_else(|| "Robot mesh disappeared while loading".to_string())?;
            mesh_bytes = mesh_bytes.saturating_add(mesh_content.len());
            if mesh_bytes > MAX_TOTAL_MESH_BYTES {
                return Err("Robot meshes exceed the 16 MiB total byte limit".to_string());
            }
            urdf::validate_mesh_bytes(format, &mesh_content)?;
            *data = Some(base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                mesh_content,
            ));
        }
    }
    Ok(robot)
}

#[tauri::command]
fn parse_robot_description(
    access: tauri::State<'_, FsAccess>,
    workspace_root: String,
    path: String,
    content: String,
) -> Result<urdf::RobotDescription, String> {
    render_robot_description(&access, &workspace_root, &path, &content)
}

#[cfg(test)]
mod urdf_rendering_tests {
    use super::{render_robot_description, FsAccess};
    use crate::urdf::RobotGeometry;
    use std::fs;

    fn fixture_workspace() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/urdf_workspace")
            .canonicalize()
            .unwrap()
    }

    #[test]
    fn expands_xacro_and_loads_only_workspace_package_meshes() {
        let access = FsAccess::default();
        let workspace = fixture_workspace();
        access.approve_directory(&workspace).unwrap();
        let description_path = workspace
            .join("src/demo_robot/urdf/robot.xacro")
            .canonicalize()
            .unwrap();
        let content = fs::read_to_string(&description_path).unwrap();
        let robot = render_robot_description(
            &access,
            workspace.to_str().unwrap(),
            description_path.to_str().unwrap(),
            &content,
        )
        .unwrap();

        assert_eq!(robot.name, "demo_robot");
        assert_eq!(robot.root_link, "base");
        assert_eq!(robot.links[0].visuals[0].color, Some([0.1, 0.3, 0.8, 1.0]));
        assert_eq!(robot.links[0].inertial.as_ref().unwrap().mass, 1.5);
        assert_eq!(robot.links[0].collisions.len(), 1);
        let RobotGeometry::Mesh {
            filename,
            scale,
            format,
            data,
        } = &robot.links[0].visuals[0].geometry
        else {
            panic!("expected resolved STL mesh");
        };
        assert_eq!(filename, "package://demo_robot/meshes/triangle.stl");
        assert_eq!(*scale, [1.0, 2.0, 1.0]);
        assert_eq!(format, "stl");
        assert!(data.as_ref().is_some_and(|mesh| !mesh.is_empty()));
    }

    #[test]
    fn rejects_unapproved_roots_and_package_traversal_in_mesh_uris() {
        let workspace = fixture_workspace();
        let access = FsAccess::default();
        let description_path = workspace.join("src/demo_robot/urdf/robot.xacro");
        let content = fs::read_to_string(&description_path).unwrap();

        assert!(render_robot_description(
            &access,
            workspace.to_str().unwrap(),
            description_path.to_str().unwrap(),
            &content,
        )
        .unwrap_err()
        .contains("approved selected workspace"));

        access.approve_directory(&workspace).unwrap();
        let traversal = content.replace(
            "package://demo_robot/meshes/triangle.stl",
            "package://demo_robot/../../outside.stl",
        );
        assert!(render_robot_description(
            &access,
            workspace.to_str().unwrap(),
            description_path.to_str().unwrap(),
            &traversal,
        )
        .unwrap_err()
        .contains("cannot traverse"));
    }

    #[test]
    fn resolves_and_validates_texture_free_collada_meshes() {
        let access = FsAccess::default();
        let workspace = fixture_workspace();
        access.approve_directory(&workspace).unwrap();
        let description_path = workspace
            .join("src/demo_robot/urdf/robot.xacro")
            .canonicalize()
            .unwrap();
        let content = r#"<robot name="dae_test"><link name="base"><visual><geometry><mesh filename="package://demo_robot/meshes/triangle.dae"/></geometry></visual></link></robot>"#;
        let robot = render_robot_description(
            &access,
            workspace.to_str().unwrap(),
            description_path.to_str().unwrap(),
            content,
        )
        .unwrap();
        let RobotGeometry::Mesh {
            format,
            data: Some(data),
            ..
        } = &robot.links[0].visuals[0].geometry
        else {
            panic!("expected resolved DAE mesh");
        };
        assert_eq!(format, "dae");
        let decoded =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data).unwrap();
        let original = fs::read(workspace.join("src/demo_robot/meshes/triangle.dae")).unwrap();
        assert_eq!(decoded, original);
    }
}

fn import_world_contents(
    access: &FsAccess,
    workspace: &Path,
    source: &Path,
    content: &str,
) -> Result<PathBuf, String> {
    validate_sdf_file(source, content)?;
    let workspace = access
        .resolve_existing(workspace)
        .map_err(|error| format!("ROS workspace is unavailable: {error}"))?;
    let worlds_dir = workspace.join("worlds");
    access.ensure_directory(&worlds_dir)?;

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
        let destination = worlds_dir.join(&name);
        match access.create_new_file(&destination, content.as_bytes()) {
            Ok(()) => return Ok(destination),
            Err(error) if error.contains("File exists") => continue,
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
    let imported = import_world_contents(&access, &workspace, &source, &content)?;
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
            set_ros_workspace,
            parse_robot_description,
            pick_save_path,
            pick_keybindings_file,
            import_world,
            start_ros_stream,
            run_colcon_build,
            run_colcon_build_streaming,
            cancel_colcon_build,
            list_managed_processes,
            list_ros_packages,
            start_gazebo_sim,
            stop_gazebo_sim,
            reset_gazebo_sim,
            ask_ai,
            initialize_ros_environment,
            publish_twist,
            invoke_safe_ros_service,
            send_safe_ros_action_goal,
            cancel_safe_ros_action_goal,
            start_topic_inspection,
            stop_topic_inspection,
            set_emergency_stop,
            get_safety_state,
            get_robot_config,
            save_robot_config,
            reset_robot_config,
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
static LAST_ROS_EVENT_SEQUENCE_BY_WORKSPACE: OnceLock<Mutex<HashMap<String, u64>>> =
    OnceLock::new();
static ROS_SERVER_TLS_CERTIFICATE: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();

fn set_ros_server_tls_certificate(certificate: Vec<u8>) -> Result<(), String> {
    if !certificate.starts_with(b"-----BEGIN CERTIFICATE-----")
        || !certificate.ends_with(b"-----END CERTIFICATE-----\n")
    {
        return Err("ROS service returned an invalid TLS certificate".to_string());
    }
    *ROS_SERVER_TLS_CERTIFICATE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "ROS TLS certificate state is unavailable".to_string())? = Some(certificate);
    Ok(())
}

async fn connect_ros_engine() -> Result<RosEngineClient<tonic::transport::Channel>, String> {
    let certificate = ROS_SERVER_TLS_CERTIFICATE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "ROS TLS certificate state is unavailable".to_string())?
        .clone()
        .ok_or_else(|| "ROS service TLS certificate is not initialized".to_string())?;
    let tls = tonic::transport::ClientTlsConfig::new()
        .ca_certificate(tonic::transport::Certificate::from_pem(certificate))
        .domain_name("localhost");
    let endpoint = tonic::transport::Endpoint::from_static("https://127.0.0.1:50051")
        .tls_config(tls)
        .map_err(|error| format!("Could not configure ROS TLS: {error}"))?;
    let channel = endpoint
        .connect()
        .await
        .map_err(|error| format!("Could not establish authenticated ROS TLS: {error:?}"))?;
    Ok(RosEngineClient::new(channel))
}

fn ros_event_cursor(workspace: &str) -> Result<u64, String> {
    let cursors = LAST_ROS_EVENT_SEQUENCE_BY_WORKSPACE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "ROS event recovery state is unavailable".to_string())?;
    Ok(*cursors.get(workspace).unwrap_or(&0))
}

fn update_ros_event_cursor(
    workspace: &str,
    sequence: u64,
    reset_on_gap: bool,
) -> Result<(), String> {
    let mut cursors = LAST_ROS_EVENT_SEQUENCE_BY_WORKSPACE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "ROS event recovery state is unavailable".to_string())?;
    let cursor = cursors.entry(workspace.to_string()).or_default();
    if reset_on_gap && sequence < *cursor {
        *cursor = sequence;
    } else {
        *cursor = (*cursor).max(sequence);
    }
    Ok(())
}

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
            let stream_workspace = match get_default_workspace_dir() {
                Ok(workspace) => workspace,
                Err(error) => {
                    let _ = app.emit("ros-error", error);
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    continue;
                }
            };
            let token = match read_grpc_auth_token(&app) {
                Ok(token) => token,
                Err(error) => {
                    stop_stream_for_auth_recovery(&app, &error);
                    return;
                }
            };
            let _ = app.emit("ros-connecting", ());
            println!("[ros-stream] attempting authenticated TLS connection to ROS service");
            let client_result =
                tokio::time::timeout(std::time::Duration::from_secs(5), connect_ros_engine()).await;

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
            let request = match authenticated_request(
                rosengine::ReplayEventsRequest {
                    has_after_sequence: true,
                    after_sequence: match ros_event_cursor(&stream_workspace) {
                        Ok(sequence) => sequence,
                        Err(error) => {
                            let _ = app.emit("ros-error", error);
                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                            continue;
                        }
                    },
                },
                &token,
            ) {
                Ok(request) => request,
                Err(error) => {
                    let _ = app.emit("ros-error", error);
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    continue;
                }
            };
            let mut stream = match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                client.replay_events(request),
            )
            .await
            {
                Ok(Ok(s)) => {
                    println!("[ros-stream] replayable event stream established");
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
                    let _ = app.emit("ros-error", "replay_events timed out".to_string());
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    continue;
                }
            };

            loop {
                match stream.message().await {
                    Ok(Some(evt)) => {
                        if !get_default_workspace_dir()
                            .is_ok_and(|workspace| workspace == stream_workspace)
                        {
                            break;
                        }
                        println!("[ros-stream] received event: {:?}", evt);
                        let _ = app.emit("ros-event", format!("{:?}", evt));
                        if evt.sequence > 0 {
                            if let Err(error) =
                                update_ros_event_cursor(&stream_workspace, evt.sequence, false)
                            {
                                let _ = app.emit("ros-error", error);
                            }
                        }

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
                        if let Some(rosengine::workspace_event::Event::TopicInspectionUpdate(
                            update,
                        )) = &evt.event
                        {
                            let _ = app.emit(
                                "topic-inspection-update",
                                TopicInspectionUpdateResponse {
                                    topic_name: update.topic_name.clone(),
                                    message_type: update.message_type.clone(),
                                    json_payload: update.json_payload.clone(),
                                    active: update.active,
                                    truncated: update.truncated,
                                    error: update.error.clone(),
                                },
                            );
                        }
                        if let Some(rosengine::workspace_event::Event::ActionGoalUpdate(update)) =
                            &evt.event
                        {
                            let _ = app.emit(
                                "safe-action-goal-update",
                                SafeActionGoalUpdateResponse {
                                    goal_id: update.goal_id.clone(),
                                    action_name: update.action_name.clone(),
                                    action_type: update.action_type.clone(),
                                    status: update.status.clone(),
                                    feedback_json: update.feedback_json.clone(),
                                    result_json: update.result_json.clone(),
                                    error: update.error.clone(),
                                    terminal: update.terminal,
                                    truncated: update.truncated,
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
                                    topic_name: tf.topic_name.clone(),
                                    source_timestamp_ms: tf.source_timestamp_ms,
                                    received_at_ms: evt.timestamp_ns.div_euclid(1_000_000),
                                },
                            );
                        }
                        if let Some(rosengine::workspace_event::Event::NodeSnapshot(snap)) =
                            &evt.event
                        {
                            let _ = app.emit("node-snapshot", snap.node_names.clone());
                        }
                        if let Some(rosengine::workspace_event::Event::RosGraphSnapshot(snapshot)) =
                            &evt.event
                        {
                            let _ = app
                                .emit("ros-graph-snapshot", ros_graph_snapshot_response(snapshot));
                        }
                        if let Some(rosengine::workspace_event::Event::JointStateUpdate(update)) =
                            &evt.event
                        {
                            let _ = app.emit(
                                "joint-state-update",
                                JointStateUpdateResponse {
                                    topic_name: update.topic_name.clone(),
                                    names: update.names.clone(),
                                    positions: update.positions.clone(),
                                    truncated: update.truncated,
                                },
                            );
                        }
                        if let Some(rosengine::workspace_event::Event::SafetyState(state)) =
                            &evt.event
                        {
                            let _ = app.emit("safety-state", safety_state_response(state));
                        }
                        if let Some(rosengine::workspace_event::Event::EventHealth(health)) =
                            &evt.event
                        {
                            let _ = app.emit(
                                "event-health",
                                EventHealthResponse {
                                    queue_capacity: health.queue_capacity,
                                    subscriber_count: health.subscriber_count,
                                    last_sequence: health.last_sequence,
                                    dropped_telemetry: health.dropped_telemetry,
                                    critical_overflows: health.critical_overflows,
                                    max_backlog: health.max_backlog,
                                    persistence_healthy: health.persistence_healthy,
                                    persistence_error: health.persistence_error.clone(),
                                    oldest_available_sequence: health.oldest_available_sequence,
                                    retained_critical_events: health.retained_critical_events,
                                },
                            );
                        }
                        if let Some(rosengine::workspace_event::Event::EventRecovery(recovery)) =
                            &evt.event
                        {
                            if recovery.gap_detected {
                                match ros_event_cursor(&stream_workspace) {
                                    Ok(cursor) if recovery.latest_sequence < cursor => {
                                        if let Err(error) = update_ros_event_cursor(
                                            &stream_workspace,
                                            recovery.latest_sequence,
                                            true,
                                        ) {
                                            let _ = app.emit("ros-error", error);
                                        }
                                    }
                                    Err(error) => {
                                        let _ = app.emit("ros-error", error);
                                    }
                                    _ => {}
                                }
                            }
                            let _ = app.emit(
                                "event-recovery",
                                EventRecoveryResponse {
                                    gap_detected: recovery.gap_detected,
                                    oldest_available_sequence: recovery.oldest_available_sequence,
                                    latest_sequence: recovery.latest_sequence,
                                    snapshot_included: recovery.snapshot_included,
                                    retained_critical_events: recovery.retained_critical_events,
                                },
                            );
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

    let mut client = connect_ros_engine()
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

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SafetyStateResponse {
    state: String,
    detail: String,
    updated_at_ns: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct EventHealthResponse {
    queue_capacity: u32,
    subscriber_count: u32,
    last_sequence: u64,
    dropped_telemetry: u64,
    critical_overflows: u64,
    max_backlog: u32,
    persistence_healthy: bool,
    persistence_error: String,
    oldest_available_sequence: u64,
    retained_critical_events: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct EventRecoveryResponse {
    gap_detected: bool,
    oldest_available_sequence: u64,
    latest_sequence: u64,
    snapshot_included: bool,
    retained_critical_events: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RosGraphNodeResponse {
    name: String,
    namespace: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RosGraphEndpointResponse {
    node_name: String,
    node_namespace: String,
    topic_type: String,
    reliability: String,
    durability: String,
    history: String,
    depth: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RosGraphTopicResponse {
    name: String,
    types: Vec<String>,
    publishers: Vec<RosGraphEndpointResponse>,
    subscribers: Vec<RosGraphEndpointResponse>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RosGraphServiceResponse {
    name: String,
    types: Vec<String>,
    server_nodes: Vec<String>,
    client_nodes: Vec<String>,
    safe_types: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RosGraphActionResponse {
    name: String,
    types: Vec<String>,
    server_nodes: Vec<String>,
    client_nodes: Vec<String>,
    safe_types: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RosGraphSnapshotResponse {
    nodes: Vec<RosGraphNodeResponse>,
    topics: Vec<RosGraphTopicResponse>,
    services: Vec<RosGraphServiceResponse>,
    actions: Vec<RosGraphActionResponse>,
    truncated: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct JointStateUpdateResponse {
    topic_name: String,
    names: Vec<String>,
    positions: Vec<f64>,
    truncated: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TopicInspectionUpdateResponse {
    topic_name: String,
    message_type: String,
    json_payload: String,
    active: bool,
    truncated: bool,
    error: String,
}

fn ros_graph_snapshot_response(snapshot: &rosengine::RosGraphSnapshot) -> RosGraphSnapshotResponse {
    RosGraphSnapshotResponse {
        nodes: snapshot
            .nodes
            .iter()
            .map(|node| RosGraphNodeResponse {
                name: node.name.clone(),
                namespace: node.namespace.clone(),
            })
            .collect(),
        topics: snapshot
            .topics
            .iter()
            .map(|topic| RosGraphTopicResponse {
                name: topic.name.clone(),
                types: topic.types.clone(),
                publishers: topic
                    .publishers
                    .iter()
                    .map(|endpoint| RosGraphEndpointResponse {
                        node_name: endpoint.node_name.clone(),
                        node_namespace: endpoint.node_namespace.clone(),
                        topic_type: endpoint.topic_type.clone(),
                        reliability: endpoint.reliability.clone(),
                        durability: endpoint.durability.clone(),
                        history: endpoint.history.clone(),
                        depth: endpoint.depth,
                    })
                    .collect(),
                subscribers: topic
                    .subscribers
                    .iter()
                    .map(|endpoint| RosGraphEndpointResponse {
                        node_name: endpoint.node_name.clone(),
                        node_namespace: endpoint.node_namespace.clone(),
                        topic_type: endpoint.topic_type.clone(),
                        reliability: endpoint.reliability.clone(),
                        durability: endpoint.durability.clone(),
                        history: endpoint.history.clone(),
                        depth: endpoint.depth,
                    })
                    .collect(),
            })
            .collect(),
        services: snapshot
            .services
            .iter()
            .map(|service| RosGraphServiceResponse {
                name: service.name.clone(),
                types: service.types.clone(),
                server_nodes: service.server_nodes.clone(),
                client_nodes: service.client_nodes.clone(),
                safe_types: service.safe_types.clone(),
            })
            .collect(),
        actions: snapshot
            .actions
            .iter()
            .map(|action| RosGraphActionResponse {
                name: action.name.clone(),
                types: action.types.clone(),
                server_nodes: action.server_nodes.clone(),
                client_nodes: action.client_nodes.clone(),
                safe_types: action.safe_types.clone(),
            })
            .collect(),
        truncated: snapshot.truncated,
    }
}

fn safety_state_response(state: &rosengine::SafetyState) -> SafetyStateResponse {
    SafetyStateResponse {
        state: match state.state {
            0 => "READY",
            1 => "ACTIVE",
            2 => "RESET_PENDING",
            3 => "FAULT",
            _ => "DISCONNECTED",
        }
        .to_string(),
        detail: state.detail.clone(),
        updated_at_ns: state.updated_at_ns,
    }
}

async fn connect_ros_client(
    app: &AppHandle,
) -> Result<(RosEngineClient<tonic::transport::Channel>, String), String> {
    let token = read_grpc_auth_token(app)?;
    let client = tokio::time::timeout(std::time::Duration::from_secs(5), connect_ros_engine())
        .await
        .map_err(|_| "ROS service connection timed out".to_string())??;
    Ok((client, token))
}

const SAFE_READ_ONLY_SERVICE_TYPES: &[&str] = &[
    "lifecycle_msgs/srv/GetAvailableStates",
    "lifecycle_msgs/srv/GetAvailableTransitions",
    "lifecycle_msgs/srv/GetState",
    "lifecycle_msgs/srv/GetTransitionGraph",
    "rcl_interfaces/srv/DescribeParameters",
    "rcl_interfaces/srv/GetParameterTypes",
    "rcl_interfaces/srv/GetParameters",
    "rcl_interfaces/srv/ListParameters",
];
const SAFE_NON_MOTION_ACTION_TYPES: &[&str] = &["action_tutorials_interfaces/action/Fibonacci"];

#[tauri::command]
async fn invoke_safe_ros_service(
    app: AppHandle,
    service_name: String,
    service_type: String,
    request_json: String,
    timeout_ms: u32,
) -> Result<SafeServiceResponse, String> {
    if !is_valid_ros_topic(&service_name) {
        return Err("Invalid ROS service name".to_string());
    }
    if !SAFE_READ_ONLY_SERVICE_TYPES.contains(&service_type.as_str()) {
        return Err("This ROS service type is not on the read-only allowlist".to_string());
    }
    if !(100..=5_000).contains(&timeout_ms) {
        return Err("Service timeout must be between 100 and 5000 milliseconds".to_string());
    }
    if request_json.len() > 16 * 1024
        || !serde_json::from_str::<serde_json::Value>(&request_json)
            .is_ok_and(|value| value.is_object())
    {
        return Err("Service request must be a JSON object no larger than 16 KiB".to_string());
    }
    let (mut client, token) = connect_ros_client(&app).await?;
    let response = client
        .invoke_safe_service(authenticated_request(
            rosengine::SafeServiceRequest {
                service_name,
                service_type,
                request_json,
                timeout_ms,
            },
            &token,
        )?)
        .await
        .map_err(|error| format!("Safe ROS service call failed: {}", error.message()))?
        .into_inner();
    Ok(SafeServiceResponse {
        service_name: response.service_name,
        service_type: response.service_type,
        response_json: response.response_json,
        truncated: response.truncated,
    })
}

#[tauri::command]
async fn send_safe_ros_action_goal(
    app: AppHandle,
    action_name: String,
    action_type: String,
    goal_json: String,
) -> Result<SafeActionGoalResponse, String> {
    if !is_valid_ros_topic(&action_name) {
        return Err("Invalid ROS action name".to_string());
    }
    if !SAFE_NON_MOTION_ACTION_TYPES.contains(&action_type.as_str()) {
        return Err("This ROS action type is not on the non-motion allowlist".to_string());
    }
    if goal_json.len() > 16 * 1024
        || !serde_json::from_str::<serde_json::Value>(&goal_json)
            .is_ok_and(|value| value.is_object())
    {
        return Err("Action goal must be a JSON object no larger than 16 KiB".to_string());
    }
    let (mut client, token) = connect_ros_client(&app).await?;
    let response = client
        .send_safe_action_goal(authenticated_request(
            rosengine::SafeActionGoalRequest {
                action_name,
                action_type,
                goal_json,
            },
            &token,
        )?)
        .await
        .map_err(|error| format!("Could not submit ROS action goal: {}", error.message()))?
        .into_inner();
    Ok(SafeActionGoalResponse {
        goal_id: response.goal_id,
    })
}

#[tauri::command]
async fn cancel_safe_ros_action_goal(
    app: AppHandle,
    goal_id: String,
) -> Result<SafeActionCancelResponse, String> {
    if goal_id.len() != 36
        || !goal_id
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() || ch == '-')
    {
        return Err("Invalid action goal identifier".to_string());
    }
    let (mut client, token) = connect_ros_client(&app).await?;
    let response = client
        .cancel_action_goal(authenticated_request(
            rosengine::ActionGoalIdRequest { goal_id },
            &token,
        )?)
        .await
        .map_err(|error| format!("Could not cancel ROS action goal: {}", error.message()))?
        .into_inner();
    Ok(SafeActionCancelResponse {
        requested: response.requested,
    })
}

#[tauri::command]
async fn start_topic_inspection(app: AppHandle, topic_name: String) -> Result<(), String> {
    if !is_valid_ros_topic(&topic_name) {
        return Err("Invalid ROS topic name".to_string());
    }
    let (mut client, token) = connect_ros_client(&app).await?;
    client
        .start_topic_inspection(authenticated_request(
            rosengine::TopicInspectionRequest { topic_name },
            &token,
        )?)
        .await
        .map_err(|error| format!("Could not inspect ROS topic: {}", error.message()))?;
    Ok(())
}

#[tauri::command]
async fn stop_topic_inspection(app: AppHandle, topic_name: String) -> Result<(), String> {
    if !is_valid_ros_topic(&topic_name) {
        return Err("Invalid ROS topic name".to_string());
    }
    let (mut client, token) = connect_ros_client(&app).await?;
    client
        .stop_topic_inspection(authenticated_request(
            rosengine::TopicInspectionRequest { topic_name },
            &token,
        )?)
        .await
        .map_err(|error| format!("Could not stop ROS topic inspection: {}", error.message()))?;
    Ok(())
}

#[tauri::command]
async fn set_emergency_stop(app: AppHandle, activate: bool) -> Result<SafetyStateResponse, String> {
    let (mut client, token) = connect_ros_client(&app).await?;
    let response = client
        .set_emergency_stop(authenticated_request(
            rosengine::EmergencyStopRequest {
                action: if activate { 1 } else { 2 },
            },
            &token,
        )?)
        .await
        .map_err(|error| format!("Emergency-stop request failed: {}", error.message()))?
        .into_inner();
    Ok(safety_state_response(&response))
}

#[tauri::command]
async fn get_safety_state(app: AppHandle) -> Result<SafetyStateResponse, String> {
    let (mut client, token) = connect_ros_client(&app).await?;
    let response = client
        .get_safety_state(authenticated_request(Empty {}, &token)?)
        .await
        .map_err(|error| format!("Could not retrieve safety state: {}", error.message()))?
        .into_inner();
    Ok(safety_state_response(&response))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RobotConfigResponse {
    topic_name: String,
    max_linear_velocity: f64,
    max_angular_velocity: f64,
    max_linear_acceleration: f64,
    max_angular_acceleration: f64,
    command_lease_ms: u32,
    command_timeout_ms: u32,
    permitted_motion_modes: Vec<String>,
}

impl From<rosengine::RobotConfig> for RobotConfigResponse {
    fn from(config: rosengine::RobotConfig) -> Self {
        Self {
            topic_name: config.topic_name,
            max_linear_velocity: config.max_linear_velocity,
            max_angular_velocity: config.max_angular_velocity,
            max_linear_acceleration: config.max_linear_acceleration,
            max_angular_acceleration: config.max_angular_acceleration,
            command_lease_ms: config.command_lease_ms,
            command_timeout_ms: config.command_timeout_ms,
            permitted_motion_modes: config.permitted_motion_modes,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RobotConfigInput {
    topic_name: String,
    max_linear_velocity: f64,
    max_angular_velocity: f64,
    max_linear_acceleration: f64,
    max_angular_acceleration: f64,
    command_lease_ms: u32,
    command_timeout_ms: u32,
    permitted_motion_modes: Vec<String>,
}

#[tauri::command]
async fn get_robot_config(
    app: AppHandle,
    topic_name: String,
) -> Result<RobotConfigResponse, String> {
    if !is_valid_ros_topic(&topic_name) {
        return Err("Invalid robot command topic".to_string());
    }
    let (mut client, token) = connect_ros_client(&app).await?;
    let response = client
        .get_robot_config(authenticated_request(
            rosengine::RobotConfigRequest { topic_name },
            &token,
        )?)
        .await
        .map_err(|error| format!("Could not retrieve robot settings: {}", error.message()))?
        .into_inner();
    Ok(response.into())
}

#[tauri::command]
async fn save_robot_config(
    app: AppHandle,
    config: RobotConfigInput,
) -> Result<RobotConfigResponse, String> {
    if !is_valid_ros_topic(&config.topic_name) {
        return Err("Invalid robot command topic".to_string());
    }
    let proto_config = rosengine::RobotConfig {
        topic_name: config.topic_name,
        max_linear_velocity: config.max_linear_velocity,
        max_angular_velocity: config.max_angular_velocity,
        max_linear_acceleration: config.max_linear_acceleration,
        max_angular_acceleration: config.max_angular_acceleration,
        command_lease_ms: config.command_lease_ms,
        command_timeout_ms: config.command_timeout_ms,
        permitted_motion_modes: config.permitted_motion_modes,
        reset_to_defaults: false,
    };
    let (mut client, token) = connect_ros_client(&app).await?;
    let response = client
        .set_robot_config(authenticated_request(proto_config, &token)?)
        .await
        .map_err(|error| format!("Robot settings were rejected: {}", error.message()))?
        .into_inner();
    Ok(response.into())
}

#[tauri::command]
async fn reset_robot_config(
    app: AppHandle,
    topic_name: String,
) -> Result<RobotConfigResponse, String> {
    if !is_valid_ros_topic(&topic_name) {
        return Err("Invalid robot command topic".to_string());
    }
    let (mut client, token) = connect_ros_client(&app).await?;
    let response = client
        .set_robot_config(authenticated_request(
            rosengine::RobotConfig {
                topic_name,
                max_linear_velocity: 0.0,
                max_angular_velocity: 0.0,
                max_linear_acceleration: 0.0,
                max_angular_acceleration: 0.0,
                command_lease_ms: 0,
                command_timeout_ms: 0,
                permitted_motion_modes: Vec::new(),
                reset_to_defaults: true,
            },
            &token,
        )?)
        .await
        .map_err(|error| format!("Could not reset robot settings: {}", error.message()))?
        .into_inner();
    Ok(response.into())
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

fn workspace_selection_file() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| "Could not determine the current user's home directory".to_string())?;
    Ok(PathBuf::from(home)
        .join(".robotics-studio")
        .join("selected-workspace.txt"))
}

fn load_selected_workspace() -> Result<Option<PathBuf>, String> {
    let path = workspace_selection_file()?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Could not inspect selected workspace: {error}")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 4096 {
        return Err("Selected workspace record is unsafe or invalid".to_string());
    }
    let value = fs::read_to_string(path)
        .map_err(|error| format!("Could not read selected workspace record: {error}"))?;
    let selected = PathBuf::from(value.trim());
    if !selected.is_absolute() {
        return Err("Selected workspace record must contain an absolute path".to_string());
    }
    let canonical = selected
        .canonicalize()
        .map_err(|error| format!("Previously selected ROS workspace is unavailable: {error}"))?;
    if canonical != selected || !canonical.is_dir() {
        return Err("Previously selected ROS workspace was moved or replaced".to_string());
    }
    Ok(Some(canonical))
}

fn workspace_path_for_shutdown() -> Result<PathBuf, String> {
    if let Some(workspace) = ACTIVE_ROS_WORKSPACE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Active ROS workspace state is unavailable".to_string())?
        .clone()
    {
        return Ok(workspace);
    }
    let selection_file = workspace_selection_file()?;
    match fs::symlink_metadata(&selection_file) {
        Ok(metadata)
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > 4096 =>
        {
            return Err("Selected workspace record is unsafe or invalid".to_string());
        }
        Ok(_) => {
            let value = fs::read_to_string(selection_file)
                .map_err(|error| format!("Could not read selected workspace record: {error}"))?;
            let workspace = PathBuf::from(value.trim());
            if !workspace.is_absolute() {
                return Err("Selected workspace record must contain an absolute path".to_string());
            }
            return Ok(workspace);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Could not inspect selected workspace: {error}")),
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| "Could not determine the current user's home directory".to_string())?;
    Ok(PathBuf::from(home)
        .join(".robotics-studio")
        .join("ros2-workspace"))
}

fn persist_selected_workspace(workspace: &Path) -> Result<(), String> {
    let selection_file = workspace_selection_file()?;
    let parent = selection_file
        .parent()
        .ok_or_else(|| "Workspace settings directory is invalid".to_string())?;
    match fs::symlink_metadata(parent) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err("Workspace settings directory is unsafe".to_string());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(parent)
                .map_err(|error| format!("Could not create workspace settings: {error}"))?;
        }
        Err(error) => return Err(format!("Could not inspect workspace settings: {error}")),
    }
    static NEXT_TEMP_FILE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temporary = parent.join(format!(
        ".selected-workspace-{}-{}.tmp",
        std::process::id(),
        NEXT_TEMP_FILE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|error| format!("Could not create workspace settings: {error}"))?;
        std::io::Write::write_all(&mut file, workspace.to_string_lossy().as_bytes())
            .map_err(|error| format!("Could not save selected workspace: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Could not sync selected workspace: {error}"))?;
        #[cfg(windows)]
        if selection_file.exists() {
            fs::remove_file(&selection_file)
                .map_err(|error| format!("Could not replace selected workspace: {error}"))?;
        }
        fs::rename(&temporary, &selection_file)
            .map_err(|error| format!("Could not save selected workspace: {error}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn get_default_workspace_dir() -> Result<String, String> {
    let active = ACTIVE_ROS_WORKSPACE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Active ROS workspace state is unavailable".to_string())?
        .clone()
        .or(load_selected_workspace()?);
    if let Some(workspace) = active {
        let current = workspace
            .canonicalize()
            .map_err(|error| format!("Selected ROS workspace is unavailable: {error}"))?;
        if current != workspace || !current.is_dir() {
            return Err("Selected ROS workspace was moved or replaced".to_string());
        }
        *ACTIVE_ROS_WORKSPACE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| "Active ROS workspace state is unavailable".to_string())? =
            Some(current.clone());
        return Ok(current.to_string_lossy().into_owned());
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| "Could not determine the current user's home directory".to_string())?;
    let fallback = std::path::PathBuf::from(home)
        .join(".robotics-studio")
        .join("ros2-workspace");
    if fallback.exists() {
        let canonical = fallback
            .canonicalize()
            .map_err(|error| format!("Default ROS workspace is unavailable: {error}"))?;
        if canonical.is_dir() {
            return Ok(canonical.to_string_lossy().into_owned());
        }
    }
    fs::create_dir_all(&fallback)
        .map_err(|error| format!("Could not create default ROS workspace: {error}"))?;
    Ok(fallback
        .canonicalize()
        .map_err(|error| format!("Default ROS workspace is unavailable: {error}"))?
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

fn stop_workspace_container(workspace: &Path) -> Result<(), String> {
    let label_filter = format!(
        "label=devcontainer.local_folder={}",
        workspace.to_string_lossy()
    );
    let listed = docker_command()
        .args(["ps", "-q", "--filter", &label_filter])
        .output()
        .map_err(|error| format!("Could not inspect the current ROS container: {error}"))?;
    if !listed.status.success() {
        return Err(format!(
            "Could not inspect the current ROS container: {}",
            String::from_utf8_lossy(&listed.stderr).trim()
        ));
    }
    let listed_ids = String::from_utf8_lossy(&listed.stdout);
    let ids = listed_ids
        .lines()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Ok(());
    }
    let stopped = docker_command()
        .arg("stop")
        .arg("--time")
        .arg("5")
        .args(&ids)
        .output()
        .map_err(|error| format!("Could not stop the previous ROS environment: {error}"))?;
    if !stopped.status.success() {
        return Err(format!(
            "Could not stop the previous ROS environment: {}",
            String::from_utf8_lossy(&stopped.stderr).trim()
        ));
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct RosPackageInfo {
    name: String,
    path: String,
    source: String,
    package_type: String,
    version: Option<String>,
    description: Option<String>,
    dependencies: Vec<String>,
    dependency_sources: HashMap<String, String>,
    executables: Vec<String>,
    launch_files: Vec<String>,
    config_files: Vec<String>,
    metadata_error: Option<String>,
}

fn resolve_workspace_package_path(
    workspace: &Path,
    package_name: &str,
    relative_path: &str,
) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative_path);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "ROS package {package_name} returned a path outside the selected workspace"
        ));
    }

    let canonical_workspace = workspace
        .canonicalize()
        .map_err(|error| format!("Could not validate selected ROS workspace: {error}"))?;
    let canonical_package_path = canonical_workspace
        .join(relative_path)
        .canonicalize()
        .map_err(|error| {
            format!("Could not validate path for ROS package {package_name}: {error}")
        })?;
    if !canonical_package_path.starts_with(&canonical_workspace) || !canonical_package_path.is_dir()
    {
        return Err(format!(
            "ROS package {package_name} resolved outside the selected workspace"
        ));
    }
    Ok(canonical_package_path)
}

#[tauri::command]
fn list_ros_packages() -> Result<Vec<RosPackageInfo>, String> {
    let container_name = find_dev_container()?;
    let workspace = PathBuf::from(get_default_workspace_dir()?);
    let output = docker_command()
        .args([
            "exec",
            &container_name,
            "bash",
            "-lc",
            "cd /opt/robotics-studio && python3 -c \"import json; from server_core import discover_ros_packages; print(json.dumps(discover_ros_packages('/workspace')))\"",
        ])
        .output()
        .map_err(|error| format!("Could not inspect ROS packages: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Could not inspect ROS packages: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut parsed: Vec<RosPackageInfo> = serde_json::from_str(stdout.trim())
        .map_err(|error| format!("Could not parse ROS package metadata: {error}"))?;
    for package in &mut parsed {
        match package.source.as_str() {
            "workspace" => {
                package.path =
                    resolve_workspace_package_path(&workspace, &package.name, &package.path)?
                        .to_string_lossy()
                        .into_owned();
            }
            "underlay" => {
                if !package.path.is_empty() {
                    return Err(format!(
                        "ROS underlay package {} unexpectedly returned a workspace path",
                        package.name
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "ROS package {} returned an unknown package source",
                    package.name
                ));
            }
        }
    }
    Ok(parsed)
}

#[tauri::command]
fn set_ros_workspace(
    access: tauri::State<'_, FsAccess>,
    workspace_root: String,
) -> Result<String, String> {
    let workspace = access.resolve_existing(Path::new(&workspace_root))?;
    if !workspace.is_dir() {
        return Err("Selected ROS workspace is not a directory".to_string());
    }
    let previous = workspace_path_for_shutdown()?;
    if previous != workspace {
        stop_workspace_container(&previous)?;
    }
    persist_selected_workspace(&workspace)?;
    *ACTIVE_ROS_WORKSPACE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Active ROS workspace state is unavailable".to_string())? =
        Some(workspace.clone());
    Ok(workspace.to_string_lossy().into_owned())
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
static BUILD_CONTAINER: OnceLock<Mutex<Option<(String, String)>>> = OnceLock::new();
static BUILD_RUN_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ManagedProcessStatus {
    Created,
    Starting,
    Running,
    Stopping,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildFinishedEvent {
    success: bool,
    cancelled: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ManagedProcess {
    pub id: String,
    pub process_type: String,
    pub command: Vec<String>,
    pub workspace: Option<String>,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub status: ManagedProcessStatus,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    pub parent_task_id: Option<String>,
}

#[derive(Default)]
struct ProcessRegistry {
    processes: Mutex<HashMap<String, ManagedProcess>>,
}

impl ProcessRegistry {
    #[allow(dead_code)]
    fn register(
        &self,
        id: String,
        process_type: String,
        command: Vec<String>,
        workspace: Option<String>,
        parent_task_id: Option<String>,
    ) -> ManagedProcess {
        let process = ManagedProcess {
            id: id.clone(),
            process_type,
            command,
            workspace,
            created_at: current_unix_seconds(),
            started_at: None,
            completed_at: None,
            status: ManagedProcessStatus::Created,
            exit_code: None,
            error: None,
            parent_task_id,
        };
        self.processes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(id.clone(), process.clone());
        process
    }

    fn mark_starting(&self, id: &str) -> bool {
        let mut processes = self
            .processes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(process) = processes.get_mut(id) else {
            return false;
        };
        if process.status != ManagedProcessStatus::Created {
            return false;
        }
        process.status = ManagedProcessStatus::Starting;
        true
    }

    fn mark_running(&self, id: &str) -> bool {
        let mut processes = self
            .processes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(process) = processes.get_mut(id) else {
            return false;
        };
        match process.status {
            ManagedProcessStatus::Starting => {
                process.status = ManagedProcessStatus::Running;
            }
            ManagedProcessStatus::Stopping => {}
            _ => return false,
        }
        process.started_at = Some(current_unix_seconds());
        true
    }

    #[allow(dead_code)]
    fn get(&self, id: &str) -> Option<ManagedProcess> {
        self.processes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(id)
            .cloned()
    }

    fn list(&self) -> Vec<ManagedProcess> {
        let processes = self
            .processes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut entries: Vec<_> = processes.values().cloned().collect();
        entries.sort_by_key(|entry| entry.created_at);
        entries
    }

    fn mark_completed(
        &self,
        id: &str,
        exit_code: Option<i32>,
        status: ManagedProcessStatus,
        error: Option<String>,
    ) {
        let mut processes = self
            .processes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(process) = processes.get_mut(id) {
            process.status = status;
            process.exit_code = exit_code;
            process.error = error;
            process.completed_at = Some(current_unix_seconds());
        }
    }

    fn request_stop(&self, id: &str) -> bool {
        let mut processes = self
            .processes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(process) = processes.get_mut(id) else {
            return false;
        };
        if !matches!(
            process.status,
            ManagedProcessStatus::Starting | ManagedProcessStatus::Running
        ) {
            return false;
        }
        process.status = ManagedProcessStatus::Stopping;
        process.error = Some("Cancellation requested; awaiting process exit".to_string());
        true
    }

    fn restore_running_after_stop_error(&self, id: &str) {
        let mut processes = self
            .processes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(process) = processes.get_mut(id) {
            if process.status == ManagedProcessStatus::Stopping {
                process.status = ManagedProcessStatus::Running;
                process.error = None;
            }
        }
    }

    #[allow(dead_code)]
    fn remove_finished(&self, id: &str) -> Option<ManagedProcess> {
        let mut processes = self
            .processes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        processes.remove(id)
    }
}

static MANAGED_PROCESSES: OnceLock<ProcessRegistry> = OnceLock::new();

fn managed_process_registry() -> &'static ProcessRegistry {
    MANAGED_PROCESSES.get_or_init(ProcessRegistry::default)
}

fn current_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

const COLCON_BUILD_SCRIPT: &str = r#"
source /opt/ros/jazzy/setup.bash || exit $?
cd /workspace || exit $?
run_id="$1"
case "$run_id" in *[!0-9-]*|"") exit 2 ;; esac
pid_file="/tmp/robotics-studio-build-$run_id.pid"
cancel_file="/tmp/robotics-studio-build-$run_id.cancel"
if [ -e "$cancel_file" ]; then
  rm -f "$cancel_file"
  exit 130
fi
setsid bash -c 'exec colcon build' &
build_pid=$!
printf '%s\n' "$build_pid" > "$pid_file"
while kill -0 "$build_pid" 2>/dev/null; do
  if [ -e "$cancel_file" ]; then
    kill -TERM -- "-$build_pid" 2>/dev/null || true
    break
  fi
  sleep 0.1
done
wait "$build_pid"
status=$?
rm -f "$pid_file" "$cancel_file"
exit "$status"
"#;

const CANCEL_COLCON_BUILD_SCRIPT: &str = r#"
run_id="$1"
case "$run_id" in *[!0-9-]*|"") exit 2 ;; esac
pid_file="/tmp/robotics-studio-build-$run_id.pid"
cancel_file="/tmp/robotics-studio-build-$run_id.cancel"
touch "$cancel_file" || exit $?
if [ -r "$pid_file" ]; then
  build_pid=$(cat "$pid_file" 2>/dev/null || true)
  case "$build_pid" in
    "") ;;
    *[!0-9]*) exit 2 ;;
    *) kill -TERM -- "-$build_pid" 2>/dev/null || true ;;
  esac
fi
"#;
static INITIALIZATION_RUNNING: AtomicBool = AtomicBool::new(false);

struct BuildRunGuard;
struct InitializationGuard;

impl Drop for BuildRunGuard {
    fn drop(&mut self) {
        if let Some(state) = BUILD_CONTAINER.get() {
            *state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        }
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
    let build_id = format!(
        "{}-{}",
        std::process::id(),
        BUILD_RUN_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let workspace = Some(get_default_workspace_dir()?);
    managed_process_registry().register(
        build_id.clone(),
        "colcon-build".to_string(),
        vec![
            "docker".to_string(),
            "exec".to_string(),
            container_name.clone(),
            "bash".to_string(),
            "-lc".to_string(),
            "colcon build".to_string(),
        ],
        workspace,
        None,
    );
    managed_process_registry().mark_starting(&build_id);
    *BUILD_CONTAINER
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Build process state is unavailable".to_string())? =
        Some((container_name.clone(), build_id.clone()));

    tauri::async_runtime::spawn(async move {
        let _build_guard = build_guard;
        let child = docker_command()
            .args([
                "exec",
                &container_name,
                "bash",
                "-c",
                COLCON_BUILD_SCRIPT,
                "robotics-studio-build",
                &build_id,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(error) => {
                managed_process_registry().mark_completed(
                    &build_id,
                    None,
                    ManagedProcessStatus::Failed,
                    Some(error.to_string()),
                );
                let _ = app.emit(
                    "build-error",
                    format!("{}: {error}", docker_lost_connection_message()),
                );
                let _ = app.emit(
                    "build-finished",
                    BuildFinishedEvent {
                        success: false,
                        cancelled: false,
                    },
                );
                return;
            }
        };
        managed_process_registry().mark_running(&build_id);

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
                let exit_code = status.code();
                let cancellation_confirmed =
                    managed_process_registry()
                        .get(&build_id)
                        .is_some_and(|process| {
                            process.status == ManagedProcessStatus::Stopping
                                && matches!(exit_code, Some(130 | 137 | 143))
                        });
                let final_status = if cancellation_confirmed {
                    ManagedProcessStatus::Cancelled
                } else if status.success() {
                    ManagedProcessStatus::Completed
                } else {
                    ManagedProcessStatus::Failed
                };
                managed_process_registry().mark_completed(
                    &build_id,
                    Some(exit_code.unwrap_or(-1)),
                    final_status,
                    if status.success() {
                        None
                    } else {
                        Some(if cancellation_confirmed {
                            "Build cancellation was confirmed by the process exit".to_string()
                        } else {
                            "colcon build exited with a non-zero status".to_string()
                        })
                    },
                );
                let _ = app.emit(
                    "build-finished",
                    BuildFinishedEvent {
                        success: status.success(),
                        cancelled: cancellation_confirmed,
                    },
                );
            }
            Err(e) => {
                managed_process_registry().mark_completed(
                    &build_id,
                    None,
                    ManagedProcessStatus::Failed,
                    Some(e.to_string()),
                );
                let _ = app.emit("build-error", e.to_string());
                let _ = app.emit(
                    "build-finished",
                    BuildFinishedEvent {
                        success: false,
                        cancelled: false,
                    },
                );
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
fn cancel_colcon_build() -> Result<(), String> {
    if !BUILD_RUNNING.load(Ordering::SeqCst) {
        return Err("No build is currently running".to_string());
    }
    let (container, build_id) = BUILD_CONTAINER
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Build process state is unavailable".to_string())?
        .clone()
        .ok_or_else(|| "Build process state is unavailable".to_string())?;
    if !managed_process_registry().request_stop(&build_id) {
        return Err("The build process is no longer running".to_string());
    }
    let output = docker_command()
        .args([
            "exec",
            &container,
            "bash",
            "-c",
            CANCEL_COLCON_BUILD_SCRIPT,
            "robotics-studio-cancel",
            &build_id,
        ])
        .output()
        .map_err(|error| format!("Could not request build cancellation: {error}"))?;
    if !output.status.success() {
        managed_process_registry().restore_running_after_stop_error(&build_id);
        return Err(format!(
            "Could not request build cancellation: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

#[tauri::command]
fn list_managed_processes() -> Result<Vec<ManagedProcess>, String> {
    Ok(managed_process_registry().list())
}

fn parse_gazebo_world_control_service(output: &str) -> Result<Option<String>, String> {
    let mut services = Vec::new();
    for service in output.lines().map(str::trim) {
        let Some(world) = service
            .strip_prefix("/world/")
            .and_then(|world| world.strip_suffix("/control"))
        else {
            continue;
        };
        if world.is_empty()
            || !world
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "_-".contains(character))
        {
            return Err(format!("Invalid Gazebo world control service: {service}"));
        }
        services.push(service);
    }
    match services.as_slice() {
        [] => Ok(None),
        [service] => Ok(Some((*service).to_string())),
        _ => Err("Multiple Gazebo worlds are active; reset is ambiguous".to_string()),
    }
}

fn gazebo_world_control_service(container_name: &str) -> Result<Option<String>, String> {
    let output = docker_command()
        .args([
            "exec",
            container_name,
            "bash",
            "-lc",
            "source /opt/ros/jazzy/setup.bash && gz service -l",
        ])
        .output()
        .map_err(|error| format!("Could not inspect Gazebo readiness: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Could not inspect Gazebo readiness: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_gazebo_world_control_service(&String::from_utf8_lossy(&output.stdout))
}

fn validate_sdf_with_gazebo(container_name: &str, world_path: &str) -> Result<(), String> {
    let output = docker_command()
        .args([
            "exec",
            container_name,
            "bash",
            "-lc",
            "source /opt/ros/jazzy/setup.bash && gz sdf --check \"$1\"",
            "robotics-studio-sdf-check",
            world_path,
        ])
        .output()
        .map_err(|error| format!("Could not run SDFormat validation: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if detail.is_empty() {
            "Gazebo rejected the SDF world".to_string()
        } else {
            format!("Gazebo rejected the SDF world: {detail}")
        });
    }
    Ok(())
}

fn stop_gazebo_processes(container_name: &str) -> Result<(), String> {
    let output = docker_command()
        .args([
            "exec",
            container_name,
            "bash",
            "-c",
            "pkill -TERM -f '[g]z sim -s' || true; \
             pkill -TERM -f '[p]arameter_bridge' || true",
        ])
        .output()
        .map_err(|error| format!("Could not stop Gazebo processes: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Could not stop Gazebo processes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
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

    validate_sdf_with_gazebo(&container_name, &container_world_path)?;

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

    let mut world_control_service = None;
    for _ in 0..20 {
        if let Some(service) = gazebo_world_control_service(&container_name)? {
            world_control_service = Some(service);
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    if world_control_service.is_none() {
        let cleanup = stop_gazebo_processes(&container_name);
        return Err(match cleanup {
            Ok(()) => "Gazebo did not become ready within 20 seconds".to_string(),
            Err(error) => {
                format!("Gazebo did not become ready within 20 seconds; cleanup failed: {error}")
            }
        });
    }

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
        let cleanup = stop_gazebo_processes(&container_name);
        if ensure_docker_available().is_err() {
            return Err(docker_lost_connection_message());
        }
        return Err(match cleanup {
            Ok(()) => String::from_utf8_lossy(&bridge_result.stderr).to_string(),
            Err(error) => format!(
                "{}; simulator cleanup failed: {error}",
                String::from_utf8_lossy(&bridge_result.stderr)
            ),
        });
    }

    Ok("Gazebo and bridge started".to_string())
}

#[tauri::command]
fn stop_gazebo_sim() -> Result<String, String> {
    let container_name = find_dev_container()?;
    stop_gazebo_processes(&container_name)?;
    Ok("Gazebo and ROS-Gazebo bridge stopped".to_string())
}

#[tauri::command]
fn reset_gazebo_sim() -> Result<String, String> {
    let container_name = find_dev_container()?;
    let service = gazebo_world_control_service(&container_name)?.ok_or_else(|| {
        "Gazebo world is not ready; start the simulation before resetting".to_string()
    })?;

    let result = docker_command()
        .args([
            "exec",
            &container_name,
            "bash",
            "-c",
            "source /opt/ros/jazzy/setup.bash && \
             gz service -s \"$1\" --reqtype gz.msgs.WorldControl \
             --reptype gz.msgs.Boolean --timeout 2000 --req 'reset: {all: true}'",
            "robotics-studio",
            &service,
        ])
        .output()
        .map_err(|_| docker_lost_connection_message())?;

    if !result.status.success() {
        return Err(format!(
            "Could not reset Gazebo world: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&result.stdout).to_string())
}

use tauri::Manager;

fn copy_dir_recursive(src: &Path, dst: &Path, access: &FsAccess) -> Result<(), String> {
    copy_dir_recursive_impl(src, dst, access, 0)
}

fn copy_dir_recursive_impl(
    src: &Path,
    dst: &Path,
    access: &FsAccess,
    depth: u32,
) -> Result<(), String> {
    if depth > 50 {
        return Err(format!(
            "copy_dir_recursive exceeded max depth (50) — possible cyclic path: {:?} -> {:?}",
            src, dst
        ));
    }
    access.ensure_directory(dst)?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_symlink() {
            continue; // skip symlinks entirely — avoids cycles, and we don't need them copied
        }

        if file_type.is_dir() {
            copy_dir_recursive_impl(&src_path, &dst_path, access, depth + 1)?;
        } else if file_type.is_file() {
            access.copy_file_if_missing(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

const GRPC_AUTH_CONTAINER_PATH: &str = "/run/robotics-studio/grpc-auth.json";
const GRPC_AUTH_LIFETIME_SECONDS: u64 = 24 * 60 * 60;
const ROS_SERVER_CONTAINER_DIRECTORY: &str = "/opt/robotics-studio";
const GRPC_CA_CERTIFICATE_PATH: &str = "/run/robotics-studio/grpc-ca.crt";
const GRPC_CA_PRIVATE_KEY_PATH: &str = "/run/robotics-studio/grpc-ca.key";
const GRPC_SERVER_CERTIFICATE_PATH: &str = "/run/robotics-studio/grpc-server.crt";
const GRPC_SERVER_PRIVATE_KEY_PATH: &str = "/run/robotics-studio/grpc-server.key";
const GRPC_SERVER_CSR_PATH: &str = "/run/robotics-studio/grpc-server.csr";
const GRPC_SERVER_EXTENSIONS_PATH: &str = "/run/robotics-studio/grpc-server.ext";

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
    access: &FsAccess,
) -> Result<(), String> {
    fs::create_dir_all(backup_dir)
        .map_err(|error| format!("Could not prepare server migration backup: {error}"))?;
    for file_name in ["server.py", "server_core.py", "proto/ros_engine.proto"] {
        let bundled_path = resource_dir.join(file_name);
        let target_path = workspace_dir.join(file_name);
        let bundled = fs::read(&bundled_path)
            .map_err(|error| format!("Could not read bundled {file_name}: {error}"))?;
        let bundled_text = std::str::from_utf8(&bundled)
            .map_err(|error| format!("Bundled {file_name} is not valid UTF-8: {error}"))?;
        let target_parent = target_path
            .parent()
            .ok_or_else(|| format!("Invalid workspace path for {file_name}"))?;
        access
            .ensure_directory(target_parent)
            .map_err(|error| format!("Could not safely access {file_name} directory: {error}"))?;
        let installed = access.read_file_if_present(&target_path)?;
        if installed
            .as_ref()
            .is_some_and(|contents| contents == &bundled)
        {
            continue;
        }

        if let Some(installed) = &installed {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| error.to_string())?
                .as_nanos();
            let backup_path = backup_dir.join(format!(
                "{}.pre-auth-{}-{timestamp}",
                file_name.replace('/', "_"),
                std::process::id()
            ));
            fs::write(&backup_path, installed)
                .map_err(|error| format!("Could not preserve existing {file_name}: {error}"))?;
        }

        write_approved_file(access, &target_path, bundled_text)
            .map_err(|error| format!("Could not install secure {file_name}: {error}"))?;
    }
    Ok(())
}

fn install_ros_server_resources(container_id: &str, resource_dir: &Path) -> Result<(), String> {
    let prepared = docker_command()
        .args([
            "exec",
            container_id,
            "mkdir",
            "-p",
            ROS_SERVER_CONTAINER_DIRECTORY,
        ])
        .output()
        .map_err(|error| format!("Could not prepare the trusted ROS service: {error}"))?;
    if !prepared.status.success() {
        return Err(format!(
            "Could not prepare the trusted ROS service: {}",
            String::from_utf8_lossy(&prepared.stderr)
        ));
    }

    for file_name in ["server.py", "server_core.py"] {
        let source = resource_dir.join(file_name);
        let destination = format!("{container_id}:{ROS_SERVER_CONTAINER_DIRECTORY}/{file_name}");
        let copied = docker_command()
            .args(["cp", &source.to_string_lossy(), &destination])
            .output()
            .map_err(|error| format!("Could not install trusted {file_name}: {error}"))?;
        if !copied.status.success() {
            return Err(format!(
                "Could not install trusted {file_name}: {}",
                String::from_utf8_lossy(&copied.stderr)
            ));
        }
    }
    let proto_source = resource_dir.join("proto/ros_engine.proto");
    let proto_destination = format!("{container_id}:{ROS_SERVER_CONTAINER_DIRECTORY}/proto");
    let proto_directory = docker_command()
        .args([
            "exec",
            container_id,
            "mkdir",
            "-p",
            &format!("{ROS_SERVER_CONTAINER_DIRECTORY}/proto"),
        ])
        .output()
        .map_err(|error| format!("Could not prepare the trusted ROS protocol: {error}"))?;
    if !proto_directory.status.success() {
        return Err(format!(
            "Could not prepare the trusted ROS protocol: {}",
            String::from_utf8_lossy(&proto_directory.stderr)
        ));
    }
    let copied = docker_command()
        .args(["cp", &proto_source.to_string_lossy(), &proto_destination])
        .output()
        .map_err(|error| format!("Could not install the trusted ROS protocol: {error}"))?;
    if !copied.status.success() {
        return Err(format!(
            "Could not install the trusted ROS protocol: {}",
            String::from_utf8_lossy(&copied.stderr)
        ));
    }

    let certificate_command = format!(
        "set -eu; \
         mkdir -p /run/robotics-studio; chmod 700 /run/robotics-studio; \
         if [ ! -s '{GRPC_CA_CERTIFICATE_PATH}' ] || \
            [ ! -s '{GRPC_CA_PRIVATE_KEY_PATH}' ] || \
            [ ! -s '{GRPC_SERVER_CERTIFICATE_PATH}' ] || \
            [ ! -s '{GRPC_SERVER_PRIVATE_KEY_PATH}' ]; then \
           openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
             -keyout '{GRPC_CA_PRIVATE_KEY_PATH}' \
             -out '{GRPC_CA_CERTIFICATE_PATH}' \
             -subj '/CN=Robotics Studio local gRPC CA' \
             -addext 'basicConstraints=critical,CA:TRUE' \
             -addext 'keyUsage=critical,keyCertSign,cRLSign'; \
           openssl req -new -newkey rsa:2048 -nodes \
             -keyout '{GRPC_SERVER_PRIVATE_KEY_PATH}' \
             -out '{GRPC_SERVER_CSR_PATH}' \
             -subj '/CN=127.0.0.1'; \
           printf '%s\\n' \
             'basicConstraints=critical,CA:FALSE' \
             'keyUsage=critical,digitalSignature,keyEncipherment' \
             'extendedKeyUsage=serverAuth' \
             'subjectAltName=IP:127.0.0.1,DNS:localhost' \
             > '{GRPC_SERVER_EXTENSIONS_PATH}'; \
           openssl x509 -req -in '{GRPC_SERVER_CSR_PATH}' \
             -CA '{GRPC_CA_CERTIFICATE_PATH}' \
             -CAkey '{GRPC_CA_PRIVATE_KEY_PATH}' -CAcreateserial \
             -out '{GRPC_SERVER_CERTIFICATE_PATH}' -days 3650 -sha256 \
             -extfile '{GRPC_SERVER_EXTENSIONS_PATH}'; \
         fi; \
         chmod 600 '{GRPC_CA_PRIVATE_KEY_PATH}' '{GRPC_SERVER_PRIVATE_KEY_PATH}'; \
         chmod 644 '{GRPC_CA_CERTIFICATE_PATH}' '{GRPC_SERVER_CERTIFICATE_PATH}'; \
         cat '{GRPC_CA_CERTIFICATE_PATH}'"
    );
    let certificate = docker_command()
        .args(["exec", container_id, "bash", "-c", &certificate_command])
        .output()
        .map_err(|error| format!("Could not provision ROS TLS credentials: {error}"))?;
    if !certificate.status.success() {
        return Err(format!(
            "Could not provision ROS TLS credentials: {}",
            String::from_utf8_lossy(&certificate.stderr)
        ));
    }
    set_ros_server_tls_certificate(certificate.stdout)?;
    Ok(())
}

fn start_ros_engine_server(
    container_id: &str,
    auth_file: &Path,
    resource_dir: &Path,
) -> Result<(), String> {
    install_ros_server_resources(container_id, resource_dir)?;
    let generate = docker_command()
        .args([
            "exec",
            container_id,
            "bash",
            "-c",
            "source /opt/ros/jazzy/setup.bash && \
             mkdir -p /opt/robotics-studio/generated && \
             /opt/robotics-studio/venv/bin/python -m grpc_tools.protoc \
               -I /opt/robotics-studio/proto \
               --python_out=/opt/robotics-studio/generated \
               --grpc_python_out=/opt/robotics-studio/generated \
               /opt/robotics-studio/proto/ros_engine.proto",
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
            "pkill -f '[v]env/bin/python /opt/robotics-studio/server.py'",
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
            "source /opt/ros/jazzy/setup.bash && \
             cd /opt/robotics-studio && \
             ROBOTICS_STUDIO_AUTH_FILE=/run/robotics-studio/grpc-auth.json \
             ROBOTICS_STUDIO_GENERATED_PATH=/opt/robotics-studio/generated \
             exec /opt/robotics-studio/venv/bin/python \
               /opt/robotics-studio/server.py > /tmp/server.log 2>&1",
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
        if let Ok(Ok(mut client)) =
            tokio::time::timeout(std::time::Duration::from_secs(2), connect_ros_engine()).await
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
    copy_dir_recursive(&resource_path, workspace_path, &access)?;
    let backup_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("server-migration-backups");
    install_authenticated_server_files(&resource_path, workspace_path, &backup_dir, &access)?;

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
        let resource_path = resource_path.clone();
        tauri::async_runtime::spawn(async move {
            let _initialization_guard = initialization_guard;
            let _ = app.emit(
                "init-progress",
                LogEvent {
                    kind: "append".to_string(),
                    text: "Existing environment found. Checking ROS engine...".to_string(),
                },
            );
            let result = match start_ros_engine_server(&container_name, &auth_file, &resource_path)
            {
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

        match start_ros_engine_server(&container_id, &auth_file, &resource_path) {
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
        .args(["MemTotal", "/proc/meminfo"])
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
    topic_name: String,
    source_timestamp_ms: i64,
    received_at_ms: i64,
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
    if let Err(error) = copy_dir_recursive(&template_path, &dest, &access) {
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
    fn resolves_the_active_gazebo_world_control_service() {
        assert_eq!(
            parse_gazebo_world_control_service(
                "/world/diff_drive/control\n/world/diff_drive/scene/info\n"
            )
            .unwrap(),
            Some("/world/diff_drive/control".to_string())
        );
        assert_eq!(
            parse_gazebo_world_control_service("/world/diff_drive/scene/info\n").unwrap(),
            None
        );
        assert!(
            parse_gazebo_world_control_service("/world/one/control\n/world/two/control\n").is_err()
        );
        assert!(parse_gazebo_world_control_service("/world/../control\n").is_err());
    }

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
    fn ros_package_paths_are_confined_to_selected_workspace() {
        let workspace = TempDirectory::new();
        let outside = TempDirectory::new();
        let package = workspace.0.join("src/robot");
        fs::create_dir_all(&package).unwrap();
        fs::create_dir_all(outside.0.join("robot")).unwrap();

        assert_eq!(
            resolve_workspace_package_path(&workspace.0, "robot", "src/robot").unwrap(),
            package.canonicalize().unwrap()
        );
        assert!(
            resolve_workspace_package_path(&workspace.0, "robot", "../robot")
                .unwrap_err()
                .contains("outside the selected workspace")
        );
        assert!(resolve_workspace_package_path(
            &workspace.0,
            "robot",
            outside.0.to_string_lossy().as_ref()
        )
        .is_err());

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside.0, workspace.0.join("linked")).unwrap();
            assert!(resolve_workspace_package_path(&workspace.0, "robot", "linked/robot").is_err());
        }
    }

    #[test]
    fn workspace_service_marker_does_not_authenticate_modified_server_files() {
        let directory = TempDirectory::new();
        let resources = directory.0.join("resources");
        let workspace = directory.0.join("workspace");
        let backup = directory.0.join("backups");
        fs::create_dir_all(resources.join("proto")).unwrap();
        fs::create_dir_all(workspace.join("proto")).unwrap();
        for (name, trusted) in [
            ("server.py", b"trusted service".as_slice()),
            ("server_core.py", b"trusted core".as_slice()),
            ("proto/ros_engine.proto", b"trusted protocol".as_slice()),
        ] {
            fs::write(resources.join(name), trusted).unwrap();
            fs::write(
                workspace.join(name),
                b"AUTH_PROTOCOL_VERSION = 1\nmalicious replacement",
            )
            .unwrap();
        }
        let access = FsAccess::default();
        access.approve_directory(&workspace).unwrap();

        install_authenticated_server_files(&resources, &workspace, &backup, &access).unwrap();

        for (name, trusted) in [
            ("server.py", b"trusted service".as_slice()),
            ("server_core.py", b"trusted core".as_slice()),
            ("proto/ros_engine.proto", b"trusted protocol".as_slice()),
        ] {
            assert_eq!(fs::read(workspace.join(name)).unwrap(), trusted);
        }
        let preserved = fs::read_dir(&backup)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(preserved.len(), 3);
        assert!(preserved.iter().all(|entry| {
            fs::read(entry.path()).is_ok_and(|contents| {
                contents == b"AUTH_PROTOCOL_VERSION = 1\nmalicious replacement"
            })
        }));
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
        fs::write(outside.0.join("secret.txt"), "outside").unwrap();
        let access = FsAccess::default();
        access.approve_directory(&directory.0).unwrap();
        symlink(&outside.0, directory.0.join("escape")).unwrap();

        assert!(access
            .resolve_existing(&directory.0.join("escape"))
            .is_err());
        assert!(read_approved_file(&access, &directory.0.join("escape/secret.txt")).is_err());
        assert!(
            write_approved_file(&access, &directory.0.join("escape/secret.txt"), "changed")
                .is_err()
        );
        assert_eq!(
            fs::read_to_string(outside.0.join("secret.txt")).unwrap(),
            "outside"
        );
    }

    #[cfg(unix)]
    #[test]
    fn recursive_copy_rejects_symlinked_destination_directories() {
        use std::os::unix::fs::symlink;

        let directory = TempDirectory::new();
        let source = directory.0.join("source");
        let workspace = directory.0.join("workspace");
        let outside = directory.0.join("outside");
        fs::create_dir_all(source.join("proto")).unwrap();
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(source.join("proto/ros_engine.proto"), "bundled").unwrap();
        symlink(&outside, workspace.join("proto")).unwrap();
        let access = FsAccess::default();
        access.approve_directory(&workspace).unwrap();

        assert!(copy_dir_recursive(&source, &workspace, &access).is_err());
        assert!(!outside.join("ros_engine.proto").exists());
    }

    #[cfg(unix)]
    #[test]
    fn authenticated_service_install_rejects_symlinked_proto_directory() {
        use std::os::unix::fs::symlink;

        let directory = TempDirectory::new();
        let resources = directory.0.join("resources");
        let workspace = directory.0.join("workspace");
        let outside = directory.0.join("outside");
        fs::create_dir_all(resources.join("proto")).unwrap();
        fs::create_dir(&workspace).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(resources.join("server.py"), "service").unwrap();
        fs::write(resources.join("server_core.py"), "core").unwrap();
        fs::write(resources.join("proto/ros_engine.proto"), "protocol").unwrap();
        symlink(&outside, workspace.join("proto")).unwrap();
        let access = FsAccess::default();
        access.approve_directory(&workspace).unwrap();

        assert!(install_authenticated_server_files(
            &resources,
            &workspace,
            &directory.0.join("backups"),
            &access
        )
        .is_err());
        assert!(!outside.join("ros_engine.proto").exists());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_replaced_approved_root_after_selection() {
        let directory = TempDirectory::new();
        let moved = directory.0.with_extension("moved");
        let access = FsAccess::default();
        access.approve_directory(&directory.0).unwrap();
        fs::rename(&directory.0, &moved).unwrap();
        fs::create_dir(&directory.0).unwrap();
        fs::write(directory.0.join("sentinel.txt"), "replacement").unwrap();

        assert!(read_approved_file(&access, &directory.0.join("sentinel.txt")).is_err());
        assert!(
            write_approved_file(&access, &directory.0.join("sentinel.txt"), "must not write")
                .is_err()
        );
        assert_eq!(
            fs::read_to_string(directory.0.join("sentinel.txt")).unwrap(),
            "replacement"
        );
        fs::remove_dir_all(moved).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn concurrent_parent_symlink_swaps_never_escape_the_approved_root() {
        use std::os::unix::fs::symlink;
        use std::sync::atomic::{AtomicBool, Ordering};

        let directory = TempDirectory::new();
        let outside = TempDirectory::new();
        let nested = directory.0.join("nested");
        let held = directory.0.join("held");
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join("value.txt"), "inside").unwrap();
        fs::write(outside.0.join("value.txt"), "outside").unwrap();
        let access = Arc::new(FsAccess::default());
        access.approve_directory(&directory.0).unwrap();
        let running = Arc::new(AtomicBool::new(true));
        let thread_running = Arc::clone(&running);
        let thread_root = directory.0.clone();
        let thread_outside = outside.0.clone();
        let attacker = std::thread::spawn(move || {
            let nested = thread_root.join("nested");
            let held = thread_root.join("held");
            while thread_running.load(Ordering::Relaxed) {
                if fs::rename(&nested, &held).is_ok() {
                    if symlink(&thread_outside, &nested).is_ok() {
                        let _ = fs::remove_file(&nested);
                    }
                    let _ = fs::rename(&held, &nested);
                }
            }
            if fs::symlink_metadata(&nested).is_ok_and(|metadata| metadata.file_type().is_symlink())
            {
                let _ = fs::remove_file(&nested);
            }
            if held.exists() && !nested.exists() {
                let _ = fs::rename(held, nested);
            }
        });

        for _ in 0..500 {
            if let Ok(contents) = read_approved_file(&access, &nested.join("value.txt")) {
                assert_eq!(contents, "inside");
            }
            let _ = write_approved_file(&access, &nested.join("value.txt"), "inside");
        }

        running.store(false, Ordering::Relaxed);
        attacker.join().unwrap();
        assert_eq!(
            fs::read_to_string(outside.0.join("value.txt")).unwrap(),
            "outside"
        );
        assert!(!held.exists());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_file_replacement_for_atomic_writes() {
        use std::os::unix::fs::symlink;

        let directory = TempDirectory::new();
        let outside = TempDirectory::new();
        fs::write(outside.0.join("secret.txt"), "outside").unwrap();
        let access = FsAccess::default();
        access.approve_directory(&directory.0).unwrap();
        symlink(outside.0.join("secret.txt"), directory.0.join("link.txt")).unwrap();

        assert!(write_approved_file(&access, &directory.0.join("link.txt"), "changed").is_err());
        assert_eq!(
            fs::read_to_string(outside.0.join("secret.txt")).unwrap(),
            "outside"
        );
    }

    #[test]
    fn active_workspace_context_switches_between_approved_roots() {
        let first = TempDirectory::new();
        let second = TempDirectory::new();
        let previous = ACTIVE_ROS_WORKSPACE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap()
            .clone();
        let first = first.0.canonicalize().unwrap();
        let second = second.0.canonicalize().unwrap();

        *ACTIVE_ROS_WORKSPACE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap() = Some(first.clone());
        assert_eq!(
            get_default_workspace_dir().unwrap(),
            first.to_string_lossy()
        );
        *ACTIVE_ROS_WORKSPACE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap() = Some(second.clone());
        assert_eq!(
            get_default_workspace_dir().unwrap(),
            second.to_string_lossy()
        );

        *ACTIVE_ROS_WORKSPACE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap() = previous;
    }

    #[test]
    fn ros_event_recovery_cursors_are_isolated_by_workspace() {
        let first = format!("workspace-a-{}", std::process::id());
        let second = format!("workspace-b-{}", std::process::id());

        update_ros_event_cursor(&first, 12, false).unwrap();
        update_ros_event_cursor(&second, 3, false).unwrap();
        assert_eq!(ros_event_cursor(&first).unwrap(), 12);
        assert_eq!(ros_event_cursor(&second).unwrap(), 3);

        update_ros_event_cursor(&second, 1, true).unwrap();
        assert_eq!(ros_event_cursor(&second).unwrap(), 1);
        assert_eq!(ros_event_cursor(&first).unwrap(), 12);
    }

    #[cfg(unix)]
    #[test]
    fn colcon_cancellation_script_terminates_only_its_process_group() {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;
        use std::sync::mpsc;

        let directory = TempDirectory::new();
        let ready_file = directory.0.join("colcon-ready");
        let fake_colcon = directory.0.join("colcon");
        fs::write(
            &fake_colcon,
            "#!/bin/sh\n: > \"$COLCON_TEST_READY_FILE\"\nexec sleep 30\n",
        )
        .unwrap();
        let fake_setsid = directory.0.join("setsid");
        fs::write(
            &fake_setsid,
            "#!/usr/bin/env python3\nimport os, sys\nos.setsid()\nos.execvp(sys.argv[1], sys.argv[1:])\n",
        )
        .unwrap();
        for executable in [&fake_colcon, &fake_setsid] {
            let mut permissions = fs::metadata(executable).unwrap().permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(executable, permissions).unwrap();
        }
        let run_id = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let pid_file = PathBuf::from(format!("/tmp/robotics-studio-build-{run_id}.pid"));
        let cancel_file = PathBuf::from(format!("/tmp/robotics-studio-build-{run_id}.cancel"));
        let script = COLCON_BUILD_SCRIPT
            .replace("source /opt/ros/jazzy/setup.bash || exit $?", ":")
            .replace(
                "cd /workspace || exit $?",
                &format!("cd '{}' || exit $?", directory.0.display()),
            );
        let path = format!(
            "{}:{}",
            directory.0.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut child = Command::new("bash")
            .args(["-c", &script, "test-colcon-build", &run_id])
            .env("PATH", path)
            .env("COLCON_TEST_READY_FILE", &ready_file)
            .spawn()
            .unwrap();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !ready_file.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            pid_file.exists() && ready_file.exists(),
            "fake colcon process did not start"
        );
        let build_pid = fs::read_to_string(&pid_file).unwrap();
        let build_is_alive = Command::new("bash")
            .args([
                "-c",
                "kill -0 \"$1\"",
                "test-build-status",
                build_pid.trim(),
            ])
            .status()
            .unwrap();
        assert!(
            build_is_alive.success(),
            "fake colcon process did not start"
        );
        let cancellation = Command::new("bash")
            .args([
                "-c",
                CANCEL_COLCON_BUILD_SCRIPT,
                "test-colcon-cancel",
                &run_id,
            ])
            .status()
            .unwrap();
        assert!(cancellation.success());

        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(child.wait());
        });
        let status = receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("build did not exit after cancellation")
            .unwrap();
        assert!(!status.success());
        assert!(!pid_file.exists());
        assert!(!cancel_file.exists());
    }

    #[test]
    fn imports_worlds_to_unique_safe_paths_without_overwriting() {
        let directory = TempDirectory::new();
        let access = FsAccess::default();
        let source = Path::new("../../Dockerfile.sdf");
        let sdf = "<sdf version=\"1.9\"><world name=\"test\"/></sdf>";
        let workspace = access.approve_directory(&directory.0).unwrap();
        let first = import_world_contents(&access, &workspace, source, sdf).unwrap();
        let second = import_world_contents(&access, &workspace, source, sdf).unwrap();

        assert_ne!(first, second);
        assert!(first.starts_with(workspace.join("worlds")));
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
        let access = FsAccess::default();
        let workspace = access.approve_directory(&directory.0).unwrap();
        assert!(
            import_world_contents(&access, &workspace, Path::new("world.txt"), "<sdf></sdf>")
                .is_err()
        );
        assert!(import_world_contents(
            &access,
            &workspace,
            Path::new("world.sdf"),
            "<world></world>"
        )
        .is_err());
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
        let access = FsAccess::default();
        let workspace = access.approve_directory(&directory.0).unwrap();

        assert!(
            import_world_contents(&access, &workspace, Path::new("world.sdf"), "<sdf></sdf>")
                .is_err()
        );
        assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0);
    }
}

#[cfg(test)]
mod process_registry_tests {
    use super::*;

    #[test]
    fn process_registry_confirms_cancellation_only_after_process_exit() {
        let registry = ProcessRegistry::default();
        let process = registry.register(
            "test-proc".to_string(),
            "build".to_string(),
            vec![
                "bash".to_string(),
                "-lc".to_string(),
                "sleep 30".to_string(),
            ],
            Some("/tmp/workspace".to_string()),
            None,
        );
        assert_eq!(process.status, ManagedProcessStatus::Created);
        assert!(process.started_at.is_none());
        assert!(registry.mark_starting("test-proc"));
        assert!(registry.mark_running("test-proc"));
        assert!(registry.request_stop("test-proc"));
        assert_eq!(
            registry.get("test-proc").unwrap().status,
            ManagedProcessStatus::Stopping
        );
        assert!(registry.get("test-proc").unwrap().completed_at.is_none());
        registry.mark_completed(
            "test-proc",
            Some(130),
            ManagedProcessStatus::Cancelled,
            Some("Cancelled by user".to_string()),
        );
        let process = registry.get("test-proc").unwrap();
        assert_eq!(process.exit_code, Some(130));
        assert!(process.completed_at.is_some());
        assert_eq!(process.status, ManagedProcessStatus::Cancelled);
    }

    #[test]
    fn process_registry_keeps_pre_spawn_cancellation_pending_until_confirmation() {
        let registry = ProcessRegistry::default();
        registry.register(
            "starting".to_string(),
            "build".to_string(),
            vec!["docker".to_string()],
            None,
            None,
        );
        assert!(registry.mark_starting("starting"));
        assert!(registry.request_stop("starting"));
        assert!(registry.mark_running("starting"));
        let process = registry.get("starting").unwrap();
        assert_eq!(process.status, ManagedProcessStatus::Stopping);
        assert!(process.started_at.is_some());
        assert!(process.completed_at.is_none());
    }

    #[test]
    fn process_registry_tracks_spawn_failure_without_claiming_running() {
        let registry = ProcessRegistry::default();
        registry.register(
            "spawn-failure".to_string(),
            "build".to_string(),
            vec!["missing-command".to_string()],
            None,
            None,
        );
        assert!(registry.mark_starting("spawn-failure"));
        registry.mark_completed(
            "spawn-failure",
            None,
            ManagedProcessStatus::Failed,
            Some("spawn failed".to_string()),
        );
        let process = registry.get("spawn-failure").unwrap();
        assert_eq!(process.status, ManagedProcessStatus::Failed);
        assert!(process.started_at.is_none());
        assert!(process.completed_at.is_some());
    }

    #[test]
    fn process_registry_rejects_unknown_or_finished_process_ids() {
        let registry = ProcessRegistry::default();
        assert!(registry.get("missing").is_none());
        assert!(!registry.request_stop("missing"));

        let process = registry.register(
            "done".to_string(),
            "sim".to_string(),
            vec!["true".to_string()],
            None,
            None,
        );
        assert_eq!(process.status, ManagedProcessStatus::Created);
        assert!(registry.mark_starting("done"));
        assert!(registry.mark_running("done"));
        registry.mark_completed("done", Some(0), ManagedProcessStatus::Completed, None);
        assert_eq!(
            registry.get("done").unwrap().status,
            ManagedProcessStatus::Completed
        );
        assert!(!registry.request_stop("done"));
    }
}

#[cfg(test)]
mod ros_docker_integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires a Docker ROS service listening on localhost:50051"]
    async fn pinned_tls_client_authenticates_to_docker_ros_service() {
        let certificate_path = std::env::var_os("ROBOTICS_STUDIO_TEST_TLS_CERTIFICATE")
            .expect("ROBOTICS_STUDIO_TEST_TLS_CERTIFICATE must point to the pinned certificate");
        let certificate =
            fs::read(certificate_path).expect("could not read pinned TLS certificate");
        let token = std::env::var("ROBOTICS_STUDIO_TEST_AUTH_TOKEN")
            .expect("ROBOTICS_STUDIO_TEST_AUTH_TOKEN must contain the test token");
        set_ros_server_tls_certificate(certificate).unwrap();
        let mut client = connect_ros_engine().await.unwrap();
        let response = client
            .get_safety_state(authenticated_request(Empty {}, &token).unwrap())
            .await
            .unwrap()
            .into_inner();
        assert_eq!(response.state, 0);
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
