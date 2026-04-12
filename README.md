# ApiGuard

Auto-generate agentic AI guides from API specifications.

## Install

```bash
cargo install --path .
```

## Usage

```bash
# Auto-detect spec format, output markdown
apiguard openapi-spec.json

# Force specific format
apiguard schema.graphql --format graphql

# JSON IR output (for WisdomGuard pipeline)
apiguard spec.json --output-format json -o api-ir.json

# Write to file
apiguard petstore.yaml -o AGENTIC_AI_PETSTORE_GUIDE.md
```

## Supported Spec Formats

| Format | Detection | File Types |
|--------|-----------|------------|
| OpenAPI 3.x | `openapi` + `3.` | JSON, YAML |
| Swagger 2.0 | `swagger` + `2.` | JSON, YAML |
| GraphQL | `type Query` / `type Mutation` | .graphql |
| gRPC Proto | `syntax` + `proto` + `service` | .proto |

## Output Formats

- **Markdown** (`--output-format md`, default) — agentic guide with endpoints, auth, curl examples
- **JSON** (`--output-format json`) — structured IR for WisdomGuard enhancement

## Pipeline

```bash
# Generate base guide + enhance with VertexAI
apiguard spec.json -o api-guide.md
apiguard spec.json --output-format json -o api-ir.json
wisdomguard api-ir.json --base-guide api-guide.md -o AGENTIC_AI_API_GUIDE.md
```

## License

MIT — Guard8.ai
