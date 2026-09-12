//! `cad` — the CLI binary. Thin: all real logic lives in `cad_cli`'s
//! library target so it can be tested in-process (`std::process::exit`
//! aside, this file has no logic of its own worth testing directly).

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match cad_cli::parse_args(&args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("error: {err}");
            eprintln!("usage: cad build <path.aicad> [--json] [--output <path>]");
            std::process::exit(2);
        }
    };

    let report = cad_cli::run_build(&parsed.path, parsed.output.as_deref());
    if parsed.json {
        println!("{}", report.to_json().to_canonical_string());
    } else {
        print!("{}", report.to_human_string());
    }
    std::process::exit(report.exit_code());
}
