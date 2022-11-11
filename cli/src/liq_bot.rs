use super::*;
use crate::helpers::get_config;
use signal_hook::{
    consts::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};

use channels::broadcast::UnboundedBroadcast;

use std::sync::Arc;
use tokio::{
    signal::unix::{signal, SignalKind},
    task,
};

pub async fn start_simple(matches: &clap::ArgMatches<'_>, config_file_path: String) -> Result<()> {
    let cfg = get_config(&config_file_path)?;
    let simple_liquidator = liquidator::simple::SimpleLiquidator::new(cfg)?;
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
    simple_liquidator.start(receiver).await
}

pub async fn start_refresher(
    _matches: &clap::ArgMatches<'_>,
    config_file_path: String,
) -> Result<()> {
    let cfg = get_config(&config_file_path)?;
    let simple_liquidator = liquidator::refresher::Refresher::new(cfg)?;

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

    simple_liquidator.start(receiver).await
}
