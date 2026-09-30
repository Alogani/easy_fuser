# Instructions for coding agents

Read [CONTRIBUTING.md](CONTRIBUTING.md) for the project map, invariants, and verification workflow. Follow the user's task and repository conventions; these instructions do not limit which AI tools or workflows a user may choose.

## Implementation style

- Prefer idiomatic, readable Rust and the smallest change that solves the requested problem.
- Follow nearby patterns when they are sound. Preserve existing public behavior unless the task calls for a behavior change.
- Keep ownership, error paths, and synchronization visible. Handle errors deliberately; do not replace recoverable errors with `unwrap()` or silently ignore them without a reason.
- Avoid unnecessary dependencies, generic frameworks, duplicated configuration, and abstractions that have only one unclear use.
- Account for the crate's serial, parallel, and async modes when changing generated code or handler contracts.
- Keep comments useful: explain non-obvious invariants and reasons, not what the next line of code already says.

### Examples

**Do:** follow the mode-specific lock and ownership pattern used by the neighboring code, then compile each affected mode.

**Don't:** add a synchronous mutex around an async callback and hold it across `.await` just to make shared state compile.

**Do:** return the underlying filesystem error with enough context to identify the failing operation.

**Don't:** use `unwrap()` on a normal I/O result and turn an expected failure into a process panic.

**Do:** add a focused regression test for a resolver lifetime bug.

**Don't:** refactor unrelated modules or add a broad abstraction as part of a narrow fix.

## Review work

- Review the requested diff and its surrounding call paths; do not modify files unless asked to implement fixes.
- Report actionable findings first, ordered by impact. For each, include the file and line, the concrete failure or risk, and the conditions under which it occurs.
- Separate bugs from optional improvements. Do not report speculative concerns as confirmed defects; say what remains uncertain.
- If no blocking findings are present, state that plainly and mention important verification gaps. Do not invent findings to make a review appear useful.

## Completion notes

When implementation is requested, summarize what changed and why, list checks actually run, and identify material limitations. Keep the summary concise and link to changed files.
