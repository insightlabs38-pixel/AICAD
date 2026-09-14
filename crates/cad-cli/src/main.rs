//! `cad` — the CLI binary. Thin: all real logic lives in `cad_cli`'s
//! library target so it can be tested in-process (`std::process::exit`
//! aside, this file has no logic of its own worth testing directly).

use cad_cli::Command;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match cad_cli::parse_command(&args) {
        Ok(command) => command,
        Err(err) => {
            eprintln!("error: {err}");
            eprintln!(
                "usage: cad build <path.aicad> [--json] [--output <path>] [--name <binding>[.<field>]]"
            );
            eprintln!("       cad refs check <path.aicad> [--json]");
            std::process::exit(2);
        }
    };

    match command {
        Command::Build(parsed) => {
            let report = cad_cli::run_build(
                &parsed.path,
                parsed.output.as_deref(),
                parsed.name.as_deref(),
            );
            if parsed.json {
                println!("{}", report.to_json().to_canonical_string());
            } else {
                print!("{}", report.to_human_string());
            }
            std::process::exit(report.exit_code());
        }
        Command::RefsCheck(parsed) => {
            let report = cad_cli::run_refs_check(&parsed.path);
            if parsed.json {
                println!("{}", report.to_json().to_canonical_string());
            } else {
                print!("{}", report.to_human_string());
            }
            std::process::exit(report.exit_code());
        }
    }
}
