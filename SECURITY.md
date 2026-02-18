# Security Policy

## Reporting a Vulnerability

**Do not report security vulnerabilities via public GitHub Issues.**

Instead, use [GitHub Security Advisories](https://github.com/rakeshdhote/tauriclaw/security/advisories/new) to report vulnerabilities privately.

Include as much detail as possible:

- Affected component (agent, channels, security/, credentials, etc.)
- Description of the vulnerability
- Steps to reproduce
- Potential impact / attack scenario
- Suggested fix (optional)

### Response SLA

| Stage | Target |
|-------|--------|
| Acknowledge receipt | 48 hours |
| Initial severity assessment | 1 week |
| Fix for critical / high severity | 2 weeks |
| Fix for medium / low severity | 4 weeks |
| Public disclosure | After fix is released |

We will credit you in the release notes unless you prefer to remain anonymous.

---

## Supported Versions

| Version | Status |
|---------|--------|
| `0.x.x` (current) | Active |

---

## Security Architecture

MesoClaw is a local-first desktop application. The main attack surfaces are:

### Agent Tool Execution

- All shell commands are subject to policy checks in `src-tauri/src/security/`
- Injection attacks (`&&`, `||`, `;`, backticks, newlines) are blocked
- Env-variable assignments (`VAR=value`) are not treated as executables
- Autonomy levels limit what the agent can do without human approval

### Credential Storage

- API keys are stored in the OS keyring (never on disk in plaintext)
- The `zeroize` crate ensures keys are wiped from memory after use
- Credentials are never logged at any log level

### IPC (Frontend ↔ Backend)

- Tauri capability system restricts which commands the frontend can invoke
- All command inputs are validated at the Rust boundary
- No dynamic command dispatch from untrusted sources

### Channels (Telegram, IPC)

- Telegram allow-lists (`allowed_chat_ids`) restrict which senders are processed
- Messages from unknown chat IDs are silently discarded
- MarkdownV2 output is escaped to prevent entity injection

### Extension System (WASM, feature-gated)

- WASM extensions run in a sandboxed `wasmtime` instance
- Host functions are explicitly allow-listed
- Memory access is bounded

---

## Security Best Practices for Contributors

- Never `unwrap()` on untrusted input
- Validate and sanitise all data arriving from external sources (Telegram messages, webhook payloads, user input)
- Use parameterised queries — never string concatenation in SQL
- Prefer `spawn_blocking` for CPU-bound work rather than blocking the async runtime
- Add `// ## SECURITY` comments when a code path has non-obvious security implications
