#[macro_use]
extern crate tracing;

use crate::error::Error;

pub mod config;
pub mod error;

pub type Result<T> = anyhow::Result<T, Error>;
