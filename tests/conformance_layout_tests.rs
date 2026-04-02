use std::path::Path;

#[test]
fn required_conformance_fixture_directories_exist() {
    let required_dirs = [
        "tests/conformance/core",
        "tests/conformance/hypervisor",
        "tests/conformance/policy",
        "tests/conformance/canonical",
    ];

    for dir in required_dirs {
        assert!(
            Path::new(dir).is_dir(),
            "required conformance fixture directory is missing: {dir}"
        );
    }
}
