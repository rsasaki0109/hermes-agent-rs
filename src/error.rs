#[derive(thiserror::Error, Debug)]
pub enum LlmError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("api error (status {status}): {body}")]
    Api { status: u16, body: String },
}
