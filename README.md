# isdocker

[![Crates.io](https://img.shields.io/crates/v/isdocker.svg)](https://crates.io/crates/isdocker)
[![Docs.rs](https://docs.rs/isdocker/badge.svg)](https://docs.rs/isdocker)
[![CI](https://github.com/your-username/isdocker/actions/workflows/ci.yml/badge.svg)](https://github.com/your-username/isdocker/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A tiny, **zero-dependency** Rust library that detects whether the current
process is running inside a Docker container or a compatible OCI runtime.

---

## What is `isdocker`?

`isdocker` exposes a single function:

```rust
pub fn is_docker() -> bool
```

Call it anywhere in your application to find out whether you are inside a
container. That's it.

---

## Why this crate exists

Many applications need to adapt their behaviour depending on their runtime
environment — logging format, configuration paths, signal handling, and more.
Checking for Docker is a common pattern, but writing the detection logic
correctly (handling I/O errors, supporting multiple container runtimes,
avoiding false positives) takes more care than it first appears.

`isdocker` packages that logic into a single well-tested function so you never
have to think about it again.

---

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
isdocker = "0.1"
```

---

## Usage

```rust
use isdocker::is_docker;

fn main() {
    if is_docker() {
        println!("Running in Docker 🐳");
    } else {
        println!("Not running in Docker");
    }
}
```

Conditional logging configuration:

```rust
use isdocker::is_docker;

fn init_logging() {
    if is_docker() {
        // Emit structured JSON logs for log aggregators.
        init_json_logger();
    } else {
        // Pretty human-readable output for local development.
        init_pretty_logger();
    }
}
```

---

## How detection works

On **Linux**, two checks are performed in sequence. The first positive result
short-circuits further checks.

### 1. `/.dockerenv`

Docker creates the file `/.dockerenv` in the root of every container
filesystem. Checking for its existence is fast (a single `stat` syscall) and
reliable for Docker-managed containers.

### 2. `/proc/self/cgroup`

The Linux cgroup virtual filesystem exposes the cgroup hierarchy of the current
process at `/proc/self/cgroup`. Docker, containerd, and Kubernetes all place
containers in named cgroup paths that include recognisable substrings:

| Runtime / orchestrator | Marker in cgroup path |
|------------------------|-----------------------|
| Docker Engine          | `docker`              |
| Kubernetes             | `kubepods`            |
| containerd             | `containerd`          |

If any of these strings appear in the cgroup file, the function returns `true`.

### Non-Linux platforms

On macOS, Windows, and other platforms the function always returns `false`
without performing any I/O. Container detection is only meaningful on Linux.

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
