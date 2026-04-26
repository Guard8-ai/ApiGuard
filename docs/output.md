# Output Formats

ApiGuard produces two output formats: Markdown (default) and JSON IR.

---

## Markdown Guide (`--output-format md`)

The default output. A structured Markdown document containing everything an AI agent needs to call your API.

### Sections

| Section | Content |
|---------|---------|
| `# <API Name> for AI Agents` | Name, version, description, base URL |
| `## Authentication` | One subsection per auth method with curl examples |
| `## Quick Reference` | All endpoints in a compact bash block, grouped by tag |
| `## Endpoint Reference` | Per-endpoint: parameters table, request body, responses, curl example |
| `## Data Models` | Per-schema: properties table, enum values, required fields |
| Footer | `**Format**: <spec> \| **Version**: <version>` |

### Authentication Section Examples

**API Key:**
```markdown
## Authentication

### API Key
Include your API key in the `X-Api-Key` header:
```bash
curl -H "X-Api-Key: YOUR_KEY" https://api.example.com/endpoint
```
```

**Bearer Token:**
```markdown
### Bearer Token
Include your token in the `Authorization` header:
```bash
curl -H "Authorization: Bearer YOUR_TOKEN" https://api.example.com/endpoint
```
```

### Quick Reference Example

```markdown
## Quick Reference

```bash
# Users
GET    /users              # List users
POST   /users              # Create user
GET    /users/{id}         # Get user
PUT    /users/{id}         # Update user
DELETE /users/{id}         # Delete user

# Orders
GET    /orders             # List orders
POST   /orders             # Create order
```
```

---

## JSON IR (`--output-format json`)

The structured Intermediate Representation. Used as input for WisdomGuard enhancement. Written as pretty-printed JSON.

### Top-Level Schema

```json
{
  "name": "Petstore API",
  "version": "1.0.0",
  "description": "A sample API",
  "base_url": "https://petstore.example.com/v1",
  "spec_format": "openapi3",
  "auth_methods": [...],
  "endpoints": [...],
  "schemas": [...]
}
```

### `auth_methods` Array

Each element is one of:

```json
{ "type": "ApiKey", "location": "header", "name": "X-Api-Key" }
{ "type": "Bearer" }
{ "type": "OAuth2", "flows": ["authorization_code", "client_credentials"] }
{ "type": "Basic" }
{ "type": "None" }
```

### `endpoints` Array

```json
{
  "path": "/users/{id}",
  "method": "GET",
  "summary": "Get user by ID",
  "description": "Returns a single user",
  "parameters": [
    {
      "name": "id",
      "location": "path",
      "description": "User ID",
      "required": true,
      "schema_type": "string"
    }
  ],
  "request_body": null,
  "responses": [
    { "status_code": "200", "description": "Success", "schema_ref": "User" },
    { "status_code": "404", "description": "Not found", "schema_ref": null }
  ],
  "tags": ["users"]
}
```

**`method` values:** `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`  
**`location` values:** `path`, `query`, `header`, `cookie`

### `schemas` Array

```json
{
  "type_name": "User",
  "properties": [
    { "name": "id", "type": "integer", "description": "Unique user ID", "required": true },
    { "name": "email", "type": "string", "description": "Email address", "required": true },
    { "name": "role", "type": "string", "description": "User role", "required": false }
  ],
  "required_fields": ["id", "email"],
  "enum_values": []
}
```

---

## Output File Permissions

When `--output` is specified, the file is written with mode `0o600` (owner read/write only). Writes to system directories (`/etc`, `/dev`, `/proc`, `/sys`, `/boot`) and paths containing `..` are blocked.
