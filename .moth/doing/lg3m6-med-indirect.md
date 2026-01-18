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

- **Compiled JavaScript in containers**: The TypeScript server files are compiled to JavaScript and mounted into containers. This avoids needing ts-node in the container and reduces complexity.

- **SELinux volume labels**: For Podman on Fedora, volumes need the `:z` suffix to allow container access due to SELinux.

- **Architecture binding at CDKTF level**: The `architectureBinding.bind()` call happens in the CDKTF stack, associating the architectural component with its deployment endpoint. This allows the binding to know where each service is deployed.

- **Docker DNS for service discovery**: Services communicate using container names as hostnames (e.g., `http://jsonstore:3001`). Docker/Podman DNS automatically resolves these.

- **CDKTF for deployment**: Using `cdktf deploy` with the Docker provider for deployment. The Docker provider connects to Podman's socket via `XDG_RUNTIME_DIR/podman/podman.sock`.

- **Database connection retry**: jsonstore-server has retry logic (30 retries, 1s delay) to wait for Postgres to become available, since CDKTF doesn't support health check dependencies like docker-compose.

- **Generic DockerApiServer**: Rather than hand-writing Express routes, DockerApiServer reads the ApiContainer's route definitions and automatically sets up Express routes. This ensures the runtime server matches the architectural specification.

- **StorageAdapter for pluggable storage**: The DockerApiServer accepts an optional StorageAdapter that handles storage operations. This allows different backends (Postgres, in-memory, etc.) without changing the server logic.

- **RemoteClient for cross-service calls**: DockerApiServer creates RemoteClient instances for all bound non-local components. This provides a typed client for making HTTP calls to other services.

### Decisions Rejected

- **ts-node in containers**: Initially tried running TypeScript directly with ts-node in containers, but path resolution issues made this unreliable. Compiling to JavaScript first is more robust.

- **docker-compose**: Initially used docker-compose/podman-compose, but switched to CDKTF for consistency with the architecture-as-code approach.

## Implementation Details

### Project Structure Changes

```
src/                        # Core library
├── docker-api-server.ts    # DockerApiServer, StorageAdapter, RemoteClient
├── binding.ts              # ArchitectureBinding for service discovery
├── api-container.ts        # ApiContainer base class
├── function.ts             # Function construct
├── architecture.ts         # Architecture construct
└── index.ts                # Exports all public API

example/
├── src/                    # Source code
│   ├── json-store.ts       # JsonStore architectural component (example-specific)
│   ├── architecture.ts     # Architecture definition
│   └── main.ts             # CDKTF stack with Docker provider
└── server/                 # Server code (runtime)
    ├── api-server.ts       # Express API server using DockerApiServer
    ├── jsonstore-server.ts # Express JsonStore server with Postgres StorageAdapter
    ├── tsconfig.json       # Server-specific TypeScript config
    └── dist/               # Compiled JavaScript
```

### How It Works

1. **JsonStore extends ApiContainer**: In the example, `JsonStore` extends `ApiContainer` from the core library and registers `POST /store/{collection}` and `GET /get/{collection}` as routes with corresponding Function handlers.

2. **ArchitectureBinding**: A new `ArchitectureBinding` class in the core library provides:
   - `bind(component, endpoint)` - associates an architectural component with a service endpoint
   - `getEndpoint(component)` - retrieves the endpoint for a component
   - `getAllBindings()` - returns all registered bindings
   - `setLocal(component)` / `isLocal(component)` - tracks which component is served locally
   - `createHttpWrapper(endpoint, route)` - creates a function that makes HTTP calls to the remote service

3. **DockerApiServer**: A generic class in the core library that constructs Express servers from ApiContainer definitions:
   - Takes an initialized and bound `ApiContainer` in its constructor
   - `createApp(express, storage?)` - creates an Express app with routes from the container's API definitions
   - `getRemoteClient(component)` - returns a `RemoteClient` for cross-service HTTP calls
   - Routes are automatically set up based on the container's route definitions
   - Path parameters (e.g., `{collection}`) are extracted and passed to handlers
   - POST/PUT body is passed as the last argument

4. **StorageAdapter**: Interface for pluggable storage backends:
   - `store(collection, document)` - stores a document in a collection
   - `get(collection)` - retrieves all documents from a collection
   - Used by DockerApiServer to delegate storage operations to concrete implementations (e.g., Postgres)

5. **RemoteClient**: HTTP client for calling remote ApiContainers:
   - `store(collection, document)` - POST to `/store/{collection}`
   - `get(collection)` - GET from `/get/{collection}`
   - `call(method, path, body?)` - generic HTTP call method

6. **Express servers**: Two separate Express servers:
   - `api-server.ts` - uses DockerApiServer to get RemoteClient for JsonStore, custom route for hello endpoint
   - `jsonstore-server.ts` - uses DockerApiServer with PostgresStorage adapter for automatic route handling

7. **CDKTF Docker deployment** (main.ts):
   - Creates Docker network for service communication
   - Deploys `postgres` container with health check
   - Deploys `jsonstore` container with volume mounts
   - Deploys `hello-api` container exposed on port 3000
   - All containers on shared network for DNS resolution

8. **npm scripts**:
   - `npm run deploy` - builds TypeScript and runs `cdktf deploy --auto-approve`
   - `npm run destroy` - runs `cdktf destroy --auto-approve`
