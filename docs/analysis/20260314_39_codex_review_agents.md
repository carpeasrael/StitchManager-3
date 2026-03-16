# Problem description

The repository's [CLAUDE.md](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/CLAUDE.md) mandates four review agents in Phase 3: two Codex CLI reviewers and two Claude CLI reviewers. The project already includes the Claude-side review prompts in [.claude/commands/review-code.md](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/.claude/commands/review-code.md) and [.claude/commands/review-task.md](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/.claude/commands/review-task.md), but there is no equivalent project-local Codex setup. The missing Codex review agents need to be defined and exposed in-repo so contributors can invoke the required code-review and task-resolution review roles consistently.

# Affected components

- [CLAUDE.md](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/CLAUDE.md) for the required review workflow and output expectations
- [.claude/commands/review-code.md](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/.claude/commands/review-code.md) as the reference prompt for the Claude code reviewer
- [.claude/commands/review-task.md](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/.claude/commands/review-task.md) as the reference prompt for the Claude task reviewer
- New project-local Codex agent definitions and invocation/documentation files to be added under the repository
- Project documentation that tells contributors where the Codex review agents live and how to run them

# Root cause / rationale

The review policy names Codex CLI reviewer 1 and Codex CLI reviewer 2 as mandatory, but only the Claude reviewer prompts have been implemented in the repository. That leaves the workflow asymmetric and forces contributors to improvise Codex review prompts or depend on undocumented local conventions. To satisfy the project policy and make the reviewers reusable, the Codex prompts should live inside the repository, mirror the Claude review responsibilities, and define stable output requirements for the corresponding `docs/reviews/*codex*` files.

# Proposed approach

1. Add two project-local Codex reviewer prompt files, one for code review and one for task-resolution review, aligned with the responsibilities described in [CLAUDE.md](/Users/carpeasrael/NextCloud_int/mac_project_2/StitchManager-3/CLAUDE.md).
2. Encode output expectations in those prompts so each reviewer writes its own findings file in `docs/reviews/` and uses deterministic zero-findings text compatible with the project workflow.
3. Place the files in a project-local Codex-specific location so they can be reused by anyone working in this repository rather than kept as ad hoc prompt text.
4. Add a short project note describing where the Codex reviewer prompts live and how they are intended to be invoked, alongside the existing Claude command setup.
5. Keep the change limited to agent definitions and documentation; do not alter application code or the existing Claude reviewer prompts unless needed for cross-reference clarity.
