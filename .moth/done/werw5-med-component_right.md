`component` command should
- essentially concentrate on Function nodes, their placement and relations
- include the system and container nodes
- exclude infrastructure nodes as they belong to `deployment`
- bug. it should correctly filter the nodes given a --container option (still including parents)

## Specification

### Overview

The `component` command produces a C4 component-level view. It builds on the container view (System + Backend/Frontend/Client + containers) and adds Function/TBDFunction nodes as the components inside those containers.

### Node Selection

**Included:**
- System node (derived from root directory name)
- Architecture (Backend) nodes and their container children (DataStore, ApiContainer, WsContainer, etc.)
- Function and TBDFunction nodes from arch-defining packages only

**Excluded:**
- Frontend and Client nodes — these are external actors, not hierarchically related to the backend
- Infrastructure constructs (Worker, WorkersKvNamespace, Image, Network, etc.) — these belong in `deployment`
- Class-internal TBDFunctions with unresolvable placement (scope is `this`, no var_name) — e.g. DataStore's template `store`/`get`/`list` TBDFunctions

**Arch-defining package**: any package that contains at least one `Architecture` instance. Infra packages (infra-cloudflare, infra-docker) have no Architecture instance and are therefore skipped.

### Function Placement

A Function/TBDFunction is included if it has resolvable placement:
- Its `scope_var` maps to a known construct (e.g. scoped to `arch`), OR
- Its `var_name` appears as a `handler_var` in any route

Relations added:
- `Architecture contains Function` (via scope_var → Architecture)
- `Container handles Function` (via route entries)
- `Function uses Container` (via called_vars — variables used as method-call objects in the Function body that resolve to container constructs)

### `--container` Filter

When `--container <id>` is given:
- All System, Backend, and container nodes are always included (parent context)
- Functions are included if they are **handled by** the filter container (via route handler_vars) **OR** if their `called_vars` contain the filter container's variable name
- Container is matched by id or var_name

The previous bug was filtering by `scope_var == filter`, which missed Functions scoped to Architecture but routed through the container. The `called_vars` match allows detecting indirect access chains (e.g. `api → reactFunction → reaction-store`).

### Implementation Details

`component::run` builds the base document directly (not via `container::run`) to exclude Frontend/Client:
- Adds System node from root directory name
- Adds Architecture (Backend) nodes and their container children directly
- Calls `scan_projects(root)` and filters to arch-defining packages only
- Builds a `var_name → ConstructInstance` map and flattens routes from those packages
- Computes optional `filter_info: Option<(&str, &str)>` = (container_id, container_var_name) and `filter_handler_vars: HashSet<&str>` for `--container` mode
- Adds Function/TBDFunction nodes that pass placement criteria (resolvable scope or routed) and filter criteria (handled or calls container)
- Emits `contains` (scope), `handles` (route), and `uses` (called_vars → container) relations for included functions
