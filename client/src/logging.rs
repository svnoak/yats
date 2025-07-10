use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_subscriber() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tunnel_client=debug,reqwest=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
