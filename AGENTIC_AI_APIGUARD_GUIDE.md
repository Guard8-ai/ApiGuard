# apiguard for AI Agents

Auto-generate agentic AI guides from API specifications

## Quick Reference

```bash
# Parse an OpenAPI 3.x spec → Markdown guide
apiguard openapi.yaml

# Parse a Swagger 2.0 spec → Markdown guide
apiguard swagger.json

# Parse a GraphQL schema
apiguard schema.graphql

# Parse a gRPC proto file
apiguard service.proto

# Force spec format (skip auto-detection)
apiguard spec.yaml --format openapi3

# Output to a file
apiguard openapi.yaml --output api-guide.md

# Output JSON IR (for WisdomGuard)
apiguard openapi.yaml --output-format json --output api_ir.json

# Pipe JSON IR into WisdomGuard
apiguard openapi.yaml --output-format json | wisdomguard /dev/stdin --project my-gcp-project
```

## Global Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-f, --format` | string | auto | Force a specific spec format (openapi3, swagger2, graphql, grpc) |
| `-o, --output` | string | stdout | Output file path (stdout if not specified) |
| `--output-format` | string | md | Output format: md or json [possible values: md, json] |

---

## Common Workflows

### Generate a Markdown guide from an OpenAPI spec

```bash
# Parse the spec and write the guide to a file
apiguard openapi.yaml --output docs/api-guide.md

# The guide contains: auth section, Quick Reference, Endpoint Reference, Data Models
cat docs/api-guide.md
```

### Generate an enhanced guide with WisdomGuard

```bash
# Step 1: produce the JSON IR
apiguard openapi.yaml --output-format json --output api_ir.json

# Step 2: enhance with Gemini (adds workflows, gotchas, error solutions)
wisdomguard api_ir.json \
  --project my-gcp-project \
  --output docs/api-guide-enhanced.md
```

### Pipe into WisdomGuard without an intermediate file

```bash
apiguard openapi.yaml --output-format json \
  | wisdomguard /dev/stdin \
      --project my-gcp-project \
      --output api-guide.md
```

### Parse a GraphQL schema

```bash
# Auto-detected from .graphql extension
apiguard schema.graphql --output graphql-guide.md

# Force format if using a non-standard extension
apiguard api-schema.txt --format graphql --output graphql-guide.md
```

### Generate guides for multiple specs in CI

```bash
for spec in openapi/*.yaml; do
  name=$(basename "$spec" .yaml)
  apiguard "$spec" \
    --output-format json \
    --output "ir/${name}.json"
done

for ir in ir/*.json; do
  name=$(basename "$ir" .json)
  wisdomguard "$ir" \
    --project "$GCP_PROJECT" \
    --output "docs/${name}-guide.md"
done
```

### Inspect the JSON IR schema

```bash
# Emit JSON and pretty-print the top-level fields
apiguard openapi.yaml --output-format json | jq 'keys'

# Count endpoints
apiguard openapi.yaml --output-format json | jq '.endpoints | length'

# List all endpoint paths
apiguard openapi.yaml --output-format json | jq '[.endpoints[] | "\(.method) \(.path)"]'
```

---

## Common Mistakes

| Wrong | Right | Why |
|-------|-------|-----|
| `apiguard spec.json` when the file is OpenAPI 3 but named `.json` | `apiguard spec.json --format openapi3` | Auto-detection uses file content heuristics; a `.json` extension alone doesn't distinguish OpenAPI 3 from Swagger 2 |
| Passing the JSON IR directly to `apiguard` a second time | Use `wisdomguard` for enhancement | `apiguard` parses *spec* formats (OpenAPI, GraphQL, gRPC) — it does not read its own IR output |
| `apiguard openapi.yaml > ir.json` without `--output-format json` | `apiguard openapi.yaml --output-format json > ir.json` | Without `--output-format json`, stdout is Markdown, not the IR that WisdomGuard expects |
| Writing output to `/etc/` or paths with `..` | Use relative paths or `--output ./output/guide.md` | Writes to system directories and path-traversal paths are blocked (exit code 1) |
| Relying on `--format` for `.proto` files with no proto content markers | Provide a valid `.proto` file or use `--format grpc` explicitly | gRPC parser expects `service` and `rpc` blocks; malformed proto produces an empty IR |

---

## Key Commands

- `apiguard <spec>` — parse spec, output Markdown guide to stdout
- `apiguard <spec> --output guide.md` — write guide to file
- `apiguard <spec> --output-format json` — emit JSON IR (for WisdomGuard)
- `apiguard <spec> --format openapi3` — force spec format, skip auto-detection
- `apiguard <spec> --output-format json | wisdomguard /dev/stdin --project <id>` — full pipeline, no temp file

---

## Error Messages

| Error / Exit Code | Meaning | Solution |
|-------------------|---------|---------|
| Exit code `1` | I/O error or invalid output path | Check the file exists, path has no `..`, and the target directory is writable |
| Exit code `2` | Parse error — could not read the spec | Validate the spec with `swagger-cli validate` or `spectral lint`; check for syntax errors |
| `Unsupported format: …` | Auto-detection failed | Pass `--format openapi3`, `--format swagger2`, `--format graphql`, or `--format grpc` explicitly |
| `No endpoints found` | Spec parsed but is empty | Confirm the spec has at least one path/operation; YAML anchors or `$ref` cycles can silently empty the tree |
| Output file is empty or only headers | GraphQL schema has only scalars/enums, no queries/mutations | Add `type Query` or `type Mutation` blocks to the schema |
| `Permission denied` writing output | File or directory not writable | Check directory permissions; output file is written with mode `0o600` |

---
**Framework**: clap | **Version**: apiguard 0.1.0
