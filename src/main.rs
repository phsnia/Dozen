use std::process::ExitCode;

fn main() -> ExitCode {
    match dozen::cli::run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
