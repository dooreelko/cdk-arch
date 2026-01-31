http bindings are prevalent and repetitive, so instead of calling

```
architectureBinding.bind(api, {
  ...endpoint,
  overloads: {
    hello: httpHandler(endpoint, api, 'hello'),
    hellos: httpHandler(endpoint, api, 'hellos')
  }
})
```

we need a helper function createHttpBindings that will take api, endpoint and list of route names to override and return
an object that for each overriden route will expose an async function with signature same as the handler of the route.

## Feature Specification

### Core Requirements

1. **createHttpBindings helper function**
   - Takes: endpoint (ServiceEndpoint), container (ApiContainer), route names (string[]), optional fetcher
   - Returns: a callable client object where each route name maps to an async function
   - The returned functions have the same signature as the route handlers and make HTTP calls to the remote endpoint

2. **Use as a callable client**
   - Instead of `api.getRoute('hello').handler.invoke()`, users can call `client.hello()`
   - Cleaner, more intuitive API for making remote calls

3. **Use as overloads for architectureBinding.bind**
   - The returned object can be passed directly as the `overloads` option
   - Reduces boilerplate when binding remote services

4. **Strongly typed ApiContainer**
   - `ApiContainer<TRoutes>` type parameter preserves route names and handler signatures
   - `createHttpBindings` returns `Pick<RouteHandlers<TRoutes>, K>` - a typed subset of the specified routes
   - Route names are validated at compile time (only valid route names accepted)
   - Handler signatures (arguments and return type) are preserved in the client

### Decisions Taken

- **Callable client approach**: The function returns an object that can be called directly (e.g., `client.hello('John')`) rather than requiring `api.getRoute('hello').handler.invoke()`. This provides a more ergonomic API.

- **Generic type parameter on ApiContainer**: Added `TRoutes extends ApiRoutes` type parameter to `ApiContainer` to preserve route type information through the type system.

- **Utility types for handler extraction**:
  - `HandlerOf<T>` - extracts `(...args: Args) => Promise<Return>` from a `Function<Args, Return>`
  - `RouteHandlers<TRoutes>` - maps route names to their handler signatures

- **Pick for subset selection**: `createHttpBindings` returns `Pick<RouteHandlers<TRoutes>, K>` where K is the union of specified route names, giving a precisely typed subset.

- **readonly route names array**: The `routeNames` parameter is `readonly K[]` to preserve literal types when using `as const`.

- **Optional fetcher parameter**: Supports custom fetchers (like Cloudflare service bindings) as the fourth parameter.

- **Fetcher type alias**: Extracted `Fetcher` type as `() => { fetch: typeof fetch }` for reuse.

### Decisions Rejected

- **Returning only overloads object**: Initially considered returning an object only for use with `architectureBinding.bind()`, but the callable client approach is more versatile.

- **Untyped ApiContainer**: Keeping `ApiContainer` untyped would be simpler but loses compile-time safety for route names and handler signatures.

## Implementation Details

- `ApiContainer<TRoutes>` added type parameter with default `ApiRoutes` for backward compatibility
- `RouteEntry<TArgs, TReturn>` captures handler argument and return types
- `HandlerOf<T>` and `RouteHandlers<TRoutes>` utility types exported for external use
- `httpHandler` and `createHttpBindings` use generics to constrain route names to valid keys
- `ArchitectureBinding.bind()`, `DockerApiServer`, and `createWorkerHandler` updated to accept typed containers
- All examples work with full type inference - no explicit type annotations needed
