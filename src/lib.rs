#[macro_use]
extern crate getset;
#[macro_use]
extern crate tracing;

use crate::error::Error;

pub mod config;
pub mod converter;
pub mod error;
pub mod server;

pub type Result<T> = anyhow::Result<T, Error>;
