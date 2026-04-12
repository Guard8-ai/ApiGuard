mod generator;
mod ir;
mod parser;
pub mod security;

use crate::ir::SpecFormat;
use crate::parser::ParserRegistry;
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "apiguard", version, about = "Auto-generate agentic AI guides from API specifications")]
struct Cli {
    /// Path to the API spec file (`OpenAPI`, Swagger, `GraphQL`, or Proto)
    spec_file: PathBuf,

    /// Force a specific spec format (openapi3, swagger2, graphql, grpc)
    #[arg(short, long)]
    format: Option<String>,

    /// Output file path (stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format: md or json
    #[arg(long, default_value = "md")]
    output_format: OutputFormat,
}

#[derive(Clone, clap::ValueEnum)]
enum OutputFormat {
    Md,
    Json,
}

fn parse_spec_format(name: &str) -> Result<SpecFormat> {
    match name.to_lowercase().as_str() {
        "openapi3" | "openapi" => Ok(SpecFormat::OpenApi3),
        "swagger2" | "swagger" => Ok(SpecFormat::Swagger2),
        "graphql" | "gql" => Ok(SpecFormat::GraphQl),
        "grpc" | "proto" | "protobuf" => Ok(SpecFormat::GrpcProto),
        other => anyhow::bail!("Unknown spec format: {other}. Supported: openapi3, swagger2, graphql, grpc"),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Atomically load spec file (symlink check + size check + read in one operation)
    let content = security::load_spec_safe(&cli.spec_file)
        .context("Could not load spec file")?;

    // Validate JSON depth if content looks like JSON
    if content.trim_start().starts_with('{') || content.trim_start().starts_with('[') {
        security::validate_json_depth(&content)
            .context("Spec file validation failed")?;
    }

    let registry = ParserRegistry::new();

    // Parse
    let spec = if let Some(ref fmt_name) = cli.format {
        let format = parse_spec_format(fmt_name)?;
        registry.parse_with_format(&format, &content)?
    } else {
        registry.detect_and_parse(&content)?
    };

    // Generate output
    let output = match cli.output_format {
        OutputFormat::Json => serde_json::to_string_pretty(&spec)
            .context("Failed to serialize to JSON")?,
        OutputFormat::Md => generator::generate_guide(&spec)
            .context("Failed to generate guide")?,
    };

    // Write output
    if let Some(ref path) = cli.output {
        security::write_output_safe(path, &output)
            .context("Failed to write output file")?;
        eprintln!("Guide written to output file");
    } else {
        print!("{output}");
    }

    Ok(())
}
