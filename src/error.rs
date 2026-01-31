#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub message: String,
    pub position: usize,
}

impl Error {
    pub fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }
}
