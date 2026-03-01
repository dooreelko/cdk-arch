when generating `system` level of c4 description, the output should only
inclide system components defining Architecture and its final consumers.
e.g. in case of reveal-interact, the system-level nodes are the
Architecture itself (it defines the entire backend), the plugin (it
interacts with the backend) and the web/example (it interacts with the backend via revint-lib)

The infra modules (infra-cloudflare and infra-docker) are relevant at the deployment level
The e2e-tests are never deployed, so not interesting at any level
The revint-lib is a transient user of the api, so also must be excluded.

c43 should identify all those cases and filter out what's not required.
also this should be reflected in the `type` attribute of the node.
An api provider is "backend", an obvious web site should be "frontend",
the other api consumers are "client".

## Specification

### System-level node types
- **system** — the repo itself (unchanged)
- **backend** — the Architecture instance (replaces "architecture" type)
- **frontend** — a final consumer package detected as a web application
- **client** — a final consumer package that is not a frontend

### Package classification (for filtering)

Every consumer package is classified into one of these roles. Only "final consumers"
(frontend, client) appear in the system output.

1. **Architecture definer** — package that defines Architecture constructs. Excluded as
   a separate node (the Architecture node with type "backend" represents it).
2. **Infrastructure** — package that has `architectureBinding.bind()` calls (bindings).
   Excluded from system view (relevant at deployment level).
3. **Test package** — detected by BOTH name pattern (contains "test", "e2e", "spec")
   AND presence of test framework devDependencies (jest, mocha, cucumber, playwright,
   cypress, vitest, puppeteer). Excluded from system view.
4. **Library** — a consumer package that is imported by other workspace packages.
   Detected by checking if any other workspace package's imports reference this package name.
   Excluded from system view (transient consumer).
5. **Frontend** — a final consumer with web framework deps (react, vue, svelte, angular,
   next, nuxt, solid-js, preact in dependencies) OR web file heuristics (index.html,
   vite.config.*, webpack.config.*). Check deps first, fall back to file heuristics.
6. **Client** — any remaining final consumer.

### Transitive consumer detection
Packages that import from a library (which re-exports arch constructs) are also consumers.
The existing `build_exported_constructs_map` resolves one level of re-exports, so packages
importing from a library that re-exports arch constructs are already detected as consumers.

### Expected output for reveal-interact
```
nodes:
  system:reveal-interact  type=system
  reveal-interact         type=backend
  @revint/plugin          type=client
  @revint/example         type=frontend (imports via @revint/lib)

excluded:
  @revint/arch            → arch definer (Architecture node covers it)
  @revint/api             → workspace root (contains sub-packages)
  @revint/infra-cloudflare → infrastructure (has bindings)
  @revint/infra-docker     → infrastructure (has bindings)
  @revint/plugin-e2e-tests → test package
  @revint/lib              → library (imported by @revint/example)
```

### Expected output for cdk-arch
```
nodes:
  system:cdk-arch  type=system
  hello-world      type=backend
  cdk-arch-web     type=frontend (has react dep)

excluded:
  architecture     → arch definer
  local-docker     → infrastructure (has bindings)
  cloudflare       → infrastructure (has bindings)
```

### Decisions taken
- Test detection requires BOTH name pattern AND devDependency check (avoids false positives)
- Frontend detection uses deps check first, file heuristics as fallback
- Library detection is based on "imported by other workspace packages" (not re-export analysis)
- Architecture node type changes from "architecture" to "backend" at system level

### Decisions rejected
- Detecting libraries by re-export analysis alone (too fragile, doesn't distinguish
  a library from a package that happens to re-export)
- Detecting tests by name pattern alone (could false-positive on packages that test other things)
- Detecting frontend by file heuristics only (deps are more reliable)

## Implementation details

`PackageMeta` struct added to `analysis.rs` alongside `ProjectData`. It carries dependency
names, devDependency names, and web file heuristic booleans (has_index_html, has_web_config).
Populated by `read_package_meta()` during `scan_directory()`.

In `system.rs`, a classification pipeline determines each package's role using priority order:
arch definer → infra → test → library → frontend → client.

Transitive consumer detection uses a fixpoint loop: starting from direct consumers (packages
importing arch constructs), expand to any package that imports from a known consumer. This
catches facade libraries like @revint/lib → @revint/example chain where the library doesn't
re-export constructs but wraps them in its own API.

Architecture-to-consumer linking uses `find_used_architectures()` which traces the import
graph from a consumer back to arch-defining packages via BFS.

Architecture nodes use type "backend" instead of "architecture" at the system level.
