//! Form data extraction

use async_trait::async_trait;
use reinhardt_apps::Request;
use serde::de::DeserializeOwned;
use std::fmt::{self, Debug};
use std::ops::Deref;

use crate::{ParamContext, ParamError, ParamResult, extract::FromRequest};

#[cfg(feature = "multipart")]
use futures_util::{future::ready, stream::once};
#[cfg(feature = "multipart")]
use serde_json::Value;

/// Extract form data from request body
pub struct Form<T>(pub T);

impl<T> Form<T> {
    /// Unwrap the Form and return the inner value
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_params::Form;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct LoginForm {
    ///     username: String,
    ///     password: String,
    /// }
    ///
    /// let form = Form(LoginForm {
    ///     username: "alice".to_string(),
    ///     password: "secret123".to_string(),
    /// });
    /// let inner = form.into_inner();
    /// assert_eq!(inner.username, "alice");
    /// assert_eq!(inner.password, "secret123");
    /// ```
    pub fn into_inner(self) -> T {
        self.0
    }

    /// Parse multipart/form-data from request
    ///
    /// This method handles `multipart/form-data` content type, which is commonly
    /// used for file uploads. Only text fields are extracted; file fields are ignored.
    ///
    /// Note: This is an internal method. Use `Form<T>` with `FromRequest` trait instead.
    #[cfg(feature = "multipart")]
    async fn from_multipart_internal(req: &Request) -> ParamResult<Form<T>>
    where
        T: DeserializeOwned,
    {
        // Extract boundary from Content-Type header
        let content_type = req
            .headers
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ParamError::InvalidParameter {
                name: "content-type".to_string(),
                message: "Missing Content-Type header".to_string(),
            })?;

        // Parse boundary
        let boundary =
            multer::parse_boundary(content_type).map_err(|e| ParamError::InvalidParameter {
                name: "content-type".to_string(),
                message: format!("Failed to parse boundary: {}", e),
            })?;

        // Read body
        let body = req
            .read_body()
            .map_err(|e| ParamError::BodyError(format!("Failed to read body: {}", e)))?;

        // Convert Bytes to Stream
        let stream = once(ready(Ok::<_, std::io::Error>(body)));

        // Create multipart parser
        let mut multipart = multer::Multipart::new(stream, boundary);

        // Extract text fields into a map
        let mut fields = serde_json::Map::new();

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| ParamError::BodyError(format!("Failed to read multipart field: {}", e)))?
        {
            let name = field
                .name()
                .ok_or_else(|| ParamError::BodyError("Field name missing".to_string()))?
                .to_string();

            // Only extract text fields, skip file fields
            if field.file_name().is_none() {
                let text = field.text().await.map_err(|e| {
                    ParamError::BodyError(format!("Failed to read text field: {}", e))
                })?;

                fields.insert(name, Value::String(text));
            }
        }

        // Deserialize the fields map into T
        let data: T = serde_json::from_value(Value::Object(fields))
            .map_err(|e| ParamError::DeserializationError(e))?;

        Ok(Form(data))
    }
}

impl<T> Deref for Form<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Debug> Debug for Form<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[async_trait]
impl<T> FromRequest for Form<T>
where
    T: DeserializeOwned + Send,
{
    async fn from_request(req: &Request, _ctx: &ParamContext) -> ParamResult<Self> {
        // Extract form data from request body
        // Form data is typically sent as application/x-www-form-urlencoded

        // Check Content-Type header
        let content_type = req
            .headers
            .get(http::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("application/x-www-form-urlencoded")
            && !content_type.contains("multipart/form-data")
        {
            return Err(ParamError::InvalidParameter {
                name: "Content-Type".to_string(),
                message: format!(
                    "Expected application/x-www-form-urlencoded or multipart/form-data, got {}",
                    content_type
                ),
            });
        }

        // Parse the body as form data
        if content_type.contains("application/x-www-form-urlencoded") {
            let body_bytes = req
                .read_body()
                .map_err(|e| ParamError::BodyError(format!("Failed to read body: {}", e)))?;

            let body_str = std::str::from_utf8(&body_bytes)
                .map_err(|e| ParamError::BodyError(format!("Invalid UTF-8 in body: {}", e)))?;

            serde_urlencoded::from_str(body_str)
                .map(Form)
                .map_err(|e| e.into())
        } else if content_type.contains("multipart/form-data") {
            #[cfg(feature = "multipart")]
            {
                Self::from_multipart_internal(req).await
            }
            #[cfg(not(feature = "multipart"))]
            {
                Err(ParamError::BodyError(
                    "multipart/form-data parsing requires 'multipart' feature".to_string(),
                ))
            }
        } else {
            Err(ParamError::InvalidParameter {
                name: "Content-Type".to_string(),
                message: format!("Unsupported content type: {}", content_type),
            })
        }
    }
}
