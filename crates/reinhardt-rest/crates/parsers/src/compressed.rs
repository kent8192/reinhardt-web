//! Compression support for parsers
//!
//! Provides transparent decompression of request bodies with gzip, brotli, or deflate encoding.

use crate::parser::{ParseResult, ParsedData, Parser};
use async_trait::async_trait;
use brotli::Decompressor;
use bytes::Bytes;
use flate2::read::{DeflateDecoder, GzDecoder};
use reinhardt_exception::Error;
use std::io::Read;
use std::sync::Arc;

/// Supported compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionEncoding {
    /// Gzip compression (Content-Encoding: gzip)
    Gzip,
    /// Brotli compression (Content-Encoding: br)
    Brotli,
    /// Deflate compression (Content-Encoding: deflate)
    Deflate,
}

impl CompressionEncoding {
    /// Parse compression encoding from Content-Encoding header
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_parsers::compressed::CompressionEncoding;
    ///
    /// assert_eq!(
    ///     CompressionEncoding::from_header("gzip"),
    ///     Some(CompressionEncoding::Gzip)
    /// );
    /// assert_eq!(
    ///     CompressionEncoding::from_header("br"),
    ///     Some(CompressionEncoding::Brotli)
    /// );
    /// assert_eq!(
    ///     CompressionEncoding::from_header("deflate"),
    ///     Some(CompressionEncoding::Deflate)
    /// );
    /// assert_eq!(CompressionEncoding::from_header("unknown"), None);
    /// ```
    pub fn from_header(encoding: &str) -> Option<Self> {
        match encoding.to_lowercase().as_str() {
            "gzip" => Some(Self::Gzip),
            "br" => Some(Self::Brotli),
            "deflate" => Some(Self::Deflate),
            _ => None,
        }
    }

    /// Decompress data using this encoding
    fn decompress(&self, data: &[u8]) -> ParseResult<Vec<u8>> {
        match self {
            Self::Gzip => {
                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(|e| Error::ParseError(format!("Gzip decompression error: {}", e)))?;
                Ok(decompressed)
            }
            Self::Brotli => {
                let mut decoder = Decompressor::new(data, 4096);
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(|e| Error::ParseError(format!("Brotli decompression error: {}", e)))?;
                Ok(decompressed)
            }
            Self::Deflate => {
                let mut decoder = DeflateDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed).map_err(|e| {
                    Error::ParseError(format!("Deflate decompression error: {}", e))
                })?;
                Ok(decompressed)
            }
        }
    }
}

/// Wrapper parser that handles compressed request bodies
///
/// # Examples
///
/// ```
/// use reinhardt_parsers::compressed::CompressedParser;
/// use reinhardt_parsers::json::JSONParser;
/// use std::sync::Arc;
///
/// let json_parser = JSONParser::new();
/// let compressed_parser = CompressedParser::new(Arc::new(json_parser));
/// ```
pub struct CompressedParser {
    inner: Arc<dyn Parser>,
}

impl CompressedParser {
    /// Create a new CompressedParser wrapping an existing parser.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_parsers::compressed::CompressedParser;
    /// use reinhardt_parsers::json::JSONParser;
    /// use std::sync::Arc;
    ///
    /// let json_parser = JSONParser::new();
    /// let parser = CompressedParser::new(Arc::new(json_parser));
    /// ```
    pub fn new(inner: Arc<dyn Parser>) -> Self {
        Self { inner }
    }

    /// Decompress body if Content-Encoding header is present
    ///
    /// This method is intended for future use when Content-Encoding can be explicitly passed.
    /// Currently unused but kept for API completeness.
    #[allow(dead_code)]
    fn decompress_if_needed(
        &self,
        content_encoding: Option<&str>,
        body: Bytes,
    ) -> ParseResult<Bytes> {
        if let Some(encoding_str) = content_encoding {
            if let Some(encoding) = CompressionEncoding::from_header(encoding_str) {
                let decompressed = encoding.decompress(&body)?;
                return Ok(Bytes::from(decompressed));
            }
        }
        Ok(body)
    }
}

#[async_trait]
impl Parser for CompressedParser {
    fn media_types(&self) -> Vec<String> {
        self.inner.media_types()
    }

    async fn parse(&self, content_type: Option<&str>, body: Bytes) -> ParseResult<ParsedData> {
        // Note: In a real implementation, we would extract Content-Encoding from request headers
        // For now, we try to detect compression from the body itself
        // This is a simplified version - in production, you should pass content_encoding explicitly

        // Try to decompress with each algorithm
        // If decompression succeeds AND the decompressed data is valid, use it
        // Otherwise, treat as uncompressed

        // Try gzip
        if let Ok(data) = CompressionEncoding::Gzip.decompress(&body) {
            let decompressed = Bytes::from(data);
            if let Ok(result) = self.inner.parse(content_type, decompressed.clone()).await {
                return Ok(result);
            }
        }

        // Try brotli
        if let Ok(data) = CompressionEncoding::Brotli.decompress(&body) {
            let decompressed = Bytes::from(data);
            if let Ok(result) = self.inner.parse(content_type, decompressed.clone()).await {
                return Ok(result);
            }
        }

        // Try deflate
        if let Ok(data) = CompressionEncoding::Deflate.decompress(&body) {
            let decompressed = Bytes::from(data);
            if let Ok(result) = self.inner.parse(content_type, decompressed.clone()).await {
                return Ok(result);
            }
        }

        // Not compressed or unknown compression - parse as-is
        self.inner.parse(content_type, body).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::JSONParser;
    use flate2::write::{DeflateEncoder, GzEncoder};
    use flate2::Compression;
    use std::io::Write;

    #[test]
    fn test_compression_encoding_from_header() {
        assert_eq!(
            CompressionEncoding::from_header("gzip"),
            Some(CompressionEncoding::Gzip)
        );
        assert_eq!(
            CompressionEncoding::from_header("GZIP"),
            Some(CompressionEncoding::Gzip)
        );
        assert_eq!(
            CompressionEncoding::from_header("br"),
            Some(CompressionEncoding::Brotli)
        );
        assert_eq!(
            CompressionEncoding::from_header("deflate"),
            Some(CompressionEncoding::Deflate)
        );
        assert_eq!(CompressionEncoding::from_header("unknown"), None);
    }

    #[tokio::test]
    async fn test_compressed_parser_gzip() {
        let json_data = r#"{"name":"John","age":30}"#;

        // Compress with gzip
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json_data.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        let parser = CompressedParser::new(Arc::new(JSONParser::new()));
        let result = parser
            .parse(Some("application/json"), Bytes::from(compressed))
            .await
            .unwrap();

        match result {
            ParsedData::Json(value) => {
                assert_eq!(value["name"], "John");
                assert_eq!(value["age"], 30);
            }
            _ => panic!("Expected JSON data"),
        }
    }

    #[tokio::test]
    async fn test_compressed_parser_deflate() {
        let json_data = r#"{"name":"Alice","city":"NYC"}"#;

        // Compress with deflate
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json_data.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        let parser = CompressedParser::new(Arc::new(JSONParser::new()));
        let result = parser
            .parse(Some("application/json"), Bytes::from(compressed))
            .await
            .unwrap();

        match result {
            ParsedData::Json(value) => {
                assert_eq!(value["name"], "Alice");
                assert_eq!(value["city"], "NYC");
            }
            _ => panic!("Expected JSON data"),
        }
    }

    #[tokio::test]
    async fn test_compressed_parser_brotli() {
        let json_data = r#"{"product":"widget","price":19.99}"#;

        // Compress with brotli
        let mut compressed = Vec::new();
        {
            let mut encoder = brotli::CompressorWriter::new(
                &mut compressed,
                4096,
                11, // quality
                22, // lg_window_size
            );
            encoder.write_all(json_data.as_bytes()).unwrap();
        }

        let parser = CompressedParser::new(Arc::new(JSONParser::new()));
        let result = parser
            .parse(Some("application/json"), Bytes::from(compressed))
            .await
            .unwrap();

        match result {
            ParsedData::Json(value) => {
                assert_eq!(value["product"], "widget");
                assert_eq!(value["price"], 19.99);
            }
            _ => panic!("Expected JSON data"),
        }
    }

    #[tokio::test]
    async fn test_compressed_parser_uncompressed() {
        let json_data = r#"{"uncompressed":true}"#;

        let parser = CompressedParser::new(Arc::new(JSONParser::new()));
        let result = parser
            .parse(Some("application/json"), Bytes::from(json_data))
            .await
            .unwrap();

        match result {
            ParsedData::Json(value) => {
                assert_eq!(value["uncompressed"], true);
            }
            _ => panic!("Expected JSON data"),
        }
    }

    #[test]
    fn test_compressed_parser_media_types() {
        let parser = CompressedParser::new(Arc::new(JSONParser::new()));
        let media_types = parser.media_types();

        assert!(media_types.contains(&"application/json".to_string()));
    }

    #[tokio::test]
    async fn test_gzip_decompression() {
        let original = b"Hello, World!";

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decompressed = CompressionEncoding::Gzip.decompress(&compressed).unwrap();

        assert_eq!(decompressed, original);
    }

    #[tokio::test]
    async fn test_deflate_decompression() {
        let original = b"Test data for deflate";

        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decompressed = CompressionEncoding::Deflate
            .decompress(&compressed)
            .unwrap();

        assert_eq!(decompressed, original);
    }

    #[tokio::test]
    async fn test_brotli_decompression() {
        let original = b"Brotli compressed content";

        let mut compressed = Vec::new();
        {
            let mut encoder = brotli::CompressorWriter::new(&mut compressed, 4096, 11, 22);
            encoder.write_all(original).unwrap();
        }

        let decompressed = CompressionEncoding::Brotli.decompress(&compressed).unwrap();

        assert_eq!(decompressed, original);
    }

    #[tokio::test]
    async fn test_invalid_gzip_data() {
        let invalid_data = b"not gzip data";
        let result = CompressionEncoding::Gzip.decompress(invalid_data);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_brotli_data() {
        let invalid_data = b"not brotli data";
        let result = CompressionEncoding::Brotli.decompress(invalid_data);
        assert!(result.is_err());
    }
}
