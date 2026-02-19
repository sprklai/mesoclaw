# Code Review (Post `e1820d08`) - 2026-02-19

## Scope
Reviewed changes after the baseline captured in `docs/codereview/code-review-e1820d08-2026-02-18.md`.

## Findings (by severity)

### 1. High - WebSocket auth mismatch likely breaks browser WS connections
- Client opens WS with query token:
  - `src/lib/gateway-client.ts:204`
- Server auth middleware validates only `Authorization: Bearer` header:
  - `src-tauri/src/gateway/auth.rs:66`
  - `src-tauri/src/gateway/auth.rs:81`
- Gateway applies auth middleware to routes that include WS:
  - `src-tauri/src/gateway/daemon.rs:105`

Impact:
- Browser `WebSocket` cannot set custom `Authorization` headers directly.
- WS handshake likely fails with auth errors even when HTTP requests succeed.

Recommendation:
- Add WS-compatible auth path (query token and/or secure cookie) for WS handshake.
- Keep header-based auth for standard HTTP APIs.
- Document the canonical auth strategy in gateway docs.

### 2. High - WS `CancelSession` appears to acknowledge but not cancel
- WS command handler receives cancel:
  - `src-tauri/src/gateway/ws.rs:112`
- Current behavior publishes a `SystemError` message with `cancel_request:<session_id>`:
  - `src-tauri/src/gateway/ws.rs:118`
- No consumer of `cancel_request:` marker was found in codebase.

Impact:
- Clients may receive `cancel_ack` while underlying work continues.
- Creates false-positive UX and operational risk for long-running sessions.

Recommendation:
- Route cancel requests to the real session cancellation mechanism (cancel token/session registry).
- Return explicit failure when session ID is unknown or already complete.
- Add integration test: start session -> issue WS cancel -> assert loop termination.

### 3. Medium - Internal channel filter compares wrong identifier variant
- Bridge filtering uses underscore variant:
  - `src-tauri/src/lib.rs:404` (`"tauri_ipc"`)
- Canonical channel name is hyphenated:
  - `src-tauri/src/channels/tauri_ipc.rs:33` (`"tauri-ipc"`)

Impact:
- Internal channel events may not be filtered consistently.
- Potential duplicate forwarding or unexpected bridge behavior.

Recommendation:
- Normalize channel identifiers and compare against one canonical value.
- Consider centralizing channel constants to prevent drift.

### 4. Low - `connect_channel_command` is currently a health check, not a true connect flow
- Command currently checks existing channel health:
  - `src-tauri/src/commands/channels.rs:31`

Impact:
- API semantics can mislead callers expecting registration/start behavior.

Recommendation:
- Either rename command to reflect health/probe semantics,
  or implement full connect flow (credential load -> register -> start listener).

## Validation Notes
- Previously reported scheduler/cancellation regressions from earlier review are mostly addressed in current code.
- Local test run status in this environment: most tests pass; keyring credential-store tests fail due to OS keyring permission limitations.
