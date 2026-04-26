# Input Formats

ApiGuard supports four API specification formats. Format is auto-detected from file content by default.

---

## Auto-Detection

ApiGuard reads the file content and applies these heuristics in order:

| Format | Detection condition |
|--------|---------------------|
| OpenAPI 3.x | Content contains `openapi` AND `3.` |
| Swagger 2.0 | Content contains `swagger` AND `2.` |
| GraphQL | Content contains `type Query` OR `type Mutation` |
| gRPC Proto | Content contains `syntax` AND `proto` AND `service` |

If none match, ApiGuard exits with an error. Use `--format` to force a specific parser.

---

## OpenAPI 3.x

**File extensions:** `.json`, `.yaml`, `.yml`  
**`--format` value:** `openapi3` or `openapi`

Supports the full OpenAPI 3.x specification:
- All HTTP methods: GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS
- Path, query, header, and cookie parameters
- Request bodies with content type schemas
- Response schemas per status code
- `components/schemas` data models
- Security schemes: `apiKey`, `http` (Bearer/Basic), `oauth2`, `openIdConnect`
- Tags for endpoint grouping

**Example:**
```bash
apiguard openapi-spec.json
apiguard api.yaml --format openapi3
```

---

## Swagger 2.0

**File extensions:** `.json`, `.yaml`, `.yml`  
**`--format` value:** `swagger2` or `swagger`

Supports Swagger 2.0 (the predecessor to OpenAPI 3.x):
- `basePath` + `host` → base URL
- Path and query parameters
- `definitions` → data models
- `securityDefinitions`: `apiKey`, `basic`, `oauth2`

**Example:**
```bash
apiguard swagger2-spec.json
apiguard legacy-api.yaml --format swagger2
```

---

## GraphQL

**File extensions:** `.graphql`  
**`--format` value:** `graphql` or `gql`

Parses GraphQL SDL (Schema Definition Language):
- `type Query` → GET-equivalent endpoints
- `type Mutation` → POST/PUT/DELETE-equivalent endpoints
- Field arguments → parameters
- Return types and input types → schemas
- Scalar types, enums, and object types

**Example:**
```bash
apiguard schema.graphql
apiguard api.graphql --format graphql
```

---

## gRPC / Protobuf

**File extensions:** `.proto`  
**`--format` value:** `grpc`, `proto`, or `protobuf`

Parses `.proto` service definitions:
- `service` blocks → API groups
- `rpc` methods → endpoints
- `message` types → schemas
- Field types and repeated fields

**Example:**
```bash
apiguard service.proto
apiguard api.proto --format grpc
```

---

## File Size and Safety Limits

| Limit | Value |
|-------|-------|
| Maximum file size | 50 MB |
| Maximum JSON nesting depth | 128 levels |
| Maximum endpoints processed | 10,000 |
| Symlinks | Rejected |
