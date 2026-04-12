pub mod graphql;
pub mod openapi3;
pub mod proto;
pub mod swagger2;

use crate::ir::{ApiSpec, SpecFormat};
use anyhow::Result;

/// Trait that each API spec parser implements.
pub trait ApiParser {
    /// Human-readable name of this parser.
    fn name(&self) -> &'static str;

    /// The spec format this parser handles.
    fn format(&self) -> SpecFormat;

    /// Returns true if the content appears to be this spec format.
    fn detect(&self, content: &str) -> bool;

    /// Parse the spec content into an `ApiSpec`.
    fn parse(&self, content: &str) -> Result<ApiSpec>;
}

/// Registry of all available parsers, used for auto-detection.
pub struct ParserRegistry {
    parsers: Vec<Box<dyn ApiParser>>,
}

impl ParserRegistry {
    /// Create a registry with all built-in parsers.
    #[must_use]
    pub fn new() -> Self {
        Self {
            parsers: vec![
                Box::new(openapi3::OpenApi3Parser),
                Box::new(swagger2::Swagger2Parser),
                Box::new(graphql::GraphQlParser),
                Box::new(proto::ProtoParser),
            ],
        }
    }

    /// Auto-detect format from content and parse.
    pub fn detect_and_parse(&self, content: &str) -> Result<ApiSpec> {
        for parser in &self.parsers {
            if parser.detect(content) {
                eprintln!("Detected format: {}", parser.name());
                return parser.parse(content);
            }
        }
        anyhow::bail!("Could not detect API spec format. Supported: OpenAPI 3.x, Swagger 2.0, GraphQL, gRPC Proto")
    }

    /// Parse using a specific format.
    pub fn parse_with_format(&self, format: &SpecFormat, content: &str) -> Result<ApiSpec> {
        for parser in &self.parsers {
            if &parser.format() == format {
                return parser.parse(content);
            }
        }
        anyhow::bail!("No parser registered for format: {format}")
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

