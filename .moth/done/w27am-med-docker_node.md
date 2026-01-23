# Docker Node Bundling

## Feature Specification

Instead of copying the entire monorepo to the Docker container and building inside, add an esbuild build step (like in the cloudflare example) to pre-bundle entrypoints before Docker build.

### Requirements

1. **Esbuild Bundling Step**: Create a build script that bundles entrypoints into self-contained JS files
   - Bundle `api-server.ts` and `jsonstore-server.ts` to `dist/docker/` directory
   - Include all dependencies in the bundle (no need for `node_modules` in container)

2. **Simplified Dockerfile**: Replace the multi-package copy approach with a simple copy of bundled files
   - Use Node.js runtime instead of Bun (consistent with bundled output)
   - Much smaller Docker context (only bundled JS files needed)
   - Faster builds (no `npm install` or TypeScript compilation in container)

3. **Node.js Runtime**: Switch from Bun to Node.js for container runtime
   - Use `node:20-alpine` as base image
   - Run bundled JS files with `node`

4. **Name-based Route Overloads**: Changed `addRoute` API to accept a route name as first argument, enabling name-based overload binding instead of path-based.

### Decisions Taken

- **Node.js over Bun**: Switching to Node.js runtime since we're bundling anyway. This is more consistent with the cloudflare example approach and Node.js has broader production deployment support.

- **Platform: node**: Use `platform: 'node'` in esbuild config (vs cloudflare's `platform: 'browser'`) since we're targeting Node.js runtime.

- **Output Directory**: Using `dist/docker/` subdirectory for bundled files, mirroring cloudflare's `dist/cloudflare/` pattern.

- **Separate Bundle Script**: Create `scripts/bundle-servers.js` following the cloudflare pattern rather than inlining in package.json.

- **Name-based Route Overloads**: Using route names instead of paths for overloads decouples the binding from the exact HTTP path. This prevents bugs where overload paths don't match route definitions.

### Decisions Rejected

- **Keep Bun Runtime**: Could work with bundled files but adds complexity for no clear benefit. Node.js is more widely understood and deployed.

- **Keep Multi-Package Copy**: The original approach works but is slower and results in larger images. Bundling is cleaner.

## Implementation Details

### Build Script

`scripts/bundle-servers.js` uses esbuild to bundle both server entrypoints. Configuration targets Node.js platform with ESM format.

### Dockerfile Changes

Simplified from multi-stage build with full monorepo copy to a minimal Dockerfile that only copies bundled JS files and runs with Node.js.

### Build Process

1. `npm run build` - TypeScript compilation (for type checking)
2. `npm run build:docker` - esbuild bundles servers into `dist/docker/`
3. `cdktf deploy` - Terraform uses bundled files

### Package.json Changes

- Added `esbuild` dependency
- Added `build:docker` script
- Updated `deploy` script to run bundling first

### cdk-arch Changes

- `ApiContainer.addRoute(name, path, handler)` - accepts name as first argument
- `ApiContainer.getRouteByName(name)` - lookup route by name
- `ApiContainer.listNamedRoutes()` - list all named routes
- `ArchitectureBinding.bind()` - resolves overloads by route name only, throws error if route not found

### httpHandler Changes

Updated signature: `httpHandler(endpoint, container, routeName)` - looks up the route path from the container's registry instead of requiring it as a parameter. This ensures the correct path is always used.
