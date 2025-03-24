use std::env;

use scheduletf::Config;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    _ = dotenvy::dotenv();

    let config = Config::from_env()?;

    let fmt = tracing_subscriber::fmt().with_env_filter(
        env::var(EnvFilter::DEFAULT_ENV)
            .as_deref()
            .unwrap_or("warn,scheduletf=trace,sea_orm=debug"),
    );

    if config.production {
        fmt.compact().init();
    } else {
        fmt.pretty().init();
    }

    scheduletf::run(config).await?;

    Ok(())
}
