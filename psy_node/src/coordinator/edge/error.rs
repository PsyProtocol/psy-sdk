use jsonrpsee::types::{ErrorObject, ErrorObjectOwned};

pub struct RpcError;

impl RpcError {
    pub fn Anyhow(err: anyhow::Error) -> ErrorObjectOwned {
        ErrorObjectOwned::owned(
            1,
            err.to_string(),
            None::<()>,
        )
    }

    pub fn NotFound(msg: String) -> ErrorObjectOwned {
        ErrorObjectOwned::owned(
            -32001,
            msg,
            None::<()>,
        )
    }
}
