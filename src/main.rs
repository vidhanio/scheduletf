use std::env;

use matchbox::Config;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    _ = dotenvy::dotenv();

    let config = Config::from_env()?;

    let fmt = tracing_subscriber::fmt().with_env_filter(
        env::var(EnvFilter::DEFAULT_ENV)
            .as_deref()
            .unwrap_or("warn,matchbox=trace,sea_orm=debug"),
    );

    if config.production {
        fmt.compact().init();
    } else {
        fmt.pretty().init();
    }

    matchbox::run(config).await?;

    Ok(())
}
