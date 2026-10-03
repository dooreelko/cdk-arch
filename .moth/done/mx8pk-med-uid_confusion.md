The self-loops come from an ID collision between an Architecture and one of its own containers. Both use the same string ID, and the generator uses that bare ID as the node's uid.

In rebob:

// packages/llm/arch/src/architecture.ts
export const llmArch = new Architecture('llm');
export const llm     = new LlmGateway(llmArch, 'llm');

// packages/dispatcher/arch/src/architecture.ts
export const dispatcherArch          = new Architecture('dispatcher');
export const dispatcherApiContainer  = new ApiContainer(dispatcherArch, 'dispatcher', {...});

chat, memory and event-storage follow the same pattern: an ApiContainer with the same ID as its architecture. Those three plus dispatcher and llm are exactly the five self-loops in the output.

What happens in packages/c43/src/cmd/component.rs:
1. Around line 168, the Architecture is emitted as a Backend node with uid arch.id ("dispatcher").
2. Around line 180, the child container is emitted with uid child.id, which is also "dispatcher". C4Document::add_node (model.rs:51) silently drops duplicate uids, so the container node is lost.
3. add_relation(&arch.id, "contains", &child.id) then becomes dispatcher contains dispatcher.

There's a second effect besides the loop: the container node disappears. Any handles or uses edges that pointed at the dispatcher ApiContainer now attach to the Backend node instead.

container.rs:67 follows the same arch.id contains child.id pattern, so it may have the same bug, though I only ran the component command.

Possible fixes:
- Namespace the uids (recommended): use something like dispatcher/dispatcher for containers, or prefix by type (backend:dispatcher), and keep name as the display label. This is the only option that fixes the lost-node problem too.

----- AI agent updates -------

## Problem (generalised)

Node uids were bare construct ids and variable names were resolved through one global, first-seen-wins map. Two separate confusions followed:

1. **Uid collisions.** Construct ids are only unique inside their Architecture. The same id appears (a) on an Architecture and its own container (`dispatcher`, `chat`, `memory`, `event-storage`, `llm`) → self-loop + lost container node, and (b) on constructs in different Architectures (`llm` container in arch `llm` and arch `llm-legacy`; functions `runtime`, `complete`, `embed`, ...) → nodes merged, edges attached to the wrong node (`llm-legacy contains llm` pointed at the LlmGateway of arch `llm`).
2. **Variable collisions.** The same variable name defined in unrelated packages (`llmArch` in `@bob/llm-arch` and `@bob/llm-server-docker`) was treated as one variable, so scope/route/call lookups could bind to the wrong construct. Children were matched to an architecture by comparing scope variable *names*, so an arch picked up children of a same-named variable in another package.

## Rules

- **Node identity = construct definition**, never a variable name. Any number of variables in any number of modules/packages that refer to the same construct (e.g. every package doing `import { llm } from '@bob/llm-arch'`) map to the same single node. Same-named variables that refer to different definitions map to different nodes.
- **Variable resolution is per package**: a variable seen from package P resolves to (1) a construct defined in P under that name, else (2) a named import in P (respecting `import { a as b }`), followed into the source package, which may itself resolve through re-exports (`export { x } from`, `export * from`). Bounded depth; namespace/default imports are not followed.
- **Uid scheme** (applies to `system`, `container`, `component` commands):
  - System: `system:<repo>` (unchanged).
  - Architecture (Backend): `backend:<arch id>` — type prefix, same convention as `system:`.
  - Anything inside an Architecture (containers, functions, other constructs): `<arch id>/<scope ids...>/<construct id>`, always qualified, even when unique. The path is the chain of scopes from the nearest enclosing Architecture down to the construct. Constructs scoped directly to the arch get `<arch>/<id>`; constructs a container creates internally get `<arch>/<container>/<id>` (e.g. `memory/graph/getShortTerm` vs the arch-level `memory/getShortTerm`).
  - Constructs with no resolvable owning Architecture keep their bare id.
  - Frontend/Client package nodes keep the package name as uid.
- **`name` stays the bare id** (display label); uid is only for identity/edges.
- **Node `type`** = concrete class name as written (`consolidatorapi`, `observedfunction`). **`attributes.kind`** = resolved cdk-arch base kind (`architecture`, `apicontainer`, `function`, `construct`). Renderers decide by kind, not by type string.
- A route belongs to the **container** (never the Architecture, which may share the id) with the route's container id declared **in the same package** as the route. `handles` edges therefore always start at a container node.
- `component --container <X>` filter accepts container id, container variable name, or qualified uid.

## Construct kinds: everything rooted in cdk-arch `Construct`

- cdk-arch hierarchy: `Architecture`, `ApiContainer`, `Function` and others extend `Construct`; `TBDFunction` extends `Function`. (`ApiContainer` does **not** extend `Architecture`.) Projects subclass these, e.g. `ConsolidatorApi`/`Graph`/`LlmGateway extends ApiContainer`, `ObservedFunction extends Function` (from `@bob/base`, often imported as `ObservedFunction as Function`).
- A construct's **kind** is found by following its class name through import aliases, re-exports and `class A extends B` chains across scanned packages until a cdk-arch base class is reached (`@arinoto/cdk-arch`). Kinds: Architecture, ApiContainer, Function (Function and TBDFunction), other Construct. All view logic uses the kind, never the literal class name.
- `new X(...)` of a class that does not root in Construct (globals such as `Error`/`Map`, or classes with no Construct base) is not a construct and is dropped. A class imported from an unscanned non-cdk-arch package cannot be inspected and is kept as a generic Construct.
- **Class templates.** Constructs a class creates in its constructor or field initialisers (`this.x = new TBDFunction(this, 'id')`), and routes it adds (`this.addRoute(name, path, this.x)`), belong to every instance of that class and of its subclasses. For an instance held in variable `v`, a template construct in field `f` becomes a construct with variable `v.f` scoped to `v`, and `this.f` route handlers become `v.f`. Nested instances are expanded recursively.
- Class-internal template functions now have a resolvable placement (through their instance), so they are included. This supersedes werw5's exclusion of "class-internal TBDFunctions with unresolvable placement" (e.g. a DataStore's `store`/`get` TBDFunctions now appear as `<arch>/<store>/<id>`, handled by their store).
- Constructor route literals (`new X(scope, id, { routes })`) are read for any class; function bodies (third constructor argument) are scanned for called variables for any class.

## Container view: collapsed `uses`

- The container view shows direct non-function children of Architectures. `uses` edges are derived from all function calls (not only routed handlers), collapsed to the nodes in this view, across Architectures:
  - A function **acts through** the shown containers that route it. If no container routes it, it acts through the node enclosing it: its container, or the `backend:<arch>` node for functions scoped directly to the Architecture (e.g. consolidator's worker functions → `backend:consolidator`).
  - A non-function construct acts through itself if shown, else through its nearest enclosing shown container or Architecture.
  - For each call, every node the caller acts through `uses` every node the callee acts through. Self-edges are dropped.
- `routes to` (container → non-function handler) is kept, using the same collapsing.

## Component view: who uses whom

- **Function → Function `uses`.** A function calling another construct (e.g. `sendMessage.invoke(...)` on a Function imported from another arch) yields `uses` to that construct's node, whether it is a container or a function. Previously only containers were targets, so cross-arch calls (sub-bob-manager → chat/memory) were invisible.
- **Internal vs external.** Only truly external systems are excluded. A package that has or uses Architecture constructs is internal. So Frontend/Client packages (same classification as the system view: consumers of arch constructs, directly or transitively) are shown in the component view. This **reverses** the werw5 decision to exclude Frontend/Client as "external actors".
- Each Frontend/Client `uses` every view node (backend, container, function) that one of its imports resolves to, through the same per-package resolution. With `--container`, only Frontend/Client packages that use an emitted node are shown.

## Decisions

- Chosen: type prefix for Architectures (`backend:`) + always-qualified children (`arch/id`).
- Rejected: prefix-on-collision-only (uids would depend on unrelated packages, unstable); prefixing every type (`apicontainer:...`, verbose); hierarchical full path (`arch/container/fn`) — initially rejected as long, later adopted (see uid path decision below); dropping self-loop relations (hides symptom, node still lost).
- Var-name scoping fix included in this task (same root cause: identity by name instead of by definition).
- Function→Function: chosen to target the called function itself. Rejected: targeting the container that handles the called function (coarser), or emitting both.
- Frontend/Client classification is shared between the system and component views (one definition of "consumer package").
- Uid path: the earlier "`<arch>/<id>` only" was extended to the full scope path below the arch once class templates produced real same-arch collisions (`memory/getShortTerm` vs `Graph`'s internal `getShortTerm`). Arch-level constructs keep the short form.
- Container view: unrouted functions act through `backend:<arch>` (accepted: an edge may start at a group node). Rejected: dropping such edges (would lose consolidator → llm).
- Kind by inheritance (not by class name) is in scope: everything rooted in Construct must be considered.

## Implementation details (abstract)

- A per-package variable resolver over the scanned projects (local definitions → imports → re-exports) replaces the global var→construct map; construct identity compared by definition, not by id/name.
- The resolver walks scope links to compute the scope path and the uid of any construct.
- After scanning, a class resolver computes each construct's kind and instantiates class templates per instance. Non-constructs are removed at that point, so all commands see resolved data.
- Two uid helpers (backend uid, child uid) live with the document model and are used by all view commands.
- Frontend/Client classification is extracted from the system view so it can be reused. The component view links these packages by resolving their imports.
- `deployment` command untouched (single package, own var map; kinds not resolved there).

## Open questions (raised, not decided)

- C4 level of the component view: should it be per container (one diagram per `--container`) rather than whole-system? Current whole-system output (146 nodes) is closer to a code map than a C4 component diagram.
- Collapsing: decide whether coarsening (functions → containers, etc.) belongs in extraction (separate views) or in the renderer (one detailed model, collapsed at render time).
- Host/server packages (e.g. `@bob/manager-server`): fold into the container they host instead of showing them as separate Client nodes?
- Plain aliases / client wrappers as references (see known limits): track or not.

## Known limits

- Server/host packages (e.g. `@bob/manager-server`): in the standalone c43 they appeared as Client nodes using many internals (accurate but noisy). In cdk-arch they are classified ClientServer (hitc7) and are not shown in the component view.
- Frontend/Client packages that only reach constructs through a library that does not re-export them get a node with no `uses` edges in the component view.
- ASCII rendering does not show `uses` edges that start at Frontend/Client nodes, nor container-level `uses` in the container view (they are in the JSON).
- Two constructs with the same id in the **same scope** still share a uid.
- Calls inside class-template functions on `this.<field>` are not tracked as called variables (calls through an instance, `inst.field.invoke()`, are). A plain-identifier route handler inside a class is resolved in the instance's package, not the class's package.
- Classes declared in an unscanned package other than cdk-arch are opaque: kept as generic Construct, no templates.
- Plain aliases (`const client = llm;`) and client wrappers are not tracked as references; only imports/re-exports are. None present in rebob.

## Verification (rebob)

- No self-loops in any view; no relations to missing nodes.
- Before kinds/templates: container view 30→38 nodes, component 66→75 (recovered lost containers/functions).
- Component view: 80 nodes, 189 relations. New edges: `sub-bob-manager/dispatchCommand uses chat/sendMessage`, `sub-bob-manager/getBobState uses memory/getShortTerm`, `dispatcher/submitCommand uses sub-bob-manager/dispatchCommand`. The 5 Frontend/Client packages are present.
- After kinds + templates: container view 38 nodes / 66 relations, including `memory/memory uses llm/llm`, `backend:consolidator uses llm/llm`, `sub-bob-manager/manager uses chat/chat`, `sub-bob-manager/manager uses memory/memory`. Component view 146 nodes / 318 relations. Subclass routes are picked up (`memory/graph` handles 20, `consolidator/consolidator-api` handles 1). Both `memory/getShortTerm` and `memory/graph/getShortTerm` exist. No relation of the previous output was lost.
- cdk-arch repo: system/container unchanged; component gains the JsonStore's `store-handler`/`get-handler` (under `hello-world/greeted-store`) and the `cdk-arch-web` frontend.
- `llm-legacy/llm` (RoutedLlm) and `llm/llm` (LlmGateway) are distinct; all consumers importing `llm` from `@bob/llm-arch` (memory, consolidator) point at `llm/llm`.

## Port into cdk-arch (`packages/c43`)

mx8pk was first implemented in the standalone c43 repository, which was forked from this package before auk45, hitc7, uyzmn, bvthw and bc3mn. Porting it here keeps those decisions and combines them as follows:

- **System view (hitc7 + mx8pk).** Kept: test package/id/file exclusion, ClientServer/Infrastructure packages hidden with their bindings lifted to Architecture-level `uses`, ArchDefiner → imported Architecture `uses`. Changed by mx8pk: Backend uids are `backend:<id>` (also in lifted edges). Architecture detection uses the resolved kind. A binding's target is found with the per-package resolver, and its Architecture by walking the scope chain to the nearest enclosing Architecture. Previously a global construct-id → arch map, keyed by bare id, was used, which (a) confused same-id constructs in different Architectures and (b) only covered direct children. The system output for rebob and this repo is unchanged apart from the uid prefix.
- **Package classification is shared.** System and component views use one classification (hitc7's priority pipeline: test package first, then arch definer, ClientServer, infrastructure, library, frontend, client). The component view shows only Frontend/Client packages, so test and ClientServer packages are excluded there too.
- **ASCII (auk45).** The visited-set and the bracketed relation suffix are kept. Function detection uses `attributes.kind`.
- **Instance-field calls.** A call through an instance field (`store.get.invoke()`, including when `store` is imported) resolves to that instance's class-template construct. The called path is recorded as a dotted path, and the resolver resolves the head variable, then the field on that instance's package.
- **Tests.** The existing lifting test now expects `backend:` uids. New regression tests cover: same ids across Architecture/container and across Architectures; kinds by inheritance through an imported alias, class templates and their routes, non-constructs dropped; cross-Architecture Fn→Fn `uses` and the collapsed container-level `uses`, including an unrouted function acting through `backend:<arch>`.
- **Verification (rebob, cdk-arch build).** system 14 nodes/27 relations (same as before the port, modulo prefix); container 35/64 (was 27/54 with 5 self-loops); component 143/287 (was 66/146 with 5 self-loops). No self-loops, no dangling relations. The node counts are lower than in the standalone c43 because hitc7 hides test and server packages.
- **Acceptance (this repo).** `npm run build` from the root, `npm test` (vitest + all c43 cargo tests) and `npm run e2e` in `packages/example/local-docker` pass.
- **Status.** Ported on branch `mx8pk-uid-confusion`; accepted by the user on 2026-10-03 and merged into `main`.
