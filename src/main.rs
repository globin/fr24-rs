pub mod flight_history;
pub mod login;

use flight_history::FlightNoMap;
use login::login;

use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::signal::unix::{signal, SignalKind};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::flight_history::{consolidate_flight_info, history_by_flight_number};

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    mail: String,
    password: String,
}

#[derive(Parser, Clone, Debug)]
struct Config {
    #[clap(short, long, env)]
    mail: String,
    #[clap(short, long, env)]
    password: String,
    #[clap(short, long, env, hide_env = true)]
    flight_number: String,
}

async fn term_signal() -> Result<&'static str> {
    let mut sighup = signal(SignalKind::hangup())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigquit = signal(SignalKind::quit())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    let signal = tokio::select! {
        _ = sighup.recv() => { "SIGHUP" }
        _ = sigint.recv() => { "SIGINT" }
        _ = sigquit.recv() => { "SIGQUIT" }
        _ = sigterm.recv() => { "SIGTERM" }
    };

    Ok(signal)
}

fn init_tracing() -> Result<()> {
    let tracing_subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .finish();

    tracing::subscriber::set_global_default(tracing_subscriber)
        .with_context(|| format!("failed to set global default tracing subscriber"))?;

    Ok(())
}

async fn flight_info(config: &Config, flight_no: &str) -> Result<FlightNoMap> {
    let user = User {
        mail: config.mail.clone(),
        password: config.password.clone(),
    };
    let token = login(&user).await?;
    let flights = history_by_flight_number(&token, flight_no).await?;
    Ok(consolidate_flight_info(flights))
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    let config = Config::parse();

    let sigwait = tokio::spawn(async move { term_signal().await });

    let data_fetch_task = tokio::spawn(async move { flight_info(&config, &config.flight_number).await });

    tokio::select! {
            res = sigwait => {
                match res {
                    Ok(Ok(sig)) => info!("received signal {sig}"),
                    e => error!("{e:?}")
                }
                info!("terminating");
            }
            res = data_fetch_task => {
                match res {
                    Ok(Ok(f)) => {
                        let json = serde_json::to_string(&f).unwrap();
                        println!("{json}")
                    },
                    _ => info!("data fetching unsuccessful: {res:?}"),
                }
                
            }
        }

    Ok(())
}
