# API Endpoints

Canonical list of all HTTP endpoints served by the Zenii gateway.
One entry per HTTP method+path pair. Used by `scripts/count-endpoints.sh` to compute `totalEndpoints` in `website-data.json`.

**Count command:** `grep -cE '^(GET|POST|PUT|DELETE|PATCH) ' docs/api-endpoints.md`

**Update rule:** When adding or removing a route in `routes.rs`, update this file and re-run the count.

---

## System (no auth)

GET /health
GET /.well-known/agent.json

## Sessions

POST /sessions
GET /sessions
GET /sessions/{id}
PUT /sessions/{id}
DELETE /sessions/{id}
POST /sessions/{id}/generate-title

## Messages

GET /sessions/{id}/messages
POST /sessions/{id}/messages
DELETE /sessions/{id}/messages/{message_id}/and-after

## Chat

POST /chat

## Memory

POST /memory
GET /memory
GET /memory/{key}
PUT /memory/{key}
DELETE /memory/{key}

## Wiki

GET /wiki
GET /wiki/search
POST /wiki/ingest
POST /wiki/upload
POST /wiki/sync
GET /wiki/graph
POST /wiki/query
POST /wiki/lint
GET /wiki/prompt
PUT /wiki/prompt
GET /wiki/sources
DELETE /wiki/sources
DELETE /wiki/sources/{filename}
POST /wiki/sources/{filename}/regenerate
DELETE /wiki/pages
DELETE /wiki/pages/{slug}
POST /wiki/regenerate
GET /wiki/dir
GET /wiki/converter/status
GET /wiki/{slug}

## Config

GET /config
PUT /config
GET /config/file

## Setup / Onboarding

GET /setup/status

## Credentials

POST /credentials
GET /credentials
DELETE /credentials/{key}
GET /credentials/{key}/value
GET /credentials/{key}/exists

## Providers

GET /providers
POST /providers
GET /providers/with-key-status
GET /providers/default
PUT /providers/default
GET /providers/{id}
PUT /providers/{id}
DELETE /providers/{id}
POST /providers/{id}/test
POST /providers/{id}/models
DELETE /providers/{id}/models/{model_id}

## Tools

GET /tools
POST /tools/{name}/execute

## Permissions

GET /permissions
GET /permissions/{surface}
PUT /permissions/{surface}/{tool}
DELETE /permissions/{surface}/{tool}

## System Info

GET /system/info

## Models

GET /models

## Identity

GET /identity
POST /identity/reload
GET /identity/{name}
PUT /identity/{name}

## Skills

GET /skills
POST /skills
POST /skills/reload
GET /skills/{id}
PUT /skills/{id}
DELETE /skills/{id}

## Skill Proposals

GET /skills/proposals
POST /skills/proposals/{id}/approve
POST /skills/proposals/{id}/reject
DELETE /skills/proposals/{id}

## User

GET /user/observations
POST /user/observations
DELETE /user/observations
GET /user/observations/{key}
DELETE /user/observations/{key}
GET /user/profile

## Embeddings

GET /embeddings/status
POST /embeddings/test
POST /embeddings/embed
POST /embeddings/download
POST /embeddings/reindex

## Plugins

GET /plugins
POST /plugins/install
GET /plugins/available
GET /plugins/{name}
DELETE /plugins/{name}
PUT /plugins/{name}/toggle
POST /plugins/{name}/update
GET /plugins/{name}/config
PUT /plugins/{name}/config

## Channels (credential test — always available)

POST /channels/{name}/test

## Agent Delegation

GET /agents/active
POST /agents/{id}/cancel

## Approvals

GET /approvals/rules
DELETE /approvals/rules/{id}
POST /approvals/{id}/respond

## WebSocket

GET /ws/chat
GET /ws/notifications

---

## Feature-gated routes

The routes below are compiled in only when the specified feature flag is enabled.
They are included in the total count.

### [feature: channels]

GET /channels/sessions
GET /channels/sessions/{id}/messages
GET /channels
GET /channels/{name}/status
POST /channels/{name}/send
POST /channels/{name}/connect
POST /channels/{name}/disconnect
GET /channels/{name}/health
POST /channels/{name}/message

### [feature: scheduler]

GET /scheduler/jobs
POST /scheduler/jobs
PUT /scheduler/jobs/{id}/toggle
PUT /scheduler/jobs/{id}
DELETE /scheduler/jobs/{id}
GET /scheduler/jobs/{id}/history
GET /scheduler/status

### [feature: workflows]

POST /workflows/generate
GET /workflows
POST /workflows
GET /workflows/{id}
PUT /workflows/{id}
DELETE /workflows/{id}
GET /workflows/{id}/raw
POST /workflows/{id}/run
POST /workflows/{id}/runs/{run_id}/cancel
GET /workflows/{id}/history
GET /workflows/{id}/runs/{run_id}

### [feature: api-docs]

GET /api-docs
GET /api-docs/openapi.json
