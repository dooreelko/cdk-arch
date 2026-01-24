# No Name In RouteEntry

## Feature Specification

Remove the `name` field from the `RouteEntry` interface.

### Core Requirements

1. **Remove `name` field from `RouteEntry` interface**
   - The field is redundant since the name is already used as the key in `ApiRoutes[name]`

2. **Update `addRoute` method**
   - Remove `name` from the object stored in `this.routes[name]`

### Rationale

The `name` field duplicates information already present in the data structure:
- `ApiRoutes` is defined as `{ [name: string]: RouteEntry }`
- When calling `addRoute(name, path, handler)`, the `name` becomes the dictionary key
- Code consuming routes (DockerApiServer, worker-adapter) only accesses `entry.path` and `entry.handler`
- If the name is needed, it's available via `Object.entries(routes)` or `Object.keys(routes)`

### Decisions Taken

- **Name accessible via dictionary key**: Consumers needing the route name can use `Object.entries(container.routes)` to get both key and entry together.

## Implementation Details

Two changes in `src/api-container.ts`:
- Remove `name: string;` from `RouteEntry` interface
- Change `addRoute` to store `{ path, handler }` instead of `{ name, path, handler }`
