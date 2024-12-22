use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ffi::CStr;

#[derive(Debug)]
pub enum ArgValue {
    Bool(bool),
    String(String),
}

impl ArgValue {
    pub fn matches_type(&self, arg_type: &ArgValueT) -> bool {
        matches!(
            (self, arg_type),
            (ArgValue::Bool(_), ArgValueT::Bool) | (ArgValue::String(_), ArgValueT::String)
        )
    }
}

#[derive(Debug, Default)]
struct ArgMatches {
    args: BTreeMap<String, ArgValue>,
}

impl ArgMatches {
    pub fn new() -> Self {
        Self {
            args: BTreeMap::new(),
        }
    }

    pub fn add_arg(&mut self, name: &str, value: ArgValue) {
        self.args.insert(name.to_string(), value);
    }

    pub fn get_arg(&self, name: &str) -> Option<&ArgValue> {
        self.args.get(name)
    }
}

#[derive(Debug)]
pub struct CommandMatches {
    pub name: String,
    args: ArgMatches,
}

impl CommandMatches {
    pub fn get_arg(&self, name: &str) -> Option<&ArgValue> {
        self.args.get_arg(name)
    }

    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.get_arg(name).map_or(Some(false), |v| {
            if let ArgValue::Bool(b) = v {
                Some(*b)
            } else {
                None
            }
        })
    }

    pub fn get_string(&self, name: &str) -> Option<String> {
        self.get_arg(name).and_then(|v| {
            if let ArgValue::String(s) = v {
                Some(s.clone())
            } else {
                None
            }
        })
    }
}

#[derive(Debug)]
pub struct CliMatches {
    command: Option<CommandMatches>,
    global: ArgMatches,
}

impl CliMatches {
    pub fn get_arg(&self, name: &str) -> Option<&ArgValue> {
        self.global.get_arg(name)
    }

    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.get_arg(name).map_or(Some(false), |v| {
            if let ArgValue::Bool(b) = v {
                Some(*b)
            } else {
                None
            }
        })
    }

    pub fn get_string(&self, name: &str) -> Option<String> {
        self.get_arg(name).and_then(|v| {
            if let ArgValue::String(s) = v {
                Some(s.clone())
            } else {
                None
            }
        })
    }

    pub fn get_command(&self) -> Option<&CommandMatches> {
        self.command.as_ref()
    }
}

pub struct Cli {
    name: &'static str,
    version: Option<&'static str>,
    about: Option<&'static str>,
    args: Vec<ArgDefinition>,
    commands: Vec<Command>,
}

impl Cli {
    pub fn new(name: &'static str) -> Self {
        let mut cli = Self {
            name,
            version: None,
            about: None,
            args: Vec::new(),
            commands: Vec::new(),
        };
        cli.args.push(
            ArgDefinition::new("help")
                .short('h')
                .long("help")
                .help("Show this help message"),
        );
        cli.args.push(
            ArgDefinition::new("version")
                .short('V')
                .long("version")
                .help("Show the version information"),
        );
        cli
    }

    pub fn version(mut self, version: &'static str) -> Self {
        self.version = Some(version);
        self
    }

    pub fn about(mut self, about: &'static str) -> Self {
        self.about = Some(about);
        self
    }

    pub fn arg(mut self, definition: ArgDefinition) -> Self {
        self.args.push(definition);
        self
    }

    pub fn command(mut self, command: Command) -> Self {
        self.commands.push(command);
        self
    }

    fn print_help(&self) {
        println!(
            "{} - {}",
            self.name,
            self.about.unwrap_or("No description available")
        );
        if let Some(version) = self.version {
            println!("Version: {}", version);
        }
        println!("\nCommands:");
        for command in &self.commands {
            println!(
                "  {} - {}",
                command.name,
                command.about.unwrap_or("No description")
            );
        }
        println!("\nGlobal Options:");
        for arg in &self.args {
            println!(
                "  {} {} - {}",
                arg.short.map_or(String::new(), |s| format!("-{}", s)),
                arg.long
                    .clone()
                    .map_or(String::new(), |l| format!("--{}", l)),
                arg.help.clone().unwrap_or_default()
            );
        }
    }

    fn print_version(&self) {
        println!("{} version {}", self.name, self.version.unwrap_or("N/A"));
    }

    pub fn get_matches(&self) -> Result<CliMatches, String> {
        let mut arg: crate::bindings::process_arguments = crate::bindings::process_arguments {
            argc: 0,
            argv: core::ptr::null_mut(),
        };

        unsafe {
            crate::bindings::polyos_process_get_args(&mut arg);
        };

        let mut input = Vec::with_capacity((arg.argc - 1) as usize);
        for i in 1..arg.argc {
            let ptr = unsafe { *arg.argv.offset(i as isize) };
            let cstr = unsafe { CStr::from_ptr(ptr) };
            let str = cstr.to_str().unwrap();
            input.push(str);
        }

        let mut global = ArgMatches::new();
        let mut command_match: Option<CommandMatches> = None;
        let mut iter = input.iter().peekable();
        while let Some(&arg) = iter.next() {
            if self.handle_argument(arg, &mut global, &mut iter).is_err() {
                if let Some(command) = self.commands.iter().find(|c| c.name == arg) {
                    command_match = Some(command.parse(&mut iter)?);
                    break;
                } else {
                    return Err(format!("Unknown command or argument: {}", arg));
                }
            }
        }

        let required_args = self
            .args
            .iter()
            .filter(|a| a.required)
            .map(|a| a.name.clone())
            .collect::<Vec<String>>();
        for arg in required_args {
            if global.get_arg(&arg).is_none() {
                return Err(format!(
                    "Missing required argument: {}, use --help for usage information",
                    arg
                ));
            }
        }

        Ok(CliMatches {
            command: command_match,
            global,
        })
    }

    fn handle_argument(
        &self,
        arg: &str,
        global: &mut ArgMatches,
        iter: &mut core::iter::Peekable<core::slice::Iter<'_, &str>>,
    ) -> Result<(), ()> {
        let (name, value) = if arg.starts_with("--") {
            let name = arg.trim_start_matches("--");
            if let Some(definition) = self.args.iter().find(|a| a.long.as_deref() == Some(name)) {
                if definition.value_type == ArgValueT::Bool {
                    (definition.name.clone(), ArgValue::Bool(true))
                } else if let Some(next) = iter.next() {
                    if !next.starts_with('-') {
                        (definition.name.clone(), ArgValue::String(next.to_string()))
                    } else {
                        return Err(());
                    }
                } else {
                    return Err(());
                }
            } else {
                return Err(());
            }
        } else if arg.starts_with('-') {
            let name = arg.trim_start_matches("-");
            if let Some(definition) = self
                .args
                .iter()
                .find(|a| a.short.is_some_and(|s| s.to_string() == name))
            {
                if definition.value_type == ArgValueT::Bool {
                    (definition.name.clone(), ArgValue::Bool(true))
                } else if let Some(next) = iter.next() {
                    if !next.starts_with('-') {
                        (definition.name.clone(), ArgValue::String(next.to_string()))
                    } else {
                        return Err(());
                    }
                } else {
                    return Err(());
                }
            } else {
                return Err(());
            }
        } else {
            return Err(());
        };

        global.add_arg(&name, value);
        match name.as_str() {
            "help" => {
                self.print_help();
                crate::process::exit(0);
            }
            "version" => {
                self.print_version();
                crate::process::exit(0);
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Command {
    name: &'static str,
    about: Option<&'static str>,
    args: Vec<ArgDefinition>,
}

impl Command {
    pub fn new(name: &'static str) -> Self {
        let mut command = Self {
            name,
            about: None,
            args: Vec::new(),
        };
        command.args.push(
            ArgDefinition::new("help")
                .short('h')
                .long("help")
                .help("Show help for this command"),
        );
        command
    }

    pub fn about(mut self, about: &'static str) -> Self {
        self.about = Some(about);
        self
    }

    pub fn arg(mut self, definition: ArgDefinition) -> Self {
        self.args.push(definition);
        self
    }

    pub fn parse(
        &self,
        iter: &mut core::iter::Peekable<core::slice::Iter<'_, &str>>,
    ) -> Result<CommandMatches, String> {
        let mut matches = ArgMatches::new();

        while let Some(&arg) = iter.next() {
            let (name, value) = if arg.starts_with("--") {
                (
                    arg.trim_start_matches("--"),
                    if let Some(next) = iter.peek() {
                        if !next.starts_with('-') {
                            ArgValue::String(iter.next().unwrap().to_string())
                        } else {
                            ArgValue::Bool(true)
                        }
                    } else {
                        ArgValue::Bool(true)
                    },
                )
            } else if arg.starts_with('-') {
                (
                    arg.trim_start_matches('-'),
                    if let Some(next) = iter.peek() {
                        if !next.starts_with('-') {
                            ArgValue::String(iter.next().unwrap().to_string())
                        } else {
                            ArgValue::Bool(true)
                        }
                    } else {
                        ArgValue::Bool(true)
                    },
                )
            } else {
                return Err(format!("Unknown argument: {}", arg));
            };

            if let Some(definition) = self.args.iter().find(|a| {
                a.long.as_deref() == Some(name) || a.short.is_some_and(|s| s.to_string() == name)
            }) {
                if !value.matches_type(&definition.value_type) {
                    return Err(format!("Invalid value type for argument: {}", arg));
                }
                matches.add_arg(&definition.name, value);
                if definition.name == "help" {
                    self.print_help();
                    crate::process::exit(0);
                }
            } else {
                return Err(format!("Unknown argument: {}", arg));
            }
        }
        let required_args = self
            .args
            .iter()
            .filter(|a| a.required)
            .map(|a| a.name.clone())
            .collect::<Vec<String>>();
        for arg in required_args {
            if matches.get_arg(&arg).is_none() {
                return Err(format!("Missing required argument: {}, for command: {}, use --help for usage information", arg, self.name));
            }
        }

        Ok(CommandMatches {
            name: self.name.to_string(),
            args: matches,
        })
    }

    fn print_help(&self) {
        println!(
            "Command: {}

Options:",
            self.name
        );
        for arg in &self.args {
            println!(
                "  {} {} - {}",
                arg.short.map_or(String::new(), |s| format!("-{}", s)),
                arg.long
                    .clone()
                    .map_or(String::new(), |l| format!("--{}", l)),
                arg.help.clone().unwrap_or_default()
            );
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ArgValueT {
    Bool,
    String,
}

#[derive(Debug)]
pub struct ArgDefinition {
    required: bool,
    name: String,
    short: Option<char>,
    long: Option<String>,
    help: Option<String>,
    value_type: ArgValueT,
}

impl ArgDefinition {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            short: None,
            long: None,
            help: None,
            required: false,
            value_type: ArgValueT::Bool,
        }
    }

    pub fn short(mut self, short: char) -> Self {
        self.short = Some(short);
        self
    }

    pub fn long(mut self, long: &str) -> Self {
        self.long = Some(long.to_string());
        self
    }

    pub fn help(mut self, help: &str) -> Self {
        self.help = Some(help.to_string());
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn value_type(mut self, value_type: ArgValueT) -> Self {
        self.value_type = value_type;
        self
    }
}
