we need to restructure the code to have smallest-possible runtime packages

- create packages/ directory and move cdk-arch and example under it
- split example into separate packages:
	- architecture
	- local-docker
	- cloudflare

for cloudflare return to use cdk-arch, cloudflare's workers
have crypto polyfill: https://developers.cloudflare.com/workers/runtime-apis/nodejs/crypto/

## Decisions

### Runtime code placement
- **Worker adapter (createWorkerHandler, serviceBindingHandler)**: Keep in packages/example/cloudflare as minimal adapter
- **Docker runtime (DockerApiServer, httpHandler)**: Keep in packages/example/local-docker, not in cdk-arch

The goal is minimal runtime packages - deployment-specific code stays with its deployment package.

### Cloudflare uses cdk-arch
Cloudflare workers now use cdk-arch directly:
- Import `architectureBinding` from cdk-arch to set up overloads
- Import architecture components (`api`, `jsonStore`) from architecture package
- Use `worker-adapter.ts` for minimal Worker-specific code (createWorkerHandler, serviceBindingHandler)
- Cloudflare Workers support Node.js crypto API via polyfill: https://developers.cloudflare.com/workers/runtime-apis/nodejs/crypto/

## Target Structure

```
packages/
├── cdk-arch/           # Core library (constructs only)
│   ├── src/
│   │   ├── index.ts
│   │   ├── architecture.ts
│   │   ├── function.ts
│   │   ├── api-container.ts
│   │   └── binding.ts
│   ├── package.json
│   └── tsconfig.json
│
└── example/            # Example deployments
    ├── architecture/   # Shared architecture definition
    │   ├── src/
    │   │   ├── index.ts
    │   │   ├── architecture.ts
    │   │   └── json-store.ts
    │   ├── package.json
    │   └── tsconfig.json
    │
    ├── local-docker/   # Docker deployment
    │   ├── src/
    │   │   ├── main.ts (terraform synth)
    │   │   ├── terraform.ts
    │   │   ├── docker-api-server.ts
    │   │   ├── http-handler.ts
    │   │   ├── Dockerfile
    │   │   └── entrypoints/
    │   │       ├── api-server.ts
    │   │       └── jsonstore-server.ts
    │   ├── e2e.sh
    │   ├── package.json
    │   ├── cdktf.json
    │   └── tsconfig.json
    │
    └── cloudflare/     # Cloudflare deployment
        ├── src/
        │   ├── main.ts (terraform synth)
        │   ├── terraform.ts
        │   ├── worker-adapter.ts (minimal Worker adapter using cdk-arch)
        │   └── entrypoints/
        │       ├── api-worker.ts
        │       └── jsonstore-worker.ts
        ├── scripts/
        │   └── bundle-workers.js
        ├── e2e.sh
        ├── package.json
        ├── cdktf.json
        └── tsconfig.json
```

## Implementation Details

### Package dependencies:
- `cdk-arch`: depends on `constructs`
- `architecture`: depends on `cdk-arch`
- `local-docker`: depends on `architecture`, `cdk-arch`, cdktf providers, express, pg
- `cloudflare`: depends on `architecture`, `cdk-arch`, cdktf providers

### Workspace setup:
- Use npm workspaces at repository root with paths:
  - packages/cdk-arch
  - packages/example/architecture
  - packages/example/local-docker
  - packages/example/cloudflare
- Local package references via workspace protocol (`"cdk-arch": "*"`)

### Version constraints:
- cdktf: ^0.20.0 (required by provider packages)
- cdktf-cli: ^0.20.0
- @cdktf/provider-docker: ^11.0.0
- @cdktf/provider-cloudflare: ^11.0.0
- @cdktf/provider-null: ^10.0.0 (requires cdktf ^0.20.0)

### Build order (handled by npm workspaces):
1. cdk-arch (no deps on other local packages)
2. architecture (depends on cdk-arch)
3. local-docker, cloudflare (depend on cdk-arch, architecture)

### API endpoints:
- `GET /v1/api/hello/{name}` - Greets the given name and stores the greeting
- `GET /v1/api/hellos` - Returns all stored greetings (name and timestamp)

### E2E testing:
Both deployments have `e2e.sh` scripts that:
1. Deploy the stack
2. Call `/v1/api/hello/E2ETest` to create a greeting
3. Call `/v1/api/hellos` to verify the greeting was stored
4. Clean up by destroying the stack

