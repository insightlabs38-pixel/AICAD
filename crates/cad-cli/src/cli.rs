//! Hand-rolled argument parsing for `cad build`/`cad refs check` — no
//! third-party crate, matching every other Stage-2 crate's
//! zero-external-dependency policy. `AICAD-095` adds this crate's second
//! subcommand (`refs check`) alongside the original `build`; [`Command`]/
//! [`parse_command`] is the resulting top-level dispatch this crate's own
//! module doc comment previously said did not exist yet.

use std::path::PathBuf;

/// One parsed `cad build` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedArgs {
    pub path: PathBuf,
    pub json: bool,
    pub output: Option<PathBuf>,
    /// `--name <binding>[.<field>]` (`AICAD-079`): selects a specific
    /// top-level named output (a plain top-level binding, or one named
    /// field of a top-level `part`'s own exposed outputs, `AICAD-071`)
    /// as the `--output` export target, instead of `build_source`'s own
    /// pre-`AICAD-079` default of "the last geometry-producing node
    /// created anywhere in the program's shared `GeometryGraph`". See
    /// `crate::build::resolve_named_output`'s own doc comment for exactly
    /// what this does and does not resolve (an `explicit`-durability-level
    /// binding-name lookup only — never a query/topology search).
    pub name: Option<String>,
}

/// One parsed `cad refs check` invocation (`AICAD-095`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefsCheckArgs {
    pub path: PathBuf,
    pub json: bool,
}

/// The top-level subcommand this binary dispatches on (`AICAD-095`: `cad
/// build` gains a sibling, `cad refs check`) — see [`parse_command`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Build(ParsedArgs),
    RefsCheck(RefsCheckArgs),
}

/// Every way parsing a `cad <subcommand> ...` argument list can fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgsError {
    MissingSubcommand,
    UnknownSubcommand(String),
    MissingPath,
    TooManyPositionalArguments,
    UnknownFlag(String),
    MissingValueFor(&'static str),
}

impl std::fmt::Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgsError::MissingSubcommand => {
                write!(f, "expected a subcommand ('build' or 'refs check')")
            }
            ArgsError::UnknownSubcommand(name) => {
                write!(
                    f,
                    "unknown subcommand '{name}' (only 'build'/'refs check' are supported)"
                )
            }
            ArgsError::MissingPath => write!(f, "expected a '.aicad' source path"),
            ArgsError::TooManyPositionalArguments => {
                write!(f, "expected exactly one source path")
            }
            ArgsError::UnknownFlag(flag) => write!(f, "unknown flag '{flag}'"),
            ArgsError::MissingValueFor(flag) => write!(f, "'{flag}' requires a value"),
        }
    }
}

/// Parses `args` (expected to already exclude the program's own name, i.e.
/// `std::env::args().skip(1)`) as `build <path> [--json] [--output
/// <path>]`.
pub fn parse_args(args: &[String]) -> Result<ParsedArgs, ArgsError> {
    let mut iter = args.iter();
    let subcommand = iter.next().ok_or(ArgsError::MissingSubcommand)?;
    if subcommand != "build" {
        return Err(ArgsError::UnknownSubcommand(subcommand.clone()));
    }

    let mut path: Option<PathBuf> = None;
    let mut json = false;
    let mut output: Option<PathBuf> = None;
    let mut name: Option<String> = None;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--output" => {
                let value = iter.next().ok_or(ArgsError::MissingValueFor("--output"))?;
                output = Some(PathBuf::from(value));
            }
            "--name" => {
                let value = iter.next().ok_or(ArgsError::MissingValueFor("--name"))?;
                name = Some(value.clone());
            }
            other if other.starts_with("--") => {
                return Err(ArgsError::UnknownFlag(other.to_string()));
            }
            positional => {
                if path.is_some() {
                    return Err(ArgsError::TooManyPositionalArguments);
                }
                path = Some(PathBuf::from(positional));
            }
        }
    }

    Ok(ParsedArgs {
        path: path.ok_or(ArgsError::MissingPath)?,
        json,
        output,
        name,
    })
}

/// Parses `refs check <path> [--json]` (`AICAD-095`). `args` includes the
/// leading `refs` token, mirroring [`parse_args`]'s own "`args` includes
/// the leading `build` token" convention.
pub fn parse_refs_check_args(args: &[String]) -> Result<RefsCheckArgs, ArgsError> {
    let mut iter = args.iter();
    let subcommand = iter.next().ok_or(ArgsError::MissingSubcommand)?;
    if subcommand != "refs" {
        return Err(ArgsError::UnknownSubcommand(subcommand.clone()));
    }
    let sub = iter.next().ok_or(ArgsError::MissingSubcommand)?;
    if sub != "check" {
        return Err(ArgsError::UnknownSubcommand(format!("refs {sub}")));
    }

    let mut path: Option<PathBuf> = None;
    let mut json = false;

    for arg in iter {
        match arg.as_str() {
            "--json" => json = true,
            other if other.starts_with("--") => {
                return Err(ArgsError::UnknownFlag(other.to_string()));
            }
            positional => {
                if path.is_some() {
                    return Err(ArgsError::TooManyPositionalArguments);
                }
                path = Some(PathBuf::from(positional));
            }
        }
    }

    Ok(RefsCheckArgs {
        path: path.ok_or(ArgsError::MissingPath)?,
        json,
    })
}

/// Parses the full `cad <subcommand> ...` invocation: `build ...` (see
/// [`parse_args`]) or `refs check ...` (see [`parse_refs_check_args`]).
/// The one place this crate decides *which* subcommand's own parser to
/// run — `main.rs`'s only entry point into argument parsing.
pub fn parse_command(args: &[String]) -> Result<Command, ArgsError> {
    match args.first().map(String::as_str) {
        Some("build") => parse_args(args).map(Command::Build),
        Some("refs") => parse_refs_check_args(args).map(Command::RefsCheck),
        Some(other) => Err(ArgsError::UnknownSubcommand(other.to_string())),
        None => Err(ArgsError::MissingSubcommand),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_bare_build_invocation() {
        let args: Vec<String> = vec!["build".into(), "part.aicad".into()];
        let parsed = parse_args(&args).unwrap();
        assert_eq!(parsed.path, PathBuf::from("part.aicad"));
        assert!(!parsed.json);
        assert_eq!(parsed.output, None);
        assert_eq!(parsed.name, None);
    }

    #[test]
    fn parses_json_and_output_flags_in_either_order() {
        let args: Vec<String> = vec![
            "build".into(),
            "--json".into(),
            "part.aicad".into(),
            "--output".into(),
            "out.step".into(),
        ];
        let parsed = parse_args(&args).unwrap();
        assert_eq!(parsed.path, PathBuf::from("part.aicad"));
        assert!(parsed.json);
        assert_eq!(parsed.output, Some(PathBuf::from("out.step")));
    }

    #[test]
    fn parses_a_name_flag() {
        let args: Vec<String> = vec![
            "build".into(),
            "part.aicad".into(),
            "--name".into(),
            "Bracket.body".into(),
        ];
        let parsed = parse_args(&args).unwrap();
        assert_eq!(parsed.name, Some("Bracket.body".to_string()));
    }

    #[test]
    fn name_without_a_value_is_an_error() {
        let args: Vec<String> = vec!["build".into(), "part.aicad".into(), "--name".into()];
        assert_eq!(
            parse_args(&args).unwrap_err(),
            ArgsError::MissingValueFor("--name")
        );
    }

    #[test]
    fn missing_subcommand_is_an_error() {
        assert_eq!(parse_args(&[]).unwrap_err(), ArgsError::MissingSubcommand);
    }

    #[test]
    fn unknown_subcommand_is_an_error() {
        let args: Vec<String> = vec!["test".into()];
        assert_eq!(
            parse_args(&args).unwrap_err(),
            ArgsError::UnknownSubcommand("test".to_string())
        );
    }

    #[test]
    fn missing_path_is_an_error() {
        let args: Vec<String> = vec!["build".into(), "--json".into()];
        assert_eq!(parse_args(&args).unwrap_err(), ArgsError::MissingPath);
    }

    #[test]
    fn output_without_a_value_is_an_error() {
        let args: Vec<String> = vec!["build".into(), "part.aicad".into(), "--output".into()];
        assert_eq!(
            parse_args(&args).unwrap_err(),
            ArgsError::MissingValueFor("--output")
        );
    }

    #[test]
    fn two_positional_arguments_is_an_error() {
        let args: Vec<String> = vec!["build".into(), "a.aicad".into(), "b.aicad".into()];
        assert_eq!(
            parse_args(&args).unwrap_err(),
            ArgsError::TooManyPositionalArguments
        );
    }

    #[test]
    fn unknown_flag_is_an_error() {
        let args: Vec<String> = vec!["build".into(), "part.aicad".into(), "--bogus".into()];
        assert_eq!(
            parse_args(&args).unwrap_err(),
            ArgsError::UnknownFlag("--bogus".to_string())
        );
    }

    #[test]
    fn parses_a_bare_refs_check_invocation() {
        let args: Vec<String> = vec!["refs".into(), "check".into(), "part.aicad".into()];
        let parsed = parse_refs_check_args(&args).unwrap();
        assert_eq!(parsed.path, PathBuf::from("part.aicad"));
        assert!(!parsed.json);
    }

    #[test]
    fn parses_refs_check_with_json() {
        let args: Vec<String> = vec![
            "refs".into(),
            "check".into(),
            "--json".into(),
            "part.aicad".into(),
        ];
        let parsed = parse_refs_check_args(&args).unwrap();
        assert!(parsed.json);
    }

    #[test]
    fn refs_without_check_is_an_error() {
        let args: Vec<String> = vec!["refs".into(), "list".into()];
        assert_eq!(
            parse_refs_check_args(&args).unwrap_err(),
            ArgsError::UnknownSubcommand("refs list".to_string())
        );
    }

    #[test]
    fn refs_check_missing_path_is_an_error() {
        let args: Vec<String> = vec!["refs".into(), "check".into()];
        assert_eq!(
            parse_refs_check_args(&args).unwrap_err(),
            ArgsError::MissingPath
        );
    }

    #[test]
    fn parse_command_dispatches_build_and_refs_check() {
        let build_args: Vec<String> = vec!["build".into(), "part.aicad".into()];
        assert_eq!(
            parse_command(&build_args).unwrap(),
            Command::Build(parse_args(&build_args).unwrap())
        );

        let refs_args: Vec<String> = vec!["refs".into(), "check".into(), "part.aicad".into()];
        assert_eq!(
            parse_command(&refs_args).unwrap(),
            Command::RefsCheck(parse_refs_check_args(&refs_args).unwrap())
        );
    }

    #[test]
    fn parse_command_with_an_unknown_subcommand_is_an_error() {
        let args: Vec<String> = vec!["explode".into()];
        assert_eq!(
            parse_command(&args).unwrap_err(),
            ArgsError::UnknownSubcommand("explode".to_string())
        );
    }

    #[test]
    fn parse_command_with_no_arguments_is_an_error() {
        assert_eq!(
            parse_command(&[]).unwrap_err(),
            ArgsError::MissingSubcommand
        );
    }
}
