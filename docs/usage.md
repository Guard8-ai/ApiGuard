# Usage & CLI Reference

## Synopsis

```
apiguard <spec_file> [OPTIONS]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<spec_file>` | Path to the API spec file. Supported: `.json`, `.yaml`, `.yml`, `.graphql`, `.proto` |

## Options

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--format <FORMAT>` | `-f` | auto-detect | Force input spec format (see below) |
| `--output <FILE>` | `-o` | stdout | Write output to file instead of stdout |
| `--output-format <FMT>` | — | `md` | Output format: `md` (Markdown) or `json` (IR) |
| `--help` | `-h` | — | Print help |
| `--version` | `-V` | — | Print version |

### `--format` values

| Value | Aliases | Spec type |
|-------|---------|-----------|
| `openapi3` | `openapi` | OpenAPI 3.x |
| `swagger2` | `swagger` | Swagger 2.0 |
| `graphql` | `gql` | GraphQL schema |
| `grpc` | `proto`, `protobuf` | gRPC Protobuf |

Omit `--format` to let ApiGuard auto-detect from file content — see [Input Formats](formats.md).

## Examples

```bash
# Auto-detect format, print Markdown to stdout
apiguard openapi-spec.json

# Write Markdown guide to file
apiguard petstore.yaml -o AGENTIC_AI_PETSTORE_GUIDE.md

# Force GraphQL parser
apiguard schema.graphql --format graphql

# Generate JSON IR (for WisdomGuard pipeline)
apiguard spec.json --output-format json -o api-ir.json

# Force format + JSON IR in one step
apiguard spec.yaml --format openapi3 --output-format json -o ir.json
```

## Limits

| Limit | Value |
|-------|-------|
| Max file size | 50 MB |
| Max JSON nesting depth | 128 levels |
| Max endpoints processed | 10,000 |
| Max description length | 500 characters (truncated) |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error (bad arguments, unsupported format, parse failure) |
| `2` | File/security error (symlink input, path traversal, system directory) |

## Security Notes

- Symlink inputs are rejected (TOCTOU-safe)
- Output files are written with mode `0o600`
- Writes to `/etc`, `/dev`, `/proc`, `/sys`, `/boot` are blocked
- `..` in output paths is rejected
- All descriptions are markdown- and shell-escaped in generated output
