//! # isdocker
//!
//! A tiny, zero-dependency library that detects whether the current process is
//! running inside a Docker container (or a compatible container runtime such as
//! Podman, containerd, or Kubernetes).
//!
//! ## Detection strategy
//!
//! On **Linux**, five checks are performed in order. The first conclusive result
//! short-circuits further checks.
//!
//! | Step | Check | Indicates container when… |
//! |------|-------|---------------------------|
//! | 1 | `ISDOCKER` env var | `"1"`, `"true"`, or `"yes"` (case-insensitive) |
//! | 2 | `/.dockerenv` exists | file is present (Docker) |
//! | 3 | `/run/.containerenv` exists | file is present (Podman) |
//! | 4 | `/proc/self/mountinfo` contents | contains Docker or Podman overlay paths |
//! | 5 | `/proc/self/cgroup` contents | contains `docker`, `kubepods`, or `containerd` |
//!
//! ## Environment variable override
//!
//! Setting `ISDOCKER=1` (or `true` / `yes`) forces the function to return
//! `true`. Setting `ISDOCKER=0` (or `false` / `no`) forces it to return
//! `false`. Any other value (or absence) falls through to filesystem checks.
//!
//! This is the **recommended approach** for production systems that need
//! reliable, testable environment detection rather than heuristic-based
//! auto-detection.
//!
//! ## Known limitations
//!
//! The filesystem checks are heuristics, not guarantees:
//!
//! * **cgroupv2** — On modern Linux with the unified cgroup hierarchy,
//!   `/proc/self/cgroup` contains only `0::/` with no runtime markers. The
//!   mountinfo check (step 4) mitigates this, but the env var override is the
//!   most reliable option.
//! * **Docker BuildKit** — `/.dockerenv` is not created during `docker build`
//!   with BuildKit/buildx.
//! * **Podman + `/run` volume mount** — `/run/.containerenv` is not created when
//!   a volume is mounted over `/run`.
//! * **Non-Linux platforms** — always returns `false` without performing any I/O.
//!
//! ## Safety guarantees
//!
//! * Never panics.
//! * Returns `false` on any I/O error rather than propagating errors.
//! * No heap allocations on the fast path (`.dockerenv` check).
//! * `#![forbid(unsafe_code)]` — no unsafe code anywhere.
//! * Zero dependencies beyond `std`.
//!
//! ## Example
//!
//! ```rust
//! use isdocker::is_docker;
//!
//! if is_docker() {
//!     println!("Running inside a container");
//! } else {
//!     println!("Not running inside a container");
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

/// Returns `true` if the current process appears to be running inside a Docker
/// container (or a compatible OCI container runtime such as Podman, containerd,
/// or a Kubernetes pod), and `false` otherwise.
///
/// # Detection strategy
///
/// | Step | Check | Indicates container when… |
/// |------|-------|---------------------------|
/// | 1 | `ISDOCKER` env var | `"1"`, `"true"`, or `"yes"` |
/// | 2 | `/.dockerenv` exists | file is present (Docker) |
/// | 3 | `/run/.containerenv` exists | file is present (Podman) |
/// | 4 | `/proc/self/mountinfo` | contains container overlay paths |
/// | 5 | `/proc/self/cgroup` | contains `docker`, `kubepods`, or `containerd` |
///
/// The first conclusive result short-circuits further checks. The `ISDOCKER`
/// env var can also force `false` by setting it to `"0"`, `"false"`, or `"no"`.
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
///         println!("Running inside a container");
///     }
/// }
/// ```
pub fn is_docker() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Some(forced) = env_override() {
            return forced;
        }
        dockerenv_exists()
            || containerenv_exists()
            || mountinfo_indicates_container()
            || cgroup_indicates_container()
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Alias for [`is_docker`].
///
/// This crate detects Docker, Podman, containerd, and Kubernetes containers.
/// Use whichever name reads better in your code.
///
/// # Example
///
/// ```rust
/// use isdocker::is_container;
///
/// if is_container() {
///     println!("Running inside a container");
/// }
/// ```
pub fn is_container() -> bool {
    is_docker()
}

/// Checks the `ISDOCKER` environment variable for an explicit override.
///
/// Returns `Some(true)` for `"1"`, `"true"`, `"yes"` (case-insensitive),
/// `Some(false)` for `"0"`, `"false"`, `"no"`, and `None` for any other
/// value or if the variable is not set.
#[cfg(target_os = "linux")]
fn env_override() -> Option<bool> {
    std::env::var("ISDOCKER")
        .ok()
        .and_then(|v| parse_env_override(&v))
}

/// Pure parsing logic for the `ISDOCKER` env var value.
#[cfg(target_os = "linux")]
fn parse_env_override(val: &str) -> Option<bool> {
    let lower = val.to_ascii_lowercase();
    match lower.as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

/// Returns `true` when `/.dockerenv` is present.
///
/// Docker creates this file in the root of every container's filesystem.
#[cfg(target_os = "linux")]
fn dockerenv_exists() -> bool {
    std::path::Path::new("/.dockerenv").exists()
}

/// Returns `true` when `/run/.containerenv` is present.
///
/// Podman creates this file inside every container it runs.
#[cfg(target_os = "linux")]
fn containerenv_exists() -> bool {
    std::path::Path::new("/run/.containerenv").exists()
}

/// Returns `true` when `/proc/self/mountinfo` contains overlay filesystem
/// paths associated with container runtimes.
///
/// This check works on both cgroupv1 and cgroupv2 systems, covering the case
/// where `/proc/self/cgroup` contains only `0::/` on cgroupv2.
#[cfg(target_os = "linux")]
fn mountinfo_indicates_container() -> bool {
    match std::fs::read_to_string("/proc/self/mountinfo") {
        Ok(contents) => contains_overlay_marker(&contents),
        Err(_) => false,
    }
}

/// Scans mountinfo contents for known container runtime overlay paths.
#[cfg(target_os = "linux")]
fn contains_overlay_marker(contents: &str) -> bool {
    contents.contains("/docker/containers/")
        || contents.contains("/var/lib/docker/overlay2/")
        || contents.contains("/var/lib/containers/storage/overlay")
}

/// Returns `true` when `/proc/self/cgroup` contains a known container runtime
/// identifier.
///
/// This is a cgroupv1 fallback. On cgroupv2 systems, the cgroup file typically
/// contains only `0::/` with no runtime markers.
#[cfg(target_os = "linux")]
fn cgroup_indicates_container() -> bool {
    match std::fs::read_to_string("/proc/self/cgroup") {
        Ok(contents) => contains_cgroup_marker(&contents),
        Err(_) => false,
    }
}

/// Scans cgroup file contents for known container runtime markers.
#[cfg(target_os = "linux")]
fn contains_cgroup_marker(contents: &str) -> bool {
    contents.contains("docker")
        || contents.contains("kubepods")
        || contents.contains("containerd")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_bool() {
        let _ = is_docker();
    }

    #[test]
    fn is_container_matches_is_docker() {
        assert_eq!(is_docker(), is_container());
    }

    // --- env var parsing ---

    #[cfg(target_os = "linux")]
    #[test]
    fn env_override_truthy() {
        assert_eq!(parse_env_override("1"), Some(true));
        assert_eq!(parse_env_override("true"), Some(true));
        assert_eq!(parse_env_override("TRUE"), Some(true));
        assert_eq!(parse_env_override("True"), Some(true));
        assert_eq!(parse_env_override("yes"), Some(true));
        assert_eq!(parse_env_override("Yes"), Some(true));
        assert_eq!(parse_env_override("YES"), Some(true));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn env_override_falsy() {
        assert_eq!(parse_env_override("0"), Some(false));
        assert_eq!(parse_env_override("false"), Some(false));
        assert_eq!(parse_env_override("FALSE"), Some(false));
        assert_eq!(parse_env_override("False"), Some(false));
        assert_eq!(parse_env_override("no"), Some(false));
        assert_eq!(parse_env_override("No"), Some(false));
        assert_eq!(parse_env_override("NO"), Some(false));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn env_override_unrecognised() {
        assert_eq!(parse_env_override(""), None);
        assert_eq!(parse_env_override("maybe"), None);
        assert_eq!(parse_env_override("2"), None);
        assert_eq!(parse_env_override("docker"), None);
    }

    // --- file existence checks ---

    #[cfg(target_os = "linux")]
    #[test]
    fn dockerenv_exists_does_not_panic() {
        let _ = dockerenv_exists();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn containerenv_exists_does_not_panic() {
        let _ = containerenv_exists();
    }

    // --- mountinfo overlay markers ---

    #[cfg(target_os = "linux")]
    #[test]
    fn overlay_marker_docker_overlay2() {
        assert!(contains_overlay_marker(
            "36 1 0:30 / / rw - overlay overlay rw,lowerdir=/var/lib/docker/overlay2/abc123/diff"
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn overlay_marker_docker_containers() {
        assert!(contains_overlay_marker(
            "100 80 0:50 / /etc/hostname rw - ext4 /dev/sda1 rw /docker/containers/abc123/hostname"
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn overlay_marker_podman() {
        assert!(contains_overlay_marker(
            "36 1 0:30 / / rw - overlay overlay rw,lowerdir=/var/lib/containers/storage/overlay/abc/diff"
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn overlay_marker_negative() {
        assert!(!contains_overlay_marker(
            "36 1 0:30 / / rw - ext4 /dev/sda1 rw"
        ));
        assert!(!contains_overlay_marker(""));
    }

    // --- cgroup markers ---

    #[cfg(target_os = "linux")]
    #[test]
    fn cgroup_marker_positive() {
        assert!(contains_cgroup_marker("12:blkio:/docker/abc123"));
        assert!(contains_cgroup_marker(
            "11:cpu:/kubepods/besteffort/pod123"
        ));
        assert!(contains_cgroup_marker(
            "10:memory:/containerd/tasks/abc"
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cgroup_marker_negative() {
        assert!(!contains_cgroup_marker(
            "11:cpu:/user.slice/user-1000.slice/session-1.scope"
        ));
        assert!(!contains_cgroup_marker("0::/"));
        assert!(!contains_cgroup_marker(""));
    }

    // --- cgroup check ---

    #[cfg(target_os = "linux")]
    #[test]
    fn cgroup_indicates_container_does_not_panic() {
        let _ = cgroup_indicates_container();
    }

    // --- mountinfo check ---

    #[cfg(target_os = "linux")]
    #[test]
    fn mountinfo_indicates_container_does_not_panic() {
        let _ = mountinfo_indicates_container();
    }

    // --- non-linux ---

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn always_false_on_non_linux() {
        assert!(!is_docker());
        assert!(!is_container());
    }
}
