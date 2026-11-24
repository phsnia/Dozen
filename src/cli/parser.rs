//! Hand-rolled argument parser that mirrors the argparse subset `dox.py` uses.
//!
//! Supported forms: long flags (`--force`), long options (`--output x` and
//! `--output=x`), short flags (`-h`), combined short flags (`-va`), short
//! options with detached (`-o x`) or attached (`-ox`) values, positionals, and
//! `--` to stop option parsing.

use std::collections::HashMap;

/// The kind of argument a [`Field`] describes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// A boolean flag that takes no value.
    Flag,
    /// An option that consumes one value.
    Opt,
    /// A positional value.
    Pos,
}

/// One accepted argument, declared statically by a command.
#[derive(Clone, Copy, Debug)]
pub struct Field {
    /// Long name without the leading dashes.
    pub long: &'static str,
    /// Optional single-character short name.
    pub short: Option<char>,
    /// Argument kind.
    pub kind: Kind,
}

impl Field {
    /// Declare an option with an optional short name.
    pub const fn option(long: &'static str, short: Option<char>) -> Self {
        Self {
            long,
            short,
            kind: Kind::Opt,
        }
    }

    /// Declare a boolean flag.
    pub const fn flag(long: &'static str) -> Self {
        Self {
            long,
            short: None,
            kind: Kind::Flag,
        }
    }

    /// Declare a boolean flag with a short name.
    pub const fn short_flag(long: &'static str, short: char) -> Self {
        Self {
            long,
            short: Some(short),
            kind: Kind::Flag,
        }
    }

    /// Declare a positional argument.
    pub const fn positional(long: &'static str) -> Self {
        Self {
            long,
            short: None,
            kind: Kind::Pos,
        }
    }

    /// True when this field accepts the long option name `name`.
    fn accepts_long(&self, name: &str) -> bool {
        self.kind != Kind::Pos && self.long == name
    }

    /// True when this field accepts the short option name `name`.
    fn accepts_short(&self, name: char) -> bool {
        self.kind != Kind::Pos && self.short == Some(name)
    }
}

/// Parsed arguments grouped by how they were declared.
#[derive(Default, Debug)]
pub struct Parser {
    /// Long names of flags that were present.
    pub flags: Vec<String>,
    /// Long name to value for options that were present.
    pub opts: HashMap<String, String>,
    /// Positional values in source order.
    pub positional: Vec<String>,
}

impl Parser {
    /// Returns true when flag `name` was provided.
    pub fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|flag| flag == name)
    }

    /// Returns the value of option `name`, if it was provided.
    pub fn opt(&self, name: &str) -> Option<&str> {
        self.opts.get(name).map(String::as_str)
    }

    /// Returns the first positional argument, if any.
    pub fn str(&self) -> Option<&str> {
        self.positional.first().map(String::as_str)
    }

    /// Parse `args` according to `spec`.
    pub fn parse(args: impl Iterator<Item = String>, spec: &[Field]) -> Result<Self, String> {
        let mut parsed = Self::default();
        let mut args = args;
        let mut positional_only = false;

        while let Some(arg) = args.next() {
            if positional_only {
                parsed.positional.push(arg);
                continue;
            }

            if arg == "--" {
                positional_only = true;
                continue;
            }

            if arg == "-" || !arg.starts_with('-') {
                parsed.positional.push(arg);
                continue;
            }

            if let Some(long) = arg.strip_prefix("--") {
                parsed.parse_long(long, &mut args, spec)?;
                continue;
            }

            parsed.parse_short(&arg[1..], &mut args, spec)?;
        }

        Ok(parsed)
    }

    /// Parse one `--name[=value]` argument.
    fn parse_long(
        &mut self,
        long: &str,
        args: &mut impl Iterator<Item = String>,
        spec: &[Field],
    ) -> Result<(), String> {
        let (name, inline_value) = match long.split_once('=') {
            Some((name, value)) => (name, Some(value.to_owned())),
            None => (long, None),
        };

        let field = spec
            .iter()
            .find(|field| field.accepts_long(name))
            .ok_or_else(|| format!("unknown option: --{name}"))?;

        match field.kind {
            Kind::Flag => {
                if inline_value.is_some() {
                    return Err(format!("flag --{name} does not take a value"));
                }
                self.push_flag(field.long);
            }
            Kind::Opt => {
                let value = inline_value
                    .or_else(|| args.next())
                    .ok_or_else(|| format!("option --{name} requires a value"))?;
                self.opts.insert(field.long.to_owned(), value);
            }
            Kind::Pos => unreachable!(),
        }

        Ok(())
    }

    /// Parse one cluster of short options such as `-va` or `-oout.md`.
    fn parse_short(
        &mut self,
        short: &str,
        args: &mut impl Iterator<Item = String>,
        spec: &[Field],
    ) -> Result<(), String> {
        for (index, name) in short.char_indices() {
            let field = spec
                .iter()
                .find(|field| field.accepts_short(name))
                .ok_or_else(|| format!("unknown option: -{name}"))?;

            match field.kind {
                Kind::Flag => self.push_flag(field.long),
                Kind::Opt => {
                    let rest = &short[index + name.len_utf8()..];
                    let value = if rest.is_empty() {
                        args.next()
                            .ok_or_else(|| format!("option -{name} requires a value"))?
                    } else {
                        rest.to_owned()
                    };
                    self.opts.insert(field.long.to_owned(), value);
                    break;
                }
                Kind::Pos => unreachable!(),
            }
        }

        Ok(())
    }

    /// Record a flag once, even when it is repeated.
    fn push_flag(&mut self, long: &str) {
        if !self.flag(long) {
            self.flags.push(long.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &[Field] = &[
        Field::short_flag("verbose", 'v'),
        Field::short_flag("all", 'a'),
        Field::option("output", Some('o')),
        Field::option("title", Some('t')),
        Field::positional("path"),
    ];

    fn parse(args: &[&str]) -> Result<Parser, String> {
        Parser::parse(args.iter().map(|arg| (*arg).to_owned()), SPEC)
    }

    #[test]
    fn parses_long_options_and_flags() {
        let parser = parse(&["--verbose", "--output=out.md", "--title", "Title"]).unwrap();

        assert!(parser.flag("verbose"));
        assert_eq!(parser.opt("output"), Some("out.md"));
        assert_eq!(parser.opt("title"), Some("Title"));
    }

    #[test]
    fn parses_short_flags_and_attached_option_values() {
        let parser = parse(&["-va", "-oout.md"]).unwrap();

        assert!(parser.flag("verbose"));
        assert!(parser.flag("all"));
        assert_eq!(parser.opt("output"), Some("out.md"));
    }

    #[test]
    fn collects_positionals_and_stops_at_double_dash() {
        let parser = parse(&["one", "--", "--two", "-v"]).unwrap();

        assert_eq!(parser.positional.len(), 3);
        assert_eq!(parser.str(), Some("one"));
        assert_eq!(parser.positional[1], "--two");
        assert_eq!(parser.positional[2], "-v");
        assert!(!parser.flag("verbose"));
    }

    #[test]
    fn last_option_wins_and_flags_are_unique() {
        let parser = parse(&["-v", "--verbose", "-t", "a", "-t", "b"]).unwrap();

        assert_eq!(parser.flags.len(), 1);
        assert_eq!(parser.flags[0], "verbose");
        assert_eq!(parser.opt("title"), Some("b"));
    }

    #[test]
    fn rejects_unknown_and_incomplete_options() {
        assert_eq!(parse(&["--nope"]).unwrap_err(), "unknown option: --nope");
        assert_eq!(parse(&["-x"]).unwrap_err(), "unknown option: -x");
        assert_eq!(
            parse(&["--output"]).unwrap_err(),
            "option --output requires a value"
        );
        assert_eq!(parse(&["-o"]).unwrap_err(), "option -o requires a value");
        assert_eq!(
            parse(&["--verbose=yes"]).unwrap_err(),
            "flag --verbose does not take a value"
        );
    }
}
