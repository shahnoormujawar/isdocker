# isdocker

[![Crates.io](https://img.shields.io/crates/v/isdocker.svg)](https://crates.io/crates/isdocker)
[![Docs.rs](https://docs.rs/isdocker/badge.svg)](https://docs.rs/isdocker)
[![CI](https://github.com/your-username/isdocker/actions/workflows/ci.yml/badge.svg)](https://github.com/your-username/isdocker/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A tiny, **zero-dependency** Rust library that detects whether the current
process is running inside a Docker container or a compatible OCI runtime
(Podman, containerd, Kubernetes).

---

## What is `isdocker`?

`isdocker` exposes two functions:

```rust
pub fn is_docker() -> bool
pub fn is_container() -> bool  // alias for is_docker
```

Call either one anywhere in your application to find out whether you are inside
a container. That's it.

---

## Why this crate exists

Many applications need to adapt their behaviour depending on their runtime
environment — logging format, configuration paths, signal handling, and more.
Checking for Docker is a common pattern, but writing the detection logic
correctly (handling I/O errors, supporting multiple container runtimes,
handling cgroupv2, avoiding false positives) takes more care than it first
appears.

`isdocker` packages that logic into a single well-tested function so you never
have to think about it again.

---

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
isdocker = "1.0"
```

---

## Usage

```rust
use isdocker::is_docker;

fn main() {
    if is_docker() {
        println!("Running in a container");
    } else {
        println!("Not running in a container");
    }
}
```

### Environment variable override

For production systems that need reliable, testable environment detection,
set the `ISDOCKER` environment variable instead of relying on auto-detection:

```bash
ISDOCKER=1 ./my-app   # force true
ISDOCKER=0 ./my-app   # force false
```

Accepted values: `1`, `true`, `yes` (returns `true`); `0`, `false`, `no`
(returns `false`). Case-insensitive. Any other value falls through to
filesystem checks.

---

## How detection works

On **Linux**, five checks are performed in sequence. The first conclusive
result short-circuits further checks.

### 1. `ISDOCKER` environment variable

If set to a truthy or falsy value, the result is returned immediately without
any filesystem I/O. This is the most reliable and testable approach.

### 2. `/.dockerenv`

Docker creates the file `/.dockerenv` in the root of every container
filesystem. Checking for its existence is fast (a single `stat` syscall) and
reliable for Docker-managed containers.

### 3. `/run/.containerenv`

Podman creates this file inside every container it runs. This check adds
Podman detection support.

### 4. `/proc/self/mountinfo`

Container runtimes use overlay filesystems with recognisable paths. This
check works on both **cgroupv1 and cgroupv2** systems:

| Runtime | Marker in mountinfo |
|---------|---------------------|
| Docker  | `/docker/containers/` or `/var/lib/docker/overlay2/` |
| Podman  | `/var/lib/containers/storage/overlay` |

### 5. `/proc/self/cgroup` (cgroupv1 fallback)

On cgroupv1 systems, container runtimes place processes in named cgroup paths:

| Runtime / orchestrator | Marker in cgroup path |
|------------------------|-----------------------|
| Docker Engine          | `docker`              |
| Kubernetes             | `kubepods`            |
| containerd             | `containerd`          |

**Note:** On cgroupv2 (now default on modern Linux), this file contains only
`0::/` with no runtime markers. Steps 1-4 cover this gap.

### Non-Linux platforms

On macOS, Windows, and other platforms the function always returns `false`
without performing any I/O.

---

## Known limitations

* **cgroupv2** — `/proc/self/cgroup` is empty on cgroupv2. The mountinfo
  check mitigates this, but for guaranteed results use the `ISDOCKER` env var.
* **Docker BuildKit** — `/.dockerenv` is not created during `docker build`
  with BuildKit/buildx.
* **Podman + `/run` volume** — `/run/.containerenv` is not created when a
  volume is mounted over `/run`.
* **LXC / systemd-nspawn** — These runtimes are not detected by the current
  checks. Use `ISDOCKER=1` as a workaround.
* **False positives** — Non-container overlay filesystems (e.g., live USB
  setups) could theoretically trigger a match.

---

## Safety guarantees

* **Never panics** — all I/O is handled with `match`; errors silently return
  `false`.
* **No `unsafe` code** — enforced with `#![forbid(unsafe_code)]`.
* **No allocations on the fast path** — the `.dockerenv` check is a single
  existence test with no heap allocation.
* **Zero dependencies** — nothing beyond `std`.

---

## Minimum Supported Rust Version (MSRV)

Rust **1.56** (edition 2021). The crate is tested against stable Rust.

---

## License

MIT — see [LICENSE](LICENSE).
