c43 layout should have options for output file names so that iterative runs don't overwrite each other's output

also we need --agent-help command, see ~/projects/moth/src/cmd/agent_help.rs and ~/projects/moth/.moth/done/ywcsl-med-agent_help.md
same logic, same format

----- AI agent updates -------

## Specification

### Feature 1: Customizable layout output file names

Add `--out-txt` and `--out-json` options to the `c43 layout` subcommand, allowing callers to redirect output to non-default paths. This enables iterative runs without clobbering previous results.

**Command signature:**
```bash
c43 layout <layout.json> [--out-txt <path>] [--out-json <path>] [--auto] [--max-evals <n>]
```

- `--out-txt` defaults to `result.txt`  
- `--out-json` defaults to `result.json`
- Both defaults preserve backward compatibility — existing callers need no changes.
- Stale-deletion on startup deletes whichever paths the options resolve to (not hardcoded names), so custom output paths are also cleared before the run.

### Feature 2: `--agent-help` flag

Add `--agent-help` as a root-level boolean flag on the `Cli` struct. When set, it prints a recursive overview of all commands, subcommands, and their options/arguments in an LLM-readable indented format, then exits successfully.

**Command signature:**
```bash
c43 --agent-help
```

**Output format** (mirrors moth's `--agent-help` exactly):
```
c43
  C4 model extractor for cdk-arch TypeScript projects
  Options:
    --ascii
    --agent-help
  Subcommands:
    # <about text>
    c43 <subcommand> [--flag] ...

```

### Design Decisions

- `command` field in `Cli` becomes `Option<Commands>` so `--agent-help` can be used without a subcommand. A missing subcommand (without `--agent-help`) exits with code 2 and an error message.
- `layout::run` signature extended with `txt_out: &Path, json_out: &Path` parameters; internal `json_path`/`txt_path` bindings are reassigned to these. No other callers exist.
- `agent_help` module placed at `src/cmd/agent_help.rs`, registered in `src/cmd/mod.rs`. Implementation copied from moth's `agent_help.rs` verbatim (same recursive walk of `clap::Command` tree).
- No `anyhow` dependency added; `run()` returns `()` instead of `Result<()>`.
- `clap::CommandFactory` trait imported in `main.rs` to call `Cli::command()`.

### Implementation Details

- `src/main.rs`: add `agent_help: bool` field + `CommandFactory` import; make `command: Option<Commands>`; dispatch `--agent-help` before subcommand dispatch; pass `out_txt`/`out_json` to `layout::run`.
- `src/cmd/layout/mod.rs`: extend `run(...)` signature; bind `json_path = json_out`, `txt_path = txt_out`; stale-delete loops over `[json_out, txt_out]`.
- `src/cmd/agent_help.rs`: new file, recursive clap tree printer (mirrors moth implementation).
- `src/cmd/mod.rs`: add `pub mod agent_help;`.
