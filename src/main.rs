mod account;
mod context;
mod crypto;
mod database;
mod initialization;
mod keys;
mod messages;
mod notifications;
mod profiles;
mod registration;
mod settings;
mod webserver;
mod telemetry;

use anyhow::Result;
use clap::Parser;
use context::AppContext;
use settings::Settings;
use std::error::Error;
use std::path::PathBuf;
use tokio::spawn;
use tracing::{debug, info, error};
use prism_telemetry::telemetry::shutdown_telemetry;
use crate::telemetry::metrics_registry::get_metrics;
use crate::telemetry::init::init;

pub static PRISM_MESSENGER_SERVICE_ID: &str = "prism_messenger";

/// Command line arguments for the Prism Messenger Server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the settings file
    #[arg(short, long)]
    settings: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let cli = Cli::parse();

    // Load settings with optional custom config path
    let settings = match cli.settings {
        Some(path) => Settings::load_from_path(&path)?,
        None => Settings::load()?,
    };

    let telemetry_config = match settings.telemetry.clone() {
        Some(cfg) => cfg,
        None => {
            return Err(anyhow::anyhow!("Telemetry configuration is missing").into());
        }
    };

    // Initialize telemetry
    let attributes: Vec<(String, String)> = vec![
        ("labvel".to_string(), "value".to_string()),
    ];
    let (meter_provider, log_provider) = init(
        telemetry_config.clone(),
        attributes,
    )?;

    if let Some(metrics) = get_metrics() {
        metrics.record_node_info(
            vec![
                ("version".to_string(), env!("CARGO_PKG_VERSION").to_string()),
                ("prism_host".to_string(), settings.prism.host.to_string() + ":" + &settings.prism.port.to_string()),
            ]
        );
    }

    let context = AppContext::from_settings(&settings).await?;

    context
        .initialization_service
        .initialize_messenger_server()
        .await?;

    let webserver_task_handle = spawn(async move {
        debug!("starting webserver");
        if let Err(e) = webserver::start(&settings.webserver, context).await {
            error!("Error occurred while running webserver: {:?}", e);
        }
    });

    tokio::select! {
        _ = webserver_task_handle => {
            info!("Webserver task completed")
        }
    }

    shutdown_telemetry(telemetry_config, meter_provider, log_provider);

    Ok(())
}
