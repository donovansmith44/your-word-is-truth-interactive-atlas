use std::process::Command;

const COMPILE_STEP: &str = env!("CARGO_BIN_EXE_atlas-graph-compile");
const EXIT_CODE_ON_A_REFUSAL: Option<i32> = Some(1);

#[test]
fn the_compile_step_refuses_to_run_without_a_data_directory_and_says_which_flag_it_wants() {
    // Arrange
    let no_arguments: [&str; 0] = [];

    // Act
    let run = Command::new(COMPILE_STEP).args(no_arguments).output().expect("the atlas-graph-compile binary must run");

    // Assert
    assert_eq!(
        (run.status.code(), String::from_utf8_lossy(&run.stderr).replace("\r\n", "\n")),
        (EXIT_CODE_ON_A_REFUSAL, "Error: --data-dir is required, e.g. --data-dir ../data/compiled\n".to_string())
    );
}
