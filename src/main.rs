mod account;
mod context;
mod crypto;
mod database;
mod initialization;
mod keys;
mod messages;
mod notifications;
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
use opentelemetry::global::{self};
use telemetry::metrics_registry::{init_metrics_registry, get_metrics};
use prism_telemetry::telemetry::{init_telemetry, build_resource, set_global_attributes};
use prism_telemetry::logs::setup_log_subscriber;

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

    let mut attributes: Vec<(String, String)> = Vec::new();
    attributes.extend(telemetry_config.global_labels.labels.clone().into_iter().map(|(k, v)| (k, v)));
    attributes.push(("prism_host".to_string(), settings.prism.host.to_string() + ":" + &settings.prism.port.to_string()));

    set_global_attributes(attributes.clone());

    let resource = build_resource("prism-messenger-server".to_string(), attributes);

    let (meter_provider, log_provider) = init_telemetry(&telemetry_config, resource).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    if let Some(ref provider) = meter_provider {
        global::set_meter_provider(provider.clone());

        // Initialize the metrics registry after setting the global meter provider
        init_metrics_registry();
    }

    if let Some(ref provider) = log_provider {
        // Initialize tracing subscriber
        setup_log_subscriber(
            telemetry_config.logs.enabled,
            Some(provider)
        );
    }

    if let Some(metrics) = get_metrics() {
        metrics.record_node_info(
            vec![
                ("version".to_string(), env!("CARGO_PKG_VERSION").to_string()),
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
            error!("Error occurred while running prover: {:?}", e);
        }
    });

    tokio::select! {
        _ = webserver_task_handle => {
            info!("Webserver task completed")
        }
    }

    Ok(())
}
