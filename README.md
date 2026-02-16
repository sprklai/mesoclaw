# Tauri AI Boilerplate

A production-ready Tauri 2 desktop application boilerplate with integrated AI capabilities. Build cross-platform desktop apps powered by multiple AI providers with secure credential storage and a modern React frontend.

## Features

### 🤖 Multi-Provider AI Integration
- **OpenAI** - GPT-4, GPT-3.5-turbo
- **Anthropic** - Claude 3.5 Sonnet, Claude 3 Opus
- **Vercel AI Gateway** - Unified API gateway
- **OpenRouter** - Access to 100+ models
- **Ollama** - Local LLM support

### 🔐 Secure Credential Management
- OS-level keyring storage for API keys
- Encrypted credential management
- Secure AI provider configuration

### 🎨 Modern Frontend Stack
- **React 19** with TypeScript
- **Tailwind CSS 4** for styling
- **TanStack Router** for type-safe routing
- **Zustand** for state management
- **Base UI** components (shadcn-style)
- **AI SDK Elements** for chat interfaces

### ⚙️ Powerful Backend
- **Rust** with Tauri 2 framework
- **Diesel ORM** for settings persistence
- **Tokio** async runtime
- **SQLite** for local app database

### 🛠️ Developer Experience
- Hot reload development mode
- TypeScript type safety
- Ultracite code formatting
- Comprehensive error handling
- Cross-platform builds (Windows, macOS, Linux)

### Completed

- ✅ **Phase 1.1**: Database Provider Abstraction ([docs](docs/phases/PHASE_1.1_DATABASE_PROVIDER_ABSTRACTION.md))
  - Core trait interface for database operations
  - SQLite provider with full schema introspection
  - Connection pool management and configuration
  - Comprehensive metadata structures
- ✅ **Phase 1.2**: SQLite Provider Implementation ([docs](docs/phases/PHASE_1.2_SQLITE_PROVIDER.md))
  - Complete SQLite provider with all introspection methods
  - Connection lifecycle management
  - Read-only mode enforcement
  - Lightweight statistics gathering
- ✅ **Phase 1.3**: Schema Introspection Engine ([docs](docs/phases/PHASE_1.3_SCHEMA_INTROSPECTION_ENGINE.md))
  - Table role inference (junction, audit, config, transactional)
  - Column semantic detection (timestamps, soft deletes, user tracking)
  - Implicit relationship inference with confidence scoring
  - Naming convention and pattern detection
  - Docker-based test environment
- ✅ **Phase 1.4**: Schema Fingerprinting & Change Detection ([docs](docs/phases/PHASE_1.4_SCHEMA_FINGERPRINTING.md))
  - SHA-256 cryptographic fingerprinting
  - Table-level granularity for efficient cache invalidation
  - Detailed schema diff computation
  - Change detection with 100% test coverage
- ✅ **Phase 1.5**: Workspace Management ([docs](docs/phases/PHASE_1.5_WORKSPACE_MANAGEMENT.md))
  - Workspace CRUD operations with UUID-based IDs
  - Schema snapshot storage and retrieval
  - Cascade deletion of related data
  - 3/3 tests passing
- ✅ **Phase 1.6**: Knowledge Store ([docs](docs/phases/PHASE_1.6_KNOWLEDGE_STORE.md))
  - Explanation caching with fingerprint validation
  - User annotation storage
  - Cache invalidation logic
  - 5/5 tests passing
- ✅ **Phase 1.7**: Credential Store ([docs](docs/phases/PHASE_1.7_CREDENTIAL_STORE.md))
  - OS-native keychain integration (macOS, Windows, Linux)
  - Secure API key storage for Vercel AI Gateway
  - Memory-safe credential handling with zeroize
  - 6/6 tests passing
- ✅ **Phase 1.8**: Database Migrations ([docs](docs/phases/PHASE_1.8_DATABASE_MIGRATIONS.md))
  - All migrations verified and documented
  - 4 tables with proper indexes and foreign keys
  - Complete rollback scripts
  - Schema.rs up to date

## Phase 1 Complete! 🎉

All backend infrastructure is now in place:

- Database abstraction layer ✅
- Schema introspection and fingerprinting ✅
- Workspace and knowledge management ✅
- Secure credential storage ✅
- Complete database migrations ✅

## Phase 2 Complete! 🎉

All AI integration components are now implemented:

- LLM provider abstraction ✅
- Vercel AI Gateway integration ✅
- Prompt template system ✅
- Schema interpretation agent ✅
- Relationship inference agent ✅
- Query explanation agent ✅
- Onboarding documentation agent ✅
- Explanation caching system ✅

### Phase 2 Details

- ✅ **Phase 2.1**: LLM Provider Abstraction ([docs](docs/phases/PHASE_2.1_LLM_PROVIDER_ABSTRACTION.md))
  - LLMProvider trait for multiple AI providers
  - Message types (CompletionRequest, CompletionResponse)
  - Context window management with token estimation
  - Streaming response support
  - 7/7 tests passing

- ✅ **Phase 2.2**: Vercel AI Gateway Provider ([docs](docs/phases/PHASE_2.2_VERCEL_AI_GATEWAY_PROVIDER.md))
  - Production-ready HTTP client with retry logic
  - Streaming support via Server-Sent Events (SSE)
  - Multi-provider access (OpenAI, Anthropic, Google, xAI)
  - Model-specific context window detection
  - Secure API key storage integration
  - 6/6 tests passing

- ✅ **Phase 2.3**: Prompt Template System ([docs](docs/phases/PHASE_2.3_PROMPT_TEMPLATE_SYSTEM.md))
  - PromptTemplate trait for structured prompts
  - Schema prompts (tables, columns, patterns)
  - Relationship prompts (explicit, implicit, network)
  - Query prompts (explain, compare, optimize)
  - Onboarding prompts (overview, key entities, quick start)
  - 24/24 tests passing

- ✅ **Phase 2.4**: Schema Interpretation Agent ([docs](docs/phases/PHASE_2.4_SCHEMA_INTERPRETATION_AGENT.md))
  - Multi-stage table explanation with evidence linking
  - Column interpretation with context
  - Pattern identification across database
  - Confidence scoring (High/Medium/Low)
  - Fingerprint-based cache invalidation
  - 6/6 tests passing

- ✅ **Phase 2.5**: Relationship Inference Agent ([docs](docs/phases/PHASE_2.5_RELATIONSHIP_INFERENCE_AGENT.md))
  - Explicit relationship explanation with cardinality
  - Lifecycle behavior detection (cascade, restrict)
  - Implicit relationship inference from naming patterns
  - Relationship network analysis
  - Configurable confidence filtering
  - 9/9 tests passing

- ✅ **Phase 2.6**: Query Explanation Agent ([docs](docs/phases/PHASE_2.6_QUERY_EXPLANATION_AGENT.md))
  - SQL parser integration (sqlparser-rs)
  - Query structure analysis (tables, joins, filters, aggregations)
  - Semantic query explanation
  - Query comparison with structural and semantic diffs
  - Optimization suggestions
  - 17/17 tests passing

- ✅ **Phase 2.7**: Onboarding Documentation Synthesis Agent ([docs](docs/phases/PHASE_2.7_ONBOARDING_AGENT.md))
  - Database overview generation
  - Key entity identification
  - Exploration path suggestions
  - Quick start guide creation
  - Fingerprint-based caching
  - 6/6 tests passing

- ✅ **Phase 2.8**: Explanation Cache ([docs](docs/phases/PHASE_2.8_EXPLANATION_CACHE.md))
  - Two-tier caching (in-memory LRU + persistent KnowledgeStore)
  - Configurable cache size (default 1000 entries)
  - Cache statistics tracking (hits, misses, hit rate)
  - Fingerprint-based invalidation
  - Thread-safe access
  - 8/8 tests passing

## Phase 3: Frontend UI (In Progress)

### Completed

- ✅ **Phase 3.1**: Connection Management
  - ConnectionDialog component with SQLite file picker
  - SQLiteFilePicker with browse and recent files
  - WorkspaceSwitcher dropdown for workspace management
  - ConnectionStatus indicator with read-only mode display
  - Connection testing and validation
  - Form validation and error handling

- ✅ **Phase 3.2**: Schema Explorer ([docs](docs/phases/PHASE_3.2_SCHEMA_EXPLORER.md))
  - SchemaTree component with search and filtering
  - SchemaTreeNode with expand/collapse and context menu
  - TableDetailPanel with tabs for columns, relationships, and stats
  - RelationshipGraph visualization with reactflow
  - ColumnList and RelationshipList components
  - Schema store for state management
  - Backend Tauri commands for schema metadata

- ✅ **Phase 3.3**: Explanation Interface
  - ExplanationPanel component with markdown support
  - EvidenceCitation and EvidenceList components
  - ConfidenceIndicator with visual feedback
  - AnnotationInput for user notes
  - Export to markdown functionality
  - Loading, error, and cached states
  - Integration with backend explanation commands

- ✅ **Phase 3.4**: Query Understanding ([docs](docs/phases/PHASE_3.4_QUERY_UNDERSTANDING.md))
  - QueryInput with CodeMirror SQL editor
  - SQL syntax highlighting and autocomplete
  - QueryStructure component for visual query breakdown
  - QueryExplanationView with tabbed interface
  - QueryComparisonView for side-by-side query analysis
  - Read-only validation and query history
  - Structural and semantic diff visualization

- ✅ **Phase 3.5**: Onboarding Dashboard ([docs](docs/phases/PHASE_3.5_ONBOARDING_DASHBOARD.md))
  - OnboardingDashboard component with tabbed interface
  - KeyEntitiesList for displaying important entities
  - PatternsList for detected schema patterns
  - QuickStartGuide with exploration paths
  - Export documentation to Markdown
  - Automatic data fetching and regeneration
  - Loading, error, and success states

- ✅ **Phase 3.6**: Settings ([docs](docs/phases/PHASE_3.6_SETTINGS.md))
  - SettingsDialog with tabbed interface (AI Provider, Privacy, Cache)
  - VercelAISettings for API key and model configuration
  - PrivacySettings for cloud AI toggle and verbosity control
  - CacheManagement for clearing cached explanations
  - Test connection functionality with visual feedback
  - 9 pre-configured AI models (OpenAI, Anthropic, Google, xAI)
  - Privacy notice and cache information sections
  - API key persistence on app restart via LLM store initialization

- ✅ **Phase 3.7**: State Management ([docs](docs/phases/PHASE_3.7_STATE_MANAGEMENT.md))
  - Workspace Store for workspace CRUD and connection management
  - Schema Store for schema metadata, search, and filtering
  - Explanation Store for AI-generated explanations and annotations
  - LLM Store for API key and model configuration with app initialization
  - Zustand-based state management with TypeScript
  - Cache-first strategy for explanations
  - Map/Set-based efficient lookups
  - Consistent error handling and loading states

- ✅ **Phase 3.8**: Routing ([docs](docs/phases/PHASE_3.8_ROUTING.md))
  - TanStack Router file-based routing
  - Home route with connection dialog and auto-navigation
  - Workspace routes (schema explorer, table details, query, onboarding)
  - Settings route for application configuration
  - Connection status integration with "New Connection" button
  - Workspace validation and connection guards
  - Deep-linking support for tables
  - Type-safe route parameters

## Phase 3 Complete! 🎉

All frontend UI components are now implemented:

- Connection management ✅
- Schema explorer ✅
- Explanation interface ✅
- Query understanding ✅
- Onboarding dashboard ✅
- Settings ✅
- State management ✅
- Routing ✅

## Phase 4: IPC Commands (In Progress)

### Completed

- ✅ **Phase 4.1**: Database Commands ([docs](docs/phases/PHASE_4.1_DATABASE_COMMANDS.md))
  - create_workspace_command - Create workspace and connect to SQLite
  - list_workspaces_command - List all workspaces
  - switch_workspace_command - Switch workspace and update timestamp
  - delete_workspace_command - Delete workspace with cascade cleanup
  - test_connection_command - Test database connection
  - disconnect_workspace_command - Disconnect from workspace
  - Read-only enforcement and connection testing

- ✅ **Phase 4.2**: Schema Commands ([docs](docs/phases/PHASE_4.2_SCHEMA_COMMANDS.md))
  - get_schema_metadata_command - Complete schema snapshot
  - get_table_details_command - Detailed table information
  - get_relationships_command - All relationships (explicit and inferred)
  - search_schema_command - Search with relevance scoring
  - Advanced search algorithm (exact, starts-with, contains, fuzzy)
  - Fingerprint-based caching for performance

- ✅ **Phase 4.3**: Explanation Commands ([docs](docs/phases/PHASE_4.3_EXPLANATION_COMMANDS.md))
  - explain_table_command - AI explanation of table purpose and role
  - explain_column_command - AI explanation of column meaning
  - explain_relationship_command - AI explanation of table relationships
  - explain_query_command - SQL query parsing and explanation
  - compare_queries_command - Compare two queries with structural and semantic diff
  - Multi-stage LLM prompting with evidence extraction
  - Confidence scoring and fingerprint-based caching

- ✅ **Phase 4.4**: Onboarding Commands ([docs](docs/phases/PHASE_4.4_ONBOARDING_COMMANDS.md))
  - generate_onboarding_summary_command - Generate comprehensive onboarding documentation
  - Database overview generation
  - Key entity identification with importance scores
  - Common pattern detection (soft deletes, audit timestamps, user tracking)
  - Exploration path suggestions
  - Quick start guide with example queries
  - Markdown-formatted output

- ✅ **Phase 4.5**: Knowledge Commands ([docs](docs/phases/PHASE_4.5_KNOWLEDGE_COMMANDS.md))
  - save_annotation_command - Save user annotations for schema entities
  - get_annotations_command - Retrieve annotations by entity ID
  - clear_cache_command - Clear all cached explanations
  - get_cache_stats_command - Get cache statistics breakdown
  - Persistent annotation storage with timestamps
  - Cache management with workspace isolation

### Next: Phase 4.6-4.7

Remaining IPC commands to implement:

- LLM commands (configure provider, test connection, get config)
- Command registration finalization

## SSH Tunnel Support

**Status**: ✅ Implementation complete (~95% complete, testing pending)

See [network/SSH_TUNNEL.md](docs/network/SSH_TUNNEL.md) for SSH tunnel usage and [database/DATABASE_CONNECTION_METHODS.md](docs/database/DATABASE_CONNECTION_METHODS.md) for all connection methods.

**Completed Components:**

- ✅ SSH tunnel service with `ssh2` crate (password + private key auth)
- ✅ PostgreSQL provider SSH tunnel integration
- ✅ MySQL provider SSH tunnel integration
- ✅ SSH configuration UI component (`SshTunnelConfig.tsx`)
- ✅ Connection dialog with database type selector (SQLite/PostgreSQL/MySQL)
- ✅ Workspace store updates for network databases
- ✅ SSH connection testing command
- ✅ **SSH keyring integration** (complete):
  - SSH passwords stored in OS keyring on workspace creation
  - SSH passphrases (for encrypted keys) stored in OS keyring
  - Credentials automatically loaded from keyring when connecting to existing workspaces
  - Credentials automatically deleted from keyring when workspace is deleted
  - Credentials managed during workspace updates
- ✅ Secure credential types in keyring (SshPassword, SshPassphrase)

**Remaining Tasks:**

- ⏳ Integration tests with Docker environment
- ⏳ Manual testing and final documentation

**Security Features:**

- Credentials never written to disk unencrypted
- Sensitive data zeroized from memory
- Private key permissions validated
- No credentials in logs or error messages
- OS keyring storage for SSH passwords and passphrases
- Automatic keyring cleanup on workspace deletion

## MongoDB Integration

**Status**: ✅ ~95% Complete - See [plans/TASKS_MONGODB.md](docs/plans/TASKS_MONGODB.md) for implementation details

MongoDB integration adds support for document-based NoSQL databases with intelligent schema inference for schema-less collections and automatic relationship detection.

**Implemented Features:**

- ✅ Database paradigm abstraction (Relational vs Document)
- ✅ MongoDB provider with connection string parsing (standard + SRV)
- ✅ Schema inference engine for heterogeneous collections
- ✅ Field metadata with multiple BSON types and presence percentages
- ✅ Relationship detection (manual references, DBRef, embedded documents, array references)
- ✅ Collection types (regular, capped, view, time-series)
- ✅ SSH tunnel support for MongoDB
- ✅ Frontend UI (CollectionDetailPanel, FieldMetadataList, SchemaInferencePanel)
- ✅ AI prompts for collection and field explanation
- ✅ Integration tests (18 tests)
- ✅ Manual testing documentation

**Documentation:**

- [MongoDB Connection Guide](docs/database/MONGODB_CONNECTION_GUIDE.md)
- [Manual Testing Plan](docs/MANUAL_TESTING_MONGODB.md)
- [Docker Test Environment](dockertests/README.md#mode-3-mongodb-testing)

**Remaining:**

- Documentation completion (Phase 7.14)

## Supabase Database Integration (Planned)

**Status**: Planned for post-MVP - See [plans/TASKS_SUPABASE.md](docs/plans/TASKS_SUPABASE.md) for implementation details

Supabase integration will add specialized support for Supabase-hosted PostgreSQL databases with SSH tunneling capabilities.

**Planned Features:**

- Connection string parser for Supabase formats
- Supabase provider extending PostgreSQL
- Auth, Storage, and Functions schema detection
- SSL enforcement (Supabase requires SSL)
- Connection mode support (direct, pooler, pooler-transaction)

**Estimated Effort**: 26-38 hours of development

## AI Skill System

**Status**: ✅ Complete - See [docs/features/SKILL_SYSTEM.md](docs/features/SKILL_SYSTEM.md) for full documentation

The Skill System provides modular, composable AI capabilities for database analysis. Skills can be enabled/disabled per workspace and composed together for complex analyses.

**Available Skills (8 total):**

| Category          | Skills                                                                                       |
| ----------------- | -------------------------------------------------------------------------------------------- |
| **Understanding** | Schema Explainer, Relationship Mapper, Query Explainer, Query Comparer, Collection Explainer |
| **Performance**   | Query Optimizer                                                                              |
| **Security**      | Security Audit                                                                               |
| **Documentation** | Onboarding Generator                                                                         |

**Key Features:**

- **Modular Design**: Enable/disable skills per workspace
- **Skill Composition**: Combine skills for complex analyses (merge, chain, parallel)
- **Automatic Selection**: LLM-based skill selection from user requests
- **Skill Suggestions**: Get relevant skill recommendations for any query
- **Extensible**: Add custom skills via markdown + YAML frontmatter

**Architecture:**

```
┌─────────────────────────────────────────────────────────────┐
│                     SKILL ENGINE                            │
│  Loader → Selector → Composer → Executor                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   APPLICATION ADAPTER                       │
│  Database context • Tools • Output mapping                  │
└─────────────────────────────────────────────────────────────┘
```

**Usage:**

```typescript
// Get skill suggestions
const suggestions = await suggestSkills(
  workspaceId,
  "optimize this query",
  "postgresql"
);

// Execute with automatic skill selection
const result = await executeWithSkills(
  workspaceId,
  "Explain the users table",
  providerId,
  apiKey
);

// Execute a specific skill
const result = await executeSkill(
  workspaceId,
  "query-optimizer",
  query,
  providerId,
  apiKey
);
```

## Features (Planned)

- **Database Comprehension** — AI-powered explanations of database schemas, tables, and relationships
- **Quick Onboarding** — Understand a new database within 30-90 minutes
- **Local-first** — All schema metadata and explanations stored locally
- **Privacy-focused** — Optional cloud LLM with clear indicators, local LLM support planned
- **Read-only** — Safe introspection without modifying your database
- **Multi-database** — SQLite (complete), PostgreSQL and MySQL (in progress)
- **SSH Tunneling** — Secure remote database connections via SSH tunnel (in progress)
- **MongoDB Support** — NoSQL/document database support with schema inference (planned)
- **Supabase Support** — Specialized Supabase integration with connection string parsing and schema detection (planned)
- **Internationalization** — Multi-language support with frontend-only i18n (planned - see [i18n Implementation Plan](docs/plans/TASKS_I18N.md))

## MVP Scope

- **Database Support**: SQLite (complete), PostgreSQL and MySQL (in progress)
- **AI Integration**: Cloud-first via Vercel AI Gateway
- **Features**: Schema introspection, table/column explanations, relationship inference, query understanding

**Post-MVP Plans:**

- SSH tunnel support (in development - ~85% complete)
- MongoDB/NoSQL database integration (planned - 178-239 hours)
- Supabase database integration (planned - 26-38 hours)
- Local LLM support (planned)

## Quick Start

```bash
# Clone the repository
git clone https://github.com/zap-studio/aiboilerplate.git aiboilerplate
cd aiboilerplate

# Install dependencies
bun install

# Run in development mode
bun run tauri dev
```

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/tools/install)
- [bun](https://bun.sh/) (recommended)

## Testing

### Backend Unit Tests

The project has **120 comprehensive unit tests** covering all backend components.

```bash
# Run all unit tests
cd src-tauri
cargo test --lib

# Run specific test module
cargo test --lib database::providers::sqlite::tests

# Run with output
cargo test --lib -- --nocapture
```

**Test Coverage**:

- Database providers (SQLite) - 17 tests
- Services (workspace, knowledge, credentials, introspection) - 15 tests
- AI agents and prompts - 86 tests
- Commands - 2 tests

See [Phase 5.1 Documentation](docs/phases/PHASE_5.1_BACKEND_UNIT_TESTS.md) for detailed test information.

## Releasing

The project uses automated release scripts and GitHub Actions for cross-platform builds.

```bash
# Check version status
./scripts/release.sh status

# Create a release (choose one)
./scripts/release.sh patch   # Bug fix: 0.0.1 → 0.0.2
./scripts/release.sh minor   # Feature: 0.0.2 → 0.1.0
./scripts/release.sh major   # Breaking: 0.1.0 → 1.0.0
```

This will:
1. Sync versions across `package.json`, `Cargo.toml`, and `tauri.conf.json`
2. Create a release commit with changelog
3. Push to `release` branch, triggering GitHub Actions
4. Build for macOS (universal), Windows, and Linux
5. Create a draft GitHub release with all artifacts

See [docs/RELEASING.md](docs/RELEASING.md) for detailed instructions including code signing setup.

## Documentation

- [Implementation Plan](PLAN.md) - Complete architecture and design
- [Task Breakdown](TASKS.md) - Detailed implementation tasks

### Active Issues & Analysis

- [Workspace Switching Analysis](docs/workspace-switching-analysis.md) - Critical bugs in workspace switching causing data leakage between databases
- [Workspace Switching Tasks](docs/workspace-switching-tasks.md) - Detailed implementation plan with 12 tasks across 4 phases

### Phase Documentation

#### Phase 1: Backend Infrastructure

- [Phase 1.1: Database Provider Abstraction](docs/phases/PHASE_1.1_DATABASE_PROVIDER_ABSTRACTION.md)
- [Phase 1.2: SQLite Provider](docs/phases/PHASE_1.2_SQLITE_PROVIDER.md)
- [Phase 1.3: Schema Introspection Engine](docs/phases/PHASE_1.3_SCHEMA_INTROSPECTION_ENGINE.md)
- [Phase 1.4: Schema Fingerprinting](docs/phases/PHASE_1.4_SCHEMA_FINGERPRINTING.md)
- [Phase 1.5: Workspace Management](docs/phases/PHASE_1.5_WORKSPACE_MANAGEMENT.md)
- [Phase 1.6: Knowledge Store](docs/phases/PHASE_1.6_KNOWLEDGE_STORE.md)
- [Phase 1.7: Credential Store](docs/phases/PHASE_1.7_CREDENTIAL_STORE.md)
- [Phase 1.8: Database Migrations](docs/phases/PHASE_1.8_DATABASE_MIGRATIONS.md)

#### Phase 2: AI Integration

- [Phase 2.1: LLM Provider Abstraction](docs/phases/PHASE_2.1_LLM_PROVIDER_ABSTRACTION.md)
- [Phase 2.2: Vercel AI Gateway Provider](docs/phases/PHASE_2.2_VERCEL_AI_GATEWAY_PROVIDER.md)
- [Phase 2.3: Prompt Template System](docs/phases/PHASE_2.3_PROMPT_TEMPLATE_SYSTEM.md)
- [Phase 2.4: Schema Interpretation Agent](docs/phases/PHASE_2.4_SCHEMA_INTERPRETATION_AGENT.md)
- [Phase 2.5: Relationship Inference Agent](docs/phases/PHASE_2.5_RELATIONSHIP_INFERENCE_AGENT.md)
- [Phase 2.6: Query Explanation Agent](docs/phases/PHASE_2.6_QUERY_EXPLANATION_AGENT.md)
- [Phase 2.7: Onboarding Documentation Agent](docs/phases/PHASE_2.7_ONBOARDING_AGENT.md)
- [Phase 2.8: Explanation Cache](docs/phases/PHASE_2.8_EXPLANATION_CACHE.md)

#### Phase 3: Frontend UI

- [Phase 3.2: Schema Explorer](docs/phases/PHASE_3.2_SCHEMA_EXPLORER.md)
- [Phase 3.4: Query Understanding](docs/phases/PHASE_3.4_QUERY_UNDERSTANDING.md)
- [Phase 3.5: Onboarding Dashboard](docs/phases/PHASE_3.5_ONBOARDING_DASHBOARD.md)
- [Phase 3.6: Settings](docs/phases/PHASE_3.6_SETTINGS.md)
- [Phase 3.7: State Management](docs/phases/PHASE_3.7_STATE_MANAGEMENT.md)
- [Phase 3.8: Routing](docs/phases/PHASE_3.8_ROUTING.md)

#### Phase 4: IPC Commands

- [Phase 4.1: Database Commands](docs/phases/PHASE_4.1_DATABASE_COMMANDS.md)
- [Phase 4.2: Schema Commands](docs/phases/PHASE_4.2_SCHEMA_COMMANDS.md)
- [Phase 4.3: Explanation Commands](docs/phases/PHASE_4.3_EXPLANATION_COMMANDS.md)
- [Phase 4.4: Onboarding Commands](docs/phases/PHASE_4.4_ONBOARDING_COMMANDS.md)
- [Phase 4.5: Knowledge Commands](docs/phases/PHASE_4.5_KNOWLEDGE_COMMANDS.md)

### Feature Documentation

- [AI Skill System](docs/features/SKILL_SYSTEM.md) - Modular AI capabilities for database analysis
- [Chat Functionality](docs/features/CHAT_FUNCTIONALITY.md) - Database chat interface

### Database Documentation

- [Database Connection Methods](docs/database/DATABASE_CONNECTION_METHODS.md) - Local, Docker, remote, and SSH tunnel connections
- [Database Connection Problems](docs/database/DATABASE_CONNECTION_PROBLEMS.md) - Common issues and solutions
- [Testing Database Connections](docs/database/TESTING_DATABASE_CONNECTIONS.md) - Comprehensive testing procedures
- [Giant Database Handling](docs/database/GIANT_DATABASE_HANDLING.md) - Strategies for large-scale databases
- [MongoDB & Document Databases](docs/database/MONGODB_DOCUMENT_DATABASES.md) - MongoDB integration design and architecture
- [MongoDB Implementation Tasks](docs/plans/TASKS_MONGODB.md) - Detailed task breakdown for MongoDB support
- [PostgreSQL Support](docs/database/PostgreSQL-Support.md) - PostgreSQL implementation details

## Tech Stack

- **Frontend** — React 19, TypeScript, TanStack Router, Tailwind CSS 4, Zustand
- **UI Components** — Base UI, Lucide React, ReactFlow, CodeMirror 6
- **Desktop** — Tauri v2 (macOS, Windows, Linux, iOS, Android)
- **Backend** — Rust with Diesel ORM, async-trait, tokio
- **Database** — SQLite (app metadata + user databases)
- **AI** — Vercel AI Gateway (MVP), local LLM support (future)
- **Build** — Vite, Turborepo, bun

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Frontend (React)                      │
│  UI Components • State Management • Routing             │
└─────────────────────────────────────────────────────────┘
                          ▼ IPC
┌─────────────────────────────────────────────────────────┐
│                  Backend (Rust/Tauri)                   │
├─────────────────────────────────────────────────────────┤
│  Database Provider Abstraction                          │
│  ├─ SQLite Provider ✅                                  │
│  ├─ PostgreSQL Provider ✅                              │
│  ├─ MySQL Provider ✅                                   │
│  └─ MongoDB Provider ✅                                 │
├─────────────────────────────────────────────────────────┤
│  Schema Intelligence                                     │
│  ├─ Introspection Engine ✅                             │
│  ├─ Fingerprinting & Change Detection ✅                │
│  └─ Pattern Recognition ✅                              │
├─────────────────────────────────────────────────────────┤
│  AI Skill System ✅                                      │
│  ├─ Skill Engine (Loader, Selector, Composer)           │
│  ├─ 8 Embedded Skills                                   │
│  ├─ Per-workspace Settings                              │
│  └─ Application Adapter                                 │
├─────────────────────────────────────────────────────────┤
│  AI Integration                                          │
│  ├─ LLM Provider Abstraction ✅                         │
│  ├─ Vercel AI Gateway Provider ✅                       │
│  ├─ Prompt Template System ✅                           │
│  ├─ Schema Interpretation Agent ✅                      │
│  └─ Relationship Inference Agent ✅                     │
├─────────────────────────────────────────────────────────┤
│  Storage                                                 │
│  ├─ Workspace Manager ✅                                │
│  ├─ Knowledge Store ✅                                  │
│  └─ Credential Store ✅                                 │
└─────────────────────────────────────────────────────────┘
```

## License

MIT License - see [LICENSE](LICENSE) for details
