use isdocker::{is_container, is_docker};

/// The function must return a `bool` without panicking in any environment.
#[test]
fn executes_safely() {
    let result = is_docker();
    let _: bool = result;
}

/// Calling `is_docker` multiple times must be idempotent.
#[test]
fn is_idempotent() {
    assert_eq!(is_docker(), is_docker());
}

/// `is_container` must always agree with `is_docker`.
#[test]
fn is_container_matches_is_docker() {
    assert_eq!(is_docker(), is_container());
}

/// On non-Linux hosts the result is always `false`.
#[cfg(not(target_os = "linux"))]
#[test]
fn always_false_outside_linux() {
    assert!(!is_docker());
    assert!(!is_container());
}
