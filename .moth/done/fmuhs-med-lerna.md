# Lerna Monorepo Manager

## Feature Specification

Add Lerna as a monorepo manager to work alongside existing npm workspaces.

### Requirements

1. **Lerna Configuration**: Add `lerna.json` with:
   - `useWorkspaces: true` - delegate package discovery to npm workspaces
   - `npmClient: "npm"` - use npm as the package manager

2. **Integration with Existing Workspaces**: Lerna should work with the existing npm workspaces configuration in package.json.

### Decisions Taken

- **No useWorkspaces option**: Lerna v9+ uses npm workspaces by default and removed the `useWorkspaces` option. The workspaces field in package.json is used automatically.

- **npmClient: npm**: Use npm as the package manager (not yarn or pnpm).

- **version: independent**: Use independent versioning so each package can have its own version number.

## Implementation Details

Add `lerna.json` configuration file at the workspace root with `npmClient` and `version` settings. Add lerna as a devDependency to the root package.json.
