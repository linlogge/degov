use thiserror::Error;

#[derive(Error, Debug)]
pub enum FrontdoorError {
    #[error("Missing listen address")]
    MissingListenAddress,
}

pub type Result<T> = std::result::Result<T, FrontdoorError>;
