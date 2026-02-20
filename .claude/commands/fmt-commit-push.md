---
description: Run cargo fmt, stage all changes, auto-generate a commit message, and push to origin
---

Run the following steps in order:

1. Run `cargo fmt` in `src-tauri/` to format all Rust code
2. Run `git add .` to stage all changes
3. **SECRET SCAN**: Run `git diff --cached` and scan for leaked secrets using these patterns:
   - OpenAI: `sk-[a-zA-Z0-9]{20}T3BlbkFJ`
   - Anthropic: `sk-ant-[a-zA-Z0-9\-]{80,}`
   - AWS: `AKIA[0-9A-Z]{16}`
   - GitHub: `ghp_|gho_|ghu_|ghs_|ghr_`
   - Slack: `xox[baprs]-[0-9]{10,13}`
   - Discord: `[MN][a-zA-Z\d]{23}\.[a-zA-Z\d]{6}\.[a-zA-Z\d]{38}`
   - Telegram: `\d{9,10}:[a-zA-Z0-9_-]{35}`
   - Private keys: `-----BEGIN.*PRIVATE KEY-----`
   - Generic: `(?i)(api_key|apikey|password|secret|token)\s*[=:]\s*['"][^'"]{20,}['"]`

   **If ANY secret is found**: STOP immediately, alert user with file:line and masked value. Do NOT proceed to commit.

4. Run `git diff --cached` and `git log --oneline -5` to understand the changes
5. Auto-generate a concise conventional commit message based on the diff (e.g. `feat(...)`, `fix(...)`, `chore(...)`)
6. Commit: `git commit -m "<generated message>\n\nCo-Authored-By: Claude Sonnet 4.6 (1M context) <noreply@anthropic.com>"`
7. Run `git push`

Report the generated commit message and the result of each step. If any step fails (especially secret scan), stop and report the error.
