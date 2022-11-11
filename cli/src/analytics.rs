extern crate hyper_native_tls;

use anyhow::{anyhow, Result};

use channels::broadcast::UnboundedBroadcast;
use config::Configuration;

use log::error;

use signal_hook::{
    consts::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};
use std::sync::Arc;
use tokio::{
    signal::unix::{signal, SignalKind},
    task,
};

pub async fn start_scraper_service(
    _matches: &clap::ArgMatches<'_>,
    config_file_path: String,
) -> Result<()> {
    let config = Configuration::load(config_file_path.as_str(), false, true)
        .expect("failed to load config file");
    let (sender, receiver) = tokio::sync::oneshot::channel();
    {
        let mut sig_int = signal(SignalKind::interrupt())?;
        let mut sig_term = signal(SignalKind::terminate())?;

        tokio::task::spawn(async move {
            tokio::select! {
                _ = sig_int.recv() => {
                    log::warn!("receive interrupt");
                }
                _ = sig_term.recv() => {
                    log::warn!("received terminate");
                }
                _ = tokio::signal::ctrl_c()  => {
                    log::warn!("received CTRL+C");
                }
            }
            if let Err(err) = sender.send(true) {
                log::error!("failed to send exit notification {:#?}", err);
            }
            log::info!("sent exit notification");
        });
    }
    let service = match analytics::Service::new(config) {
        Err(err) => {
            return Err(anyhow!("failed to init analytics service {:#?}", err));
        }
        Ok(service) => service,
    };
    if let Err(err) = service.start(receiver).await {
        error!("scraper service encountered error {:#?}", err);
        return Err(anyhow!("{:#?}", err));
    }
    Ok(())
}
