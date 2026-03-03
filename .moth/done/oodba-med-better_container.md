`container` view should contain only high level individual "containers" - a web site, mobile app. a database, an api gateway, etc.
Individual `Function` instances belong to the component ditagram

## Specification

### C4 Level Mapping

- **Container level** (`c43 container`): High-level deployable units — `ApiContainer` subclasses (e.g. `JsonStore`), `ApiContainer` itself, and any other non-Function constructs that are direct children of an `Architecture` instance.
- **Component level** (`c43 component`): Internal structure — `Function` and `TBDFunction` instances and other fine-grained constructs within a package.

### Container View Rules

The container view must exclude `Function` and `TBDFunction` instances from the nodes emitted as direct children of `Architecture`. It also must not emit "routes to" relations pointing at `Function`/`TBDFunction` nodes (which would create dangling references since those nodes are omitted).

### Decisions

- Filter by class name (`"Function"`, `"TBDFunction"`) since the extract layer only surfaces the runtime constructor name. Custom subclasses of `Function` would also need to be named explicitly if added in the future.
- Route-to-handler relations are dropped from the container view entirely when the handler is a `Function`/`TBDFunction`. At the container level, routes to other container-level constructs are still preserved.
- The component view (`component.rs`) is unchanged — it continues to show all constructs including Functions.

### Deduplication

All commands must produce unique nodes (unique uid) and unique relations. Duplicate uids arise when the same TypeScript file is reachable from multiple workspace packages (e.g. a parent `package.json` whose directory contains child workspace packages — it is not marked as a workspace root so `scan_projects` includes it, and its recursive file scan overlaps with the child packages).

Deduplication is enforced centrally in `C4Document`: `add_node` and `add_relation` track seen keys in `HashSet` fields (skipped during serialization) and silently drop duplicates. First-seen wins.

### Cross-Container Function Call Detection

When a `Function` handler is routed through a container, the container view derives inter-container `"uses"` relations by statically analysing the handler body. For each variable used as the object of a method call inside the handler (e.g. `sessionStore.get(...)` → `sessionStore`), if that variable resolves to a sibling container (another direct child of the same `Architecture`), a `"uses"` relation is emitted from the routing container to the called container.

Limitation: only **direct** method calls in the handler body are detected. Calls made through module-level helper functions are not transitively resolved.

### Implementation Details

In `packages/c43/src/extract.rs`:
- `ConstructInstance` gains a `called_vars: Vec<String>` field.
- `collect_handler_called_vars(expr)` walks the 3rd argument of `new Function(...)` (arrow or regular function expression) and collects all identifier names used as objects in method calls, excluding `this`.
- `collect_calls_in_expr` / `collect_calls_in_stmt` recursively walk the AST covering `await`, `if`, `try`, `return`, `const`, binary/ternary expressions, etc.
- Only `Function` (not `TBDFunction`) gets its body analysed; others get `called_vars: vec![]`.

In `packages/c43/src/cmd/container.rs`:
- The `children` filter for direct Architecture children now additionally excludes nodes where `class_name` is `"Function"` or `"TBDFunction"`.
- The route-linking loop: when a handler is a `Function`/`TBDFunction`, its `called_vars` are resolved against `var_to_construct`. Any matching construct that is a sibling container (in `children`) produces a `"uses"` relation from the routing container to that sibling.

In `packages/c43/src/model.rs`:
- `C4Document` gains two `#[serde(skip)]` `HashSet` fields: `seen_nodes` (by uid) and `seen_relations` (by `(start, is, end)` tuple).
- `add_node` inserts into `seen_nodes` and skips if already present; same pattern for `add_relation`.
