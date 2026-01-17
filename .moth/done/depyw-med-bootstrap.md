# Bootstrap - CDK Architecture Project

## Feature Specification

Create a TypeScript project that provides CDK primitives for defining conceptual event-driven solution architectures. This allows separating design from implementation (later done with CDKTF).

### Core Requirements

1. **Trivial implementation of ApiContainer and Function**
   - Use `constructs` npm package for the base Construct class
   - `Function` - represents a serverless function/handler with a callable handler
   - `ApiContainer` - represents an API that routes paths to Functions
   - `JsonStore` - represents a JSON document store
   - `Architecture` - root construct that synthesizes to a definition

2. **Example subdirectory with IDEA.md example**
   - Demonstrates the architecture definition from IDEA.md
   - Shows how to define an API with a hello endpoint

3. **CDKTF implementation using @cdktf/provider-docker**
   - Deploys the architecture as Docker containers
   - Uses DockerProvider from @cdktf/provider-docker

4. **Docker Compose for local running**
   - docker-compose.yml that runs the hello API locally
   - Compatible with podman-compose

### Decisions Taken

- **Use `constructs` package**: Instead of a custom Construct base class, use the standard `constructs` npm package that is shared by CDK and CDKTF. This ensures compatibility with the CDK ecosystem.

- **Trivial implementations**: The primitives are minimal stubs focused on architecture definition, not full implementations. The actual runtime behavior is provided by the CDKTF deployment.

- **JsonStore in-memory**: For the trivial implementation, JsonStore uses an in-memory Map. Real implementations would connect to actual databases.

- **Node.js inline server in Docker**: The Docker container runs a simple inline Node.js HTTP server that implements the hello endpoint. This avoids needing to build a separate container image.

### Decisions Rejected

- **Custom Construct class**: Initially created a custom Construct class, but switched to using the standard `constructs` package for ecosystem compatibility.

## Implementation Details

### Project Structure
```
cdk-arch/
├── src/                    # Core library
│   ├── index.ts           # Public exports
│   ├── architecture.ts    # Root Architecture construct
│   ├── api-container.ts   # ApiContainer construct
│   ├── function.ts        # Function construct
│   └── json-store.ts      # JsonStore construct
├── example/               # Example usage
│   ├── architecture.ts    # Architecture definition
│   ├── main.ts           # CDKTF stack with Docker provider
│   ├── docker-compose.yml # Local running
│   └── cdktf.json        # CDKTF configuration
├── package.json
└── tsconfig.json
```

### How It Works

1. **Architecture primitives** extend `Construct` from the `constructs` package. Each construct registers itself with its parent scope.

2. **Architecture.synth()** traverses all children and produces a JSON definition of the architecture components.

3. **CDKTF stack** creates Docker containers using @cdktf/provider-docker. The API container runs an inline Node.js server.

4. **docker-compose.yml** provides a simpler way to run the example locally without Terraform, using the same inline Node.js server pattern.
