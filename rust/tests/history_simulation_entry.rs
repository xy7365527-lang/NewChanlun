#![cfg(feature = "production_session")]

/// 受控 E/S Unix authority 只证明 X 接口，不冒称完整业务 producer。
#[test]
fn real_x_process_authority_restart_and_double_replay() {
    let directory = std::env::temp_dir().join(format!("issue1381-x-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/history_simulation_entry.py"
        ))
        .arg(env!("CARGO_BIN_EXE_x_execution_session"))
        .arg(&directory)
        .output()
        .expect("python3 standard-library process fixture");
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}\nevidence={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        directory.display()
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
    std::fs::remove_dir_all(&directory).unwrap();
}
