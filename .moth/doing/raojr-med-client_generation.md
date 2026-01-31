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

### Decisions Taken

- **Callable client approach**: The function returns an object that can be called directly (e.g., `client.hello('John')`) rather than requiring `api.getRoute('hello').handler.invoke()`. This provides a more ergonomic API.

- **Generic type for route names**: Uses `<T extends string>` generic to preserve route name types in the return type (`Record<T, FunctionHandler>`), enabling autocomplete for route names.

- **Optional fetcher parameter**: Supports custom fetchers (like Cloudflare service bindings) as the fourth parameter, maintaining compatibility with different runtime environments.

- **Fetcher type alias**: Extracted `Fetcher` type as `() => { fetch: typeof fetch }` for reuse and clarity.

### Decisions Rejected

- **Returning only overloads object**: Initially considered returning an object only for use with `architectureBinding.bind()`, but the callable client approach is more versatile - it can be used both for direct calls and as overloads.

## Implementation Details

- `createHttpBindings` is implemented in `http-handler.ts` alongside `httpHandler`
- Uses `reduce` to build the client object from route names, calling `httpHandler` for each route
- Exported from package index alongside existing exports
- All examples updated to use the new function, demonstrating both use cases:
  - Web example: direct client calls (`apiClient.hello()`)
  - Server examples: as overloads for `architectureBinding.bind()`
