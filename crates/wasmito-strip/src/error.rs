#[derive(Debug, thiserror::Error, Clone)]
pub enum Error {
    #[error("Regex Creation Failed: {0}")]
    RegexFailed(String),
    #[error("Parse Payload Read: {0}")]
    ParsePayloadRead(String),
}
