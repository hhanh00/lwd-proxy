use figment::Figment;
use figment::providers::{Env, Format, Toml};
use lwd_proxy::{Config, launch};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let config: Config = Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::prefixed("LWD")).extract()?;

    launch(config).await?;

    Ok(())
}

