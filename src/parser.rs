use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Flag,
    Opt,
    Pos,
}

#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub long: &'static str,
    pub short: Option<char>,
    pub kind: Kind,
}

#[derive(Default, Debug)]
pub struct Parser {
    pub flags: Vec<String>,
    pub opts: HashMap<String, String>,
    pub positional: Vec<String>,
}

impl Parser {
    /// Returns true when flag `n` was provided.
    pub fn flag(&self, n: &str) -> bool {
        self.flags.iter().any(|flag| flag == n)
    }

    /// Returns the value of option `n`, if it was provided.
    pub fn opt(&self, n: &str) -> Option<&str> {
        self.opts.get(n).map(String::as_str)
    }

    /// Returns the first positional argument, if any.
    #[allow(dead_code)]
    pub fn str(&self) -> Option<&str> {
        self.positional.first().map(String::as_str)
    }

    /// Parses arguments according to `spec`.
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
                let (name, inline_value) = match long.split_once('=') {
                    Some((name, value)) => (name, Some(value.to_owned())),
                    None => (long, None),
                };

                let field = spec
                    .iter()
                    .find(|field| field.kind != Kind::Pos && field.long == name)
                    .ok_or_else(|| format!("unknown option: --{name}"))?;

                match field.kind {
                    Kind::Flag => {
                        if inline_value.is_some() {
                            return Err(format!("flag --{name} does not take a value"));
                        }

                        if !parsed.flag(field.long) {
                            parsed.flags.push(field.long.to_owned());
                        }
                    }
                    Kind::Opt => {
                        let value = match inline_value {
                            Some(value) => value,
                            None => args
                                .next()
                                .ok_or_else(|| format!("option --{name} requires a value"))?,
                        };

                        parsed.opts.insert(field.long.to_owned(), value);
                    }
                    Kind::Pos => unreachable!(),
                }

                continue;
            }

            let short = &arg[1..];

            for (index, name) in short.char_indices() {
                let field = spec
                    .iter()
                    .find(|field| field.kind != Kind::Pos && field.short == Some(name))
                    .ok_or_else(|| format!("unknown option: -{name}"))?;

                match field.kind {
                    Kind::Flag => {
                        if !parsed.flag(field.long) {
                            parsed.flags.push(field.long.to_owned());
                        }
                    }
                    Kind::Opt => {
                        let rest = &short[index + name.len_utf8()..];
                        let value = if rest.is_empty() {
                            args.next()
                                .ok_or_else(|| format!("option -{name} requires a value"))?
                        } else {
                            rest.to_owned()
                        };

                        parsed.opts.insert(field.long.to_owned(), value);
                        break;
                    }
                    Kind::Pos => unreachable!(),
                }
            }
        }

        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &[Field] = &[
        Field {
            long: "verbose",
            short: Some('v'),
            kind: Kind::Flag,
        },
        Field {
            long: "all",
            short: Some('a'),
            kind: Kind::Flag,
        },
        Field {
            long: "output",
            short: Some('o'),
            kind: Kind::Opt,
        },
        Field {
            long: "title",
            short: Some('t'),
            kind: Kind::Opt,
        },
        Field {
            long: "path",
            short: None,
            kind: Kind::Pos,
        },
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
