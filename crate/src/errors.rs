
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WasmInterfaceError {
    #[error("The field `{0}` doesn't exist in schema")]
    InvalidField(String),
    
    #[error("The directory is empty")]
    EmptyDirectory,

    #[error("Failed to serialize directory")]
    FailedToSerializeDirectory,
    #[error("Failed to create archive root")]
    FailedToCreateArchiveRoot,
    #[error("Failed to deserialize directory")]
    FailedToDeSerializeDirectory,
   
}