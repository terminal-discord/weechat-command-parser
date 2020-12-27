use std::collections::HashSet;
use std::fmt::Formatter;
use std::{error, fmt};

#[derive(Debug, Clone)]
pub enum Error {
    RequiredArgMissing(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::RequiredArgMissing(arg) => write!(f, "Missing required arg \"{}\"", arg),
        }
    }
}

impl error::Error for Error {}

#[derive(Clone)]
struct ArgRule {
    pub name: String,
    pub required: bool,
}

#[derive(Clone)]
pub struct Command {
    name: String,
    flags: HashSet<String>,
    args: Vec<ArgRule>,
    subcommands: Vec<Command>,
}

impl Command {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            flags: HashSet::new(),
            args: Vec::new(),
            subcommands: Vec::new(),
        }
    }

    pub fn flag(mut self, flag: &str) -> Self {
        self.flags.insert(flag.into());

        self
    }

    pub fn flags(mut self, flags: &[&str]) -> Self {
        self.flags.extend(flags.iter().map(|&arg| arg.to_owned()));

        self
    }

    pub fn arg(mut self, name: &str, required: bool) -> Self {
        self.args.push(ArgRule {
            name: name.to_owned(),
            required,
        });

        self
    }

    pub fn subcommand(mut self, subcommand: Command) -> Self {
        self.subcommands.push(subcommand);

        self
    }

    pub fn parse_from<I, T>(&self, iter: I) -> Result<ParsedCommand, Error>
    where
        I: Iterator<Item = T>,
        T: Into<String>,
    {
        let mut args: Vec<String> = iter.map(Into::into).collect();
        let mut flags = HashSet::new();

        let mut subcommand_match = Box::new(None);

        if args.get(0) == Some(&self.name) {
            args.remove(0);
        }

        for subcommand in &self.subcommands {
            if args.get(0) == Some(&subcommand.name) {
                *subcommand_match = Some((
                    subcommand.name.clone(),
                    subcommand.parse_from(args[1..].iter())?,
                ));
            }
        }

        for arg in args
            .clone()
            .into_iter()
            .take_while(|arg| self.flags.contains(arg))
        {
            flags.insert(arg);
            args.remove(0);
        }

        let mut parsed_args = Vec::new();

        for (rule, arg) in self.args.iter().zip(
            args.into_iter()
                .map(Option::Some)
                .chain(std::iter::repeat(None)),
        ) {
            match arg {
                Some(arg) => parsed_args.push(ParsedArg {
                    name: rule.name.clone(),
                    value: arg,
                }),
                None => {
                    if rule.required {
                        return Err(Error::RequiredArgMissing(rule.name.clone()));
                    }
                }
            };
        }

        Ok(ParsedCommand {
            command: self.name.clone(),
            flags,
            args: parsed_args,
            subcommand_match,
        })
    }

    pub fn parse(&self, input: &str) -> Result<ParsedCommand, Error> {
        self.parse_from(input.split(' '))
    }
}

#[derive(Debug, Clone)]
struct ParsedArg {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct ParsedCommand {
    command: String,
    flags: HashSet<String>,
    args: Vec<ParsedArg>,
    subcommand_match: Box<Option<(String, ParsedCommand)>>,
}

impl ParsedCommand {
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    pub fn arg(&self, name: &str) -> Option<&str> {
        self.args
            .iter()
            .find(|arg| arg.name == name)
            .map(|arg| arg.value.as_ref())
    }

    pub fn args(&self) -> Vec<&str> {
        self.args.iter().map(|arg| arg.value.as_ref()).collect()
    }

    pub fn subcommand(&self) -> Option<(&str, ParsedCommand)> {
        (*self.subcommand_match)
            .as_ref()
            .map(|(name, cmd)| (name.as_ref(), cmd.clone()))
    }

    pub fn command(&self) -> &str {
        &self.command
    }
}

#[cfg(test)]
mod tests {
    use crate::Command;

    #[test]
    fn smoke_test() {
        let matches = Command::new("/hello")
            .flag("-foo")
            .arg("one", true)
            .arg("two", true)
            .arg("three", true)
            .arg("four", true)
            .parse("/hello -foo -spam bar -foo baz")
            .unwrap();
        assert!(matches.has_flag("-foo"));
        assert!(!matches.has_flag("-spam"));

        assert_eq!(matches.args(), &["-spam", "bar", "-foo", "baz"]);
    }

    #[test]
    fn required_arg() {
        let matches = Command::new("/hello")
            .arg("one", true)
            .arg("two", true)
            .parse("/hello foo");
        assert!(matches.is_err());
        assert_eq!(
            matches.unwrap_err().to_string(),
            "Missing required arg \"two\""
        )
    }

    #[test]
    fn optional_arg() {
        let matches = Command::new("/hello")
            .arg("one", false)
            .arg("two", false)
            .parse("/hello foo");
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().args(), &["foo"])
    }

    #[test]
    fn multiple_flags() {
        let matches = Command::new("/hello")
            .flags(&["-foo", "-spam"])
            .arg("one", true)
            .arg("two", true)
            .arg("three", true)
            .parse("/hello -foo -spam bar -foo baz")
            .unwrap();
        assert!(matches.has_flag("-foo"));
        assert!(matches.has_flag("-spam"));

        assert_eq!(matches.args(), &["bar", "-foo", "baz"]);
    }

    #[test]
    fn args() {
        let matches = Command::new("/hello")
            .flags(&["-foo", "-spam"])
            .arg("one", true)
            .arg("two", true)
            .arg("three", true)
            .parse("/hello -foo -spam bar -foo baz")
            .unwrap();
        assert!(matches.has_flag("-foo"));
        assert!(matches.has_flag("-spam"));

        assert_eq!(matches.args(), &["bar", "-foo", "baz"]);
    }

    #[test]
    fn subcommand() {
        let matches = Command::new("/hello")
            .subcommand(
                Command::new("subcommand")
                    .flags(&["-foo", "-spam"])
                    .arg("one", true)
                    .arg("two", true)
                    .arg("three", true),
            )
            .parse("/hello subcommand -foo -spam bar -foo baz")
            .unwrap();

        assert!(matches.subcommand().is_some());
        assert_eq!(matches.subcommand().unwrap().0, "subcommand");
        assert!(matches.subcommand().unwrap().1.has_flag("-foo"));
        assert!(matches.subcommand().unwrap().1.has_flag("-spam"));
        assert_eq!(
            matches.subcommand().unwrap().1.args(),
            &["bar", "-foo", "baz"]
        );
    }

    #[test]
    fn subcommand2() {
        let matches = Command::new("/discord")
            .subcommand(
                Command::new("subcommand")
                    .flags(&["-foo", "-spam"])
                    .arg("one", true)
                    .arg("two", true)
                    .arg("three", true),
            )
            .parse("/discord subcommand -foo -spam bar -foo baz")
            .unwrap();

        assert!(matches.subcommand().is_some());
        assert_eq!(matches.subcommand().unwrap().0, "subcommand");
        assert!(matches.subcommand().unwrap().1.has_flag("-foo"));
        assert!(matches.subcommand().unwrap().1.has_flag("-spam"));
        assert_eq!(
            matches.subcommand().unwrap().1.args(),
            &["bar", "-foo", "baz"]
        );
    }
}
