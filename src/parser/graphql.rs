use crate::ir::{
    ApiSpec, AuthMethod, Endpoint, HttpMethod, Parameter, ParameterLocation, Response, Schema,
    SchemaProperty, SpecFormat,
};
use crate::parser::ApiParser;
use crate::security::{MAX_ENDPOINTS, MAX_SCHEMA_PROPERTIES};
use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

static RE_TYPE_DEF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?:"""([^"]*?)"""\s*)?type\s+(\w+)\s*\{([^}]*)\}"#).unwrap());
static RE_FIELD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)(?:"""([^"]*?)"""\s*)?^\s+(\w+)(?:\(([^)]*)\))?\s*:\s*([\w\[\]!]+)"#).unwrap()
});
static RE_ARG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\w+)\s*:\s*([\w\[\]!]+)").unwrap());

pub struct GraphQlParser;

impl ApiParser for GraphQlParser {
    fn name(&self) -> &'static str {
        "GraphQL"
    }

    fn format(&self) -> SpecFormat {
        SpecFormat::GraphQl
    }

    fn detect(&self, content: &str) -> bool {
        // GraphQL schemas have type definitions with specific keywords
        (content.contains("type Query") || content.contains("type Mutation"))
            && (content.contains("schema") || content.contains("type "))
    }

    fn parse(&self, content: &str) -> Result<ApiSpec> {
        let types = parse_type_definitions(content);
        let query_fields = extract_type_fields(content, "Query");
        let mutation_fields = extract_type_fields(content, "Mutation");

        let mut endpoints = Vec::new();

        for field in &query_fields {
            if endpoints.len() >= MAX_ENDPOINTS {
                break;
            }
            endpoints.push(Endpoint {
                path: field.name.clone(),
                method: HttpMethod::Get,
                summary: field.description.clone(),
                description: field.description.clone(),
                parameters: field
                    .args
                    .iter()
                    .map(|a| Parameter {
                        name: a.name.clone(),
                        location: ParameterLocation::Query,
                        description: String::new(),
                        required: a.required,
                        schema_type: a.type_name.clone(),
                    })
                    .collect(),
                request_body: None,
                responses: vec![Response {
                    status_code: "200".to_string(),
                    description: format!("Returns {}", field.return_type),
                    schema: None,
                }],
                tags: vec!["Query".to_string()],
            });
        }

        for field in &mutation_fields {
            if endpoints.len() >= MAX_ENDPOINTS {
                break;
            }
            endpoints.push(Endpoint {
                path: field.name.clone(),
                method: HttpMethod::Post,
                summary: field.description.clone(),
                description: field.description.clone(),
                parameters: Vec::new(),
                request_body: None,
                responses: vec![Response {
                    status_code: "200".to_string(),
                    description: format!("Returns {}", field.return_type),
                    schema: None,
                }],
                tags: vec!["Mutation".to_string()],
            });
        }

        let schemas = types
            .iter()
            .map(|t| Schema {
                type_name: t.name.clone(),
                format: None,
                properties: t
                    .fields
                    .iter()
                    .take(MAX_SCHEMA_PROPERTIES)
                    .map(|f| SchemaProperty {
                        name: f.name.clone(),
                        type_name: f.return_type.clone(),
                        description: f.description.clone(),
                        required: f.name.ends_with('!'),
                    })
                    .collect(),
                required_fields: Vec::new(),
                enum_values: Vec::new(),
                description: t.description.clone(),
            })
            .collect();

        Ok(ApiSpec {
            name: "GraphQL API".to_string(),
            version: None,
            description: "GraphQL schema".to_string(),
            base_url: None,
            spec_format: SpecFormat::GraphQl,
            auth_methods: vec![AuthMethod::None],
            endpoints,
            schemas,
        })
    }
}

struct TypeDef {
    name: String,
    description: String,
    fields: Vec<FieldDef>,
}

struct FieldDef {
    name: String,
    return_type: String,
    description: String,
    args: Vec<ArgDef>,
}

struct ArgDef {
    name: String,
    type_name: String,
    required: bool,
}

fn parse_type_definitions(content: &str) -> Vec<TypeDef> {
    let mut types = Vec::new();

    for caps in RE_TYPE_DEF.captures_iter(content) {
        let name = caps[2].to_string();
        // Skip Query and Mutation as they're handled separately
        if name == "Query" || name == "Mutation" || name == "Subscription" {
            continue;
        }
        let description = caps
            .get(1)
            .map_or(String::new(), |m| m.as_str().trim().to_string());
        let body = &caps[3];
        let fields = parse_fields(body);

        types.push(TypeDef {
            name,
            description,
            fields,
        });
    }

    types
}

fn extract_type_fields(content: &str, type_name: &str) -> Vec<FieldDef> {
    let escaped_name = regex::escape(type_name);
    let pattern = format!(r"type\s+{escaped_name}\s*\{{([^}}]*)}}");
    let Ok(re) = Regex::new(&pattern) else {
        return Vec::new();
    };

    re.captures(content)
        .map_or_else(Vec::new, |caps| parse_fields(&caps[1]))
}

fn parse_fields(body: &str) -> Vec<FieldDef> {
    let mut fields = Vec::new();

    for caps in RE_FIELD.captures_iter(body) {
        let description = caps
            .get(1)
            .map_or(String::new(), |m| m.as_str().trim().to_string());
        let name = caps[2].to_string();
        let args_str = caps.get(3).map(|m| m.as_str());
        let return_type = caps[4].to_string();

        let args = args_str.map_or_else(Vec::new, parse_args);

        fields.push(FieldDef {
            name,
            return_type,
            description,
            args,
        });
    }

    fields
}

fn parse_args(args_str: &str) -> Vec<ArgDef> {
    let mut args = Vec::new();

    for caps in RE_ARG.captures_iter(args_str) {
        let name = caps[1].to_string();
        let type_name = caps[2].to_string();
        let required = type_name.ends_with('!');
        args.push(ArgDef {
            name,
            type_name,
            required,
        });
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_GRAPHQL: &str = r#"
"""A pet in the store"""
type Pet {
  id: ID!
  name: String!
  tag: String
}

type Query {
  """List all pets"""
  pets(limit: Int): [Pet!]!
  """Get a specific pet"""
  pet(id: ID!): Pet
}

type Mutation {
  """Create a new pet"""
  createPet(name: String!, tag: String): Pet!
  """Delete a pet"""
  deletePet(id: ID!): Boolean!
}
"#;

    #[test]
    fn detects_graphql() {
        let parser = GraphQlParser;
        assert!(parser.detect(SAMPLE_GRAPHQL));
    }

    #[test]
    fn does_not_detect_openapi() {
        let parser = GraphQlParser;
        assert!(!parser.detect(r#"{"openapi": "3.0.0"}"#));
    }

    #[test]
    fn parses_query_endpoints() {
        let parser = GraphQlParser;
        let spec = parser.parse(SAMPLE_GRAPHQL).unwrap();
        let queries: Vec<_> = spec
            .endpoints
            .iter()
            .filter(|e| e.method == HttpMethod::Get)
            .collect();
        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].path, "pets");
        assert_eq!(queries[0].parameters.len(), 1);
        assert_eq!(queries[0].parameters[0].name, "limit");
    }

    #[test]
    fn parses_mutation_endpoints() {
        let parser = GraphQlParser;
        let spec = parser.parse(SAMPLE_GRAPHQL).unwrap();
        let mutations: Vec<_> = spec
            .endpoints
            .iter()
            .filter(|e| e.method == HttpMethod::Post)
            .collect();
        assert_eq!(mutations.len(), 2);
        assert_eq!(mutations[0].path, "createPet");
    }

    #[test]
    fn parses_types_as_schemas() {
        let parser = GraphQlParser;
        let spec = parser.parse(SAMPLE_GRAPHQL).unwrap();
        assert_eq!(spec.schemas.len(), 1); // Pet only (Query/Mutation excluded)
        assert_eq!(spec.schemas[0].type_name, "Pet");
        assert_eq!(spec.schemas[0].properties.len(), 3);
    }
}
