# Indirect - Postgres and Service Discovery

## Feature Specification

Extend the example to include Postgres and proper service communication between containers via HTTP.

### Core Requirements

1. **Add Postgres container and network**
   - Postgres container for JsonStore data persistence
   - Docker network for service-to-service communication

2. **Move API server code to separate file**
   - Use Express for the HTTP server
   - Separate files for api-server and jsonstore-server

3. **JsonStore extends ApiContainer**
   - JsonStore declares store and get as HTTP APIs (routes)
   - Routes: `POST /store/{collection}` and `GET /get/{collection}`

4. **Use actual helloFunction code**
   - The hello endpoint calls the real function logic
   - Function stores greeting data via JsonStore

5. **CDKTF-architecture association**
   - When a CDKTF component is instantiated, it associates with its architectural counterpart
   - `ArchitectureBinding` class provides bind/getEndpoint methods

6. **HTTP wrapper for out-of-process calls**
   - Since services run in separate containers, function calls become HTTP calls
   - Service discovery via Docker DNS (container names as hostnames)

### Decisions Taken

- **JsonStore is example-specific**: JsonStore is not a generic architectural primitive - it's specific to this example. It lives in the example directory, not the core library.

- **JsonStore exposes HTTP endpoints**: Instead of just being method calls, JsonStore exposes its store/get operations as HTTP routes. This allows other services to call it over the network.

- **Express for servers**: Both api-server and jsonstore-server use Express for HTTP handling. This provides a clean, well-known API for routing and middleware.

- **Bun runtime with TypeScript**: Containers use Bun which runs TypeScript directly. The cdk-arch library is built to dist/ for proper module resolution, then linked via `bun link`.

- **bun link for local dependencies**: Using `bun link` instead of `file:..` in package.json avoids recursive symlink issues where bun would endlessly nest the example directory inside node_modules/cdk-arch/example/node_modules/cdk-arch/...

- **SELinux volume labels**: For Podman on Fedora, volumes need the `:z` suffix to allow container access due to SELinux.

- **Architecture binding at CDKTF level**: The `architectureBinding.bind()` call happens in the CDKTF stack, associating the architectural component with its deployment endpoint. This allows the binding to know where each service is deployed.

- **Docker DNS for service discovery**: Services communicate using container names as hostnames (e.g., `http://jsonstore:3001`). Docker/Podman DNS automatically resolves these.

- **CDKTF for deployment**: Using `cdktf deploy` with the Docker provider for deployment. The Docker provider connects to Podman's socket via `XDG_RUNTIME_DIR/podman/podman.sock`.

- **Database connection retry**: jsonstore-server has retry logic (30 retries, 1s delay) to wait for Postgres to become available, since CDKTF doesn't support health check dependencies like docker-compose.

- **DockerApiServer is example-specific**: DockerApiServer lives in the example directory, not the core library. It's one possible implementation for serving an ApiContainer via Express.

- **TBDFunction for interface contracts**: TBDFunction allows defining API contracts (routes) without implementations. The actual implementation is provided via overloads at runtime. This separates architecture definition from deployment-specific implementation.

- **httpHandler for remote function calls**: The httpHandler helper creates function handlers that make HTTP calls to remote services. This allows the same architectural code to work whether the function is local or remote.

### Decisions Rejected

- **ts-node in containers**: Initially tried ts-node, but path resolution issues made it unreliable. Switched to Bun which handles TypeScript natively.

- **docker-compose**: Initially used docker-compose/podman-compose, but switched to CDKTF for consistency with the architecture-as-code approach.

- **file:.. dependency in Docker**: Using `"cdk-arch": "file:.."` in package.json causes bun to create recursive symlinks when the parent package contains the example directory. This results in infinitely nested paths like `node_modules/cdk-arch/example/node_modules/cdk-arch/...`. Using `bun link` avoids this issue.

## Implementation Details

### Project Structure Changes

```
src/                        # Core library
├── binding.ts              # ArchitectureBinding for service discovery
├── api-container.ts        # ApiContainer base class
├── function.ts             # Function construct (includes TBDFunction)
├── architecture.ts         # Architecture construct
└── index.ts                # Exports all public API

example/
├── src/                    # Source code
│   ├── json-store.ts       # JsonStore with TBDFunction placeholders
│   ├── architecture.ts     # Architecture definition (api, jsonStore, helloFunction)
│   ├── main.ts             # Entry point that calls synth_terraform()
│   └── docker/             # Docker-specific code
│       ├── Dockerfile      # Multi-stage build with bun link
│       ├── terraform.ts    # CDKTF stack with Docker provider
│       ├── docker-api-server.ts  # Express server from ApiContainer routes
│       ├── http-handler.ts # HTTP wrapper for remote function calls
│       └── entrypoints/    # Container entry points
│           ├── api-server.ts       # API container entrypoint
│           └── jsonstore-server.ts # JsonStore container with Postgres
├── e2e.sh                  # End-to-end test script
└── package.json            # Example dependencies
```

### How It Works

1. **TBDFunction for placeholders**: `TBDFunction` is a Function that throws an error if invoked without an overload. JsonStore uses TBDFunction for store/get operations - these must be overloaded with actual implementations before use.

2. **Function overloading via ArchitectureBinding**: The `bind()` method accepts an `overloads` option that provides implementations for TBDFunction placeholders:
   ```typescript
   architectureBinding.bind(jsonStore, {
     host: 'jsonstore', port: 3001,
     overloads: {
       storeFunction: postgresStore,
       getFunction: postgresGet
     }
   });
   ```

3. **ArchitectureBinding**: Core library class for service discovery:
   - `bind(component, options)` - associates component with endpoint and applies function overloads
   - `getEndpoint(component)` - retrieves the endpoint for a component
   - `setLocal(component)` / `isLocal(component)` - tracks which component is served locally

4. **DockerApiServer**: Example-specific class that constructs Express servers from ApiContainer routes:
   - Reads route definitions from ApiContainer
   - Automatically sets up Express routes with parameter extraction
   - Invokes the Function's `invoke()` method (which uses overload if set)

5. **httpHandler for remote calls**: When a service needs to call another service, httpHandler creates a function that makes HTTP calls:
   ```typescript
   overloads: {
     storeFunction: httpHandler(jsonStoreEndpoint, 'POST /store/{collection}'),
     getFunction: httpHandler(jsonStoreEndpoint, 'GET /get/{collection}')
   }
   ```

6. **Container entrypoints**: Each container has its own entrypoint that:
   - Binds architectural components to endpoints
   - Provides overloads (local implementations or HTTP wrappers)
   - Creates and starts a DockerApiServer

7. **CDKTF Docker deployment** (terraform.ts):
   - Creates Docker network for service communication
   - Builds app image from Dockerfile
   - Deploys postgres, jsonstore, and hello-api containers
   - All containers on shared network for DNS resolution

8. **e2e.sh test script**:
   - Deploys the stack with `npm run deploy`
   - Tests API endpoint returns correct response
   - Verifies data was stored in Postgres
   - Cleans up with `npm run destroy`
