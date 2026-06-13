we're creating a claude code skill. see
https://code.claude.com/docs/en/plugins and https://code.claude.com/docs/en/skills#share-skills

the skill will live in the current repo and potentially will be published to the skill marketplace https://github.com/anthropics/claude-plugins-official

the current version of the plugin is under ~/projects/c43-diag

we should
- create the plugin/skill harness here
- copy the current version of the plugin including git history (that repo was a throwaway)
- make sure we carry over also the moths and the ./docs

them add a new command `c43 layout` that is layout.py from the current implementation translated to rust

the skill switches to dual use. if `c43` command is present it uses it, otherwise it falls back to use the layout.py

