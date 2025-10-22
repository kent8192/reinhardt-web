/// OpenAPI renderer stub
///
/// Note: Full OpenAPI rendering is implemented in reinhardt-openapi
#[derive(Debug, Clone)]
pub struct OpenAPIRenderer;

impl OpenAPIRenderer {
    /// Creates a new OpenAPI renderer
    ///
    /// # Examples
    ///
    /// ```
    /// use renderers_ext::OpenAPIRenderer;
    ///
    /// let renderer = OpenAPIRenderer::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenAPIRenderer {
    fn default() -> Self {
        Self::new()
    }
}
