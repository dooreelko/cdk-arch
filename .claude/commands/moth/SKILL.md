execute `moth show` and if there's a current task, start implementing it.

use IDEA.md and README.md for global context, us  `moth ls` and `moth show <id>` to view tasks in "done" status for context and past decisions.

use `moth --agent-help` for command details.

ask questions as needed. during the session update current moth with information relevant to the feature specification, including decisions taken and rejected. the information in the md should be enough to recreate the feature from the scratch. Under implementation details, don't describe the resulting code changes or structure, only an abstract of how it's done.

use past specifications under ./.moth to ensure decision consistency

never change task specification to a different one. e.g. for example if we're implementing addition and there's a request to add logging, the specification should decribe both addition and logging.

YOU MUST create a branch in form of `bob/<mothid>-<short-desc>` and NEVER commit directly to main.

each task must include tests fully covering the implementation/changes
