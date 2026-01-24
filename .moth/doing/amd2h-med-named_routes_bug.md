# Named Routes Bug Refactoring

## Feature Specification
- `ApiRoutes` now uses route names as keys instead of paths.
- The `namedRoutes` internal map has been removed from `ApiContainer` to avoid redundancy.
- `ApiRoutes` structure: `[name: string]: RouteEntry`.
- `RouteEntry` includes `name`, `path`, and `handler`.
- `getRoute(name: string)` returns a `RouteEntry` by its name.
- `getRoute(name: string)` throws an error if the route name is not found in the container.
- `getRouteByName` has been removed as it is redundant with the updated `getRoute`.

## Decisions Taken
- **Key by Name**: Using names as keys in `ApiRoutes` simplifies route management and ensures uniqueness by name, which is more stable than using paths as keys.
- **Throw on Missing Route**: `getRoute` now throws an error instead of returning `undefined` to provide immediate feedback when an invalid route name is requested.
- **Redundant name in RouteEntry**: Although the name is the key in `ApiRoutes`, it is also included in `RouteEntry` for completeness and ease of use when passing around `RouteEntry` objects.
- **Removal of getRouteByName**: Since `getRoute` now performs the lookup by name, the separate `getRouteByName` method was no longer necessary.

## Decisions Rejected
- **Optional Name in RouteEntry**: Rejected making `name` optional in `RouteEntry` to maintain type safety and ensure all route objects are fully defined.
- **Returning undefined in getRoute**: Returning `undefined` was rejected in favor of throwing an error to enforce that callers only request existing routes, simplifying error handling in architectural logic.

## Implementation Details
- Modified `ApiRoutes` interface in `packages/cdk-arch/src/api-container.ts` to be a map of `RouteEntry` objects keyed by name.
- Updated `ApiContainer` constructor and `addRoute` method to populate the `routes` property using names as keys.
- Refactored `getRoute` to perform a lookup in `this.routes` by the provided name and throw an `Error` if the entry does not exist.
- Updated all internal and example usages (e.g., `ArchitectureBinding`, `DockerApiServer`, `createWorkerHandler`) to accommodate the new `ApiRoutes` structure and `getRoute` behavior.
- Corrected package imports in examples from `cdk-arch` to `@arinoto/cdk-arch`.
