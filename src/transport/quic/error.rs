use thiserror::Error;

#[derive(Debug, Error)]
pub enum QuicError {
    #[error("QUIC connection failed")]
    ConnectionFailed,

    #[error("stream error")]
    StreamError,
}