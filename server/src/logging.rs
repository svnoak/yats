use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Sets up the global tracing subscriber based on the environment.
///
/// In development (when `APP_ENV` is not `production`), it uses a human-readable format.
/// In production, it uses a JSON format for structured logging.
/// Log levels are controlled by the `RUST_LOG` environment variable.
pub fn setup_tracing() {
    let is_production = std::env::var("APP_ENV").as_deref() == Ok("production");

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if is_production {
            "info".into()
        } else {
            "rust_tunnel_server=debug,tower_http=debug".into()
        }
    });

    let subscriber = tracing_subscriber::registry().with(filter);

    if is_production {
        subscriber.with(fmt::layer().json()).init();
    } else {
        subscriber.with(fmt::layer()).init();
    };
}
