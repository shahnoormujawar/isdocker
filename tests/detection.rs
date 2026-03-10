use isdocker::is_docker;

/// The function must return a `bool` without panicking in any environment.
#[test]
fn executes_safely() {
    let result = is_docker();
    // Explicitly assert the type is bool (compiler-enforced).
    let _: bool = result;
}

/// Calling `is_docker` multiple times must be idempotent.
#[test]
fn is_idempotent() {
    assert_eq!(is_docker(), is_docker());
}

/// On non-Linux hosts the result is always `false`.
#[cfg(not(target_os = "linux"))]
#[test]
fn always_false_outside_linux() {
    assert!(!is_docker());
}
