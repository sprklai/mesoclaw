# Secure Storage Quick Start

## Installation Complete ✅

Tauri Stronghold plugin has been successfully integrated into your project for secure storage of API keys and other sensitive information.

## Quick Usage

### 1. Store an API Key

```typescript
import { apiKeyHelpers } from "@/lib/secure-storage";

// Store OpenAI API key
await apiKeyHelpers.setOpenAIKey("sk-...");

// Store Anthropic API key
await apiKeyHelpers.setAnthropicKey("sk-ant-...");

// Store Google Gemini API key
await apiKeyHelpers.setGeminiKey("AIza...");
```

### 2. Retrieve an API Key

```typescript
// Get OpenAI key
const openaiKey = await apiKeyHelpers.getOpenAIKey();
if (openaiKey) {
  // Use the key
}

// Get all configured providers
const providers = await apiKeyHelpers.getAllProviders();
console.log(providers); // ["openai", "anthropic", "gemini"]
```

### 3. Delete an API Key

```typescript
await apiKeyHelpers.deleteProvider("openai");
```

### 4. Generic Secret Storage

```typescript
import { SecureStorage } from "@/lib/secure-storage";
import { SecretCategory } from "@/types/secure-storage";

// Store any secret
await SecureStorage.setSecret(
  "my-key",
  "my-secret-value",
  SecretCategory.DATABASE_PASSWORD,
  { host: "localhost", port: "5432" }
);

// Retrieve secret
const secret = await SecureStorage.getSecret(
  "my-key",
  SecretCategory.DATABASE_PASSWORD
);
console.log(secret.value); // "my-secret-value"
console.log(secret.metadata); // { host: "localhost", port: "5432" }

// Delete secret
await SecureStorage.deleteSecret("my-key", SecretCategory.DATABASE_PASSWORD);
```

## Available Categories

- `SecretCategory.API_KEY` - AI provider API keys
- `SecretCategory.DATABASE_PASSWORD` - Database credentials
- `SecretCategory.ENCRYPTION_KEY` - Encryption keys
- `SecretCategory.TOKEN` - OAuth/auth tokens
- `SecretCategory.CERTIFICATE` - SSL certificates
- `SecretCategory.CUSTOM` - Custom secrets

## Security Features

✅ **Encrypted at rest** - All secrets encrypted using Stronghold  
✅ **Argon2 key derivation** - Industry-standard password hashing  
✅ **No plaintext storage** - Secrets never stored in plaintext  
✅ **Secure vault** - Encrypted vault file in app data directory  
✅ **Type-safe API** - Full TypeScript support

## Files Created

### Backend

- `src-tauri/Cargo.toml` - Added `tauri-plugin-stronghold` and `rust-argon2`
- `src-tauri/src/lib.rs` - Initialized Stronghold plugin

### Frontend

- `src/types/secure-storage.ts` - TypeScript type definitions
- `src/lib/secure-storage.ts` - Secure storage API wrapper
- `package.json` - Added `@tauri-apps/plugin-stronghold`

### Documentation

- `docs/SECURE_STORAGE.md` - Complete documentation
- `docs/SECURE_STORAGE_QUICKSTART.md` - This quick start guide

## Next Steps

1. **Use in your LLM configuration**: Replace any plaintext API key storage with secure storage
2. **Migrate existing keys**: Move any existing API keys to Stronghold
3. **Add UI**: Create a settings panel for users to manage their API keys

## Example: LLM Provider Configuration

```typescript
import { apiKeyHelpers } from "@/lib/secure-storage";

async function configureLLMProvider(provider: string, apiKey: string) {
  switch (provider) {
    case "openai":
      await apiKeyHelpers.setOpenAIKey(apiKey);
      break;
    case "anthropic":
      await apiKeyHelpers.setAnthropicKey(apiKey);
      break;
    case "gemini":
      await apiKeyHelpers.setGeminiKey(apiKey);
      break;
  }
}

async function getLLMApiKey(provider: string): Promise<string | null> {
  switch (provider) {
    case "openai":
      return await apiKeyHelpers.getOpenAIKey();
    case "anthropic":
      return await apiKeyHelpers.getAnthropicKey();
    case "gemini":
      return await apiKeyHelpers.getGeminiKey();
    default:
      return null;
  }
}
```

## Vault Location

The encrypted vault is stored at:

- **Linux**: `~/.local/share/com.aiboilerplate.credentials/secrets.hold`
- **macOS**: `~/Library/Application Support/com.aiboilerplate.credentials/secrets.hold`
- **Windows**: `%APPDATA%\com.aiboilerplate.credentials\secrets.hold`

For full documentation, see [SECURE_STORAGE.md](./SECURE_STORAGE.md)
