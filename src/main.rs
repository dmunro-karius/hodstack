use std::process::ExitCode;

fn main() -> ExitCode {
    match hodstack::run() {
        Ok(code) => code,
        Err(error) => hodstack::report(&error),
    }
}
