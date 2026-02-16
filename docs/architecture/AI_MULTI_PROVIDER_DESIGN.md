> **STATUS: REVISION PLANNED — Provider consolidation in Phase 0 (Slim Down)**
>
> The multi-provider architecture below is being simplified. Three near-identical provider
> implementations (OpenAI-compatible: 678 lines, OpenRouter: 566 lines, Vercel: 554 lines)
> will be consolidated into a single `GenericProvider` using the `async-openai` crate (~200 lines).
> See `docs/tauriclaw-gap-analysis.md` item S2 for details.
>
> **What changes**:
> - All OpenAI-compatible providers merge into one `GenericProvider` struct
> - Provider differences reduced to `base_url` + optional `api_key` + optional headers
> - `async-openai` handles streaming, retries, and protocol compliance
> - Anthropic remains separate (different API format) but uses a thin adapter
>
> **What stays the same**:
> - The `LLMProvider` trait interface
> - Frontend provider selection UI
> - OS keyring storage for API keys
> - The conceptual architecture (provider-agnostic, trait-based)
>
> **Savings**: ~1,500 lines removed (20% of backend codebase)

# AI Multi-Provider Configuration Design

**Date:** 2025-01-25
**Status:** Design Phase - Ready for Implementation
**Version:** 2.0 - Provider-Agnostic Design

---

## Overview

Enable users to configure multiple AI providers (OpenAI, Anthropic, Gemini, OpenRouter, Vercel AI Gateway, Ollama, and custom providers) with per-provider API key management. Only providers with saved API keys appear in the model selector.

**Key Architectural Principle:** Provider-agnostic design where all providers (gateways, direct APIs, local servers) are treated identically at the protocol level using OpenAI-compatible endpoints.

---

## Table of Contents

1. [Architecture](#architecture)
2. [Provider Abstraction](#provider-abstraction)
3. [Provider Definitions](#provider-definitions)
4. [Data Model](#data-model)
5. [Frontend Components](#frontend-components)
6. [Backend Implementation](#backend-implementation)
7. [API Reference](#api-reference)
8. [Error Handling](#error-handling)
9. [Implementation Steps](#implementation-steps)

---

## Architecture

### Design Principles

1. **Protocol-Agnostic**: All providers use OpenAI-compatible HTTP endpoints
2. **Type-Agnostic**: No distinction between "gateway", "direct", or "local" in code
3. **Authentication-Agnostic**: Some providers require API keys, some don't (like Ollama)
4. **Secure Storage**: API keys stored in OS keychain, never in database
5. **Progressive Disclosure**: Only show providers with saved API keys
6. **YAGNI**: Pre-defined providers + custom option, no over-engineering

### What "Provider-Agnostic" Means

**Traditional Approach (What We're NOT Doing):**

```rust
enum ProviderType {
    Direct,    // Special handling
    Gateway,   // Special handling
    Local,     // Special handling
}

// Different code paths for each type
match provider_type {
    Direct => call_direct_api(),
    Gateway => call_gateway_with_prefix(),
    Local => call_local_no_auth(),
}
```

**Our Approach (Provider-Agnostic):**

```rust
// All providers are the same
struct Provider {
    id: String,
    name: String,
    base_url: String,
    requires_api_key: bool,
    // No "type" field needed!
}

// Single code path for all
fn call_provider(provider: &Provider, model: &str, prompt: &str) {
    let client = Client::new()
        .with_base_url(&provider.base_url)
        .with_api_key_optional(provider.api_key);

    client.chat().create(request).await;
}
```

### Model ID Formats by Provider

| Provider   | Model ID Format      | Example                       |
| ---------- | -------------------- | ----------------------------- |
| OpenAI     | `{model}`            | `gpt-4o`                      |
| Anthropic  | `{model}`            | `claude-sonnet-4.5`           |
| Gemini     | `{model}`            | `gemini-2.5-pro`              |
| OpenRouter | `{provider}/{model}` | `anthropic/claude-sonnet-4.5` |
| Vercel     | `{provider}/{model}` | `openai/gpt-4o`               |
| Ollama     | `{model}`            | `llama3`                      |
| Custom     | (user-defined)       | (varies)                      |

**Key Point:** The model ID format is just a string convention. The protocol doesn't care.

### Provider Characteristics

| Provider   | Base URL                                                  | API Key      | Notes                 |
| ---------- | --------------------------------------------------------- | ------------ | --------------------- |
| OpenAI     | `https://api.openai.com/v1`                               | Required     | Direct provider       |
| Anthropic  | `https://api.anthropic.com/v1`                            | Required     | Direct provider       |
| Gemini     | `https://generativelanguage.googleapis.com/v1beta/openai` | Required     | Direct provider       |
| OpenRouter | `https://openrouter.ai/api/v1`                            | Required     | Gateway               |
| Vercel     | `https://ai-gateway.vercel.sh/v1`                         | Required     | Gateway               |
| Ollama     | `http://localhost:11434/v1`                               | Not required | Local server          |
| Custom     | User-defined                                              | Optional     | User-defined endpoint |

**All providers use the same OpenAI-compatible protocol.** The only differences are:

- Base URL (endpoint)
- API Key (some don't require it)
- Model list (provider-specific)

---

## Provider Abstraction

### LLMProvider Trait

All providers implement the same trait:

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(&self, prompt: &str, model: &str) -> Result<String, String>;
    async fn stream(&self, prompt: &str, model: &str) -> Result<Pin<Box<dyn Stream>>>;
    fn context_limit(&self, model: &str) -> usize;
    fn supports_tools(&self) -> bool;
    fn provider_name(&self) -> &str;
}
```

**Key Point:** This trait doesn't care if the provider is "direct", "gateway", or "local". It only cares about the OpenAI-compatible protocol.

### Provider Implementation Pattern

All providers follow the same structure:

```rust
pub struct AnyProvider {
    api_key: Option<String>,  // None for Ollama
    base_url: String,
}

#[async_trait]
impl LLMProvider for AnyProvider {
    async fn complete(&self, prompt: &str, model: &str) -> Result<String, String> {
        let client = Client::new()
            .with_base_url(&self.base_url);

        let client = if let Some(key) = &self.api_key {
            client.with_api_key(key)
        } else {
            client // No auth for Ollama, etc.
        };

        let request = CreateChatCompletionRequest {
            model: model.to_string(),
            messages: vec![ChatCompletionMessage::user {
                content: prompt.to_string(),
                name: None,
            }],
            ..Default::default()
        };

        let response = client.chat().create(request).await
            .map_err(|e| e.to_string())?;

        Ok(response.choices[0].message.content.clone())
    }

    fn context_limit(&self, model: &str) -> usize {
        // Provider-specific mapping
        match model {
            "gpt-4o" => 128000,
            "claude-opus-4.5" => 200000,
            "gemini-2.5-pro" => 1000000,
            _ => 128000,
        }
    }
}
```

**All providers use this exact same structure.** Only the `context_limit()` mapping differs.

### Ollama: The Exception That Proves the Rule

Ollama is "local" and doesn't require an API key, but at the protocol level:

```rust
// Ollama uses OpenAI-compatible endpoint
pub struct OllamaProvider {
    base_url: String, // http://localhost:11434/v1
    // No api_key field!
}

// Implementation is IDENTICAL to other providers
// The only difference: api_key is None instead of Some(...)
```

**Conclusion:** Even "local" providers are just remote providers with:

- `base_url = "http://localhost:11434/v1"`
- `api_key = None`

---

## Provider Definitions

### File Structure

```
src/
└── lib/
    └── ai-providers.ts          [NEW] - All provider definitions and model lists
```

### Provider Configuration

```typescript
// src/lib/ai-providers.ts

export interface ProviderConfig {
  id: string;
  name: string;
  baseUrl: string;
  requiresApiKey: boolean;
  models: string[];
}

export const STANDARD_PROVIDERS: Record<string, ProviderConfig> = {
  openai: {
    id: "openai",
    name: "OpenAI",
    baseUrl: "https://api.openai.com/v1",
    requiresApiKey: true,
    models: [
      "gpt-5.2",
      "gpt-4.1",
      "gpt-4.1-mini",
      "gpt-4o",
      "gpt-4o-mini",
      "o3",
      "o3-mini",
      "o4-mini",
    ],
  },

  anthropic: {
    id: "anthropic",
    name: "Anthropic",
    baseUrl: "https://api.anthropic.com/v1",
    requiresApiKey: true,
    models: [
      "claude-opus-4.5",
      "claude-sonnet-4.5",
      "claude-haiku-4.5",
      "claude-3.7-sonnet-20250219",
      "claude-3.5-sonnet-20241022",
      "claude-3.5-haiku-20241022",
    ],
  },

  gemini: {
    id: "gemini",
    name: "Google Gemini",
    baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai",
    requiresApiKey: true,
    models: [
      "gemini-2.5-pro",
      "gemini-2.5-pro-exp",
      "gemini-2.5-flash",
      "gemini-2.5-flash-exp",
      "gemini-2.5-flash-lite",
      "gemini-3-flash",
      "gemini-3-flash-exp",
      "gemini-2.0-flash",
      "gemini-2.0-flash-exp",
      "gemini-1.5-pro",
      "gemini-1.5-flash",
    ],
  },

  openrouter: {
    id: "openrouter",
    name: "OpenRouter",
    baseUrl: "https://openrouter.ai/api/v1",
    requiresApiKey: true,
    models: [
      "anthropic/claude-opus-4.5",
      "anthropic/claude-sonnet-4.5",
      "anthropic/claude-haiku-4.5",
      "anthropic/claude-3.7-sonnet-20250219",
      "anthropic/claude-3.5-sonnet",
      "openai/gpt-5.2",
      "openai/gpt-4.1",
      "openai/gpt-4.1-mini",
      "openai/gpt-4o",
      "openai/o3-mini",
      "google/gemini-2.5-pro",
      "google/gemini-2.5-flash",
      "google/gemini-3-flash",
      "google/gemini-2.0-flash",
      "deepseek/deepseek-v3",
      "deepseek/deepseek-r1",
      "meta-llama/llama-4-maverick",
      "qwen/qwen-3-coder",
    ],
  },

  vercel_gateway: {
    id: "vercel_gateway",
    name: "Vercel AI Gateway",
    baseUrl: "https://ai-gateway.vercel.sh/v1",
    requiresApiKey: true,
    models: [
      "openai/gpt-5.2",
      "openai/gpt-4.1",
      "anthropic/claude-sonnet-4.5",
      "anthropic/claude-opus-4.5",
      "google/gemini-2.5-flash",
      "google/gemini-2.0-flash",
      "google/gemini-3-flash",
    ],
  },

  ollama: {
    id: "ollama",
    name: "Ollama (Local)",
    baseUrl: "http://localhost:11434/v1",
    requiresApiKey: false,
    models: [], // Discovered via Ollama's GET /api/tags
  },
};

// Helper to get models for a provider
export function getModelsForProvider(providerId: string): string[] {
  return STANDARD_PROVIDERS[providerId]?.models || [];
}

// Helper to check if provider is local (no API key)
export function isLocalProvider(providerId: string): boolean {
  return !STANDARD_PROVIDERS[providerId]?.requiresApiKey ?? false;
}
```

---

## Data Model

### Database Schema

**Important:** No schema changes needed! The existing `ai_providers` table is already sufficient.

```sql
-- Existing schema (NO CHANGES NEEDED)
CREATE TABLE ai_providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    requires_api_key INTEGER NOT NULL DEFAULT 1,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- No provider_type column needed!
-- All providers are treated identically.
```

**Key Point:** We don't need a `provider_type` column because all providers use the same OpenAI-compatible protocol. The "type" is just documentation metadata, not a functional requirement.

### Keychain Storage Format

```
Service: com.aiboilerplate.credentials
Key format: api_key:{providerId}

Examples:
  api_key:openai         → sk-...
  api_key:anthropic      → sk-ant-...
  api_key:openrouter     → sk-or-...
  api_key:vercel_gateway → eyJ...
  api_key:ollama         → (not stored, Ollama doesn't require API key)
  api_key:custom_xyz     → (user's custom key)
```

### Type Definitions

```typescript
// src/lib/models.ts

export interface AIProvider {
  id: string;
  name: string;
  baseUrl: string;
  requiresApiKey: boolean;
  isActive: boolean;
  // No providerType field - not needed!
}

export interface ProviderWithKeyStatus extends AIProvider {
  hasApiKey: boolean;
  models: AIModel[];
}

export interface AIModel {
  id: string;
  providerId: string;
  modelId: string;
  displayName: string;
  contextLimit?: number;
  isCustom: boolean;
  isActive: boolean;
}

export interface TestResult {
  success: boolean;
  latency?: number;
  error?: string;
  model?: string;
}
```

---

## Frontend Components

### Component Structure

```
src/
├── lib/
│   └── ai-providers.ts                 [NEW] - Provider definitions & model lists
│
├── components/
│   ├── settings/
│   │   ├── AISettingsTab.tsx           [NEW] - Main AI Providers settings tab
│   │   ├── ProviderList.tsx            [NEW] - List of configured providers
│   │   ├── ProviderCard.tsx            [NEW] - Single provider card with actions
│   │   ├── AddProviderDialog.tsx       [NEW] - Add provider/API key dialog
│   │   └── ConfigureProviderBanner.tsx [NEW] - Inline banner (no keys warning)
│   │
│   └── ai-elements/
│       └── provider-model-selector.tsx [MODIFY] - Filter by API keys
│
├── routes/
│   └── settings.tsx                    [MODIFY] - Add AI Providers tab
│
└── stores/
    └── llm.ts                          [MODIFY] - Multi-provider key management
```

### AISettingsTab Component

```typescript
// src/components/settings/AISettingsTab.tsx

interface AISettingsTabProps {
  className?: string;
}

export function AISettingsTab({ className }: AISettingsTabProps) {
  const { providers, saveApiKey, removeApiKey, testConnection } = useLLMStore();

  return (
    <div className="space-y-6">
      {/* Header with add button */}
      <div className="flex justify-between items-center">
        <h2>AI Providers</h2>
        <AddProviderButton />
      </div>

      {/* Provider list */}
      <ProviderList
        providers={providers.filter(p => p.hasApiKey || p.type === 'local')}
        onSaveKey={saveApiKey}
        onRemoveKey={removeApiKey}
        onTest={testConnection}
      />

      {/* Ollama section (always shown, no key required) */}
      <OllamaSetupCard />
    </div>
  );
}
```

### ProviderCard Component

```typescript
// src/components/settings/ProviderCard.tsx

interface ProviderCardProps {
  provider: ProviderWithKeyStatus;
  onSaveKey: (providerId: string, key: string) => Promise<void>;
  onRemoveKey: (providerId: string) => Promise<void>;
  onTest: (providerId: string) => Promise<TestResult>;
}

export function ProviderCard({ provider, onSaveKey, onRemoveKey, onTest }: ProviderCardProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [apiKey, setApiKey] = useState('');
  const [showKey, setShowKey] = useState(false);
  const [testResult, setTestResult] = useState<TestResult | null>(null);

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-3">
          <ProviderIcon providerId={provider.id} />
          <div>
            <CardTitle>{provider.name}</CardTitle>
            <CardDescription>{provider.baseUrl}</CardDescription>
          </div>
          <Badge variant={provider.hasApiKey ? 'success' : 'secondary'}>
            {provider.hasApiKey ? 'Configured' : 'No API Key'}
          </Badge>
        </div>
      </CardHeader>

      <CardContent>
        {isEditing ? (
          <ApiKeyInput
            value={apiKey}
            onChange={setApiKey}
            show={showKey}
            onToggleShow={() => setShowKey(!showKey)}
            onSave={() => onSaveKey(provider.id, apiKey)}
            onCancel={() => setIsEditing(false)}
          />
        ) : (
          <div className="flex gap-2">
            <Button onClick={() => setIsEditing(true)}>
              {provider.hasApiKey ? 'Update Key' : 'Add Key'}
            </Button>
            {provider.hasApiKey && (
              <>
                <Button variant="outline" onClick={() => onTest(provider.id)}>
                  Test Connection
                </Button>
                <Button variant="ghost" onClick={() => onRemoveKey(provider.id)}>
                  Remove
                </Button>
              </>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
```

### ConfigureProviderBanner Component

```typescript
// src/components/settings/ConfigureProviderBanner.tsx

interface ConfigureProviderBannerProps {
  onDismiss: () => void;
  onConfigure: () => void;
}

export function ConfigureProviderBanner({ onDismiss, onConfigure }: ConfigureProviderBannerProps) {
  return (
    <Alert variant="warning" className="relative">
      <AlertCircle className="h-4 w-4" />
      <AlertTitle>No AI Provider Configured</AlertTitle>
      <AlertDescription>
        Add an API key to use AI features like schema interpretation and query explanation.
      </AlertDescription>
      <div className="flex gap-2 mt-3">
        <Button size="sm" onClick={onConfigure}>
          Configure AI Provider
        </Button>
        <Button size="sm" variant="ghost" onClick={onDismiss}>
          Dismiss
        </Button>
      </div>
    </Alert>
  );
}
```

### Updated LLM Store

```typescript
// src/stores/llm.ts

interface LLMState {
  // Existing
  currentProvider: string | null;
  currentModel: string | null;
  providers: ProviderWithModels[];
  apiKeys: Record<string, string>;

  // New
  hasAnyApiKey: boolean;
  configuredProviders: ProviderWithKeyStatus[];

  // Actions
  loadApiKeys: () => Promise<void>;
  saveApiKey: (providerId: string, key: string) => Promise<void>;
  removeApiKey: (providerId: string) => Promise<void>;
  checkApiKeyStatus: () => Promise<Record<string, boolean>>;
  testProviderConnection: (
    providerId: string,
    modelId: string
  ) => Promise<TestResult>;
}
```

---

## Backend Implementation

### Database Migration

```sql
-- migrations/2025-01-26-000000_add_ai_providers_extended/up.sql

-- NO SCHEMA CHANGES NEEDED!
-- The existing ai_providers table is already sufficient.

-- Update existing providers (if needed)
UPDATE ai_providers SET
  name = 'OpenRouter',
  base_url = 'https://openrouter.ai/api/v1'
WHERE id = 'openrouter';

UPDATE ai_providers SET
  name = 'Vercel AI Gateway',
  base_url = 'https://ai-gateway.vercel.sh/v1'
WHERE id = 'vercel_gateway';

-- Insert new standard providers
INSERT INTO ai_providers (id, name, base_url, requires_api_key, is_active, created_at)
VALUES
  ('openai', 'OpenAI', 'https://api.openai.com/v1', 1, 1, datetime('now')),
  ('anthropic', 'Anthropic', 'https://api.anthropic.com/v1', 1, 1, datetime('now')),
  ('gemini', 'Google Gemini', 'https://generativelanguage.googleapis.com/v1beta/openai', 1, 1, datetime('now')),
  ('ollama', 'Ollama (Local)', 'http://localhost:11434/v1', 0, 1, datetime('now'))
ON CONFLICT(id) DO UPDATE SET
  name = excluded.name,
  base_url = excluded.base_url,
  requires_api_key = excluded.requires_api_key;

-- Insert models (see ai-providers.ts for full list)
-- ... model INSERT statements ...
```

### New Tauri Commands

```rust
// src-tauri/src/commands/provider.rs

use tauri::State;
use crate::keyring::Keyring;
use crate::db::DbPool;

#[tauri::command]
pub async fn get_providers_with_key_status_command(
    state: State<'_, DbPool>,
    keyring: State<'_, Keyring>,
) -> Result<Vec<ProviderWithKeyStatus>, String> {
    // Fetch all providers from database
    // Check keyring for API key existence
    // Return list with hasApiKey boolean
}

#[tauri::command]
pub async fn add_custom_provider_command(
    name: String,
    base_url: String,
    requires_api_key: bool,
    state: State<'_, DbPool>,
) -> Result<String, String> {
    // Generate unique ID (custom_<timestamp>)
    // Save to ai_providers table
    // Return provider ID
}

#[tauri::command]
pub async fn delete_custom_provider_command(
    provider_id: String,
    state: State<'_, DbPool>,
    keyring: State<'_, Keyring>,
) -> Result<(), String> {
    // Only allow deleting custom providers
    // Delete from database
    // Remove API key from keychain
}

#[tauri::command]
pub async fn test_provider_connection_command(
    provider_id: String,
    model_id: String,
    state: State<'_, ConnectionManager>,
) -> Result<TestResult, String> {
    // Make minimal test call to provider
    // Return success/failure with latency
}

#[tauri::command]
pub async fn list_ollama_models_command(
) -> Result<Vec<String>, String> {
    // Call Ollama GET /api/tags
    // Return list of model names
}
```

### New Provider Implementations

```rust
// src-tauri/src/ai/providers/openai.rs

use async_openai::{
    types::{CreateChatCompletionRequest, ChatCompletionMessage},
    Client,
};

pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn complete(&self, prompt: &str, model: &str) -> Result<String, String> {
        let client = Client::new()
            .with_api_key(&self.api_key)
            .with_base_url(&self.base_url);

        let request = CreateChatCompletionRequest {
            model: model.to_string(),
            messages: vec![ChatCompletionMessage::user {
                content: prompt.to_string(),
                name: None,
            }],
            ..Default::default()
        };

        let response = client.chat().create(request).await
            .map_err(|e| e.to_string())?;

        Ok(response.choices[0].message.content.clone())
    }

    fn context_limit(&self, model: &str) -> usize {
        match model {
            "gpt-5.2" => 200000,
            "gpt-4.1" => 128000,
            "gpt-4.1-mini" => 128000,
            "gpt-4o" => 128000,
            "gpt-4o-mini" => 128000,
            "o3" => 200000,
            "o3-mini" => 200000,
            _ => 128000,
        }
    }
}
```

---

## API Reference

### Frontend Tauri Invokes

| Command                                 | Purpose                        | Returns                   |
| --------------------------------------- | ------------------------------ | ------------------------- |
| `keychain_set`                          | Store API key                  | `void`                    |
| `keychain_get`                          | Retrieve API key               | `string`                  |
| `keychain_delete`                       | Delete API key                 | `void`                    |
| `keychain_exists`                       | Check if key exists            | `boolean`                 |
| `get_providers_with_key_status_command` | List providers with key status | `ProviderWithKeyStatus[]` |
| `add_custom_provider_command`           | Add custom provider            | `string` (provider ID)    |
| `delete_custom_provider_command`        | Delete custom provider         | `void`                    |
| `test_provider_connection_command`      | Test provider connectivity     | `TestResult`              |
| `list_ollama_models_command`            | Get Ollama model list          | `string[]`                |

### Existing Commands (Reused)

| Command                             | Purpose                       |
| ----------------------------------- | ----------------------------- |
| `configure_llm_provider_command`    | Save selected provider/model  |
| `get_llm_provider_config_command`   | Get current selection         |
| `list_providers_and_models_command` | List all providers and models |
| `add_custom_model_command`          | Add custom model to provider  |
| `delete_model_command`              | Delete custom model           |

---

## Error Handling

### Scenarios

| Scenario                    | Detection               | Handling                      |
| --------------------------- | ----------------------- | ----------------------------- |
| No API keys configured      | Check on AI feature use | Show banner, disable features |
| Invalid API key             | Test connection fails   | Show error, allow re-entry    |
| Provider unreachable        | HTTP error / timeout    | Show error message            |
| Ollama not running          | Connection refused      | Show "Start Ollama" message   |
| Custom provider invalid URL | URL validation on save  | Validate format before saving |
| Model deprecated            | API returns 404         | Remove from list, notify user |
| Rate limit exceeded         | 429 response            | Show retry message            |
| Insufficient quota          | 401/403 with quota info | Show quota error              |

### Test Result Type

```typescript
interface TestResult {
  success: boolean;
  latency?: number;
  error?: string;
  model?: string;
}
```

---

## Implementation Steps

### Phase 1: Foundation

1. Create `src/lib/ai-providers.ts` with all provider definitions
2. Write database migration for new providers
3. Update type definitions in `src/lib/models.ts`

### Phase 2: Backend

4. Implement `get_providers_with_key_status_command`
5. Implement `test_provider_connection_command`
6. Implement `list_ollama_models_command`
7. Implement `add_custom_provider_command`
8. Implement `delete_custom_provider_command`

### Phase 3: Provider Implementations

9. Create `openai.rs` provider
10. Create `anthropic.rs` provider
11. Create `gemini.rs` provider
12. Create `ollama.rs` provider with model discovery

### Phase 4: Frontend Store

13. Update `llm.ts` store with multi-provider support
14. Add `apiKeys` state management
15. Add `hasAnyApiKey` derived state
16. Add `checkApiKeyStatus` action

### Phase 5: UI Components

17. Create `AISettingsTab.tsx`
18. Create `ProviderCard.tsx`
19. Create `AddProviderDialog.tsx`
20. Create `ConfigureProviderBanner.tsx`

### Phase 6: Integration

21. Update `settings.tsx` to include AI Providers tab
22. Modify `provider-model-selector.tsx` to filter by API keys
23. Add banner display logic in workspace routes

### Phase 7: Testing

24. Test provider connection for each type
25. Test API key save/load/delete operations
26. Test custom provider add/delete
27. Test Ollama model discovery
28. Test model selector filtering
29. Test banner display/dismiss behavior

---

## References

- [OpenAI Models](https://platform.openai.com/docs/models)
- [Claude Models](https://platform.claude.com/docs/en/about-claude/models/overview)
- [Gemini Models](https://ai.google.dev/gemini-api/docs/models)
- [OpenRouter Models](https://openrouter.ai/models)
- [Vercel AI Gateway](https://vercel.com/docs/ai-gateway)
- [Ollama API](https://docs.ollama.com/api/introduction)
