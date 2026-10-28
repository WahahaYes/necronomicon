//! `necro`: checks a repo against its house rules (spec/v0.md).

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--version" => {
            println!("necro {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("necro: not implemented yet; only --version works");
            ExitCode::from(2)
        }
    }
}
