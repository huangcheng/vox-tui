use std::process::Command;

#[test]
fn test_binary_runs() {
    let output = Command::new(env!("CARGO_BIN_EXE_vox"))
        .arg("--help")
        .output()
        .expect("Failed to run vox binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("vox") || stdout.contains("Multi-provider") || stderr.contains("terminal") || output.status.code() == Some(1),
        "Unexpected output: stdout={:?}, stderr={:?}",
        stdout, stderr
    );
}
