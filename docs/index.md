# ApiGuard

**Auto-generate agentic AI guides from API specifications.**

ApiGuard reads an API spec file (OpenAPI, Swagger, GraphQL, or gRPC) and produces a structured Markdown guide — or a JSON Intermediate Representation (IR) — purpose-built for AI agents to consume.

---

## What It Does

| Input | Output |
|-------|--------|
| OpenAPI 3.x (`.json`, `.yaml`) | Markdown guide with endpoints, auth, curl examples |
| Swagger 2.0 (`.json`, `.yaml`) | JSON IR for WisdomGuard enhancement |
| GraphQL schema (`.graphql`) | |
| gRPC Proto (`.proto`) | |

The generated Markdown guide gives an AI agent everything it needs to call your API correctly: authentication patterns, every endpoint with parameters and response codes, data model schemas, and ready-to-run curl examples.

---

## Quick Start

```bash
# Install
cargo install --path .

# Generate a guide from an OpenAPI spec
apiguard openapi-spec.json

# Write to file
apiguard petstore.yaml -o AGENTIC_AI_PETSTORE_GUIDE.md

# Full pipeline with WisdomGuard enhancement
apiguard spec.json -o guide.md
apiguard spec.json --output-format json -o ir.json
wisdomguard ir.json --base-guide guide.md -o AGENTIC_AI_API_GUIDE.md
```

---

## Navigation

- [Installation](installation.md)
- [Usage & CLI Reference](usage.md)
- [Input Formats](formats.md)
- [Output Formats & IR Schema](output.md)
- [Pipeline Integration](pipeline.md)
