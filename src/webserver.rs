use anyhow::{Context, Result};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use utoipa::{
    OpenApi,
    openapi::{Info, OpenApiBuilder},
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    account, keys, messages, presence, profiles, registration, settings::WebserverSettings,
    startup::AppContext, websocket,
};

#[derive(OpenApi)]
struct ApiDoc;

pub async fn start(settings: &WebserverSettings, context: AppContext) -> Result<()> {
    let context_arc = Arc::new(context);
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/accounts", account::router(context_arc.clone()))
        .nest("/keys", keys::router(context_arc.clone()))
        .nest("/messages", messages::router(context_arc.clone()))
        .nest("/presence", presence::router(context_arc.clone()))
        .nest("/profile", profiles::router(context_arc.clone()))
        .nest("/registration", registration::router())
        .nest("/ws", websocket::router(context_arc.clone()))
        .with_state(context_arc)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http().on_failure(()))
        .split_for_parts();

    let api = OpenApiBuilder::from(api)
        .info(Info::new("Prism Messenger Server API", "0.1.0"))
        .build();

    let router = router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api));

    let addr = SocketAddr::new(
        settings.host.parse().expect("IP address can be parsed"),
        settings.port,
    );
    let listener = TcpListener::bind(addr)
        .await
        .expect("Binding to address works");
    let server = axum::serve(listener, router.into_make_service());

    let socket_addr = server.local_addr()?;
    info!(
        "Starting webserver on {}:{}",
        settings.host,
        socket_addr.port()
    );

    server.await.context("Server error")?;

    Ok(())
}
