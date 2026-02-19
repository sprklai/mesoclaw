---
description: Run cargo fmt, stage all changes, commit with a message, and push to origin
argument-hint: "<commit message>"
---

Run the following steps in order:

1. Run `cargo fmt` in `src-tauri/` to format all Rust code
2. Run `git add .` to stage all changes
3. Commit with the message provided as the argument: `git commit -m "$ARGUMENTS"`
4. Run `git push` to push to origin

If no commit message argument is provided, ask the user for one before proceeding.

Report the result of each step. If any step fails, stop and report the error.
