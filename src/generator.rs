use crate::ir::{ApiSpec, AuthMethod, Endpoint, HttpMethod};
use crate::security::{escape_markdown, escape_shell_example, safe_description, MAX_DESCRIPTION_LENGTH};
use std::fmt::Write;

/// Generate a markdown agentic AI guide from an `ApiSpec`.
///
/// # Errors
///
/// Returns an error if writing to the internal string buffer fails.
pub fn generate_guide(spec: &ApiSpec) -> anyhow::Result<String> {
    let mut out = String::with_capacity(4096);

    write_header(&mut out, spec)?;
    write_auth_section(&mut out, spec)?;
    write_endpoints_overview(&mut out, spec)?;
    write_endpoint_details(&mut out, spec)?;
    write_data_models(&mut out, spec)?;
    write_footer(&mut out, spec)?;

    Ok(out)
}

fn write_header(out: &mut String, spec: &ApiSpec) -> std::fmt::Result {
    writeln!(out, "# {} for AI Agents", escape_markdown(&spec.name))?;
    writeln!(out)?;
    if !spec.description.is_empty() {
        writeln!(out, "{}", safe_description(&spec.description, MAX_DESCRIPTION_LENGTH))?;
        writeln!(out)?;
    }
    if let Some(ref base_url) = spec.base_url {
        writeln!(out, "**Base URL**: `{base_url}`")?;
        writeln!(out)?;
    }
    Ok(())
}

fn write_auth_section(out: &mut String, spec: &ApiSpec) -> std::fmt::Result {
    writeln!(out, "## Authentication")?;
    writeln!(out)?;

    for auth in &spec.auth_methods {
        match auth {
            AuthMethod::Bearer => {
                writeln!(out, "**Bearer Token**")?;
                writeln!(out, "```bash")?;
                writeln!(out, "curl -H \"Authorization: Bearer <token>\" <url>")?;
                writeln!(out, "```")?;
            }
            AuthMethod::ApiKey { location, name } => {
                let safe_name = escape_shell_example(name);
                writeln!(out, "**API Key** ({location}: `{safe_name}`)")?;
                writeln!(out, "```bash")?;
                if location == "header" {
                    writeln!(out, "curl -H \"{safe_name}: <api-key>\" <url>")?;
                } else {
                    writeln!(out, "curl \"<url>?{safe_name}=<api-key>\"")?;
                }
                writeln!(out, "```")?;
            }
            AuthMethod::Basic => {
                writeln!(out, "**Basic Auth**")?;
                writeln!(out, "```bash")?;
                writeln!(out, "curl -u username:password <url>")?;
                writeln!(out, "```")?;
            }
            AuthMethod::OAuth2 { flows } => {
                writeln!(out, "**OAuth2** (flows: {})", flows.join(", "))?;
            }
            AuthMethod::None => {
                writeln!(out, "No authentication required.")?;
            }
        }
        writeln!(out)?;
    }

    Ok(())
}

fn write_endpoints_overview(out: &mut String, spec: &ApiSpec) -> std::fmt::Result {
    writeln!(out, "## Quick Reference")?;
    writeln!(out)?;
    writeln!(out, "```bash")?;

    // Group by tag
    let tags = collect_tags(spec);

    if tags.is_empty() {
        for ep in &spec.endpoints {
            write_endpoint_quick(out, ep, spec.base_url.as_ref())?;
        }
    } else {
        for tag in &tags {
            writeln!(out, "# {tag}")?;
            for ep in spec.endpoints.iter().filter(|e| e.tags.contains(tag)) {
                write_endpoint_quick(out, ep, spec.base_url.as_ref())?;
            }
            writeln!(out)?;
        }
    }

    writeln!(out, "```")?;
    writeln!(out)?;
    Ok(())
}

fn write_endpoint_quick(
    out: &mut String,
    ep: &Endpoint,
    base_url: Option<&String>,
) -> std::fmt::Result {
    let base = base_url.map_or("", String::as_str);
    let method_str = format!("{:6}", ep.method.to_string());
    let url = format!("{base}{}", ep.path);

    let summary = if ep.summary.is_empty() {
        String::new()
    } else {
        let max_len = 50;
        if ep.summary.len() > max_len {
            format!("# {}...", &ep.summary[..max_len.saturating_sub(3)])
        } else {
            format!("# {}", ep.summary)
        }
    };

    let padding = 50_usize.saturating_sub(method_str.len() + url.len());
    writeln!(out, "{method_str}{url}{}  {summary}", " ".repeat(padding))?;
    Ok(())
}

fn write_endpoint_details(out: &mut String, spec: &ApiSpec) -> std::fmt::Result {
    if spec.endpoints.is_empty() {
        return Ok(());
    }

    writeln!(out, "## Endpoint Reference")?;
    writeln!(out)?;

    for ep in &spec.endpoints {
        writeln!(out, "### `{} {}`", ep.method, ep.path)?;
        writeln!(out)?;

        if !ep.summary.is_empty() {
            writeln!(out, "{}", safe_description(&ep.summary, MAX_DESCRIPTION_LENGTH))?;
            writeln!(out)?;
        }

        // Parameters
        if !ep.parameters.is_empty() {
            writeln!(out, "**Parameters:**")?;
            writeln!(out)?;
            writeln!(out, "| Name | In | Type | Required | Description |")?;
            writeln!(out, "|------|-----|------|----------|-------------|")?;
            for param in &ep.parameters {
                writeln!(
                    out,
                    "| `{}` | {} | {} | {} | {} |",
                    escape_markdown(&param.name),
                    param.location,
                    escape_markdown(&param.schema_type),
                    if param.required { "yes" } else { "no" },
                    safe_description(&param.description, MAX_DESCRIPTION_LENGTH)
                )?;
            }
            writeln!(out)?;
        }

        // Request body
        if let Some(ref body) = ep.request_body {
            writeln!(
                out,
                "**Request Body** (`{}`{})",
                body.content_type,
                if body.required { ", required" } else { "" }
            )?;
            writeln!(out)?;
            if let Some(ref schema) = body.schema {
                if !schema.properties.is_empty() {
                    writeln!(out, "| Field | Type | Required | Description |")?;
                    writeln!(out, "|-------|------|----------|-------------|")?;
                    for prop in &schema.properties {
                        writeln!(
                            out,
                            "| `{}` | {} | {} | {} |",
                            escape_markdown(&prop.name),
                            escape_markdown(&prop.type_name),
                            if prop.required { "yes" } else { "no" },
                            safe_description(&prop.description, MAX_DESCRIPTION_LENGTH)
                        )?;
                    }
                    writeln!(out)?;
                }
            }
        }

        // Responses
        if !ep.responses.is_empty() {
            writeln!(out, "**Responses:**")?;
            writeln!(out)?;
            writeln!(out, "| Status | Description |")?;
            writeln!(out, "|--------|-------------|")?;
            for resp in &ep.responses {
                writeln!(out, "| {} | {} |", resp.status_code, resp.description)?;
            }
            writeln!(out)?;
        }

        // Curl example
        write_curl_example(out, ep, spec.base_url.as_ref())?;
    }

    Ok(())
}

fn write_curl_example(
    out: &mut String,
    ep: &Endpoint,
    base_url: Option<&String>,
) -> std::fmt::Result {
    let base = base_url.map_or("https://api.example.com", String::as_str);
    let url = format!("{base}{}", ep.path);

    writeln!(out, "**Example:**")?;
    writeln!(out, "```bash")?;

    match ep.method {
        HttpMethod::Get => {
            let query_params: Vec<String> = ep
                .parameters
                .iter()
                .filter(|p| p.location == crate::ir::ParameterLocation::Query)
                .map(|p| format!("{}=<value>", p.name))
                .collect();
            if query_params.is_empty() {
                writeln!(out, "curl \"{url}\"")?;
            } else {
                writeln!(out, "curl \"{url}?{}\"", query_params.join("&"))?;
            }
        }
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => {
            writeln!(
                out,
                "curl -X {} \"{}\" \\",
                ep.method, url
            )?;
            writeln!(out, "  -H \"Content-Type: application/json\" \\")?;
            writeln!(out, "  -d '{{}}'")?;
        }
        HttpMethod::Delete => {
            writeln!(out, "curl -X DELETE \"{url}\"")?;
        }
        _ => {
            writeln!(out, "curl \"{url}\"")?;
        }
    }

    writeln!(out, "```")?;
    writeln!(out)?;
    Ok(())
}

fn write_data_models(out: &mut String, spec: &ApiSpec) -> std::fmt::Result {
    if spec.schemas.is_empty() {
        return Ok(());
    }

    writeln!(out, "## Data Models")?;
    writeln!(out)?;

    for schema in &spec.schemas {
        writeln!(out, "### `{}`", schema.type_name)?;
        writeln!(out)?;

        if !schema.description.is_empty() {
            writeln!(out, "{}", schema.description)?;
            writeln!(out)?;
        }

        if !schema.enum_values.is_empty() {
            let safe_values: Vec<String> = schema.enum_values.iter().map(|v| escape_markdown(v)).collect();
        writeln!(out, "**Values**: {}", safe_values.join(", "))?;
            writeln!(out)?;
        }

        if !schema.properties.is_empty() {
            writeln!(out, "| Field | Type | Required | Description |")?;
            writeln!(out, "|-------|------|----------|-------------|")?;
            for prop in &schema.properties {
                writeln!(
                    out,
                    "| `{}` | {} | {} | {} |",
                    prop.name,
                    prop.type_name,
                    if prop.required { "yes" } else { "no" },
                    prop.description
                )?;
            }
            writeln!(out)?;
        }
    }

    Ok(())
}

fn write_footer(out: &mut String, spec: &ApiSpec) -> std::fmt::Result {
    writeln!(out, "---")?;
    let version_str = spec.version.as_deref().unwrap_or("unknown");
    writeln!(
        out,
        "**Format**: {} | **Version**: {}",
        spec.spec_format, version_str
    )?;
    Ok(())
}

fn collect_tags(spec: &ApiSpec) -> Vec<String> {
    let mut tags = Vec::new();
    for ep in &spec.endpoints {
        for tag in &ep.tags {
            if !tags.contains(tag) {
                tags.push(tag.clone());
            }
        }
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;

    fn sample_spec() -> ApiSpec {
        ApiSpec {
            name: "TestAPI".to_string(),
            version: Some("1.0.0".to_string()),
            description: "A test API".to_string(),
            base_url: Some("https://api.example.com".to_string()),
            spec_format: SpecFormat::OpenApi3,
            auth_methods: vec![AuthMethod::Bearer],
            endpoints: vec![Endpoint {
                path: "/users".to_string(),
                method: HttpMethod::Get,
                summary: "List users".to_string(),
                description: String::new(),
                parameters: vec![Parameter {
                    name: "limit".to_string(),
                    location: ParameterLocation::Query,
                    description: "Max items".to_string(),
                    required: false,
                    schema_type: "integer".to_string(),
                }],
                request_body: None,
                responses: vec![Response {
                    status_code: "200".to_string(),
                    description: "Success".to_string(),
                    schema: None,
                }],
                tags: vec!["users".to_string()],
            }],
            schemas: vec![Schema {
                type_name: "User".to_string(),
                format: None,
                properties: vec![SchemaProperty {
                    name: "id".to_string(),
                    type_name: "integer".to_string(),
                    description: "User ID".to_string(),
                    required: true,
                }],
                required_fields: vec!["id".to_string()],
                enum_values: Vec::new(),
                description: "A user".to_string(),
            }],
        }
    }

    #[test]
    fn generates_non_empty_guide() {
        let guide = generate_guide(&sample_spec()).unwrap();
        assert!(!guide.is_empty());
    }

    #[test]
    fn guide_has_header() {
        let guide = generate_guide(&sample_spec()).unwrap();
        assert!(guide.contains("# TestAPI for AI Agents"));
    }

    #[test]
    fn guide_has_auth() {
        let guide = generate_guide(&sample_spec()).unwrap();
        assert!(guide.contains("## Authentication"));
        assert!(guide.contains("Bearer Token"));
    }

    #[test]
    fn guide_has_quick_reference() {
        let guide = generate_guide(&sample_spec()).unwrap();
        assert!(guide.contains("## Quick Reference"));
        assert!(guide.contains("GET"));
        assert!(guide.contains("/users"));
    }

    #[test]
    fn guide_has_endpoint_details() {
        let guide = generate_guide(&sample_spec()).unwrap();
        assert!(guide.contains("## Endpoint Reference"));
        assert!(guide.contains("| `limit`"));
    }

    #[test]
    fn guide_has_data_models() {
        let guide = generate_guide(&sample_spec()).unwrap();
        assert!(guide.contains("## Data Models"));
        assert!(guide.contains("### `User`"));
    }

    #[test]
    fn guide_has_curl_example() {
        let guide = generate_guide(&sample_spec()).unwrap();
        assert!(guide.contains("curl"));
        assert!(guide.contains("limit=<value>"));
    }
}
