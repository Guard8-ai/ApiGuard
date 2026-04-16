use crate::ir::{
    ApiSpec, AuthMethod, Endpoint, HttpMethod, Parameter, ParameterLocation, Response, Schema,
    SchemaProperty, SpecFormat,
};
use crate::parser::ApiParser;
use crate::security::{MAX_ENDPOINTS, MAX_SCHEMA_PROPERTIES};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

pub struct Swagger2Parser;

impl ApiParser for Swagger2Parser {
    fn name(&self) -> &'static str {
        "Swagger 2.0"
    }

    fn format(&self) -> SpecFormat {
        SpecFormat::Swagger2
    }

    fn detect(&self, content: &str) -> bool {
        content.contains("\"swagger\"") && content.contains("\"2.")
            || content.contains("swagger:") && content.contains("\"2.")
    }

    fn parse(&self, content: &str) -> Result<ApiSpec> {
        let spec: SwaggerSpec = if content.trim_start().starts_with('{') {
            serde_json::from_str(content).context("Failed to parse Swagger 2.0 JSON")?
        } else {
            serde_yml::from_str(content).context("Failed to parse Swagger 2.0 YAML")?
        };

        let base_url = match (&spec.host, &spec.base_path) {
            (Some(host), Some(base)) => Some(format!("https://{host}{base}")),
            (Some(host), None) => Some(format!("https://{host}")),
            _ => None,
        };

        let auth_methods = extract_auth(&spec);
        let endpoints = extract_endpoints(&spec);
        let schemas = extract_definitions(&spec);

        Ok(ApiSpec {
            name: spec.info.title,
            version: Some(spec.info.version),
            description: spec.info.description.unwrap_or_default(),
            base_url,
            spec_format: SpecFormat::Swagger2,
            auth_methods,
            endpoints,
            schemas,
        })
    }
}

// Minimal Swagger 2.0 types for deserialization
#[derive(Deserialize)]
struct SwaggerSpec {
    info: SwaggerInfo,
    host: Option<String>,
    #[serde(rename = "basePath")]
    base_path: Option<String>,
    paths: BTreeMap<String, BTreeMap<String, SwaggerOperation>>,
    #[serde(default)]
    definitions: BTreeMap<String, SwaggerSchema>,
    #[serde(default, rename = "securityDefinitions")]
    security_definitions: BTreeMap<String, SwaggerSecurityDef>,
}

#[derive(Deserialize)]
struct SwaggerInfo {
    title: String,
    version: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct SwaggerOperation {
    summary: Option<String>,
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    parameters: Vec<SwaggerParameter>,
    #[serde(default)]
    responses: BTreeMap<String, SwaggerResponse>,
}

#[derive(Deserialize)]
struct SwaggerParameter {
    name: String,
    #[serde(rename = "in")]
    location: String,
    description: Option<String>,
    #[serde(default)]
    required: bool,
    #[serde(rename = "type")]
    param_type: Option<String>,
}

#[derive(Deserialize)]
struct SwaggerResponse {
    description: Option<String>,
}

#[derive(Deserialize)]
struct SwaggerSchema {
    #[serde(rename = "type")]
    schema_type: Option<String>,
    description: Option<String>,
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    properties: BTreeMap<String, SwaggerSchemaProperty>,
    #[serde(default, rename = "enum")]
    enum_values: Vec<String>,
}

#[derive(Deserialize)]
struct SwaggerSchemaProperty {
    #[serde(rename = "type")]
    prop_type: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct SwaggerSecurityDef {
    #[serde(rename = "type")]
    sec_type: String,
    name: Option<String>,
    #[serde(rename = "in")]
    location: Option<String>,
    flow: Option<String>,
}

fn extract_auth(spec: &SwaggerSpec) -> Vec<AuthMethod> {
    let mut methods = Vec::new();

    for def in spec.security_definitions.values() {
        match def.sec_type.as_str() {
            "apiKey" => {
                methods.push(AuthMethod::ApiKey {
                    location: def.location.clone().unwrap_or_default(),
                    name: def.name.clone().unwrap_or_default(),
                });
            }
            "basic" => methods.push(AuthMethod::Basic),
            "oauth2" => {
                methods.push(AuthMethod::OAuth2 {
                    flows: def.flow.iter().cloned().collect(),
                });
            }
            _ => {}
        }
    }

    if methods.is_empty() {
        methods.push(AuthMethod::None);
    }

    methods
}

fn extract_endpoints(spec: &SwaggerSpec) -> Vec<Endpoint> {
    let mut endpoints = Vec::new();

    for (path, methods) in &spec.paths {
        for (method_str, operation) in methods {
            let method = match method_str.as_str() {
                "get" => HttpMethod::Get,
                "post" => HttpMethod::Post,
                "put" => HttpMethod::Put,
                "patch" => HttpMethod::Patch,
                "delete" => HttpMethod::Delete,
                "head" => HttpMethod::Head,
                "options" => HttpMethod::Options,
                _ => continue,
            };

            let parameters = operation
                .parameters
                .iter()
                .map(|p| Parameter {
                    name: p.name.clone(),
                    #[allow(clippy::match_same_arms)]
                    location: match p.location.as_str() {
                        "path" => ParameterLocation::Path,
                        "query" => ParameterLocation::Query,
                        "header" => ParameterLocation::Header,
                        _ => ParameterLocation::Query,
                    },
                    description: p.description.clone().unwrap_or_default(),
                    required: p.required,
                    schema_type: p.param_type.clone().unwrap_or_else(|| "string".to_string()),
                })
                .collect();

            let responses = operation
                .responses
                .iter()
                .map(|(code, resp)| Response {
                    status_code: code.clone(),
                    description: resp.description.clone().unwrap_or_default(),
                    schema: None,
                })
                .collect();

            if endpoints.len() >= MAX_ENDPOINTS {
                break;
            }
            endpoints.push(Endpoint {
                path: path.clone(),
                method,
                summary: operation.summary.clone().unwrap_or_default(),
                description: operation.description.clone().unwrap_or_default(),
                parameters,
                request_body: None,
                responses,
                tags: operation.tags.clone(),
            });
        }
    }

    endpoints
}

fn extract_definitions(spec: &SwaggerSpec) -> Vec<Schema> {
    spec.definitions
        .iter()
        .map(|(name, def)| {
            let properties = def
                .properties
                .iter()
                .take(MAX_SCHEMA_PROPERTIES)
                .map(|(prop_name, prop)| SchemaProperty {
                    name: prop_name.clone(),
                    type_name: prop
                        .prop_type
                        .clone()
                        .unwrap_or_else(|| "string".to_string()),
                    description: prop.description.clone().unwrap_or_default(),
                    required: def.required.contains(prop_name),
                })
                .collect();

            Schema {
                type_name: name.clone(),
                format: def.schema_type.clone(),
                properties,
                required_fields: def.required.clone(),
                enum_values: def.enum_values.clone(),
                description: def.description.clone().unwrap_or_default(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PETSTORE_SWAGGER2: &str = r#"{
  "swagger": "2.0",
  "info": {
    "title": "Petstore",
    "version": "1.0.0",
    "description": "A sample Swagger 2.0 API"
  },
  "host": "petstore.example.com",
  "basePath": "/v1",
  "paths": {
    "/pets": {
      "get": {
        "summary": "List all pets",
        "tags": ["pets"],
        "parameters": [
          {
            "name": "limit",
            "in": "query",
            "required": false,
            "type": "integer",
            "description": "Max items to return"
          }
        ],
        "responses": {
          "200": { "description": "A list of pets" }
        }
      }
    }
  },
  "definitions": {
    "Pet": {
      "type": "object",
      "required": ["id", "name"],
      "properties": {
        "id": { "type": "integer", "description": "Unique ID" },
        "name": { "type": "string", "description": "Pet name" }
      }
    }
  },
  "securityDefinitions": {
    "api_key": {
      "type": "apiKey",
      "name": "X-API-Key",
      "in": "header"
    }
  }
}"#;

    #[test]
    fn detects_swagger2() {
        let parser = Swagger2Parser;
        assert!(parser.detect(PETSTORE_SWAGGER2));
    }

    #[test]
    fn parses_swagger2_info() {
        let parser = Swagger2Parser;
        let spec = parser.parse(PETSTORE_SWAGGER2).unwrap();
        assert_eq!(spec.name, "Petstore");
        assert_eq!(spec.version, Some("1.0.0".to_string()));
        assert_eq!(
            spec.base_url,
            Some("https://petstore.example.com/v1".to_string())
        );
    }

    #[test]
    fn parses_swagger2_endpoints() {
        let parser = Swagger2Parser;
        let spec = parser.parse(PETSTORE_SWAGGER2).unwrap();
        assert_eq!(spec.endpoints.len(), 1);
        assert_eq!(spec.endpoints[0].method, HttpMethod::Get);
        assert_eq!(spec.endpoints[0].parameters.len(), 1);
    }

    #[test]
    fn parses_swagger2_auth() {
        let parser = Swagger2Parser;
        let spec = parser.parse(PETSTORE_SWAGGER2).unwrap();
        assert!(spec
            .auth_methods
            .iter()
            .any(|a| matches!(a, AuthMethod::ApiKey { .. })));
    }

    #[test]
    fn parses_swagger2_definitions() {
        let parser = Swagger2Parser;
        let spec = parser.parse(PETSTORE_SWAGGER2).unwrap();
        assert_eq!(spec.schemas.len(), 1);
        assert_eq!(spec.schemas[0].type_name, "Pet");
        assert_eq!(spec.schemas[0].properties.len(), 2);
    }
}
