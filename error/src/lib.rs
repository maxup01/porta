use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    InvalidData(&'static str),
}
