//! Hand-rolled argument parsing for `cad build` — no third-party crate,
//! matching every other Stage-2 crate's zero-external-dependency policy.
//! Deliberately minimal: this crate implements only `cad build`
//! (this module's own doc comment / this crate's module doc comment), so
//! there is no subcommand dispatch to build — just `build`'s own flags.

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

/// Every way parsing `cad build`'s own argument list can fail.
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
            ArgsError::MissingSubcommand => write!(f, "expected a subcommand ('build')"),
            ArgsError::UnknownSubcommand(name) => {
                write!(f, "unknown subcommand '{name}' (only 'build' is supported)")
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
}
