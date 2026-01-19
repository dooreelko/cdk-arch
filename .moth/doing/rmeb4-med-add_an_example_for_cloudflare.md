# Cloudflare Example

## Feature Specification

Use Cloudflare Workers for the API and KV for the JSON store. Differentiate what to deploy (local docker, cloudflare, aws etc) using an ENV flag.

### Requirements

1. **Platform Selection via ENV**: Use `PLATFORM` environment variable to select deployment target
   - `docker` (default): Uses existing Docker-based deployment
   - `cloudflare`: Uses Cloudflare Workers + KV

2. **Cloudflare Workers for API**: Deploy the API as a Cloudflare Worker
   - Handle HTTP routing and parameter extraction
   - Support the same route definitions as Docker example

3. **Cloudflare KV for JsonStore**: Use KV namespace for document storage
   - Store documents with `{collection}:{uuid}` key pattern
   - List and retrieve documents by collection prefix

4. **Separate Workers Architecture**:
   - API Worker: Handles `/v1/api/hello/{name}` route
   - JsonStore Worker: Handles `/store/{collection}` and `/get/{collection}` routes
   - Communication via Cloudflare Service Bindings

### Decisions Taken

- **Separate Workers vs Single Worker**: Chose separate Workers with service bindings to mirror the Docker architecture where API and JsonStore run in separate containers. This maintains architectural consistency across platforms.

- **KV Key Pattern**: Using `{collection}:{uuid}` pattern for KV keys, allowing prefix-based listing per collection.

- **Worker Bundling**: Using esbuild to bundle Workers into single ESM files that can be deployed via CDKTF.

- **Minimal Worker Runtime**: Created a lightweight `worker-runtime.ts` instead of using `cdk-arch` directly, because the `constructs` library has Node.js dependencies (crypto) that don't work in Cloudflare Workers runtime. The worker-runtime provides equivalent functionality: WorkerFunction with overload support and WorkerRouter for request handling.

### Decisions Rejected

- **Single Worker**: Would be simpler but doesn't demonstrate the service-binding pattern and architectural separation.

- **Using cdk-arch in Workers**: The `constructs` library dependency requires Node.js crypto module which isn't available in Workers. The worker-runtime abstraction provides the same patterns without Node.js dependencies.

## Implementation Details

### Platform Selection

`example/src/main.ts` uses `PLATFORM` env var with conditional require to load the appropriate terraform module. Default is `docker`.

### Cloudflare Terraform Stack

`example/src/cloudflare/terraform.ts` - CDKTF stack creates:
- KV namespace (`hello-world-jsonstore`) for JsonStore data
- JsonStore Worker with KV binding (`JSONSTORE_KV`)
- API Worker with service binding (`JSONSTORE`) to JsonStore Worker

Requires `CLOUDFLARE_ACCOUNT_ID` environment variable.

### Worker Runtime

`example/src/cloudflare/worker-runtime.ts` - Minimal Worker-compatible runtime:
- `WorkerFunction`: Function wrapper with overload support (mirrors cdk-arch Function)
- `WorkerRouter`: Request routing with path parameter extraction
- `serviceBindingHandler`: Creates handlers for calling other Workers via service bindings

### Worker Entrypoints

**jsonstore-worker.ts**:
- Uses WorkerRouter with routes for store/get operations
- KV implementations accessed via `env.JSONSTORE_KV` binding
- Uses request-scoped env variable pattern

**api-worker.ts**:
- Uses WorkerRouter with `/v1/api/hello/{name}` route
- Calls JsonStore via service binding handler
- HelloFunction stores greeting then returns message

### Build Process

1. `bun run build` - TypeScript compilation via `tsc`
2. `node scripts/bundle-workers.js` - esbuild bundles Workers into `dist/cloudflare/`
3. `PLATFORM=cloudflare cdktf deploy` - Terraform reads bundled JS and deploys

### Package Dependencies Added

- `@cdktf/provider-cloudflare`: Cloudflare CDKTF provider
- `@cloudflare/workers-types`: TypeScript types for Workers
- `esbuild`: Bundling Workers for deployment
