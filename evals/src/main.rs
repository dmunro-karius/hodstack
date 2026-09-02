mod base;
mod case;
mod grade;
mod run;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let list = args.iter().any(|arg| arg == "--list");
    let runs = runs(&args);
    let filter = positional(&args);

    match run::main(list, filter.as_deref(), runs) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn runs(args: &[String]) -> usize {
    args.iter()
        .position(|arg| arg == "--runs")
        .and_then(|at| args.get(at + 1))
        .and_then(|value| value.parse().ok())
        .unwrap_or(1)
}

fn positional(args: &[String]) -> Option<String> {
    let mut skip_next = false;

    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }

        if arg == "--runs" {
            skip_next = true;
            continue;
        }

        if !arg.starts_with("--") {
            return Some(arg.clone());
        }
    }

    None
}
