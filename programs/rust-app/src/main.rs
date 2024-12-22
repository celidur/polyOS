#![no_main]
#![no_std]

use polyos_std::cli::{ArgDefinition, ArgValueT, Cli, Command};
use polyos_std::*;

#[polyos_std::main]
fn main() {
    let cli = Cli::new("MyApp")
        .version("1.0")
        .about("A configurable CLI application")
        .arg(
            ArgDefinition::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output"),
        )
        .command(
            Command::new("run")
                .about("Run the application")
                .arg(
                    ArgDefinition::new("verbose")
                        .short('v')
                        .long("verbose")
                        .help("Enable verbose output"),
                )
                .arg(
                    ArgDefinition::new("config")
                        .long("config")
                        .help("Specify the configuration file")
                        .value_type(ArgValueT::String)
                        .required(),
                ),
        )
        .command(
            Command::new("build").about("Build the application").arg(
                ArgDefinition::new("release")
                    .short('r')
                    .long("release")
                    .help("Build in release mode"),
            ),
        );

    match cli.get_matches() {
        Ok(matches) => {
            let verbose = matches.get_bool("verbose").unwrap_or(false);
            serial_println!("Verbose: {}", verbose);
            match matches.get_command() {
                Some(cmd) => match cmd.name.as_str() {
                    "run" => {
                        let config = cmd
                            .get_string("config")
                            .unwrap_or("config.json".to_string());
                        serial_println!("Running with config: {}", config);
                        let verbose = cmd.get_bool("verbose").unwrap_or(false);
                        serial_println!("Verbose: {}", verbose);
                    }
                    "build" => {
                        let release = cmd.get_bool("release").unwrap_or(false);
                        serial_println!("Building in release mode: {}", release);
                    }
                    _ => {
                        serial_println!("Unknown command: {}", cmd.name);
                    }
                },
                None => {
                    serial_println!("No command specified");
                }
            }
        }
        Err(err) => {
            println!("Error: {}", err);
        }
    }
}
