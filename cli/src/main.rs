#![allow(clippy::needless_lifetimes)]
#![allow(clippy::bool_assert_comparison)]
#![cfg(not(tarpaulin_include))]

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
use anyhow::{anyhow, Result};

use log::error;
use pprof::*;
use clap::{App, Arg, SubCommand};
use git_version::git_version;
mod config;
mod helpers;
use std::str::FromStr;

const GIT_VERSION: &str = git_version!(args = ["--abbrev=32", "--always"]);

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<()> {
    use std::fs::File;


    let skip_preflight_flag = Arg::with_name("skip-preflight")
        .long("skip-preflight")
        .help("if present skip preflight checks")
        .required(false);


    let matches = App::new("tulip-cli")
    .version("0.0.1")
    .author("TULIP Protocol Developers <contact@tulip.garden>")
    .long_version(format!("cli_git_ver {}", GIT_VERSION).as_str())
    .arg(
        Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("sets the config file")
            .takes_value(true)
    )
    .arg(    Arg::with_name("pprof")
            .help("enable profiling of performance")
            .long("pprof")
            .takes_value(false)
            .required(false),
    )
    .arg(
        Arg::with_name("pprof-path")
        .help("location to store pprof reports")
        .long("pprof-path")
        .takes_value(true)
        .value_name("FILEPATH")
        .required(false),
    )
    .arg(
        Arg::with_name("pprof-frequency")
        .help("frequency of collecting samples")
        .long("pprof-frequency")
        .takes_value(true)
        .value_name("FREQUENCY")
        .required(false)
    )
    .subcommand(
        SubCommand::with_name("config")
        .about("configuration management commands")
        .subcommands(
            vec![
                SubCommand::with_name("new")
                .about("generates a new and empty configuration file")
                .arg(
                    Arg::with_name("profile")
                    .short("p")
                    .long("profile")
                    .help("configuration profile to use as a template, defaults to devnet")
                    .value_name("PROFILE")
                    .required(false)
                ),
                SubCommand::with_name("sanitize")
                .about("sanitize configuration to make suitable for public storage"),
                SubCommand::with_name("export-as-json")
                .about("exports the yaml config file into a json file"),
                SubCommand::with_name("interest-rate")
                .about("interest rate scraper configuration management")
            ]
        )
    )
    .get_matches();
    let config_file_path = get_config_or_default(&matches);
    if matches.is_present("pprof") {
        let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(
            i32::from_str(matches.value_of("pprof-frequency").unwrap_or("100"))?,
        )
        .blocklist(&["libc", "libgcc", "pthread"]).build().unwrap();
        // need to call this here to prevent `guard` from being dropped
        match process_matches(&matches, config_file_path).await {
            Ok(_) => (),
            Err(err) => error!("encountered error {:#?}", err),
        }
        if let Ok(report) = guard.report().build() {
            let file_path = matches.value_of("pprof-path").unwrap_or("flamegraph.svg");
            let file = File::create(file_path).unwrap();
            let mut options = pprof::flamegraph::Options::default();
            options.pretty_xml = true;
            report.flamegraph_with_options(file, &mut options).unwrap();
        };
    } else {
        process_matches(&matches, config_file_path).await?;
    }
    
    Ok(())
}

#[cfg(not(tarpaulin_include))]
// returns the value of the config file argument or the default
fn get_config_or_default(matches: &clap::ArgMatches) -> String {
    matches
        .value_of("config")
        .unwrap_or("config.yaml")
        .to_string()
}

#[cfg(not(tarpaulin_include))]
async fn process_matches<'a>(
    matches: &clap::ArgMatches<'a>,
    config_file_path: String,
) -> Result<()> {
    match matches.subcommand() {
        ("config", Some(config_command)) => match config_command.subcommand() {
            ("new", Some(new_config)) => config::new_config(new_config, config_file_path),
            ("sanitize", Some(sanitize_config)) => {
                config::sanitize(sanitize_config, config_file_path)
            }
            ("export-as-json", Some(export_as_json)) => {
                config::export_as_json(export_as_json, config_file_path)
            }
            _ => invalid_subcommand("config"),
        },
        _ => invalid_command(),
    }
}

#[cfg(not(tarpaulin_include))]
fn invalid_subcommand(command_group: &str) -> Result<()> {
    Err(anyhow!(
        "invalid command found for group {}, run --help for more information",
        command_group
    ))
}

#[cfg(not(tarpaulin_include))]
fn invalid_command() -> Result<()> {
    Err(anyhow!(
        "invalid command found, run --help for more information"
    ))
}
