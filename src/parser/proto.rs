use crate::ir::{
    ApiSpec, AuthMethod, Endpoint, HttpMethod, Parameter, ParameterLocation, Response, Schema,
    SchemaProperty, SpecFormat,
};
use crate::parser::ApiParser;
use crate::security::{MAX_ENDPOINTS, MAX_SCHEMA_PROPERTIES};
use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

static RE_PACKAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"package\s+([\w.]+)\s*;").unwrap());
static RE_SERVICE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"service\s+(\w+)\s*\{([^}]*)\}").unwrap());
static RE_RPC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?://\s*(.+)\n\s*)?rpc\s+(\w+)\s*\(\s*(stream\s+)?(\w+)\s*\)\s*returns\s*\(\s*(stream\s+)?(\w+)\s*\)").unwrap()
});
static RE_MESSAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?://\s*(.+)\n\s*)?message\s+(\w+)\s*\{([^}]*)\}").unwrap());
static RE_FIELD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?://\s*(.+)\n\s*)?(repeated\s+|optional\s+|required\s+)?(\w+)\s+(\w+)\s*=\s*\d+\s*;",
    )
    .unwrap()
});

pub struct ProtoParser;

impl ApiParser for ProtoParser {
    fn name(&self) -> &'static str {
        "gRPC Proto"
    }

    fn format(&self) -> SpecFormat {
        SpecFormat::GrpcProto
    }

    fn detect(&self, content: &str) -> bool {
        content.contains("syntax") && content.contains("proto") && content.contains("service")
    }

    fn parse(&self, content: &str) -> Result<ApiSpec> {
        let package = extract_package(content);
        let services = parse_services(content);
        let messages = parse_messages(content);

        let mut endpoints = Vec::new();
        let mut api_name = "gRPC API".to_string();

        for service in &services {
            api_name.clone_from(&service.name);
            for rpc in &service.rpcs {
                let path = format!(
                    "/{}.{}/{}",
                    package.as_deref().unwrap_or(""),
                    service.name,
                    rpc.name
                );

                let parameters = vec![Parameter {
                    name: rpc.input_type.clone(),
                    location: ParameterLocation::Query,
                    description: format!("Request message: {}", rpc.input_type),
                    required: true,
                    schema_type: rpc.input_type.clone(),
                }];

                let streaming_note = match (rpc.client_streaming, rpc.server_streaming) {
                    (true, true) => " (bidirectional streaming)",
                    (true, false) => " (client streaming)",
                    (false, true) => " (server streaming)",
                    (false, false) => "",
                };

                if endpoints.len() >= MAX_ENDPOINTS {
                    break;
                }
                endpoints.push(Endpoint {
                    path,
                    method: HttpMethod::Post,
                    summary: format!("{}{streaming_note}", rpc.name),
                    description: rpc.comment.clone(),
                    parameters,
                    request_body: None,
                    responses: vec![Response {
                        status_code: "OK".to_string(),
                        description: format!("Returns {}", rpc.output_type),
                        schema: None,
                    }],
                    tags: vec![service.name.clone()],
                });
            }
        }

        let schemas = messages
            .iter()
            .map(|msg| Schema {
                type_name: msg.name.clone(),
                format: None,
                properties: msg
                    .fields
                    .iter()
                    .take(MAX_SCHEMA_PROPERTIES)
                    .map(|f| SchemaProperty {
                        name: f.name.clone(),
                        type_name: f.field_type.clone(),
                        description: f.comment.clone(),
                        required: true,
                    })
                    .collect(),
                required_fields: msg.fields.iter().map(|f| f.name.clone()).collect(),
                enum_values: Vec::new(),
                description: msg.comment.clone(),
            })
            .collect();

        Ok(ApiSpec {
            name: api_name,
            version: None,
            description: format!(
                "gRPC service (package: {})",
                package.as_deref().unwrap_or("unknown")
            ),
            base_url: None,
            spec_format: SpecFormat::GrpcProto,
            auth_methods: vec![AuthMethod::None],
            endpoints,
            schemas,
        })
    }
}

struct ServiceDef {
    name: String,
    rpcs: Vec<RpcDef>,
}

struct RpcDef {
    name: String,
    input_type: String,
    output_type: String,
    client_streaming: bool,
    server_streaming: bool,
    comment: String,
}

struct MessageDef {
    name: String,
    comment: String,
    fields: Vec<FieldDef>,
}

struct FieldDef {
    name: String,
    field_type: String,
    comment: String,
}

fn extract_package(content: &str) -> Option<String> {
    RE_PACKAGE.captures(content).map(|c| c[1].to_string())
}

fn parse_services(content: &str) -> Vec<ServiceDef> {
    let mut services = Vec::new();

    for service_caps in RE_SERVICE.captures_iter(content) {
        let name = service_caps[1].to_string();
        let body = &service_caps[2];

        let rpcs = RE_RPC
            .captures_iter(body)
            .map(|caps| RpcDef {
                name: caps[2].to_string(),
                input_type: caps[4].to_string(),
                output_type: caps[6].to_string(),
                client_streaming: caps.get(3).is_some(),
                server_streaming: caps.get(5).is_some(),
                comment: caps
                    .get(1)
                    .map_or(String::new(), |m| m.as_str().trim().to_string()),
            })
            .collect();

        services.push(ServiceDef { name, rpcs });
    }

    services
}

fn parse_messages(content: &str) -> Vec<MessageDef> {
    let mut messages = Vec::new();

    for msg_caps in RE_MESSAGE.captures_iter(content) {
        let comment = msg_caps
            .get(1)
            .map_or(String::new(), |m| m.as_str().trim().to_string());
        let name = msg_caps[2].to_string();
        let body = &msg_caps[3];

        let fields = RE_FIELD
            .captures_iter(body)
            .map(|caps| {
                let field_comment = caps
                    .get(1)
                    .map_or(String::new(), |m| m.as_str().trim().to_string());
                let modifier = caps.get(2).map_or("", |m| m.as_str().trim());
                let base_type = caps[3].to_string();
                let field_type = if modifier == "repeated" {
                    format!("[{base_type}]")
                } else {
                    base_type
                };
                FieldDef {
                    name: caps[4].to_string(),
                    field_type,
                    comment: field_comment,
                }
            })
            .collect();

        messages.push(MessageDef {
            name,
            comment,
            fields,
        });
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_PROTO: &str = r#"
syntax = "proto3";

package petstore.v1;

// A pet in the store
message Pet {
  // Unique identifier
  int64 id = 1;
  // Pet name
  string name = 2;
  // Optional tag
  string tag = 3;
}

message ListPetsRequest {
  int32 limit = 1;
}

message ListPetsResponse {
  repeated Pet pets = 1;
}

message GetPetRequest {
  int64 id = 1;
}

service PetService {
  // List all pets
  rpc ListPets(ListPetsRequest) returns (ListPetsResponse);
  // Get a specific pet
  rpc GetPet(GetPetRequest) returns (Pet);
  // Stream pet updates
  rpc WatchPets(ListPetsRequest) returns (stream Pet);
}
"#;

    #[test]
    fn detects_proto() {
        let parser = ProtoParser;
        assert!(parser.detect(SAMPLE_PROTO));
    }

    #[test]
    fn does_not_detect_graphql() {
        let parser = ProtoParser;
        assert!(!parser.detect("type Query { pets: [Pet] }"));
    }

    #[test]
    fn parses_package() {
        let pkg = extract_package(SAMPLE_PROTO);
        assert_eq!(pkg, Some("petstore.v1".to_string()));
    }

    #[test]
    fn parses_services() {
        let parser = ProtoParser;
        let spec = parser.parse(SAMPLE_PROTO).unwrap();
        assert_eq!(spec.name, "PetService");
        assert_eq!(spec.endpoints.len(), 3);
    }

    #[test]
    fn parses_streaming() {
        let parser = ProtoParser;
        let spec = parser.parse(SAMPLE_PROTO).unwrap();
        let watch = spec
            .endpoints
            .iter()
            .find(|e| e.path.contains("WatchPets"))
            .unwrap();
        assert!(watch.summary.contains("server streaming"));
    }

    #[test]
    fn parses_messages() {
        let parser = ProtoParser;
        let spec = parser.parse(SAMPLE_PROTO).unwrap();
        assert!(spec.schemas.len() >= 3); // Pet, ListPetsRequest, ListPetsResponse, GetPetRequest
        let pet = spec.schemas.iter().find(|s| s.type_name == "Pet").unwrap();
        assert_eq!(pet.properties.len(), 3);
    }
}
