this repo is getting too big.
we now have an org https://github.com/well-architected-cloud-clarc

let's split this monorepo into individual ones there

cdk-arch and example are one repo
c43, clarc and c43-layout are separate ones

let's try to preserve git history, which should be somewhat commit-per-package

each repo keeps using moth (and carries over its moths) and direnv/shell.nix and .claude

locally repos should be under ~/clarc

----- AI agent updates -------

## Target repos
Org well-architected-cloud-clarc, public, same license (duplicated LICENSE), local checkouts under ~/clarc. Names: `cdk-arch`, `c43`, `c43-layout`, `clarc`.
- `cdk-arch`: cdk-arch package + example (npm workspaces kept), README.md, IDEA.md (both cdk-arch specific), media/, docs/superpowers.
- `c43`, `c43-layout`: one crate each.
- `clarc`: the clarc crate plus the claude plugin directory (claude-plugin/c43).
Each repo is a single project with its own root manifest and carries .claude, .envrc, shell.nix, CLAUDE.md (duplicated), LICENSE, .moth. `scripts/` is duplicated into clarc and c43-layout (clarc's scripts concern only phase 1/2).

## History
Per-repo path-filtered rewrite of a fresh clone of the monorepo (commit-per-package makes this clean), built in new directories. The old monorepo stays untouched for now (not archived/deleted).

## Moths
Split by relevance; a moth touching several packages is duplicated into each relevant repo.
IDEA.md for clarc and c43-layout is newly written, seeded from the human-written parts of their initial moth tasks, and stays uncommitted until the user approves.

## Cross-repo dependencies
- cdk-arch is published on npm; consumers use the registry.
- Rust chain clarc -> c43 -> c43-layout: git references pinned to a tag/rev now; c43-layout is to be published on crates.io, then they switch to crates.io versions.
- c43's tests become self-contained: copies of the layout goldens and example fixtures it reads from sibling packages.
- The cdk-arch e2e drops its c43 and clarc smoke steps (c43 --drawio on the example, clarc --file -); both are covered by c43/clarc own tests. cdk-arch has no Rust dependency; its build is the npm workspace build only.

## Process
GitHub repos are created in the org and pushed (explicitly approved). git-filter-repo is obtained via nix-shell.

## Rejected
- Archiving/deleting the old monorepo now.
- Fetching sibling-repo fixtures at test time; installing c43/clarc binaries for cdk-arch e2e.

## Implementation notes (abstract)
- Rust cross-repo deps are git references pinned to a commit rev (tags can follow after the first push). Cargo.lock for c43 and clarc is generated after the repos are pushed (git sources cannot be resolved before); c43-layout carries its lock. Local verification used a cargo `[patch]` override pointing at the sibling checkouts.
- c43 owns copies of what it read from sibling packages: layout goldens (fixtures/layout-cases) and a trimmed cdk-arch workspace (samples/cdk-arch). The sample must live outside paths with test-ish words ("test", "fixture", "spec", "mock", "e2e"), because c43's system view treats such constructs as test code; its directory name must be `cdk-arch` (the system name derives from it).
- scripts: render.sh in c43-layout and clarc, gen-glyphs.py in clarc only (it generates clarc's glyph tables).
- CLAUDE.md gates adapted per stack: cdk-arch keeps npm build/e2e; Rust repos use cargo build --release and cargo test.
- Known follow-up: the plugin UAT script (claude-plugin/c43/uat/test.sh) still locates a c43 binary at the repo root's target/; it now lives in another repo (C43_BIN override works). Related moths s2dcx, t1yse.
- Environment: shared setup (gh identity, GIT_AUTHOR/COMMITTER env vars, npm paths, NIXPKGS_ALLOW_UNFREE) lives in an optional parent ~/projects/clarc/.envrc, not committed anywhere. Each repo .envrc is `source_up_if_exists` + `use nix`, so clones work without the identity.
- Local checkouts live under ~/projects/clarc/.

## Outcome
- All four repos created public in the org, history split as specified, pushed to main. Pinned Rust revs resolved from GitHub; Cargo.lock for c43 and clarc generated from the real git sources and committed.
- Validation: cdk-arch `npm run build` + `npm run e2e` green; c43, c43-layout, clarc `cargo test` green (c43 and clarc against the pushed git dependencies, no local override).
- IDEA.md/README.md for c43, c43-layout, clarc were written by the user after the split (not generated); seeded drafts were superseded.
- Follow-ups (not part of this task): tag releases and swap git refs for crates.io versions once c43-layout is published; plugin UAT script still assumes a c43 binary in its own target/ (C43_BIN overrides); moths s2dcx/t1yse cover the plugin rework. The old monorepo is kept as is.
