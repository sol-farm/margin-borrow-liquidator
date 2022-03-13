#![allow(clippy::needless_lifetimes)]
#![allow(clippy::bool_assert_comparison)]
#![cfg(not(tarpaulin_include))]

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
use anyhow::{anyhow, Result};

use clap::{App, Arg, SubCommand};
use git_version::git_version;
use log::error;
use pprof::*;
mod config;
mod helpers;
mod database;
mod analytics;
mod liq_bot;
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

    let ltv_filter_mode = Arg::with_name("ltv-filter-mode")
    .short("lfm")
    .help("the filter mode to use: ge, le, gt, lt")
    .long_help("specifies whether or not to get greater/less than or equal to, greater than, or less than filtering of obligations based on their ltv")
    .required(true)
    .takes_value(true)
    .value_name("MODE");

    let ltv_filter_value = Arg::with_name("ltv-filter-value")
    .short("lfv")
    .help("the ltv value to use for filtering, where 1.0 is 100% and 0.6 is 60%")
    .required(true)
    .takes_value(true)
    .value_name("LTV");


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
                .takes_value(true),
        )
        .arg(
            Arg::with_name("pprof")
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
                .required(false),
        )
        .subcommand(
            SubCommand::with_name("config")
                .about("configuration management commands")
                .subcommands(vec![
                SubCommand::with_name("new")
                .about("generates a new and empty configuration file"),
                SubCommand::with_name("sanitize")
                .about("sanitize configuration to make suitable for public storage"),
                SubCommand::with_name("export-as-json")
                .about("exports the yaml config file into a json file"),
                SubCommand::with_name("interest-rate")
                .about("interest rate scraper configuration management")
            ]),
        )
        .subcommand(
            SubCommand::with_name("analytics")
            .about("analytics management commands")
            .subcommands(vec![
                SubCommand::with_name("start-scraper-service")
                .about("starts the scraper service, which stores price feed and obligation information into the database")
            ])
        )
        .subcommand(
            SubCommand::with_name("database")
            .about("database management commands")
            .subcommands(vec![
                SubCommand::with_name("migrate")
                .about("run database migrations")
            ])
        )
        .subcommand(
            SubCommand::with_name("liquidator")
            .about("liquidator bot management commands")
            .subcommands(vec![
                SubCommand::with_name("start-simple")
                .about("starts the simple liquidator bot")
                .arg(ltv_filter_mode)
                .arg(ltv_filter_value)
            ])
        )
        .get_matches();
    let config_file_path = get_config_or_default(&matches);
    if matches.is_present("pprof") {
        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(i32::from_str(
                matches.value_of("pprof-frequency").unwrap_or("100"),
            )?)
            .blocklist(&["libc", "libgcc", "pthread"])
            .build()
            .unwrap();
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
        ("analytics", Some(analytics_command)) => match analytics_command.subcommand() {
            ("start-scraper-service", Some(start_scraper)) => {
                analytics::start_scraper_service(start_scraper, config_file_path)
            }
            _ => invalid_subcommand("analytics"),
        }
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
        ("database", Some(database_command)) => match database_command.subcommand() {
            ("migrate", Some(_)) => {
                database::run_database_migrations(config_file_path)
            }
            _ => invalid_subcommand("database")
        }
        ("liquidator", Some(liquidator_command)) => match liquidator_command.subcommand() {
            ("start-simple", Some(start)) => {
                liq_bot::start_simple(start, config_file_path)
            }
            _ => invalid_subcommand("liquidator"),
        }
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
