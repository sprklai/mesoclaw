# Splash Screen Delay Fix

## Issues Fixed

### 1. **Double Initialization (React StrictMode)**

**Problem**: React StrictMode in development mode intentionally runs effects twice to help catch bugs. This caused:

- StoreInitializer to run twice
- Stronghold eager init to attempt twice
- Splash screen close command called twice
- Confusing duplicate logs

**Fix**: Added `useRef` guard in StoreInitializer and module-level guard in secure-storage.ts

```typescript
// StoreInitializer
const hasInitialized = useRef(false);
if (hasInitialized.current) {
  console.log("[StoreInitializer] Skipping duplicate initialization");
  return;
}
hasInitialized.current = true;

// secure-storage.ts
let eagerInitStarted = false;
if (typeof window !== "undefined" && !eagerInitStarted) {
  eagerInitStarted = true;
  // ... initialize
}
```

### 2. **No Performance Visibility**

**Problem**: Couldn't see which step was causing delays

**Fix**: Added comprehensive timing measurements to all initialization steps:

- Stronghold eager init duration
- Settings init duration
- Theme init duration
- LLM store init duration (including model fetch and API key retrieval)
- Splash screen close duration
- Total initialization time

## Expected Console Output (After Fix)

### First Run (No API Key):

```
[SecureStorage] Module loaded. window type: object
=== [SecureStorage] STARTING EAGER INITIALIZATION ===
[SecureStorage] Initializing Stronghold at: /home/.../secrets.hold
=== [StoreInitializer] Starting initialization ===
[StoreInitializer] Initializing settings...
[StoreInitializer] Settings initialized ✓ (10.50ms)
[StoreInitializer] Initializing theme...
[StoreInitializer] Theme initialized ✓ (0.80ms)
[StoreInitializer] Initializing LLM store...
[LLMStore] Starting initialization...
[LLMStore] Model from backend: google/gemini-3-flash (5.20ms)
[LLMStore] Attempting to retrieve API key from Stronghold...
[SecureStorage] Getting secret: api_key:vercel-ai-gateway
[SecureStorage] Stronghold loaded successfully
[SecureStorage] Client created successfully
=== [SecureStorage] EAGER INITIALIZATION COMPLETED ✓ (123.45ms) ===
[SecureStorage] Failed to get secret api_key:vercel-ai-gateway: Error: Key not found
[LLMStore] API key not found or retrieval failed: Error: Key not found
[LLMStore] Initialization complete (Total: 135.20ms)
[StoreInitializer] LLM store initialized ✓ (135.30ms)
[StoreInitializer] All stores initialized successfully ✓ (Total: 146.60ms)
[StoreInitializer] Splash screen closed (2.10ms)
```

### With Saved API Key:

```
[SecureStorage] Module loaded. window type: object
=== [SecureStorage] STARTING EAGER INITIALIZATION ===
[SecureStorage] Initializing Stronghold at: /home/.../secrets.hold
=== [StoreInitializer] Starting initialization ===
[StoreInitializer] Initializing settings...
[StoreInitializer] Settings initialized ✓ (8.30ms)
[StoreInitializer] Initializing theme...
[StoreInitializer] Theme initialized ✓ (0.60ms)
[StoreInitializer] Initializing LLM store...
[LLMStore] Starting initialization...
[LLMStore] Model from backend: google/gemini-3-flash (4.80ms)
[LLMStore] Attempting to retrieve API key from Stronghold...
[SecureStorage] Getting secret: api_key:vercel-ai-gateway
[SecureStorage] Stronghold loaded successfully
[SecureStorage] Client loaded successfully
=== [SecureStorage] EAGER INITIALIZATION COMPLETED ✓ (98.20ms) ===
[SecureStorage] Secret retrieved successfully: api_key:vercel-ai-gateway
[LLMStore] API key retrieved successfully, length: 60 (105.40ms)
[LLMStore] Initialization complete (Total: 110.60ms)
[StoreInitializer] LLM store initialized ✓ (110.70ms)
[StoreInitializer] All stores initialized successfully ✓ (Total: 119.70ms)
[StoreInitializer] Splash screen closed (1.90ms)
```

## What the Timings Tell You

### Normal Timings:

- **Settings init**: 5-15ms
- **Theme init**: <1ms
- **Model fetch**: 3-10ms (SQLite query)
- **Stronghold init**: 80-150ms (first run with new client)
- **Stronghold init**: 50-100ms (subsequent runs)
- **API key retrieval**: <5ms (after Stronghold ready)
- **Splash screen close**: 1-5ms
- **Total**: 100-200ms typical

### If You See Long Delays:

**If Stronghold init > 500ms:**

- Disk I/O issue
- Antivirus scanning the vault file
- First run with vault creation is slower

**If Model fetch > 50ms:**

- Database connection issue
- Check SQLite performance

**If Splash screen close > 100ms:**

- Window management issue
- Check backend `close_splashscreen` command

## Files Changed

1. **src/components/store-initializer.tsx**:
   - ✅ Added `useRef` to prevent double initialization
   - ✅ Added timing measurements for all steps

2. **src/lib/secure-storage.ts**:
   - ✅ Added guard to prevent duplicate eager init
   - ✅ Added duration logging

3. **src/stores/llm.ts**:
   - ✅ Added timing for model fetch and API key retrieval

## Benefits

1. ✅ **No more duplicate initialization** - runs once even in StrictMode
2. ✅ **Clear performance visibility** - can see exactly where time is spent
3. ✅ **Faster perceived load time** - no redundant work
4. ✅ **Better debugging** - timing measurements show bottlenecks

## Testing

Run the app and check console logs:

```bash
pnpm tauri dev
```

**Look for:**

1. Only ONE "Starting initialization" message (not two)
2. Timing measurements in milliseconds for each step
3. Total initialization < 200ms typically
4. Splash screen closes quickly after initialization

**If splash screen still delays:**

- Share the timing measurements from console
- Check which step has the highest duration
- That's the bottleneck to investigate
