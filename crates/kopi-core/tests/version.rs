#[test]
fn version_is_semver() {
    let parts: Vec<&str> = kopi_core::VERSION.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()));
}
