# apiguard for AI Agents

Auto-generate agentic AI guides from API specifications

## Quick Reference

```bash
# Global options
apiguard --format <value>                   # Force a specific spec format (openapi3, swagger...
apiguard --output <value>                   # Output file path (stdout if not specified)
apiguard --output-format <value>            # Output format: md or json [default: md] [possib...
```

## Global Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-f, --format` | string | - | Force a specific spec format (openapi3, swagger2, graphql, grpc) |
| `-o, --output` | string | - | Output file path (stdout if not specified) |
| `--output-format` | string | md | Output format: md or json [default: md] [possible values: md, json] |

---
**Framework**: clap | **Version**: apiguard 0.1.0
