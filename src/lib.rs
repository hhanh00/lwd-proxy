#[path = "generated/cash.z.wallet.sdk.rpc.rs"]
pub mod lw_rpc;

use thiserror::Error;

pub const NETWORK: Network = Network::MainNetwork;

mod config;
mod server;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Tonic(#[from] tonic::Status),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

use zcash_primitives::consensus::Network;
pub use server::{connect_lightwalletd, launch};
pub use config::Config;
