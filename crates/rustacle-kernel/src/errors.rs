#[derive(thiserror::Error, Debug)]
pub enum KernelError {
    #[error("lifecycle: {0}")]
    Lifecycle(String),

    #[error("internal: {0}")]
    Internal(String),
}
