//! # isdocker
//!
//! A tiny, zero-dependency library that detects whether the current process is
//! running inside a Docker container (or a compatible container runtime).
//!
//! ## Detection strategy
//!
//! On **Linux**, two checks are performed in order:
//!
//! 1. **`.dockerenv` file** — Docker creates `/.dockerenv` inside every
//!    container. If this file exists, the process is almost certainly running
//!    inside a Docker container.
//!
//! 2. **cgroup membership** — `/proc/self/cgroup` is read and scanned for
//!    well-known container runtime identifiers: `docker`, `kubepods`, and
//!    `containerd`. A match on any of these indicates a containerised
//!    environment.
//!
//! On **non-Linux platforms** (macOS, Windows, …) the function always returns
//! `false` without performing any I/O.
//!
//! ## Safety guarantees
//!
//! * Never panics.
//! * Returns `false` on any I/O error rather than propagating errors.
//! * Performs no heap allocations on the happy path (`.dockerenv` check).
//!
//! ## Example
//!
//! ```rust
//! use isdocker::is_docker;
//!
//! if is_docker() {
//!     println!("Running inside Docker");
//! } else {
//!     println!("Not running inside Docker");
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

/// Returns `true` if the current process appears to be running inside a Docker
/// container (or a compatible OCI container runtime such as containerd or a
/// Kubernetes pod), and `false` otherwise.
///
/// # Detection strategy
///
/// | Step | Check | Indicates Docker when… |
/// |------|-------|------------------------|
/// | 1 | `/.dockerenv` exists | file is present |
/// | 2 | `/proc/self/cgroup` contents | contains `docker`, `kubepods`, or `containerd` |
///
/// The first successful positive result short-circuits further checks.
///
/// # Platform behaviour
///
/// * **Linux** — full detection is performed.
/// * **Other platforms** — always returns `false`.
///
/// # Panics
///
/// This function never panics.
///
/// # Example
///
/// ```rust
/// use isdocker::is_docker;
///
/// fn main() {
///     if is_docker() {
///         println!("Running inside Docker 🐳");
///     }
/// }
/// ```
pub fn is_docker() -> bool {
    #[cfg(target_os = "linux")]
    {
        dockerenv_exists() || cgroup_indicates_container()
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Returns `true` when `/.dockerenv` is present.
///
/// Docker creates this file in the root of every container's filesystem.
/// Its mere existence is a reliable first-pass signal.
#[cfg(target_os = "linux")]
fn dockerenv_exists() -> bool {
    std::path::Path::new("/.dockerenv").exists()
}

/// Returns `true` when `/proc/self/cgroup` contains a known container runtime
/// identifier.
///
/// The cgroup file lists the cgroup hierarchies the process belongs to. Docker,
/// containerd, and Kubernetes all place containers in cgroup paths that include
/// recognisable substrings.
#[cfg(target_os = "linux")]
fn cgroup_indicates_container() -> bool {
    match std::fs::read_to_string("/proc/self/cgroup") {
        Ok(contents) => contains_container_marker(&contents),
        Err(_) => false,
    }
}

/// Scans cgroup file contents for known container runtime markers.
#[cfg(target_os = "linux")]
fn contains_container_marker(contents: &str) -> bool {
    contents.contains("docker") || contents.contains("kubepods") || contents.contains("containerd")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_bool() {
        // Smoke-test: must return a boolean without panicking.
        let _ = is_docker();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn dockerenv_exists_does_not_panic() {
        let _ = dockerenv_exists();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cgroup_indicates_container_does_not_panic() {
        let _ = cgroup_indicates_container();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn marker_detection_positive() {
        assert!(contains_container_marker("12:blkio:/docker/abc123"));
        assert!(contains_container_marker(
            "11:cpu:/kubepods/besteffort/pod123"
        ));
        assert!(contains_container_marker("10:memory:/containerd/tasks/abc"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn marker_detection_negative() {
        assert!(!contains_container_marker(
            "11:cpu:/user.slice/user-1000.slice/session-1.scope"
        ));
        assert!(!contains_container_marker(""));
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn always_false_on_non_linux() {
        assert!(!is_docker());
    }
}
