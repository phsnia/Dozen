//! Binary entry point: delegate to the library and map errors to exit code 1.

use std::process::ExitCode;

/// Run the CLI, printing any top-level error to stderr.
fn main() -> ExitCode {
    match dozen::cli::run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
