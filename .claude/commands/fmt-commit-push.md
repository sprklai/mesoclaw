---
description: Run cargo fmt, stage all changes, auto-generate a commit message, and push to origin
---

Run the following steps in order:

1. Run `cargo fmt` in `src-tauri/` to format all Rust code
2. Run `git add .` to stage all changes
3. Run `git diff --cached` and `git log --oneline -5` to understand the changes
4. Auto-generate a concise conventional commit message based on the diff (e.g. `feat(...)`, `fix(...)`, `chore(...)`)
5. Commit: `git commit -m "<generated message>\n\nCo-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>"`
6. Run `git push`

Report the generated commit message and the result of each step. If any step fails, stop and report the error.
