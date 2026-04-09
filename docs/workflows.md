---
sidebar_position: 6
title: Workflows
slug: /workflows
---

# Workflow Best Practices

Zenii workflows are multi-step automation pipelines defined in TOML. They chain tools, LLM calls, conditions, and delays into a directed acyclic graph (DAG) that executes in topological order with per-step retries, timeouts, and failure policies.

## Table of Contents

- [Quick Start](#quick-start)
- [TOML Format Reference](#toml-format-reference)
- [Step Types](#step-types)
- [Template Variables](#template-variables)
- [Dependencies and Execution Order](#dependencies-and-execution-order)
- [Failure Handling](#failure-handling)
- [Scheduling](#scheduling)
- [Configuration](#configuration)
- [API Reference](#api-reference)
- [CLI Reference](#cli-reference)
- [Naming Conventions](#naming-conventions)
- [Common Patterns](#common-patterns)
- [Troubleshooting](#troubleshooting)
- [Limitations](#limitations)

---

## Quick Start

### 1. Enable workflows

Workflows are included by default. If building with selective features:

```bash
cargo run -p zenii-daemon --features workflows
```

### 2. Write a workflow file

```toml
id = "hello-world"
name = "Hello World"
description = "Search the web, summarize with AI, save to file"

[[steps]]
name = "search"
type = "tool"
tool = "web_search"
[steps.args]
query = "Rust programming language news today"

[[steps]]
name = "summarize"
type = "llm"
depends_on = ["search"]
prompt = "Summarize this in 3 bullet points:\n\n{{steps.search.output}}"

[[steps]]
name = "save"
type = "tool"
tool = "file_write"
depends_on = ["summarize"]
[steps.args]
path = "/tmp/news-summary.md"
content = "{{steps.summarize.output}}"
```

### 3. Register and run

```bash
# Register via CLI
zenii workflow create hello-world.toml

# Run it
zenii workflow run hello-world

# Or via API
curl -X POST http://127.0.0.1:18981/workflows \
  -H "Content-Type: application/json" \
  -d '{"toml_content": "..."}'

curl -X POST http://127.0.0.1:18981/workflows/hello-world/run
```

---

## TOML Format Reference

### Top-level fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `id` | string | Yes | — | Unique identifier (kebab-case recommended) |
| `name` | string | Yes | — | Human-readable name |
| `description` | string | Yes | — | What this workflow does |
| `schedule` | string | No | `null` | Cron expression for automatic execution |
| `steps` | array | Yes | — | Ordered list of workflow steps |

### Step fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | Yes | — | Step identifier (**must use underscores**, not hyphens) |
| `type` | string | Yes | — | One of: `tool`, `llm`, `condition`, `delay` |
| `depends_on` | array | No | `[]` | Step names this step waits for |
| `timeout_secs` | integer | No | `300` | Per-step timeout in seconds |
| `failure_policy` | string | No | `"stop"` | One of: `"stop"`, `"continue"`, or `{step = "name"}` for fallback |
| `retry` | object | No | `null` | `{max_retries = 3, retry_delay_ms = 1000}` |

### Type-specific fields

**Tool steps** (`type = "tool"`):

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `tool` | string | Yes | Registered tool name (e.g., `web_search`, `shell`, `channel_send`) |
| `args` | table | No | JSON arguments passed to the tool |

**LLM steps** (`type = "llm"`):

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `prompt` | string | Yes | Prompt text (supports template variables) |
| `model` | string | No | Model override (uses default provider if omitted) |

**Condition steps** (`type = "condition"`):

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `expression` | string | Yes | Evaluated for truthiness (non-empty and not `"false"`/`"0"`) |
| `if_true` | string | Yes | Step name to execute when true |
| `if_false` | string | No | Step name to execute when false |

**Delay steps** (`type = "delay"`):

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `seconds` | integer | Yes | Duration to pause in seconds |

---

## Step Types

### Tool

Calls any registered tool in the ToolRegistry. Template variables in `args` values are resolved before execution.

```toml
[[steps]]
name = "search"
type = "tool"
tool = "web_search"
[steps.args]
query = "latest AI coding tools 2026"
num_results = 5
```

**Available tools:**

| Tool | Key Arguments | Description |
|------|---------------|-------------|
| `web_search` | `query`, `num_results` | Web search (Tavily, Brave, DuckDuckGo cascade) |
| `system_info` | `action`: `"os"`, `"memory"`, `"cpu"`, `"all"` | System information |
| `file_read` | `path` | Read file contents |
| `file_write` | `path`, `content` | Write content to file |
| `file_list` | `path` | List directory contents |
| `file_search` | `path`, `pattern` | Search files by pattern |
| `shell` | `command` | Execute shell command (risk: high) |
| `process` | `action`: `"list"` | Process information |
| `channel_send` | `action`: `"send"`, `channel`, `message` | Send message via channel (requires `channels` feature) |
| `memory` | `action`: `"store"`, `"recall"`, `"forget"` | Agent memory operations |
| `config` | `action`: `"read"`, `"write"`, `key`, `value` | Read/write whitelisted config keys |

### LLM

Sends a prompt to the configured AI provider. Template variables in the prompt are resolved before the call.

```toml
[[steps]]
name = "analyze"
type = "llm"
depends_on = ["research"]
prompt = "Analyze this data and provide 3 key insights:\n\n{{steps.research.output}}"
model = "gpt-4o"
```

- If `model` is omitted, uses the default model from provider configuration.
- Requires the `ai` feature flag (enabled by default).

### Condition

Evaluates an expression for truthiness and branches to different steps.

```toml
[[steps]]
name = "check_health"
type = "condition"
depends_on = ["gather_metrics"]
expression = "{{steps.gather_metrics.output}}"
if_true = "report_healthy"
if_false = "alert_team"
```

**Truthiness rules:**
- Empty string → false
- `"false"` (case-insensitive) → false
- `"0"` → false
- Everything else → true

The condition step's output is the name of the branch it selected (e.g., `"report_healthy"`). Both `if_true` and `if_false` targets must be valid step names in the workflow.

### Delay

Pauses execution for a fixed duration. Useful for rate limiting or waiting between API calls.

```toml
[[steps]]
name = "cooldown"
type = "delay"
seconds = 5
depends_on = ["api_call"]
```

---

## Template Variables

Workflows use [Jinja2-style templates](https://jinja.palletsprojects.com/) (via minijinja) to pass data between steps.

### Syntax

```
{{steps.<step_name>.output}}    # Step output text
{{steps.<step_name>.success}}   # Boolean: true/false
{{steps.<step_name>.error}}     # Error message (empty string if no error)
```

### Rules

1. **Step names in templates must use underscores.** The template engine interprets `.` as property access and `-` as subtraction. A step named `web-search` in a template `{{steps.web-search.output}}` will fail with "undefined value" because it evaluates as `steps.web` minus `search.output`.

2. **Only completed upstream steps are available.** Referencing a step that hasn't executed yet (or isn't in `depends_on`) returns an empty string.

3. **Outputs are JSON-escaped** before substitution to prevent injection in nested JSON arguments.

4. **Newlines in prompts** — use `\n` in TOML strings or multi-line TOML strings with `"""`:

```toml
prompt = """
Analyze the following research results:

{{steps.research_cursor.output}}

{{steps.research_copilot.output}}

Compare pricing, features, and market positioning.
"""
```

### Example: chaining three steps

```toml
[[steps]]
name = "gather_data"
type = "tool"
tool = "web_search"
[steps.args]
query = "rust async runtime benchmarks"

[[steps]]
name = "analyze"
type = "llm"
depends_on = ["gather_data"]
prompt = "Based on this data:\n\n{{steps.gather_data.output}}\n\nWhat are the top 3 fastest runtimes?"

[[steps]]
name = "save_report"
type = "tool"
tool = "file_write"
depends_on = ["analyze"]
[steps.args]
path = "~/reports/async-benchmarks.md"
content = "# Async Runtime Benchmarks\n\n{{steps.analyze.output}}"
```

---

## Dependencies and Execution Order

### How it works

1. Steps are arranged into a **directed acyclic graph** (DAG) using `depends_on` references.
2. The executor performs a **topological sort** to determine execution order.
3. Steps without dependencies execute first. Steps wait for all their dependencies to complete.
4. The DAG is validated at registration time — cycles are rejected immediately.

### Implicit parallelism via the dependency graph

Steps that share no dependency relationship can execute in parallel implicitly. Rather than using an explicit `parallel` step type, express parallelism through the dependency graph:

```toml
# These three steps have no depends_on — they CAN run in parallel
[[steps]]
name = "research_cursor"
type = "tool"
tool = "web_search"
[steps.args]
query = "Cursor AI coding tool pricing features 2026"

[[steps]]
name = "research_copilot"
type = "tool"
tool = "web_search"
[steps.args]
query = "GitHub Copilot pricing features 2026"

[[steps]]
name = "research_windsurf"
type = "tool"
tool = "web_search"
[steps.args]
query = "Windsurf AI coding tool pricing features 2026"

# This step depends on all three — waits for all to complete
[[steps]]
name = "compare"
type = "llm"
depends_on = ["research_cursor", "research_copilot", "research_windsurf"]
prompt = """
Compare these AI coding tools:

Cursor: {{steps.research_cursor.output}}
Copilot: {{steps.research_copilot.output}}
Windsurf: {{steps.research_windsurf.output}}

Create a comparison table.
"""
```

> **Note**: The current executor runs steps serially in topological order. Steps without inter-dependencies are still executed one at a time. True parallel execution is planned for a future release.

### Diamond dependencies

The DAG supports diamond patterns where two branches converge:

```
      start
      /    \
  branch_a  branch_b
      \    /
      merge
```

```toml
[[steps]]
name = "start"
type = "tool"
tool = "system_info"
[steps.args]
action = "all"

[[steps]]
name = "branch_a"
type = "llm"
depends_on = ["start"]
prompt = "Analyze CPU usage: {{steps.start.output}}"

[[steps]]
name = "branch_b"
type = "llm"
depends_on = ["start"]
prompt = "Analyze memory usage: {{steps.start.output}}"

[[steps]]
name = "merge"
type = "llm"
depends_on = ["branch_a", "branch_b"]
prompt = "Combined analysis:\n\nCPU: {{steps.branch_a.output}}\nMemory: {{steps.branch_b.output}}"
```

### Validation errors

| Error | Cause | Fix |
|-------|-------|-----|
| `workflow contains cyclic dependencies` | Step A depends on B, B depends on A | Remove the cycle |
| `step 'X' depends on unknown step 'Y'` | Typo in `depends_on` | Check step names match exactly |
| `step 'X' has fallback to unknown step 'Y'` | Fallback references nonexistent step | Add the fallback step or fix the name |
| `workflow has N steps (max M)` | Too many steps | Increase `workflow_max_steps` config or split into multiple workflows |

---

## Failure Handling

### Failure policies

Each step can specify how the workflow responds to its failure:

**Stop** (default) — halt the entire workflow immediately:

```toml
[[steps]]
name = "critical_step"
type = "tool"
tool = "shell"
failure_policy = "stop"
[steps.args]
command = "run-migration.sh"
```

**Continue** — skip the failed step and proceed to the next:

```toml
[[steps]]
name = "optional_notify"
type = "tool"
tool = "channel_send"
failure_policy = "continue"
[steps.args]
action = "send"
channel = "telegram"
message = "Build complete"
```

**Fallback** — on failure, execute a recovery step instead:

```toml
[[steps]]
name = "primary_search"
type = "tool"
tool = "web_search"
[steps.args]
query = "rust news"

[steps.failure_policy]
fallback = { step = "backup_search" }

[[steps]]
name = "backup_search"
type = "tool"
tool = "shell"
[steps.args]
command = "curl -s https://api.example.com/fallback-news"
```

If the fallback step also fails, the workflow halts. Each fallback step is executed at most once per run.

### Retries

Configure per-step retry behavior:

```toml
[[steps]]
name = "flaky_api"
type = "tool"
tool = "shell"
[steps.args]
command = "curl https://api.example.com/data"

[steps.retry]
max_retries = 5
retry_delay_ms = 2000
```

- Default: 3 retries with 1000ms delay.
- Delay is fixed (no exponential backoff).
- Retries are attempted before the failure policy kicks in.

### Timeouts

```toml
[[steps]]
name = "slow_query"
type = "llm"
timeout_secs = 60
prompt = "Generate a detailed 5000-word report..."
```

- Default: 300 seconds (5 minutes) per step, configurable via `workflow_step_timeout_secs`.
- Per-step `timeout_secs` overrides the global default.
- On timeout, the step is marked failed and the failure policy applies.

---

## Scheduling

Add a `schedule` field to run workflows automatically via cron:

```toml
id = "morning-briefing"
name = "Morning Briefing"
description = "Daily news summary at 9 AM"
schedule = "0 9 * * *"

[[steps]]
name = "search"
type = "tool"
tool = "web_search"
[steps.args]
query = "tech news today"

[[steps]]
name = "brief"
type = "llm"
depends_on = ["search"]
prompt = "Create a morning briefing from:\n\n{{steps.search.output}}"
```

### Cron syntax

Standard 5-field cron expressions:

```
┌───── minute (0-59)
│ ┌───── hour (0-23)
│ │ ┌───── day of month (1-31)
│ │ │ ┌───── month (1-12)
│ │ │ │ ┌───── day of week (0-6, 0 = Sunday)
│ │ │ │ │
* * * * *
```

| Expression | Meaning |
|------------|---------|
| `0 9 * * *` | Every day at 9:00 AM |
| `0 9 * * 1-5` | Weekdays at 9:00 AM |
| `*/30 * * * *` | Every 30 minutes |
| `0 0 1 * *` | First day of each month at midnight |
| `0 */6 * * *` | Every 6 hours |

When a workflow with a `schedule` is registered, Zenii automatically creates a scheduler job. The scheduler checks for due jobs every second (configurable via `scheduler_tick_interval_secs`).

Requires the `scheduler` feature flag:

```bash
cargo run -p zenii-daemon --features "workflows,scheduler"
```

---

## Configuration

All workflow tunables are in `config.toml`:

```toml
# Workflow configuration
workflow_dir = "/path/to/workflows"      # Default: {data_dir}/workflows
workflow_max_steps = 50                  # Max steps per workflow (default: 50)
workflow_step_timeout_secs = 300         # Default per-step timeout (default: 300)
workflow_step_max_retries = 3            # Default retry attempts (default: 3)
workflow_max_concurrent = 5              # Max concurrent workflow runs (default: 5)
```

---

## API Reference

All endpoints are feature-gated on `workflows`. Base URL: `http://127.0.0.1:18981`.

| Method | Path | Description | Response |
|--------|------|-------------|----------|
| `POST` | `/workflows` | Create workflow from TOML | `201` with workflow JSON |
| `GET` | `/workflows` | List all workflows | `200` with array |
| `GET` | `/workflows/{id}` | Get workflow details | `200` with workflow JSON |
| `PUT` | `/workflows/{id}` | Update workflow from TOML | `200` with updated workflow |
| `DELETE` | `/workflows/{id}` | Delete workflow | `204` |
| `GET` | `/workflows/{id}/raw` | Get raw TOML source | `200` text/plain |
| `POST` | `/workflows/{id}/run` | Execute workflow (async) | `202` with `{workflow_id, run_id}` |
| `POST` | `/workflows/{id}/runs/{run_id}/cancel` | Cancel a running workflow | `200` |
| `GET` | `/workflows/{id}/history` | Get execution history | `200` with runs array |
| `GET` | `/workflows/{id}/runs/{run_id}` | Get run details with step results | `200` with run JSON |

### Create workflow

```bash
curl -X POST http://127.0.0.1:18981/workflows \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"toml_content": "id = \"my-workflow\"\nname = \"My Workflow\"..."}'
```

### Run workflow

```bash
curl -X POST http://127.0.0.1:18981/workflows/my-workflow/run \
  -H "Authorization: Bearer <token>"

# Response: {"workflow_id": "my-workflow", "run_id": "550e8400-..."}
```

Runs are asynchronous — the API returns immediately with a `run_id`. Poll the run details endpoint for status.

### Get run details

```bash
curl http://127.0.0.1:18981/workflows/my-workflow/runs/550e8400-... \
  -H "Authorization: Bearer <token>"
```

Response includes per-step results with output, success status, duration, and any errors.

---

## CLI Reference

```bash
# Register a workflow from TOML file
zenii workflow create <file.toml>

# List all registered workflows
zenii workflow list

# Get workflow details
zenii workflow get <id>

# Execute a workflow
zenii workflow run <id>

# Delete a workflow
zenii workflow delete <id>

# View execution history
zenii workflow history <id>
```

---

## Naming Conventions

### Step names: use underscores, never hyphens

**This is the single most important rule.** Step names are used as identifiers in template variable interpolation. The minijinja template engine interprets hyphens as the subtraction operator:

```
{{steps.web-search.output}}
        ^^^^^^^^^^
        Parsed as: steps.web MINUS search.output
        Result: "undefined value" error
```

| Bad | Good |
|-----|------|
| `web-search` | `web_search` |
| `gather-context` | `gather_context` |
| `notify-team` | `notify_team` |
| `check-health` | `check_health` |

### Workflow IDs: kebab-case is fine

The workflow `id` field is not used in templates, so kebab-case is acceptable:

```toml
id = "daily-news-summary"     # OK — not used in templates
name = "Daily News Summary"

[[steps]]
name = "search_news"          # Underscores — used in templates
type = "tool"
tool = "web_search"
```

### General guidelines

- Keep step names short but descriptive: `research_pricing`, `analyze_risk`, `notify_team`
- Use a verb prefix: `search_`, `analyze_`, `notify_`, `save_`, `check_`
- Keep workflow IDs under 50 characters
- Use lowercase only for step names

---

## Common Patterns

### Research and summarize

The most common pattern: gather data, process with AI, save or send results.

```toml
id = "research-pipeline"
name = "Research Pipeline"
description = "Web research with AI analysis"

[[steps]]
name = "gather"
type = "tool"
tool = "web_search"
[steps.args]
query = "topic of interest"

[[steps]]
name = "analyze"
type = "llm"
depends_on = ["gather"]
prompt = "Analyze: {{steps.gather.output}}"

[[steps]]
name = "save"
type = "tool"
tool = "file_write"
depends_on = ["analyze"]
[steps.args]
path = "~/reports/analysis.md"
content = "{{steps.analyze.output}}"
```

### Fan-out / fan-in

Run multiple independent research tasks, then merge results:

```toml
[[steps]]
name = "research_a"
type = "tool"
tool = "web_search"
[steps.args]
query = "topic A"

[[steps]]
name = "research_b"
type = "tool"
tool = "web_search"
[steps.args]
query = "topic B"

[[steps]]
name = "merge"
type = "llm"
depends_on = ["research_a", "research_b"]
prompt = "Combine:\n\nA: {{steps.research_a.output}}\n\nB: {{steps.research_b.output}}"
```

### Conditional branching

Route execution based on a prior step's output:

```toml
[[steps]]
name = "check_status"
type = "tool"
tool = "shell"
[steps.args]
command = "curl -s -o /dev/null -w '%{http_code}' https://api.example.com/health"

[[steps]]
name = "evaluate"
type = "condition"
depends_on = ["check_status"]
expression = "{{steps.check_status.output}}"
if_true = "report_ok"
if_false = "alert_team"

[[steps]]
name = "report_ok"
type = "tool"
tool = "channel_send"
depends_on = ["evaluate"]
failure_policy = "continue"
[steps.args]
action = "send"
channel = "telegram"
message = "All systems operational"

[[steps]]
name = "alert_team"
type = "tool"
tool = "channel_send"
depends_on = ["evaluate"]
failure_policy = "continue"
[steps.args]
action = "send"
channel = "telegram"
message = "ALERT: API health check failed!"
```

### System health check with notification

```toml
id = "health-check"
name = "System Health Check"
description = "Check system metrics and notify"
schedule = "0 */6 * * *"

[[steps]]
name = "os_info"
type = "tool"
tool = "system_info"
[steps.args]
action = "os"

[[steps]]
name = "memory_info"
type = "tool"
tool = "system_info"
[steps.args]
action = "memory"

[[steps]]
name = "report"
type = "llm"
depends_on = ["os_info", "memory_info"]
prompt = "Create a health status summary:\n\nOS: {{steps.os_info.output}}\nMemory: {{steps.memory_info.output}}\n\nFlag anything concerning."

[[steps]]
name = "notify"
type = "tool"
tool = "channel_send"
depends_on = ["report"]
failure_policy = "continue"
timeout_secs = 30
[steps.args]
action = "send"
channel = "telegram"
message = "Health Report:\n\n{{steps.report.output}}"
```

### Graceful degradation with fallback

```toml
[[steps]]
name = "primary_search"
type = "tool"
tool = "web_search"
[steps.args]
query = "important topic"

[steps.failure_policy]
fallback = { step = "cached_search" }

[[steps]]
name = "cached_search"
type = "tool"
tool = "file_read"
[steps.args]
path = "~/cache/last-search-result.txt"
```

---

## Troubleshooting

### "undefined value" in template rendering

**Cause**: Step name contains hyphens. Template engine interprets `{{steps.my-step.output}}` as subtraction.

**Fix**: Rename the step to use underscores: `my_step`.

### "tool 'X' not found"

**Cause**: The tool name doesn't match any registered tool.

**Fix**: Check available tools with `GET /tools` or `zenii tools list`. Tool names are exact matches — `web_search` not `websearch`.

### "workflow contains cyclic dependencies"

**Cause**: Step A depends on B, and B depends on A (directly or transitionally).

**Fix**: Draw out the dependency graph and remove the cycle. Workflows must be DAGs.

### Step times out but should succeed

**Cause**: Default timeout is 300s; some LLM calls or shell commands take longer.

**Fix**: Add `timeout_secs` to the step:

```toml
[[steps]]
name = "long_running"
type = "llm"
timeout_secs = 600
prompt = "Generate a very detailed report..."
```

### Channel notifications fail silently

**Cause**: `channel_send` tool requires the `channels` feature flag and a configured channel.

**Fix**:
1. Build with `--features channels`
2. Configure the channel credentials in Settings > Channels
3. Ensure the step has `action = "send"` in its args

### Scheduled workflow doesn't run

**Cause**: Scheduler feature not enabled, or cron expression is invalid.

**Fix**:
1. Build with `--features "workflows,scheduler"`
2. Verify the cron expression at [crontab.guru](https://crontab.guru)
3. Check scheduler logs for errors

---

## Limitations

- **Serial execution**: Steps execute one at a time in topological order. True parallel execution is planned for a future release.
- **No step-level cancellation**: Cancellation is per-run only. A running step completes before the cancellation takes effect.
- **Fixed retry delay**: No exponential backoff. All retries use the same `retry_delay_ms`.
- **Condition evaluation is simple**: Only truthy/falsy checks (non-empty, not "false"/"0"). No arithmetic or comparison operators.
- **No workflow versioning**: Updates overwrite the definition. No history of definition changes.
- **Output size unbounded**: Large tool or LLM outputs are stored in full. Long-running workflows with verbose steps can accumulate significant memory.
