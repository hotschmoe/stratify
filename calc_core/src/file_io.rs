//! # File I/O Module
//!
//! Handles project file operations with safety features:
//! - **Atomic saves**: Write to .tmp, verify, rename to prevent corruption
//! - **File locking**: Prevent concurrent edits on shared drives (native only)
//! - **Version validation**: Ensure schema compatibility
//!
//! ## File Format
//!
//! Projects are saved as `.stf` (Stratify) files containing JSON.
//! Lock files use `.stf.lock` extension with metadata about who holds the lock.
//!
//! ## Platform Notes
//!
//! File locking is only available on native platforms (Windows, macOS, Linux).
//! On WebAssembly, file operations work but locking is not available.
//!
//! ## Example
//!
//! ```rust,no_run
//! use calc_core::file_io::{save_project, load_project, FileLock};
//! use calc_core::project::Project;
//! use std::path::Path;
//!
//! let project = Project::new("Engineer", "25-001", "Client");
//! let path = Path::new("myproject.stf");
//!
//! // Acquire lock before saving (native only)
//! let lock = FileLock::acquire(path, "engineer@company.com").unwrap();
//!
//! // Save with atomic write
//! save_project(&project, path).unwrap();
//!
//! // Lock is released when dropped
//! drop(lock);
//! ```

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::errors::{CalcError, CalcResult};
use crate::project::{Project, SCHEMA_VERSION};

// File locking uses std::fs::File::{try_lock, unlock} - stable since Rust 1.89.
#[cfg(not(target_arch = "wasm32"))]
use std::fs::OpenOptions;

/// Lock file metadata stored in .stf.lock files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    /// User identifier (email or username)
    pub user_id: String,
    /// Machine name where lock was acquired
    pub machine: String,
    /// Process ID that holds the lock
    pub pid: u32,
    /// When the lock was acquired
    pub locked_at: DateTime<Utc>,
}

impl LockInfo {
    /// Create new lock info for the current process
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(user_id: impl Into<String>) -> Self {
        LockInfo {
            user_id: user_id.into(),
            machine: hostname().unwrap_or_else(|| "unknown".to_string()),
            pid: std::process::id(),
            locked_at: Utc::now(),
        }
    }

    /// Create new lock info (WASM stub)
    #[cfg(target_arch = "wasm32")]
    pub fn new(user_id: impl Into<String>) -> Self {
        LockInfo {
            user_id: user_id.into(),
            machine: "wasm".to_string(),
            pid: 0,
            locked_at: Utc::now(),
        }
    }
}

/// Get the hostname of the current machine
#[cfg(not(target_arch = "wasm32"))]
fn hostname() -> Option<String> {
    #[cfg(windows)]
    {
        std::env::var("COMPUTERNAME").ok()
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOSTNAME")
            .ok()
            .or_else(|| std::env::var("HOST").ok())
    }
}

// ============================================================================
// File Locking (Native only)
// ============================================================================

/// File lock guard that releases the lock when dropped.
///
/// Uses both:
/// 1. OS-level file locking (via fs2) for process safety
/// 2. .lock file with metadata for user visibility
///
/// **Note**: File locking is only available on native platforms.
/// On WASM, `acquire()` will return an error.
#[cfg(not(target_arch = "wasm32"))]
pub struct FileLock {
    /// Path to the main project file
    project_path: PathBuf,
    /// Path to the lock file
    lock_path: PathBuf,
    /// The underlying file handle (keeps OS lock)
    _lock_file: File,
    /// Lock metadata
    pub info: LockInfo,
}

#[cfg(not(target_arch = "wasm32"))]
impl FileLock {
    /// Acquire an exclusive lock on a project file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the .stf project file
    /// * `user_id` - Identifier for the user acquiring the lock
    ///
    /// # Returns
    ///
    /// * `Ok(FileLock)` - Lock acquired successfully
    /// * `Err(CalcError::FileLocked)` - Another process holds the lock
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use calc_core::file_io::FileLock;
    /// use std::path::Path;
    ///
    /// let lock = FileLock::acquire(Path::new("project.stf"), "user@email.com")?;
    /// // ... do work ...
    /// drop(lock); // releases lock
    /// # Ok::<(), calc_core::errors::CalcError>(())
    /// ```
    pub fn acquire(path: &Path, user_id: impl Into<String>) -> CalcResult<Self> {
        let lock_path = lock_path_for(path);
        let info = LockInfo::new(user_id);

        // Check if lock file exists and contains valid lock info
        if lock_path.exists() {
            if let Ok(existing) = read_lock_info(&lock_path) {
                // Check if the lock is stale (process no longer running)
                if !is_lock_stale(&existing) {
                    return Err(CalcError::file_locked(
                        path.display().to_string(),
                        format!("{} ({})", existing.user_id, existing.machine),
                        existing.locked_at.to_rfc3339(),
                    ));
                }
                // Lock is stale, we can take it over
            }
        }

        // Create/open the lock file
        let mut lock_file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .map_err(|e| {
                CalcError::file_error("create lock", lock_path.display().to_string(), e.to_string())
            })?;

        // Try to acquire exclusive OS-level lock (non-blocking).
        // std::fs::File::try_lock returns Result<(), TryLockError> on Rust 1.89+.
        lock_file.try_lock().map_err(|_| {
            CalcError::file_locked(
                path.display().to_string(),
                "another process".to_string(),
                "unknown".to_string(),
            )
        })?;

        // Write lock info to the file using the same handle
        let lock_json = serde_json::to_string_pretty(&info).map_err(|e| {
            CalcError::SerializationError {
                reason: e.to_string(),
            }
        })?;

        lock_file.write_all(lock_json.as_bytes()).map_err(|e| {
            CalcError::file_error("write lock", lock_path.display().to_string(), e.to_string())
        })?;

        lock_file.sync_all().map_err(|e| {
            CalcError::file_error("sync lock", lock_path.display().to_string(), e.to_string())
        })?;

        Ok(FileLock {
            project_path: path.to_path_buf(),
            lock_path,
            _lock_file: lock_file,
            info,
        })
    }

    /// Check if a file is locked without acquiring the lock.
    ///
    /// Returns `Some(LockInfo)` if locked, `None` if available.
    pub fn check(path: &Path) -> Option<LockInfo> {
        let lock_path = lock_path_for(path);
        if lock_path.exists() {
            if let Ok(info) = read_lock_info(&lock_path) {
                if !is_lock_stale(&info) {
                    return Some(info);
                }
            }
        }
        None
    }

    /// Get the path to the project file
    pub fn project_path(&self) -> &Path {
        &self.project_path
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for FileLock {
    fn drop(&mut self) {
        // Remove the lock file
        let _ = fs::remove_file(&self.lock_path);
        // OS lock is released when _lock_file is dropped
    }
}

// ============================================================================
// File Locking Stubs (WASM)
// ============================================================================

/// File lock stub for WASM (locking not supported)
#[cfg(target_arch = "wasm32")]
pub struct FileLock {
    /// Lock metadata
    pub info: LockInfo,
}

#[cfg(target_arch = "wasm32")]
impl FileLock {
    /// File locking is not available on WASM.
    ///
    /// This always returns an error indicating that locking is unsupported.
    pub fn acquire(_path: &Path, _user_id: impl Into<String>) -> CalcResult<Self> {
        Err(CalcError::file_error(
            "acquire lock",
            "N/A".to_string(),
            "File locking is not available in WebAssembly".to_string(),
        ))
    }

    /// Always returns None on WASM (no file locking support)
    pub fn check(_path: &Path) -> Option<LockInfo> {
        None
    }
}

// ============================================================================
// Helper Functions (Native only)
// ============================================================================

/// Get the lock file path for a project file
#[cfg(not(target_arch = "wasm32"))]
fn lock_path_for(project_path: &Path) -> PathBuf {
    let mut lock_path = project_path.to_path_buf();
    let extension = lock_path
        .extension()
        .map(|e| format!("{}.lock", e.to_string_lossy()))
        .unwrap_or_else(|| "lock".to_string());
    lock_path.set_extension(extension);
    lock_path
}

/// Read lock info from a lock file
#[cfg(not(target_arch = "wasm32"))]
fn read_lock_info(lock_path: &Path) -> CalcResult<LockInfo> {
    let mut file = File::open(lock_path).map_err(|e| {
        CalcError::file_error("read lock", lock_path.display().to_string(), e.to_string())
    })?;

    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|e| {
        CalcError::file_error("read lock", lock_path.display().to_string(), e.to_string())
    })?;

    serde_json::from_str(&contents).map_err(|e| CalcError::SerializationError {
        reason: e.to_string(),
    })
}

/// Check if a lock is stale (the process that created it is no longer running)
#[cfg(not(target_arch = "wasm32"))]
fn is_lock_stale(info: &LockInfo) -> bool {
    // Check if it's our machine
    if let Some(our_machine) = hostname() {
        if info.machine == our_machine {
            // Same machine - check if process is still running
            #[cfg(windows)]
            {
                // On Windows, check if PID is valid
                use std::process::Command;
                let output = Command::new("tasklist")
                    .args(["/FI", &format!("PID eq {}", info.pid), "/NH"])
                    .output();
                if let Ok(output) = output {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // If PID not found, lock is stale
                    if stdout.contains("No tasks") || !stdout.contains(&info.pid.to_string()) {
                        return true;
                    }
                }
            }
            #[cfg(unix)]
            {
                // On Unix, check /proc or use kill(0)
                if !fs::metadata(format!("/proc/{}", info.pid)).is_ok() {
                    return true;
                }
            }
        }
    }

    // If lock is more than 24 hours old, consider it stale
    let age = Utc::now() - info.locked_at;
    if age.num_hours() > 24 {
        return true;
    }

    false
}

// ============================================================================
// Save/Load (Cross-platform)
// ============================================================================

/// Save a project to a file with atomic write semantics.
///
/// The save process:
/// 1. Serialize project to JSON
/// 2. Write to a temporary file (.tmp)
/// 3. Sync to disk (fsync)
/// 4. Rename .tmp to .stf (atomic on most filesystems)
///
/// This prevents corruption if the process is interrupted during write.
///
/// # Arguments
///
/// * `project` - The project to save
/// * `path` - Path to save to (should end in .stf)
///
/// # Example
///
/// ```rust,no_run
/// use calc_core::file_io::save_project;
/// use calc_core::project::Project;
/// use std::path::Path;
///
/// let project = Project::new("Engineer", "25-001", "Client");
/// save_project(&project, Path::new("myproject.stf"))?;
/// # Ok::<(), calc_core::errors::CalcError>(())
/// ```
pub fn save_project(project: &Project, path: &Path) -> CalcResult<()> {
    // Serialize to JSON
    let json = serde_json::to_string_pretty(project).map_err(|e| CalcError::SerializationError {
        reason: e.to_string(),
    })?;

    // Create temp file path
    let tmp_path = path.with_extension("stf.tmp");

    // Write to temp file
    let mut tmp_file = File::create(&tmp_path).map_err(|e| {
        CalcError::file_error("create temp file", tmp_path.display().to_string(), e.to_string())
    })?;

    tmp_file.write_all(json.as_bytes()).map_err(|e| {
        CalcError::file_error("write temp file", tmp_path.display().to_string(), e.to_string())
    })?;

    // Sync to disk
    tmp_file.sync_all().map_err(|e| {
        CalcError::file_error("sync temp file", tmp_path.display().to_string(), e.to_string())
    })?;

    // Atomic rename
    fs::rename(&tmp_path, path).map_err(|e| {
        // Clean up temp file if rename fails
        let _ = fs::remove_file(&tmp_path);
        CalcError::file_error("rename to final", path.display().to_string(), e.to_string())
    })?;

    Ok(())
}

/// Load a project from a file.
///
/// # Arguments
///
/// * `path` - Path to the .stf file
///
/// # Returns
///
/// * `Ok(Project)` - Successfully loaded project
/// * `Err(CalcError::VersionMismatch)` - File version is incompatible
/// * `Err(CalcError::SerializationError)` - Invalid JSON
/// * `Err(CalcError::FileError)` - I/O error
///
/// # Example
///
/// ```rust,no_run
/// use calc_core::file_io::load_project;
/// use std::path::Path;
///
/// let project = load_project(Path::new("myproject.stf"))?;
/// println!("Loaded project: {}", project.meta.job_id);
/// # Ok::<(), calc_core::errors::CalcError>(())
/// ```
pub fn load_project(path: &Path) -> CalcResult<Project> {
    // Read file contents
    let mut file = File::open(path).map_err(|e| {
        CalcError::file_error("open", path.display().to_string(), e.to_string())
    })?;

    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|e| {
        CalcError::file_error("read", path.display().to_string(), e.to_string())
    })?;

    // Parse JSON
    let project: Project =
        serde_json::from_str(&contents).map_err(|e| CalcError::SerializationError {
            reason: format!("Invalid JSON in {}: {}", path.display(), e),
        })?;

    // Validate schema version
    validate_version(&project.meta.version)?;

    Ok(project)
}

/// Load a project, returning whether it's read-only due to a lock.
///
/// # Returns
///
/// * `Ok((Project, None))` - Loaded successfully, no lock
/// * `Ok((Project, Some(LockInfo)))` - Loaded, but another user has the lock
/// * `Err(_)` - Failed to load
pub fn load_project_with_lock_check(path: &Path) -> CalcResult<(Project, Option<LockInfo>)> {
    let project = load_project(path)?;
    let lock_info = FileLock::check(path);
    Ok((project, lock_info))
}

/// Validate that a file version is compatible with the current schema.
fn validate_version(file_version: &str) -> CalcResult<()> {
    // Parse semver-style versions
    let file_parts: Vec<u32> = file_version
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();
    let current_parts: Vec<u32> = SCHEMA_VERSION
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();

    if file_parts.is_empty() || current_parts.is_empty() {
        return Err(CalcError::VersionMismatch {
            file_version: file_version.to_string(),
            expected_version: SCHEMA_VERSION.to_string(),
        });
    }

    // Major version must match
    if file_parts[0] != current_parts[0] {
        return Err(CalcError::VersionMismatch {
            file_version: file_version.to_string(),
            expected_version: SCHEMA_VERSION.to_string(),
        });
    }

    // For 0.x versions, minor version must also match (breaking changes allowed)
    if current_parts[0] == 0 && file_parts.len() > 1 && current_parts.len() > 1 {
        if file_parts[1] > current_parts[1] {
            // File is newer than we support
            return Err(CalcError::VersionMismatch {
                file_version: file_version.to_string(),
                expected_version: SCHEMA_VERSION.to_string(),
            });
        }
    }

    Ok(())
}

// ============================================================================
// Tests (Native only - require filesystem)
// ============================================================================

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;
    use std::env::temp_dir;

    fn temp_project_path(name: &str) -> PathBuf {
        temp_dir().join(format!("stratify_test_{}.stf", name))
    }

    #[test]
    fn test_lock_path_generation() {
        let project_path = Path::new("/path/to/project.stf");
        let lock_path = lock_path_for(project_path);
        assert_eq!(lock_path, Path::new("/path/to/project.stf.lock"));
    }

    #[test]
    fn test_lock_info_creation() {
        let info = LockInfo::new("test@example.com");
        assert_eq!(info.user_id, "test@example.com");
        assert!(info.pid > 0);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let path = temp_project_path("roundtrip");

        // Create and save
        let project = Project::new("Test Engineer", "TEST-001", "Test Client");
        save_project(&project, &path).unwrap();

        // Load and verify
        let loaded = load_project(&path).unwrap();
        assert_eq!(loaded.meta.engineer, "Test Engineer");
        assert_eq!(loaded.meta.job_id, "TEST-001");
        assert_eq!(loaded.meta.client, "Test Client");

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_atomic_save_creates_no_tmp_file() {
        let path = temp_project_path("atomic");
        let tmp_path = path.with_extension("stf.tmp");

        let project = Project::new("Test", "TEST", "Client");
        save_project(&project, &path).unwrap();

        // Temp file should not exist after successful save
        assert!(!tmp_path.exists());
        assert!(path.exists());

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_file_lock_acquire_and_release() {
        let path = temp_project_path("lock_test");

        // Create an empty file first
        File::create(&path).unwrap();

        // Acquire lock
        let lock = FileLock::acquire(&path, "test@example.com").unwrap();
        assert_eq!(lock.info.user_id, "test@example.com");

        // Lock file should exist
        let lock_path = lock_path_for(&path);
        assert!(lock_path.exists());

        // Drop lock
        drop(lock);

        // Lock file should be removed
        assert!(!lock_path.exists());

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_version_validation() {
        // Same version should pass
        assert!(validate_version(SCHEMA_VERSION).is_ok());

        // Same major.minor should pass
        assert!(validate_version("0.1.0").is_ok());
        assert!(validate_version("0.1.5").is_ok());

        // Different major should fail
        assert!(validate_version("1.0.0").is_err());

        // Newer minor (in 0.x) should fail
        assert!(validate_version("0.2.0").is_err());
    }

    #[test]
    fn test_load_with_lock_check() {
        let path = temp_project_path("lock_check");

        // Save a project
        let project = Project::new("Test", "TEST", "Client");
        save_project(&project, &path).unwrap();

        // Load without lock - should have no lock info
        let (loaded, lock_info) = load_project_with_lock_check(&path).unwrap();
        assert_eq!(loaded.meta.job_id, "TEST");
        assert!(lock_info.is_none());

        // Clean up
        let _ = fs::remove_file(&path);
    }
}
