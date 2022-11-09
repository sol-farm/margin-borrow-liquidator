use super::*;
use crate::helpers::get_config;
use signal_hook::{
    consts::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};

use channels::broadcast::UnboundedBroadcast;

use std::sync::Arc;

pub fn start_simple(matches: &clap::ArgMatches, config_file_path: String) -> Result<()> {
    let cfg = Arc::new(get_config(&config_file_path)?);
    let ltv_filter_mode = matches.value_of("ltv-filter-mode").unwrap();
    let ltv_filter_value = matches.value_of("ltv-filter-value").unwrap();
    let ltv_filter = db::filters::LtvFilter::from_str(ltv_filter_mode, ltv_filter_value)?;
    let simple_liquidator = liquidator::simple::SimpleLiquidator::new(cfg)?;

    let mut broadcaster: UnboundedBroadcast<bool> = channels::broadcast::UnboundedBroadcast::new();
    let subscriber = broadcaster.subscribe();
    let mut signals =
        Signals::new(vec![SIGINT, SIGTERM, SIGQUIT]).expect("failed to registers signals");
    {
        tokio::task::spawn_blocking(move || {
            if let Some(sig) = signals.forever().next() {
                error!("caught signal {:#?}", sig);
            }
            if let Err(err) = broadcaster.send(true) {
                error!("failed to send exit signal: {:#?}", err);
            }
        });
    }

    simple_liquidator.start(ltv_filter, subscriber)
}

pub fn start_refresher(_matches: &clap::ArgMatches, config_file_path: String) -> Result<()> {
    let cfg = Arc::new(get_config(&config_file_path)?);
    let simple_liquidator = liquidator::refresher::Refresher::new(cfg)?;

    let mut broadcaster: UnboundedBroadcast<bool> = channels::broadcast::UnboundedBroadcast::new();
    let subscriber = broadcaster.subscribe();
    let mut signals =
        Signals::new(vec![SIGINT, SIGTERM, SIGQUIT]).expect("failed to registers signals");
    {
        tokio::task::spawn_blocking(move || {
            if let Some(sig) = signals.forever().next() {
                error!("caught signal {:#?}", sig);
            }
            if let Err(err) = broadcaster.send(true) {
                error!("failed to send exit signal: {:#?}", err);
            }
        });
    }

    simple_liquidator.start(subscriber)
}
