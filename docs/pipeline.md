# ApiGuard in the Guard8.ai Pipeline

ApiGuard is the first stage of the Guard8.ai tool chain. It converts API specs into a structured JSON IR that WisdomGuard can enhance with LLM-generated insights.

---

## Pipeline Overview

```
API Spec File          ApiGuard            WisdomGuard
(OpenAPI / Swagger  ──────────────►  JSON IR  ──────────────►  Enhanced Markdown
 GraphQL / gRPC)                     (ApiSpec)                  Guide
```

1. **ApiGuard** parses the spec file → produces a JSON IR (`ApiSpec`) or a Markdown guide
2. **WisdomGuard** reads the JSON IR → calls VertexAI Gemini → produces enriched Markdown

---

## Step-by-Step Example

### Step 1 — Generate the JSON IR

```bash
apiguard openapi.yaml --output-format json --output api_ir.json
```

`api_ir.json` now contains the `ApiSpec` structure:

```json
{
  "name": "Petstore API",
  "version": "1.0.0",
  "spec_format": "openapi3",
  "endpoints": [...],
  "schemas": [...]
}
```

### Step 2 — Enhance with WisdomGuard

```bash
wisdomguard api_ir.json \
  --project my-gcp-project \
  --output petstore-guide.md
```

`petstore-guide.md` contains:
- **Common Workflows** — realistic multi-step request sequences
- **Common Mistakes** — wrong/right/why table of API usage errors
- **Error Messages** — HTTP error codes mapped to solutions

### Step 3 (optional) — Merge into an existing Markdown guide

If you already have a handwritten `api-guide.md`, WisdomGuard can inject the enhancements at the right positions rather than replacing it:

```bash
wisdomguard api_ir.json \
  --base-guide api-guide.md \
  --project my-gcp-project \
  --output api-guide-enhanced.md
```

---

## One-Liner Pipe

```bash
apiguard openapi.yaml --output-format json | wisdomguard /dev/stdin --project my-gcp-project
```

---

## CI / Automation Example

```yaml
# .github/workflows/docs.yml
- name: Generate API guide
  run: |
    apiguard openapi.yaml --output-format json --output api_ir.json
    wisdomguard api_ir.json \
      --project ${{ vars.GCP_PROJECT }} \
      --output docs/api-guide.md
  env:
    GOOGLE_APPLICATION_CREDENTIALS: ${{ secrets.GCP_SA_KEY_PATH }}
```

---

## Skipping WisdomGuard

ApiGuard can produce a complete Markdown guide on its own — without calling any LLM:

```bash
# Full Markdown guide, no LLM needed
apiguard openapi.yaml --output api-guide.md
```

The standalone guide contains every endpoint, parameter, request body, response schema, and authentication method. It is useful when you do not have a GCP project or want a deterministic, cost-free output.

---

## Output Comparison

| Mode | Command | Contains |
|------|---------|---------|
| Markdown only | `apiguard spec.yaml` | Endpoint/schema reference, auth section, Quick Reference |
| JSON IR only | `apiguard spec.yaml --output-format json` | Machine-readable `ApiSpec` struct for tooling |
| Enhanced Markdown | `apiguard spec.yaml --output-format json \| wisdomguard /dev/stdin --project …` | All of above + Workflows, Gotchas, Error Messages |
| Merged | `wisdomguard ir.json --base-guide guide.md --project …` | Existing guide with enhancements injected at correct positions |

---

## Related

- [Output Formats](output.md) — full `ApiSpec` JSON schema
- [Supported Formats](formats.md) — which spec types ApiGuard accepts
- [WisdomGuard docs](../WisdomGuard/docs/index.md) — full WisdomGuard reference
