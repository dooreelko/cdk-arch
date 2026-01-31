# top level 
cdk-arch architectures should be used for service binding by all parties involved

## TODO

- ✅ create a simple web site using vite and react under example/web. a text input, a submit button and a list of hellos underneath
- ✅ cdk-arch has a transient dependency on node's crypto via constructs (https://github.com/aws/constructs/blob/10.x/src/private/uniqueid.ts), use vite-plugin-node-polyfills
- ✅ move httpHandler from local-docker to a cdk-arch under src/http-handler.ts
- ✅ web site should then import architecture and use architectureBinding.bind using the httpHandler
- ✅ create an e2e script that will start local-docker and run a vitest test suite against it making sure several hellos are submitted and shown
- ✅ web app tests should have a configuration option to use mocks or actual API (USE_REAL_API env var)

## Implementation Details

### Web Application Structure

Created a Vite + React web application under `packages/example/web` with:

- **Main Components**:
  - `App.tsx`: Main React component with form input and hellos list
  - `main.tsx`: Entry point for React application
  - `index.html`: HTML template
  - `index.css`: Basic styling
  - `architecture.ts`: Local architecture definition for the web app

- **Architecture Integration**:
  - Uses `architectureBinding.bind()` to connect to the local docker API endpoint
  - Uses `httpHandler` to create HTTP handlers for the `hello` and `hellos` routes
  - Imports architecture components from `@arinoto/cdk-arch`
  - Creates a local architecture that matches the local-docker architecture

- **Testing**:
  - Vitest test suite with React Testing Library
  - Test setup with JSDOM environment
  - Mock API handlers to avoid actual HTTP calls during testing
  - Tests for rendering, form submission, error handling, and component behavior

- **Build Configuration**:
  - Vite configuration with React plugin and node polyfills
  - TypeScript configuration for React with test file exclusion
  - Proxy setup for API calls during development
  - Rollup options to handle external dependencies properly

### HTTP Handler Migration

Moved `httpHandler` from `packages/example/local-docker/src/http-handler.ts` to `packages/cdk-arch/src/http-handler.ts`:

- Exported from main cdk-arch index.ts
- Maintains same functionality: creates HTTP handlers for API routes
- Used by web application to bind to local docker endpoints
- Added to cdk-arch package exports for broader usage
- Fixed circular dependency: imports directly from source modules instead of index.ts

### E2E Testing

Created comprehensive testing infrastructure:

1. **Unit Tests** (`npm run test`):
   - Runs Vitest with mocked API handlers
   - Tests component rendering, form submission, list updates
   - Fast feedback during development

2. **E2E Tests** (`npm run e2e` in web package):
   - Deploys local-docker services
   - Waits for API to be ready
   - Runs Vitest with `USE_REAL_API=true`
   - Tests against actual API endpoints

3. **Test Configuration**:
   - `USE_REAL_API` environment variable switches between mock and real API modes
   - Mock mode: Uses `vi.spyOn` to mock `handler.invoke()` methods
   - Real API mode: Uses actual httpHandler implementations

The test suite validates:
- Component rendering and user interface
- Form submission functionality
- API integration via architecture binding
- Error handling and edge cases
- Integration between web app and local-docker services

### Package Configuration

Updated `packages/cdk-arch/package.json`:
- Added ES module support for better Vite compatibility
- Ensured proper module resolution for both CommonJS and ES modules

Updated `packages/example/web/package.json`:
- Added React and testing dependencies
- Configured Vite and TypeScript for React development
- Added test scripts and e2e scripts

## Decisions Made

1. **Vite Configuration**: Used vite-plugin-node-polyfills to handle the crypto dependency from constructs library
2. **Proxy Setup**: Configured Vite proxy to route `/v1/api` requests to localhost:3000 during development
3. **Testing Approach**: Used Vitest with JSDOM for unit testing and React Testing Library for component testing
4. **Error Handling**: Added comprehensive error handling in the React component for API failures
5. **Type Safety**: Maintained TypeScript types throughout the application
6. **Mock/Real API Toggle**: `USE_REAL_API` env var allows same tests to run against mocks (fast unit tests) or real API (e2e tests)
7. **Circular Dependency Fix**: http-handler.ts imports directly from source modules to avoid ESM issues in browser

## Rejected Alternatives

1. **Create React App**: Chose Vite instead for faster builds and better developer experience
2. **Direct Fetch Calls**: Used architecture binding pattern instead of direct fetch calls to maintain architectural consistency
3. **Jest Testing**: Used Vitest instead of Jest for better integration with Vite ecosystem

