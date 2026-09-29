use std::process::Command;

const EXIT_CODE_ON_A_FAILED_START: Option<i32> = Some(1);

#[test]
fn the_server_refuses_to_start_without_a_data_directory_and_says_which_flag_it_wants() {
    // Arrange
    let binary = env!("CARGO_BIN_EXE_atlas-server");

    // Act
    let output = Command::new(binary).output().expect("the atlas-server binary must run");

    // Assert
    assert_eq!(
        (output.status.code(), String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n")),
        (EXIT_CODE_ON_A_FAILED_START, "Error: --data-dir is required, e.g. --data-dir ../data/compiled\n".to_string())
    );
}
