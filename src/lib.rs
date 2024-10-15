pub mod message;
pub mod reader;
pub mod templates;

use thiserror::Error;

use message::*;
pub use reader::*;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO: {0}")]
    IO(#[from] std::io::Error),
    #[error("Invalid format: {0}")]
    InvalidData(String),
    #[error("Unsupported: {0}")]
    UnsupportedData(String),
}

pub type Result<T> = std::result::Result<T, Error>;
