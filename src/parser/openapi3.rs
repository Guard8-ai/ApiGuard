use crate::ir::{
    ApiSpec, AuthMethod, Endpoint, HttpMethod, Parameter, ParameterLocation, RequestBody,
    Response, Schema, SchemaProperty, SpecFormat,
};
use crate::parser::ApiParser;
use crate::security::{MAX_ENDPOINTS, MAX_SCHEMA_PROPERTIES};
use anyhow::{Context, Result};

pub struct OpenApi3Parser;

impl ApiParser for OpenApi3Parser {
    fn name(&self) -> &'static str {
        "OpenAPI 3.x"
    }

    fn format(&self) -> SpecFormat {
        SpecFormat::OpenApi3
    }

    fn detect(&self, content: &str) -> bool {
        content.contains("openapi") && (content.contains("\"3.") || content.contains("'3.") || content.contains("3."))
    }

    fn parse(&self, content: &str) -> Result<ApiSpec> {
        let spec: openapiv3::OpenAPI = if content.trim_start().starts_with('{') {
            serde_json::from_str(content).context("Failed to parse OpenAPI 3.x JSON")?
        } else {
            serde_yml::from_str(content).context("Failed to parse OpenAPI 3.x YAML")?
        };

        let name = spec.info.title.clone();
        let version = Some(spec.info.version.clone());
        let description = spec.info.description.clone().unwrap_or_default();

        let base_url = spec.servers.first().map(|s| s.url.clone());

        let auth_methods = extract_auth_methods(&spec);
        let endpoints = extract_endpoints(&spec);
        let schemas = extract_schemas(&spec);

        Ok(ApiSpec {
            name,
            version,
            description,
            base_url,
            spec_format: SpecFormat::OpenApi3,
            auth_methods,
            endpoints,
            schemas,
        })
    }
}

fn extract_auth_methods(spec: &openapiv3::OpenAPI) -> Vec<AuthMethod> {
    let mut methods = Vec::new();

    if let Some(components) = &spec.components {
        for (_, scheme_ref) in &components.security_schemes {
            if let openapiv3::ReferenceOr::Item(scheme) = scheme_ref {
                match scheme {
                    openapiv3::SecurityScheme::APIKey {
                        location, name, ..
                    } => {
                        let loc = match location {
                            openapiv3::APIKeyLocation::Query => "query",
                            openapiv3::APIKeyLocation::Header => "header",
                            openapiv3::APIKeyLocation::Cookie => "cookie",
                        };
                        methods.push(AuthMethod::ApiKey {
                            location: loc.to_string(),
                            name: name.clone(),
                        });
                    }
                    openapiv3::SecurityScheme::HTTP { scheme, .. } => {
                        if scheme.eq_ignore_ascii_case("bearer") {
                            methods.push(AuthMethod::Bearer);
                        } else if scheme.eq_ignore_ascii_case("basic") {
                            methods.push(AuthMethod::Basic);
                        }
                    }
                    openapiv3::SecurityScheme::OAuth2 { flows, .. } => {
                        let mut flow_names = Vec::new();
                        if flows.implicit.is_some() {
                            flow_names.push("implicit".to_string());
                        }
                        if flows.password.is_some() {
                            flow_names.push("password".to_string());
                        }
                        if flows.client_credentials.is_some() {
                            flow_names.push("client_credentials".to_string());
                        }
                        if flows.authorization_code.is_some() {
                            flow_names.push("authorization_code".to_string());
                        }
                        methods.push(AuthMethod::OAuth2 { flows: flow_names });
                    }
                    openapiv3::SecurityScheme::OpenIDConnect { .. } => {
                        methods.push(AuthMethod::OAuth2 {
                            flows: vec!["openid_connect".to_string()],
                        });
                    }
                }
            }
        }
    }

    if methods.is_empty() {
        methods.push(AuthMethod::None);
    }

    methods
}

fn extract_endpoints(spec: &openapiv3::OpenAPI) -> Vec<Endpoint> {
    let mut endpoints = Vec::new();

    for (path, path_item_ref) in &spec.paths.paths {
        let path_item = match path_item_ref {
            openapiv3::ReferenceOr::Item(item) => item,
            openapiv3::ReferenceOr::Reference { .. } => continue,
        };

        let operations = [
            (HttpMethod::Get, &path_item.get),
            (HttpMethod::Post, &path_item.post),
            (HttpMethod::Put, &path_item.put),
            (HttpMethod::Patch, &path_item.patch),
            (HttpMethod::Delete, &path_item.delete),
            (HttpMethod::Head, &path_item.head),
            (HttpMethod::Options, &path_item.options),
        ];

        for (method, op_opt) in &operations {
            if let Some(op) = op_opt {
                let params = extract_parameters(&op.parameters);
                let request_body = op.request_body.as_ref().and_then(|rb| {
                    if let openapiv3::ReferenceOr::Item(body) = rb {
                        Some(convert_request_body(body))
                    } else {
                        None
                    }
                });
                let responses = extract_responses(&op.responses);

                if endpoints.len() >= MAX_ENDPOINTS {
                    break;
                }
                endpoints.push(Endpoint {
                    path: path.clone(),
                    method: method.clone(),
                    summary: op.summary.clone().unwrap_or_default(),
                    description: op.description.clone().unwrap_or_default(),
                    parameters: params,
                    request_body,
                    responses,
                    tags: op.tags.clone(),
                });
            }
        }
    }

    endpoints
}

fn extract_parameters(params: &[openapiv3::ReferenceOr<openapiv3::Parameter>]) -> Vec<Parameter> {
    let mut result = Vec::new();

    for param_ref in params {
        let param = match param_ref {
            openapiv3::ReferenceOr::Item(p) => p,
            openapiv3::ReferenceOr::Reference { .. } => continue,
        };

        let (name, location, data) = match param {
            openapiv3::Parameter::Query { parameter_data, .. } => {
                (parameter_data.name.clone(), ParameterLocation::Query, parameter_data)
            }
            openapiv3::Parameter::Header { parameter_data, .. } => {
                (parameter_data.name.clone(), ParameterLocation::Header, parameter_data)
            }
            openapiv3::Parameter::Path { parameter_data, .. } => {
                (parameter_data.name.clone(), ParameterLocation::Path, parameter_data)
            }
            openapiv3::Parameter::Cookie { parameter_data, .. } => {
                (parameter_data.name.clone(), ParameterLocation::Cookie, parameter_data)
            }
        };

        let schema_type = extract_schema_type_from_parameter_data(data);

        result.push(Parameter {
            name,
            location,
            description: data.description.clone().unwrap_or_default(),
            required: data.required,
            schema_type,
        });
    }

    result
}

fn extract_schema_type_from_parameter_data(data: &openapiv3::ParameterData) -> String {
    match &data.format {
        openapiv3::ParameterSchemaOrContent::Schema(schema_ref) => {
            if let openapiv3::ReferenceOr::Item(schema) = schema_ref {
                schema_type_name(&schema.schema_kind)
            } else {
                "string".to_string()
            }
        }
        openapiv3::ParameterSchemaOrContent::Content(_) => "object".to_string(),
    }
}

fn convert_request_body(body: &openapiv3::RequestBody) -> RequestBody {
    let (content_type, schema) = body
        .content
        .iter()
        .next()
        .map_or_else(
            || ("application/json".to_string(), None),
            |(ct, media)| {
                let schema = media.schema.as_ref().and_then(|s| {
                    if let openapiv3::ReferenceOr::Item(schema) = s {
                        Some(convert_schema("RequestBody", schema))
                    } else {
                        None
                    }
                });
                (ct.clone(), schema)
            },
        );

    RequestBody {
        content_type,
        schema,
        required: body.required,
        description: body.description.clone().unwrap_or_default(),
    }
}

fn extract_responses(responses: &openapiv3::Responses) -> Vec<Response> {
    let mut result = Vec::new();

    if let Some(openapiv3::ReferenceOr::Item(resp)) = &responses.default {
        result.push(Response {
            status_code: "default".to_string(),
            description: resp.description.clone(),
            schema: None,
        });
    }

    for (status, resp_ref) in &responses.responses {
        if let openapiv3::ReferenceOr::Item(resp) = resp_ref {
            result.push(Response {
                status_code: status.to_string(),
                description: resp.description.clone(),
                schema: None,
            });
        }
    }

    result
}

fn extract_schemas(spec: &openapiv3::OpenAPI) -> Vec<Schema> {
    let mut schemas = Vec::new();

    if let Some(components) = &spec.components {
        for (name, schema_ref) in &components.schemas {
            if let openapiv3::ReferenceOr::Item(schema) = schema_ref {
                schemas.push(convert_schema(name, schema));
            }
        }
    }

    schemas
}

fn convert_schema(name: &str, schema: &openapiv3::Schema) -> Schema {
    let type_name = schema_type_name(&schema.schema_kind);
    let description = schema
        .schema_data
        .description
        .clone()
        .unwrap_or_default();

    let mut properties = Vec::new();
    let mut required_fields = Vec::new();
    let mut enum_values = Vec::new();

    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) => {
            required_fields.clone_from(&obj.required);
            for (prop_name, prop_ref) in obj.properties.iter().take(MAX_SCHEMA_PROPERTIES) {
                let (prop_type, prop_desc) = match prop_ref {
                    openapiv3::ReferenceOr::Item(prop_schema) => (
                        schema_type_name(&prop_schema.schema_kind),
                        prop_schema
                            .schema_data
                            .description
                            .clone()
                            .unwrap_or_default(),
                    ),
                    openapiv3::ReferenceOr::Reference { .. } => {
                        ("string".to_string(), String::new())
                    }
                };
                properties.push(SchemaProperty {
                    name: prop_name.clone(),
                    type_name: prop_type,
                    description: prop_desc,
                    required: obj.required.contains(prop_name),
                });
            }
        }
        openapiv3::SchemaKind::Type(openapiv3::Type::String(str_type)) => {
            enum_values = str_type.enumeration.iter().filter_map(Clone::clone).collect();
        }
        _ => {}
    }

    Schema {
        type_name: if type_name == "object" {
            name.to_string()
        } else {
            type_name
        },
        format: schema.schema_data.description.clone(),
        properties,
        required_fields,
        enum_values,
        description,
    }
}

fn schema_type_name(kind: &openapiv3::SchemaKind) -> String {
    match kind {
        openapiv3::SchemaKind::Type(t) => match t {
            openapiv3::Type::String(_) => "string".to_string(),
            openapiv3::Type::Number(_) => "number".to_string(),
            openapiv3::Type::Integer(_) => "integer".to_string(),
            openapiv3::Type::Boolean(_) => "boolean".to_string(),
            openapiv3::Type::Object(_) => "object".to_string(),
            openapiv3::Type::Array(_) => "array".to_string(),
        },
        _ => "any".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PETSTORE_OPENAPI3: &str = r#"{
  "openapi": "3.0.3",
  "info": {
    "title": "Petstore",
    "version": "1.0.0",
    "description": "A sample API for pets"
  },
  "servers": [{ "url": "https://petstore.example.com/v1" }],
  "paths": {
    "/pets": {
      "get": {
        "summary": "List all pets",
        "operationId": "listPets",
        "tags": ["pets"],
        "parameters": [
          {
            "name": "limit",
            "in": "query",
            "required": false,
            "schema": { "type": "integer" },
            "description": "How many items to return"
          }
        ],
        "responses": {
          "200": { "description": "A list of pets" }
        }
      },
      "post": {
        "summary": "Create a pet",
        "operationId": "createPet",
        "tags": ["pets"],
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "type": "object",
                "required": ["name"],
                "properties": {
                  "name": { "type": "string", "description": "Pet name" },
                  "tag": { "type": "string", "description": "Optional tag" }
                }
              }
            }
          }
        },
        "responses": {
          "201": { "description": "Pet created" }
        }
      }
    },
    "/pets/{petId}": {
      "get": {
        "summary": "Info for a specific pet",
        "operationId": "showPetById",
        "tags": ["pets"],
        "parameters": [
          {
            "name": "petId",
            "in": "path",
            "required": true,
            "schema": { "type": "string" },
            "description": "The id of the pet"
          }
        ],
        "responses": {
          "200": { "description": "Expected response to a valid request" }
        }
      }
    }
  },
  "components": {
    "schemas": {
      "Pet": {
        "type": "object",
        "required": ["id", "name"],
        "properties": {
          "id": { "type": "integer", "description": "Unique identifier" },
          "name": { "type": "string", "description": "Pet name" },
          "tag": { "type": "string", "description": "Optional tag" }
        }
      }
    },
    "securitySchemes": {
      "bearerAuth": {
        "type": "http",
        "scheme": "bearer"
      }
    }
  }
}"#;

    #[test]
    fn detects_openapi3() {
        let parser = OpenApi3Parser;
        assert!(parser.detect(PETSTORE_OPENAPI3));
    }

    #[test]
    fn parses_petstore_info() {
        let parser = OpenApi3Parser;
        let spec = parser.parse(PETSTORE_OPENAPI3).unwrap();
        assert_eq!(spec.name, "Petstore");
        assert_eq!(spec.version, Some("1.0.0".to_string()));
        assert_eq!(spec.base_url, Some("https://petstore.example.com/v1".to_string()));
    }

    #[test]
    fn parses_petstore_endpoints() {
        let parser = OpenApi3Parser;
        let spec = parser.parse(PETSTORE_OPENAPI3).unwrap();
        assert_eq!(spec.endpoints.len(), 3); // GET /pets, POST /pets, GET /pets/{petId}

        let get_pets = spec.endpoints.iter().find(|e| e.path == "/pets" && e.method == HttpMethod::Get).unwrap();
        assert_eq!(get_pets.summary, "List all pets");
        assert_eq!(get_pets.parameters.len(), 1);
        assert_eq!(get_pets.parameters[0].name, "limit");
    }

    #[test]
    fn parses_petstore_auth() {
        let parser = OpenApi3Parser;
        let spec = parser.parse(PETSTORE_OPENAPI3).unwrap();
        assert!(spec.auth_methods.iter().any(|a| matches!(a, AuthMethod::Bearer)));
    }

    #[test]
    fn parses_petstore_schemas() {
        let parser = OpenApi3Parser;
        let spec = parser.parse(PETSTORE_OPENAPI3).unwrap();
        assert_eq!(spec.schemas.len(), 1);
        assert_eq!(spec.schemas[0].type_name, "Pet");
        assert_eq!(spec.schemas[0].properties.len(), 3);
    }
}
