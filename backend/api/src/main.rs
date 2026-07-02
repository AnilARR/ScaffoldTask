use scaffold_api::{build_app, config::Settings, router::build_router};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let settings = Settings::from_env();
    tracing::info!(?settings.bind_addr, use_mocks = settings.use_mocks, "starting scaffold-api");

    let state = build_app(&settings).await?;
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(&settings.bind_addr).await?;
    tracing::info!("listening on {}", settings.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
