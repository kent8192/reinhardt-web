//! Raw body extraction

use async_trait::async_trait;
use reinhardt_apps::Request;

use crate::{ParamContext, ParamError, ParamResult, extract::FromRequest};

/// Extract the raw request body as bytes
///
/// # Examples
///
/// ```
/// use reinhardt_params::Body;
///
/// let body = Body(vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]); // "Hello"
/// assert_eq!(body.0, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
/// ```
pub struct Body(pub Vec<u8>);

#[async_trait]
impl FromRequest for Body {
    async fn from_request(req: &Request, _ctx: &ParamContext) -> ParamResult<Self> {
        // Extract raw body bytes from request
        let body_bytes = req
            .read_body()
            .map_err(|e| ParamError::BodyError(format!("Failed to read body: {}", e)))?;
        Ok(Body(body_bytes.to_vec()))
    }
}
