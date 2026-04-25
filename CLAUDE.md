# general

the packages in the project are managed using npm workspaces.
a task cannot be completed before 
- `npm run build` from root succeeds
- `npm run e2e` from packages/example/local-docker succeeds
- keep original task description in moth files, append specification-relevant parts

# for typescript

- use functional style when working with arrays, e.g. instead of for-loops use .map, etc
- prefer passing a function instead of using an OO overload
- use `npm run ...` for building, testing, etc.

---

# Moth Agent Guide

This guide helps LLM agents work effectively with moth, a git-based file issue tracker.

## Overview

Moth stores issues as markdown files in `.moth/` directories organized by status (ready, doing, done). Each issue has a unique ID, severity, and slug derived from the title.

## File Structure

```
.moth/
├── config.yml          # Project configuration
├── .current            # Current issue ID (when working on an issue)
├── ready/              # Issues ready to start
│   └── {id}-{severity}-{slug}.md
├── doing/              # Issues in progress
│   └── {id}-{severity}-{slug}.md
└── done/               # Completed issues
    └── {id}-{severity}-{slug}.md
```

Prioritized issues have a numeric prefix: `001-{id}-{severity}-{slug}.md`

**Note**: Moth automatically recreates missing status directories (e.g., if git removes empty directories). As long as `config.yml` exists, moth will recover gracefully.

## Workflow Commands

### Viewing Issues

```bash
# List all active issues (excludes done)
moth ls

# List issues in specific status
moth ls -t ready
moth ls -t doing

# List all issues including done
moth ls -a

# Filter by severity
moth ls -s high
moth ls -s crit

# Show current issue details
moth show

# Show specific issue
moth show {id}
```

### Working on Issues

```bash
# Start working on an issue (moves to doing, sets as current)
moth start {id}

# Mark issue as done
moth done {id}

# Mark current issue as done
moth done

# Move issue to any status
moth mv {id} {status}
```

### Creating Issues

```bash
# Create new issue (opens editor)
moth new "Fix login bug"

# Create with severity
moth new "Critical security fix" -s crit

# Create without opening editor
moth new "Quick fix" --no-edit

# Create and immediately start working
moth new "Urgent task" --start

# Create with body from stdin (useful for piping content)
echo "Issue description" | moth new "Issue title" --stdin
```

### Issue Management

```bash
# Edit issue content
moth edit {id}

# Update issue description from stdin (replaces entire content)
echo "New description" | moth update {id}
echo "Updated spec" | moth update          # updates current issue

# Delete issue
moth rm {id}

# Change severity
moth severity {id} high
```

### Priority Management

```bash
# Set priority number
moth priority {id} 1

# Move to top priority
moth priority {id} top

# Move to bottom (removes priority)
moth priority {id} bottom

# Position relative to another issue
moth priority {id} above {other_id}
moth priority {id} below {other_id}

# Renumber priorities sequentially
moth compact
moth compact ready
```

## Severity Levels

From highest to lowest:
- `crit` - Critical, must fix immediately
- `high` - High priority
- `med` - Medium priority (default)
- `low` - Low priority

## Partial ID Matching

All commands accept partial IDs. If you have issue `abc12`, you can use:
- `moth show abc12` (full)
- `moth show abc1` (partial)
- `moth show a` (if unambiguous)

## Git Integration

### Commit Hook

Moth can auto-prefix commit messages with the current issue ID:

```bash
# Install the hook
moth hook install

# With existing hook
moth hook install --append

# Remove hook
moth hook uninstall
```

When active, commits are prefixed: `[abc12] Your commit message`

### Commit Message Format

When committing changes related to an issue, prefix with the issue ID:

```bash
git commit -m "[abc12] Fix authentication bypass"
```

This links commits to issues in the report.

## Generating Reports

```bash
# Full history as CSV
moth report

# From specific commit
moth report --since abc123

# Between commits
moth report --since abc123 --until def456
```

Output includes: commit info, story changes (created, moved, edited, deleted), and code commits referencing issues.

## Agent Best Practices

### Starting Work

1. Check current issues: `moth ls`
2. Find issue to work on or check current: `moth show`
3. Start working: `moth start {id}`

### During Development

1. Make changes and commit frequently
2. Prefix commits with issue ID: `[{id}] description`
3. Keep issue content updated if requirements change

### Completing Work

1. Ensure all changes committed
2. Mark issue done: `moth done`
3. The `.current` file is automatically cleared

### Creating New Issues

When user requests new work:
1. Create issue: `moth new "Title" -s {severity} --no-edit`
2. Optionally start immediately with `--start` flag
3. Update issue file with detailed requirements if needed

### Checking Status

```bash
# Quick status check
moth ls

# What am I working on?
moth show

# Full project state
moth ls -a
```

## Configuration Reference

`.moth/config.yml`:

```yaml
statuses:
  - name: ready
    dir: ready
    prioritized: true    # Enable priority ordering
  - name: doing
    dir: doing
  - name: done
    dir: done

default_severity: med    # Default for new issues
editor: vi               # Editor for moth edit
id_length: 5             # Length of generated IDs
no_edit: false           # Skip editor on moth new

priority:
  auto_compact: false    # Auto-renumber after priority changes
```

## Common Patterns

### Pick up next priority issue
```bash
moth ls -t ready
moth start {first-id}
```

### Quick bug fix
```bash
moth new "Fix typo in header" -s low --no-edit --start
# make fix
git commit -m "[{id}] Fix typo"
moth done
```

### Triage incoming work
```bash
moth new "Investigate performance issue" -s med --no-edit
moth priority {id} top
```

### Review what was done
```bash
moth ls -t done
moth report --since HEAD~10
```
