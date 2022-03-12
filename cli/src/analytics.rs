extern crate hyper_native_tls;


use anyhow::{anyhow, Result};
use axum::body::Body;
use axum::http::{self};
use channels::broadcast::UnboundedBroadcast; 
use config::Configuration;
use crossbeam::sync::WaitGroup;

use hyper::Request;

use log::{error, info};
use serde_json::Value;
use signal_hook::{
    consts::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};
use std::sync::Arc;
use tokio::task;

pub fn start_scraper_service(_matches: &clap::ArgMatches, config_file_path: String) -> Result<()> {
    let config = Arc::new(
        Configuration::load(config_file_path.as_str(), false, true)
            .expect("failed to load config file"),
    );
    let mut broadcaster: UnboundedBroadcast<bool> = UnboundedBroadcast::new();
    let receiver = broadcaster.subscribe();
    let mut signals =
        Signals::new(vec![SIGINT, SIGTERM, SIGQUIT]).expect("failed to registers signals");
    {
        task::spawn_blocking(move || {
            if let Some(sig) = signals.forever().next() {
                error!("caught signal {:#?}", sig);
            }
            if let Err(err) = broadcaster.send(true) {
                error!("failed to send exit signal: {:#?}", err);
            }
        });
    }
    let service = match analytics::Service::new(Arc::clone(&config)) {
        Err(err) => {
            return Err(anyhow!("failed to init analytics service {:#?}", err));
        }
        Ok(service) => service,
    };
    if let Err(err) = service.start(receiver) {
        error!("scraper service encountered error {:#?}", err);
        return Err(anyhow!("{:#?}", err));
    }
    Ok(())
}