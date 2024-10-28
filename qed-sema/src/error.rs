use thiserror::Error;

#[derive(Error, Clone, Debug)]
pub enum Error {
    #[error("infinite loop disallowed")]
    InfiniteLoop,
}

pub type Result<T> = std::result::Result<T, Error>;
