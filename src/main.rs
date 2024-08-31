use std::env;

use tf2_team_manager::Config;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    _ = dotenvy::dotenv();

    let config = Config::from_env()?;

    let fmt = tracing_subscriber::fmt().with_env_filter(
        env::var(EnvFilter::DEFAULT_ENV)
            .as_deref()
            .unwrap_or("warn,tf2_team_manager=trace"),
    );

    if config.production {
        fmt.compact().init();
    } else {
        fmt.pretty().init();
    }

    tf2_team_manager::run(config).await?;

    Ok(())
}
