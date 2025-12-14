use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnrError {
    #[error("RLP encoding error")]
    RlpError,

    #[error("invalid ENR signature")]
    InvalidSignature,

    #[error("missing required field: {0}")]
    MissingField(&'static str),
}