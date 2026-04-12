use serde::{Deserialize, Serialize};

/// HTTP method for an endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Get => write!(f, "GET"),
            Self::Post => write!(f, "POST"),
            Self::Put => write!(f, "PUT"),
            Self::Patch => write!(f, "PATCH"),
            Self::Delete => write!(f, "DELETE"),
            Self::Head => write!(f, "HEAD"),
            Self::Options => write!(f, "OPTIONS"),
        }
    }
}

/// Where a parameter is located in the request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParameterLocation {
    Path,
    Query,
    Header,
    Cookie,
}

impl std::fmt::Display for ParameterLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path => write!(f, "path"),
            Self::Query => write!(f, "query"),
            Self::Header => write!(f, "header"),
            Self::Cookie => write!(f, "cookie"),
        }
    }
}

/// Authentication method used by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    ApiKey { location: String, name: String },
    Bearer,
    #[serde(rename = "OAuth2")]
    OAuth2 { flows: Vec<String> },
    Basic,
    None,
}

impl std::fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey { location, name } => write!(f, "API Key ({location}: {name})"),
            Self::Bearer => write!(f, "Bearer Token"),
            Self::OAuth2 { flows } => write!(f, "OAuth2 ({})", flows.join(", ")),
            Self::Basic => write!(f, "Basic Auth"),
            Self::None => write!(f, "None"),
        }
    }
}

/// Which spec format was parsed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SpecFormat {
    OpenApi3,
    Swagger2,
    GraphQl,
    GrpcProto,
}

impl std::fmt::Display for SpecFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenApi3 => write!(f, "OpenAPI 3.x"),
            Self::Swagger2 => write!(f, "Swagger 2.0"),
            Self::GraphQl => write!(f, "GraphQL"),
            Self::GrpcProto => write!(f, "gRPC/Protobuf"),
        }
    }
}

/// A data model schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub type_name: String,
    pub format: Option<String>,
    pub properties: Vec<SchemaProperty>,
    pub required_fields: Vec<String>,
    pub enum_values: Vec<String>,
    pub description: String,
}

/// A single property within a schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaProperty {
    pub name: String,
    pub type_name: String,
    pub description: String,
    pub required: bool,
}

/// A request parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub location: ParameterLocation,
    pub description: String,
    pub required: bool,
    pub schema_type: String,
}

/// A request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    pub content_type: String,
    pub schema: Option<Schema>,
    pub required: bool,
    pub description: String,
}

/// A response definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub status_code: String,
    pub description: String,
    pub schema: Option<Schema>,
}

/// A single API endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub path: String,
    pub method: HttpMethod,
    pub summary: String,
    pub description: String,
    pub parameters: Vec<Parameter>,
    pub request_body: Option<RequestBody>,
    pub responses: Vec<Response>,
    pub tags: Vec<String>,
}

/// The complete specification of an API, as parsed from a spec file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSpec {
    pub name: String,
    pub version: Option<String>,
    pub description: String,
    pub base_url: Option<String>,
    pub spec_format: SpecFormat,
    pub auth_methods: Vec<AuthMethod>,
    pub endpoints: Vec<Endpoint>,
    pub schemas: Vec<Schema>,
}
