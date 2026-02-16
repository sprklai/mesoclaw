# Secure Storage with Tauri Stronghold

This document describes the secure storage implementation using Tauri's Stronghold plugin for storing sensitive information like API keys, tokens, and other credentials.

## Overview

The secure storage system provides encrypted storage for sensitive data using Tauri's Stronghold plugin. All data is encrypted at rest using industry-standard encryption (Argon2 for key derivation).

## Features

- **Generic CRUD Operations**: Add, retrieve, update, and delete secrets
- **Category-based Organization**: Organize secrets by category (API keys, tokens, passwords, etc.)
- **Metadata Support**: Attach custom metadata to secrets
- **Type-safe TypeScript API**: Fully typed frontend interface
- **Specialized API Key Helpers**: Convenient methods for common AI providers

## Architecture

### Backend Setup (`src-tauri/src/lib.rs`)

The Stronghold plugin is initialized in the Tauri builder with Argon2 password hashing:

```rust
.plugin(
    tauri_plugin_stronghold::Builder::new(|password| {
        use argon2::{hash_raw, Config, Variant, Version};
        let config = Config {
            lanes: 4,
            mem_cost: 10_000,
            time_cost: 10,
            variant: Variant::Argon2id,
            version: Version::Version13,
            ..Default::default()
        };
        let salt = b"aiboilerplate-salt";
        let key = hash_raw(password.as_ref(), salt, &config)
            .expect("failed to hash password");
        key.to_vec()
    })
    .build(),
)
```

### Frontend Layer (`src/lib/secure-storage.ts`)

The secure storage is accessed entirely from JavaScript using the Stronghold plugin's JavaScript API. The implementation provides:

- **Automatic initialization** - Stronghold vault is created on first use
- **Category-based organization** - Secrets are organized by category using key prefixes
- **Type-safe API** - Full TypeScript support with proper types
- **Singleton pattern** - Single Stronghold instance shared across the application

### Predefined Categories

```typescript
export const SecretCategory = {
  API_KEY: "api_key",
  DATABASE_PASSWORD: "database_password",
  ENCRYPTION_KEY: "encryption_key",
  TOKEN: "token",
  CERTIFICATE: "certificate",
  CUSTOM: "custom",
} as const;
```

## Frontend Usage

### TypeScript API (`src/lib/secure-storage.ts`)

#### Generic Secret Operations

```typescript
import { SecureStorage } from "@/lib/secure-storage";
import { SecretCategory } from "@/types/secure-storage";

// Store a secret
await SecureStorage.setSecret(
  "my-key",
  "my-secret-value",
  SecretCategory.API_KEY,
  { provider: "custom-provider" }
);

// Retrieve a secret
const secret = await SecureStorage.getSecret("my-key", SecretCategory.API_KEY);
console.log(secret.value); // "my-secret-value"
console.log(secret.metadata); // { provider: "custom-provider" }

// Update a secret
await SecureStorage.updateSecret(
  "my-key",
  "new-secret-value",
  SecretCategory.API_KEY
);

// Delete a secret
await SecureStorage.deleteSecret("my-key", SecretCategory.API_KEY);

// List all keys in a category
const keys = await SecureStorage.listKeys(SecretCategory.API_KEY);

// Check if a secret exists
const exists = await SecureStorage.hasSecret("my-key", SecretCategory.API_KEY);
```

#### API Key Helpers

Convenient helpers for common AI providers:

```typescript
import { apiKeyHelpers } from "@/lib/secure-storage";

// OpenAI
await apiKeyHelpers.setOpenAIKey("sk-...");
const openaiKey = await apiKeyHelpers.getOpenAIKey();

// Anthropic
await apiKeyHelpers.setAnthropicKey("sk-ant-...");
const anthropicKey = await apiKeyHelpers.getAnthropicKey();

// Google Gemini
await apiKeyHelpers.setGeminiKey("AIza...");
const geminiKey = await apiKeyHelpers.getGeminiKey();

// Groq
await apiKeyHelpers.setGroqKey("gsk_...");
const groqKey = await apiKeyHelpers.getGroqKey();

// Ollama
await apiKeyHelpers.setOllamaKey("ollama-key");
const ollamaKey = await apiKeyHelpers.getOllamaKey();

// Delete a provider's key
await apiKeyHelpers.deleteProvider("openai");

// Check if a provider has a key
const hasKey = await apiKeyHelpers.hasProvider("openai");

// Get all configured providers
const providers = await apiKeyHelpers.getAllProviders();
```

## Use Cases

### 1. AI Provider API Keys

Store API keys for various AI providers securely:

```typescript
// Store API keys for different providers
await apiKeyHelpers.setOpenAIKey(userInputKey);
await apiKeyHelpers.setAnthropicKey(userInputKey);
await apiKeyHelpers.setGeminiKey(userInputKey);

// Retrieve when making API calls
const apiKey = await apiKeyHelpers.getOpenAIKey();
if (apiKey) {
  // Use the API key for OpenAI requests
}
```

### 2. Database Credentials

Store database passwords securely:

```typescript
await SecureStorage.setSecret(
  "production-db",
  "super-secret-password",
  SecretCategory.DATABASE_PASSWORD,
  {
    host: "db.example.com",
    database: "myapp",
  }
);

// Retrieve when connecting
const dbSecret = await SecureStorage.getSecret(
  "production-db",
  SecretCategory.DATABASE_PASSWORD
);
const password = dbSecret.value;
const host = dbSecret.metadata?.host;
```

### 3. OAuth Tokens

Store authentication tokens:

```typescript
await SecureStorage.setSecret(
  "github-oauth",
  "ghp_xxxxxxxxxxxx",
  SecretCategory.TOKEN,
  {
    scope: "repo,user",
    expiresAt: "2026-12-31",
  }
);
```

### 4. Custom Secrets

Store any custom sensitive data:

```typescript
await SecureStorage.setSecret(
  "encryption-master-key",
  "base64-encoded-key",
  SecretCategory.ENCRYPTION_KEY,
  {
    algorithm: "AES-256-GCM",
    createdAt: new Date().toISOString(),
  }
);
```

## Security Considerations

1. **Encryption**: All secrets are encrypted using Stronghold with Argon2 key derivation
2. **Memory Safety**: Sensitive data is zeroized when dropped (Rust side)
3. **No Plaintext Storage**: Secrets are never stored in plaintext
4. **Secure Vault**: Stronghold creates an encrypted vault file in the app's data directory
5. **Password Protection**: The vault is protected by a password hash

## Best Practices

1. **Never log secrets**: Don't console.log or debug print secret values
2. **Use categories**: Organize secrets by category for better management
3. **Add metadata**: Include useful metadata (provider, expiry, etc.) for context
4. **Handle errors**: Always wrap secret operations in try-catch blocks
5. **Check existence**: Use `hasSecret()` before attempting to retrieve
6. **Clean up**: Delete secrets when they're no longer needed

## Example: API Key Management Component

```typescript
import { useState, useEffect } from "react";
import { apiKeyHelpers } from "@/lib/secure-storage";

export function ApiKeyManager() {
  const [providers, setProviders] = useState<string[]>([]);
  const [newKey, setNewKey] = useState("");
  const [selectedProvider, setSelectedProvider] = useState("openai");

  useEffect(() => {
    loadProviders();
  }, []);

  async function loadProviders() {
    const allProviders = await apiKeyHelpers.getAllProviders();
    setProviders(allProviders);
  }

  async function saveApiKey() {
    try {
      switch (selectedProvider) {
        case "openai":
          await apiKeyHelpers.setOpenAIKey(newKey);
          break;
        case "anthropic":
          await apiKeyHelpers.setAnthropicKey(newKey);
          break;
        case "gemini":
          await apiKeyHelpers.setGeminiKey(newKey);
          break;
      }
      setNewKey("");
      await loadProviders();
    } catch (error) {
      console.error("Failed to save API key:", error);
    }
  }

  async function deleteProvider(provider: string) {
    try {
      await apiKeyHelpers.deleteProvider(provider);
      await loadProviders();
    } catch (error) {
      console.error("Failed to delete provider:", error);
    }
  }

  return (
    <div>
      <h2>API Key Management</h2>

      <div>
        <select value={selectedProvider} onChange={(e) => setSelectedProvider(e.target.value)}>
          <option value="openai">OpenAI</option>
          <option value="anthropic">Anthropic</option>
          <option value="gemini">Google Gemini</option>
          <option value="groq">Groq</option>
        </select>

        <input
          type="password"
          value={newKey}
          onChange={(e) => setNewKey(e.target.value)}
          placeholder="Enter API key"
        />

        <button onClick={saveApiKey}>Save Key</button>
      </div>

      <div>
        <h3>Configured Providers</h3>
        <ul>
          {providers.map((provider) => (
            <li key={provider}>
              {provider}
              <button onClick={() => deleteProvider(provider)}>Delete</button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
```

## Troubleshooting

### Stronghold file location

The Stronghold vault (`secrets.hold`) is stored in the app's data directory:

- **Linux**: `~/.local/share/com.aiboilerplate.credentials/secrets.hold`
- **macOS**: `~/Library/Application Support/com.aiboilerplate.credentials/secrets.hold`
- **Windows**: `C:\Users\<username>\AppData\Roaming\com.aiboilerplate.credentials\secrets.hold`

### Common Errors

- **"Key not found"**: The secret doesn't exist, use `hasSecret()` to check first
- **"Stronghold error"**: Check file permissions and disk space
- **"Serialization error"**: Invalid data format, ensure proper JSON structure

## Migration from Keyring

If you were previously using the `keyring` crate, you can migrate to Stronghold:

```typescript
// Old keyring approach (if you had one)
// const password = await getPassword("service", "username");

// New Stronghold approach
const secret = await SecureStorage.getSecret(
  "username",
  SecretCategory.DATABASE_PASSWORD
);
const password = secret.value;
```
