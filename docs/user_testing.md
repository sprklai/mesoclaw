# MesoClaw User Testing Guide

A comprehensive, step-by-step manual testing guide for the MesoClaw desktop application. This document covers every user-facing feature with detailed test cases that can be executed by QA testers with no prior familiarity with the application.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Test Conventions](#test-conventions)
- [1. Onboarding Flow (P0)](#1-onboarding-flow-p0)
- [2. Home Dashboard (P1)](#2-home-dashboard-p1)
- [3. AI Chat (P0)](#3-ai-chat-p0)
- [4. AI Provider Configuration (P0)](#4-ai-provider-configuration-p0)
- [5. Scheduler (P1)](#5-scheduler-p1)
- [6. Heartbeat Monitoring (P1)](#6-heartbeat-monitoring-p1)
- [7. Messaging Channels (P1)](#7-messaging-channels-p1)
- [8. Memory (P2)](#8-memory-p2)
- [9. Identity (P2)](#9-identity-p2)
- [10. Skills (P2)](#10-skills-p2)
- [11. Prompt Generator (P2)](#11-prompt-generator-p2)
- [12. Modules (P2)](#12-modules-p2)
- [13. Log Viewer (P1)](#13-log-viewer-p1)
- [14. App Settings (P2)](#14-app-settings-p2)
- [15. Advanced Settings (P3)](#15-advanced-settings-p3)
- [16. Cross-Cutting Concerns](#16-cross-cutting-concerns)
- [17. Security Testing](#17-security-testing)
- [Test Results Summary](#test-results-summary)

---

## Prerequisites

### How to Build and Run the App

1. Ensure you have the following installed:
   - **Node.js** (v18+) and **Bun** package manager
   - **Rust** toolchain (stable, 2024 edition) with `cargo`
   - **Tauri CLI** (`cargo install tauri-cli`)
   - Platform-specific dependencies for Tauri (see [Tauri prerequisites](https://tauri.app/start/prerequisites/))

2. Clone the repository and install dependencies:
   ```bash
   git clone <repo-url> && cd tauriclaw
   bun install
   ```

3. Start the development build:
   ```bash
   bun run tauri dev
   ```
   This launches both the Vite dev server (frontend) and the Rust backend with hot reload.

4. Alternatively, build a production binary:
   ```bash
   bun run tauri build
   ```
   The output binary is in `src-tauri/target/release/bundle/`.

### Required Accounts and Tokens for Testing

| Feature | Requirement |
|---------|-------------|
| AI Chat (OpenAI) | OpenAI API key (starts with `sk-`) from [platform.openai.com](https://platform.openai.com) |
| AI Chat (Anthropic) | Anthropic API key from [console.anthropic.com](https://console.anthropic.com) |
| AI Chat (Google AI) | Google AI API key from [aistudio.google.com](https://aistudio.google.com) |
| AI Chat (Groq) | Groq API key from [console.groq.com](https://console.groq.com) |
| AI Chat (Ollama) | Ollama installed locally with at least one model pulled (`ollama pull llama3.2`) |
| Telegram Channel | Telegram bot token from @BotFather, your chat ID from @userinfobot |
| Discord Channel | Discord bot token from [Discord Developer Portal](https://discord.com/developers/applications) |
| Slack Channel | Slack Bot Token (`xoxb-...`) and App Token (`xapp-...`) from [api.slack.com](https://api.slack.com) |
| Matrix Channel | Matrix access token from Element (Settings > Help & About > Access Token) |

### Test Environment Setup

- Use a fresh app installation (or clear app data) for onboarding tests.
- The app database is stored in the Tauri app-local data directory.
- API keys are stored in the OS keyring (macOS Keychain, Windows Credential Manager, or Linux Secret Service).
- For full testing, have at least one AI provider configured with a valid API key.

---

## Test Conventions

### Status Markers

| Marker | Meaning |
|--------|---------|
| `[ ]` | Not tested |
| `[x]` | Passed |
| `[~]` | Partial pass (some issues noted) |
| `[!]` | Failed |

### Priority Levels

| Level | Meaning | Impact |
|-------|---------|--------|
| **P0** | Critical | Core functionality; app is unusable without it |
| **P1** | High | Important features; degraded experience if broken |
| **P2** | Medium | Secondary features; workarounds may exist |
| **P3** | Low | Nice-to-have; minimal user impact |

### How to Report Issues

When a test case fails, record:
1. **Test case ID** (e.g., TC-3.2)
2. **Actual result** (what happened)
3. **Expected result** (what should have happened)
4. **Screenshot or screen recording** if applicable
5. **Steps to reproduce** (if different from the documented steps)
6. **Environment** (OS, app version, build type)

---

## 1. Onboarding Flow (P0)

### TC-1.1: Fresh Install Welcome Screen

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- Fresh installation of MesoClaw with no prior configuration.
- Alternatively, clear app data to reset onboarding state.

**Steps:**
1. Launch the MesoClaw application for the first time.
2. Observe the screen that appears.

**Expected Results:**
- The onboarding screen loads at the `/onboarding` route.
- A large bot icon is displayed at the top center of the page.
- The heading reads "Welcome to MesoClaw" (or the configured product name).
- The subtitle text reads "Your AI-powered desktop assistant. Let's get you set up in just a few steps."
- A setup checklist is visible, listing: "Connect an AI provider so the app can respond to you" and "Add a messaging channel to interact from your favourite apps".
- A "Get started" button with a right-chevron icon is displayed at the bottom.
- A progress header at the top shows three steps: "Welcome" (current), "AI Provider", and "Channels". The "Welcome" step indicator shows a bordered circle with the number 1.

**Notes:**
- If the user has previously completed onboarding, they will be redirected to the Home page instead. To re-test, you must clear the app's stored settings.

---

### TC-1.2: AI Provider Setup

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- On the onboarding Welcome screen (TC-1.1 passed).
- Have at least one valid AI API key ready (e.g., OpenAI `sk-...`).

**Steps:**
1. Click the "Get started" button on the Welcome screen.
2. Observe the new screen that appears.
3. Wait for the "Provider" dropdown to populate with available providers.
4. Select a provider from the dropdown (e.g., "OpenAI").
5. If the provider requires an API key, observe that an "API Key" input field appears below the dropdown.
6. Enter a valid API key into the "API Key" field.
7. Click the "Save & continue" button.

**Expected Results:**
- The progress header updates to show step 2 ("AI Provider") as current, with step 1 ("Welcome") showing a green checkmark.
- The heading reads "Set up AI" with the description "Choose an AI provider and enter your API key to power the assistant."
- The "Provider" dropdown lists all available providers (OpenAI, Anthropic, Google AI, Groq, Ollama, Vercel AI Gateway).
- When a provider requiring an API key is selected, a password input labeled "API Key" appears with placeholder text "sk-...".
- Below the API Key field, a note reads "API key is stored securely in your OS keyring" with a key icon.
- A "Back" button, "Skip for now" link, and "Save & continue" button are visible.
- After clicking "Save & continue", the button text changes to "Saving..." while the key is being stored, then the app advances to the Channels step.

**Notes:**
- Selecting "Ollama" should NOT display an API Key field (Ollama is a local provider and does not require one).
- The "Skip for now" link advances to the Channels step without saving any provider configuration.

---

### TC-1.3: Channel Setup (Optional Step)

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- On the onboarding AI Provider step (TC-1.2 passed or skipped).

**Steps:**
1. Observe the Channels step screen.
2. Note that it says "Connect a channel (optional)" in the heading.
3. Observe the grid of channel cards (Telegram, Discord, Slack, Matrix).
4. Click the "Telegram" card.
5. Observe the inline configuration form that appears.
6. Optionally enter a Telegram Bot Token and Allowed Chat IDs.
7. Click "Skip for now" or "Continue" to proceed.

**Expected Results:**
- The heading reads "Connect a channel" with "(optional)" in lighter text.
- A grid of channel cards is displayed, each showing: an emoji icon, the channel name (bold), and a description.
- Unavailable channels show a "Coming soon" label and cannot be selected.
- Clicking an available channel card highlights it with a primary border and ring.
- Clicking the same card again deselects it.
- When "Telegram" is selected, an inline configuration panel appears below the grid with fields for "Bot Token" (password input) and "Allowed Chat IDs" (text input).
- A "Test Connection" button is available (disabled until a token is entered).
- "Back", "Skip for now", and "Continue" buttons are visible at the bottom.

**Notes:**
- This step is explicitly optional. Users can skip it and configure channels later in Settings > Channels.

---

### TC-1.4: Complete Onboarding

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- On the onboarding Channels step (TC-1.3 passed or skipped).

**Steps:**
1. Click "Continue" or "Skip for now" on the Channels step.
2. Observe the completion screen.
3. Click the "Open MesoClaw" button.

**Expected Results:**
- A completion screen appears with a large green checkmark icon.
- The heading reads "You're all set!"
- The description reads "MesoClaw is ready. You can adjust all settings anytime from the Settings page."
- An "Open MesoClaw" button with a right-chevron icon is displayed.
- Clicking "Open MesoClaw" navigates to the Home page (`/`).
- Subsequent app launches go directly to the Home page (onboarding is not shown again).

**Notes:**
- The progress header is hidden on the completion step.

---

## 2. Home Dashboard (P1)

### TC-2.1: Greeting Display

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Onboarding is complete. The app is on the Home page (`/`).

**Steps:**
1. Navigate to the Home page by clicking the app logo or "Home" in the sidebar.
2. Observe the greeting text at the top of the page.

**Expected Results:**
- A greeting message is displayed at the top-left: "Good morning, MesoClaw", "Good afternoon, MesoClaw", or "Good evening, MesoClaw" depending on the current time of day (morning: before 12pm, afternoon: 12pm-5pm, evening: after 5pm).
- Below the greeting, today's date is shown in the format "Wednesday, February 20" (long weekday, long month, numeric day).

**Notes:**
- The product name shown after the greeting is configurable via `APP_IDENTITY.productName`.

---

### TC-2.2: Quick Action Cards

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- On the Home page.

**Steps:**
1. Observe the "Quick Actions" section below the greeting.
2. Click on each of the four quick action cards and verify navigation.

**Expected Results:**
- A "Quick Actions" heading is displayed in uppercase muted text.
- Four cards are displayed in a responsive grid (1 column on mobile, 2 on small, 4 on large screens).
- Each card shows: a colored icon in a rounded square, a label, and a description:
  - **New Chat** (Sparkles icon) — "Start a conversation" — navigates to `/chat`
  - **Memory** (Brain icon) — "Search agent memory" — navigates to `/memory`
  - **Channels** (MessageSquare icon) — "View Telegram inbox" — navigates to `/channels`
  - **Settings** (Settings icon) — "Configure providers" — navigates to `/settings`
- Cards have hover effects (border color change, subtle shadow).
- Cards have visible focus indicators when tabbed to (keyboard accessible).

**Notes:**
- None.

---

### TC-2.3: Gateway Status Indicator

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- On the Home page.

**Steps:**
1. Observe the top-right area of the Home page, next to the greeting.
2. Note the small status indicator.

**Expected Results:**
- A gateway status indicator is displayed showing a small colored dot and a label:
  - **Green dot + "Daemon"**: The MesoClaw daemon is connected and running.
  - **Yellow pulsing dot + "Connecting..."**: The daemon connection is being checked.
  - **Gray dot + "Offline"**: The daemon is not running or not reachable.
- Hovering over the indicator shows a tooltip with a more detailed status message.
- The status updates automatically every 10 seconds.

**Notes:**
- The "Daemon" label is only visible on screens wider than the `sm` breakpoint. On mobile, only the dot is shown.

---

### TC-2.4: System Status Section

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Home page.
- At least one AI provider configured (TC-1.2 or TC-4.x).

**Steps:**
1. Scroll down to the "System Status" section on the Home page.
2. Observe the displayed information.

**Expected Results:**
- A "System Status" heading is displayed in uppercase muted text.
- A card displays two status rows:
  - **AI Provider**: Shows the configured provider ID (e.g., "openai") with a green dot, or "Not configured" with a gray dot if none is set.
  - **Model**: Shows the configured model ID (e.g., "gpt-4o") with a green dot, or "Not selected" with a gray dot if none is set.

**Notes:**
- None.

---

## 3. AI Chat (P0)

### TC-3.1: Send Message and Receive Response

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- At least one AI provider is configured with a valid API key (TC-4.1 through TC-4.5).
- The configured provider has at least one model available.

**Steps:**
1. Navigate to the Chat page by clicking "New Chat" on the Home page or "Chat" in the sidebar.
2. Observe the empty state of the chat.
3. Type "Hello, tell me a joke" into the message textarea at the bottom (placeholder text: "Type your message...").
4. Click the submit button (arrow icon) in the bottom-right corner of the input area, or press Enter.
5. Wait for the response to complete streaming.

**Expected Results:**
- The page title is "AI Chat" with the currently selected model name as the description.
- In the empty state, a Sparkles icon and the text "Start a conversation" and "Ask me anything!" are centered in the chat area.
- After typing, the submit button becomes enabled (it is disabled when the input is empty).
- After submitting, the user's message appears as a message bubble in the chat area.
- An assistant message bubble appears below it with "..." as a placeholder while streaming begins.
- The assistant's response streams in token-by-token, replacing the placeholder text.
- Once streaming completes, the message content is fully rendered.
- The submit button shows a "streaming" status indicator while the response is in progress.
- A toast notification does NOT appear (successful responses should be silent).

**Notes:**
- The context panel on the right side (visible on xl+ screens) shows the active model, session message counts (You/AI), and a "Streaming..." indicator during generation.

---

### TC-3.2: Model Selector

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- On the Chat page.
- At least two models are available across configured providers.

**Steps:**
1. In the bottom input area, locate the model selector button on the left side of the footer bar. It shows the current provider's logo icon and the model name.
2. Click the model selector button.
3. Observe the model selector dialog that opens.
4. Use the search input at the top to filter models (e.g., type "gpt").
5. Select a different model from the list.

**Expected Results:**
- Clicking the model selector opens a dialog overlay.
- The dialog contains a search input ("Search models...") at the top.
- Models are grouped by provider name (e.g., "OpenAI", "Anthropic", "Ollama").
- Each model entry shows the provider logo, model display name, and a check icon next to the currently selected model.
- Typing in the search input filters the list in real-time.
- If no models match the search, "No models found." is displayed.
- Selecting a different model closes the dialog, updates the model name shown in the input footer, and displays a success toast: "Switched to [model-name]".

**Notes:**
- If no AI models are configured at all, a toast error appears on page load: "No AI models configured" with the description "Please add models to your providers in Settings > AI Providers".

---

### TC-3.3: Streaming Response Display

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- On the Chat page with a working AI provider.

**Steps:**
1. Send a message that will produce a longer response (e.g., "Write a paragraph about the history of computers").
2. Observe the assistant's response as it streams in.

**Expected Results:**
- The assistant message bubble initially shows "..." as content.
- Text appears progressively, word-by-word or token-by-token.
- The submit button shows a "streaming" status during generation (the icon changes).
- The submit button is disabled during streaming (preventing new messages).
- The textarea is still editable but submitting is blocked.
- In the context panel (xl+ screens), a pulsing dot and "Streaming..." text appear.
- When streaming completes, the "Streaming..." indicator disappears.
- The submit button returns to the "ready" state.

**Notes:**
- If the user scrolls up during streaming, the auto-scroll may pause. A "scroll to bottom" button should appear in the conversation area.

---

### TC-3.4: Skill Suggestions

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Chat page with no messages sent yet (empty state).

**Steps:**
1. Observe the suggestion chips displayed above the input area.
2. Click one of the suggestion chips (e.g., "Explain how to build a desktop app with Tauri").

**Expected Results:**
- Five suggestion chips are displayed in a horizontal scrollable row above the input area:
  - "Explain how to build a desktop app with Tauri"
  - "What are the benefits of using React 19?"
  - "How does TypeScript improve code quality?"
  - "Best practices for state management in React"
  - "Explain the concept of hooks in React"
- Clicking a suggestion chip fills the textarea with that suggestion text.
- The suggestions disappear after the first message is sent.
- The user can edit the filled text before submitting.

**Notes:**
- Suggestions only appear when the message list is empty. Once any message exists, they are hidden permanently for the session.

---

### TC-3.5: Empty State

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Chat page with no messages.

**Steps:**
1. Observe the chat area before sending any messages.

**Expected Results:**
- The center of the chat area shows a large Sparkles icon in the primary color.
- Below the icon: "Start a conversation" in a semibold heading.
- Below the heading: "Ask me anything!" in smaller muted text.
- The input area at the bottom is fully functional with the model selector and submit button.

**Notes:**
- None.

---

### TC-3.6: Error Handling (No API Key)

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- A provider is selected in the chat model selector that requires an API key.
- The API key for that provider has NOT been configured.

**Steps:**
1. Navigate to the Chat page.
2. Select a provider/model that requires an API key (e.g., OpenAI).
3. Ensure no API key has been saved for this provider.
4. Type a message and click submit.

**Expected Results:**
- A toast error notification appears: "No API key found for [provider-id]" with the description "Please add an API key in Settings > AI Providers".
- The message is NOT added to the chat.
- The input text remains in the textarea (not cleared).
- No streaming animation begins.

**Notes:**
- Providers that do not require an API key (e.g., Ollama) should not trigger this error.

---

### TC-3.7: Clear Conversation

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Chat page with at least one message exchanged.
- The context panel is visible (requires xl+ screen width).

**Steps:**
1. Look at the context panel on the right side.
2. Click the "Clear conversation" button at the bottom of the context panel.

**Expected Results:**
- All messages are removed from the chat area.
- The empty state ("Start a conversation") reappears.
- The suggestion chips reappear above the input area.
- The context panel's message counts reset to "You: 0" and "AI: 0".
- The "Clear conversation" button disappears (it only shows when messages exist).

**Notes:**
- This button is only visible in the context panel, which requires an xl+ screen width (1280px or wider).

---

## 4. AI Provider Configuration (P0)

### TC-4.1: Add OpenAI Provider

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > AI Provider tab.
- Have a valid OpenAI API key ready.

**Steps:**
1. Navigate to Settings by clicking "Settings" in the sidebar.
2. The "AI Provider" tab should be selected by default on the left settings nav.
3. Observe the "Global Default Model" section at the top.
4. Under "AI Providers", click the "AI Providers" tab in the provider category tabs.
5. Locate the "OpenAI" provider card.
6. In the "API Key" field, enter your OpenAI API key.
7. Click the "Save" button next to the API key field.
8. Observe the confirmation.

**Expected Results:**
- The Settings page loads with a left navigation showing 9 sections: AI Provider, Skills, App Settings, Identity, Scheduler, Modules, Channels, Mobile, Advanced.
- The AI Provider tab shows a "Global Default Model" section at the top with a "Select Model" button.
- Below that, "AI Providers" heading with four category tabs: "AI Gateway", "AI Providers", "Local", and optionally "User Defined".
- The "AI Providers" tab shows provider cards in a 2-column grid.
- The OpenAI card shows the provider name "OpenAI" and the base URL.
- After entering the API key in the password field and clicking "Save", a loading spinner appears on the Save button.
- A success toast appears: "API key saved successfully".
- A green dot and "Configured" label appear next to the provider name.

**Notes:**
- The eye/eye-off toggle next to the API key input lets you reveal or hide the key value.

---

### TC-4.2: Add Anthropic Provider

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > AI Provider tab.
- Have a valid Anthropic API key ready.

**Steps:**
1. On the AI Provider settings tab, click the "AI Providers" category tab.
2. Locate the "Anthropic" provider card.
3. Enter your Anthropic API key in the "API Key" field.
4. Click "Save".

**Expected Results:**
- The Anthropic card is displayed with a "API Key" password field.
- After saving, a success toast appears: "API key saved successfully".
- The provider status changes to show a green dot and "Configured" text.

**Notes:**
- Anthropic models may be available through AI Gateway providers (OpenRouter, Vercel AI Gateway) rather than direct API access.

---

### TC-4.3: Add Google AI Provider

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > AI Provider tab.
- Have a valid Google AI API key ready.

**Steps:**
1. On the AI Provider settings tab, click the "AI Providers" category tab.
2. Locate the "Google AI" provider card.
3. Enter your Google AI API key.
4. Click "Save".

**Expected Results:**
- Same behavior as TC-4.1 but for the Google AI provider.
- Success toast: "API key saved successfully".
- Green dot and "Configured" label appear.

**Notes:**
- None.

---

### TC-4.4: Add Groq Provider

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > AI Provider tab.
- Have a valid Groq API key ready.

**Steps:**
1. On the AI Provider settings tab, click the "AI Providers" category tab.
2. Locate the "Groq" provider card.
3. Enter your Groq API key.
4. Click "Save".

**Expected Results:**
- Same behavior as TC-4.1 but for the Groq provider.
- Success toast: "API key saved successfully".
- Green dot and "Configured" label appear.

**Notes:**
- None.

---

### TC-4.5: Configure Ollama (Local)

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Ollama is installed and running on the local machine (`ollama serve`).
- At least one model is pulled (`ollama pull llama3.2`).
- Navigate to Settings > AI Provider tab.

**Steps:**
1. Click the "Local" category tab under AI Providers.
2. Locate the "Ollama" provider card.
3. Observe that no API Key field is shown (Ollama is local and does not require a key).
4. Note the green dot and "Local" label.
5. Click the refresh button (circular arrows icon) in the card header to discover models.
6. Wait for the discovery to complete.
7. Expand the "Discovered Models" collapsible section.

**Expected Results:**
- The Ollama card shows "Local" status instead of "Configured" (no API key required).
- No "API Key" input field is displayed.
- Clicking the refresh button triggers Ollama model discovery.
- A success toast appears: "Successfully discovered N new Ollama model(s)" or "Ollama models are already up to date".
- The "Discovered Models" collapsible section shows all locally available Ollama models with their model IDs.
- If Ollama is not running, the discovery silently fails without showing an error toast on auto-discovery (manual refresh shows an error toast).

**Notes:**
- Ollama must be running (`ollama serve`) for model discovery to work.
- Auto-discovery runs on page load if the Ollama provider has zero models registered.

---

### TC-4.6: Add User-Defined Provider

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > AI Provider tab.

**Steps:**
1. Click the "Add Provider" button in the top-right area of the AI Providers section.
2. In the dialog that opens, fill in:
   - **ID**: `my-provider` (lowercase, no spaces)
   - **Name**: `My Custom Provider`
   - **Base URL**: `https://api.example.com/v1`
   - **Requires API Key**: Toggle on
   - Add at least one initial model ID.
3. Click "Add" or "Save" to create the provider.

**Expected Results:**
- An "Add Provider" dialog opens with fields for ID, Name, Base URL, and a toggle for "Requires API Key".
- After filling in the fields and submitting, a success toast appears: "Successfully added provider 'My Custom Provider'".
- The view automatically switches to the "User Defined" tab.
- A new "User Defined" tab appears in the provider category tabs (if it wasn't visible before).
- The new provider card is displayed with a delete button (trash icon) that only appears for user-defined providers.

**Notes:**
- Built-in providers (OpenAI, Anthropic, etc.) cannot be deleted; only user-defined providers show a delete button.

---

### TC-4.7: Test Connection

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- At least one provider has an API key saved (TC-4.1 through TC-4.4).

**Steps:**
1. On the AI Provider settings tab, locate a configured provider card.
2. Click the "Test Connection" button at the bottom of the card.
3. Wait for the test to complete.

**Expected Results:**
- The button text changes to "Testing..." with a spinning loader icon.
- On success: a green success banner appears below the API key field reading "Connection successful! (XXms)" where XX is the latency in milliseconds. A success toast also appears.
- On failure: a red error banner appears with the error message (e.g., "Invalid API key"). An error toast also appears.
- The "Test Connection" button is disabled if the provider requires an API key and none has been entered.

**Notes:**
- The test result banner persists on the card until the API key is changed or the page is reloaded.

---

### TC-4.8: Set Default Model

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- At least one provider is configured with models available.

**Steps:**
1. On the AI Provider settings tab, locate the "Global Default Model" section at the top.
2. Click the button showing the current default model (or "Select Model").
3. In the model selector dialog that opens, browse or search for a model.
4. Select a model.

**Expected Results:**
- The Global Default Model selector dialog opens, showing all available models from configured providers.
- Models can be filtered by searching.
- Selecting a model closes the dialog.
- A success toast appears: "Global default model updated".
- The button now shows the name of the selected model.
- The selected model is used as the default across the application.

**Notes:**
- Individual providers can also be configured with a specific model via the "Configure" button on each provider card.

---

### TC-4.9: Add Custom Model

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- At least one provider card is visible on the AI Provider settings tab.

**Steps:**
1. On a provider card, click the "+" (plus) icon button in the card header.
2. In the dialog that opens, enter a model ID (e.g., `gpt-4-turbo-preview`).
3. Click "Add Model".

**Expected Results:**
- A dialog opens with the title "Add Custom Model to [Provider Name]".
- A text input for "Model ID" with a placeholder "Enter model ID" is shown.
- After entering a model ID and clicking "Add Model", the dialog closes.
- A success toast appears: "Successfully added model '[model-id]' to [Provider Name]".
- The model appears in the "Custom Models" collapsible section on the provider card.
- Custom models can be deleted individually using the trash icon next to each model entry.

**Notes:**
- Pressing Enter in the model ID field also triggers the add action.
- The dialog shows a loading spinner while the model is being added.

---

### TC-4.10: API Key Security (Verify Keyring Storage)

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- At least one provider has an API key saved.

**Steps:**
1. Save an API key for any provider (TC-4.1).
2. Close and reopen the application.
3. Navigate to Settings > AI Provider.
4. Observe that the provider shows "Configured" status.
5. Check that the API key field shows masked dots (password field).
6. Verify the API key is NOT stored in any local config file or database:
   - On macOS: Check Keychain Access for the stored key.
   - On Linux: Use `secret-tool search service mesoclaw` (or similar).
   - On Windows: Check Windows Credential Manager.

**Expected Results:**
- The API key persists across app restarts (the provider still shows "Configured").
- The API key is stored in the OS keyring, not in any plain-text file.
- The API key field displays the stored key as masked dots, but can be revealed with the eye toggle button.
- No API key values appear in application log files.

**Notes:**
- This is a critical security requirement. API keys must never be stored in plain text.

---

## 5. Scheduler (P1)

### TC-5.1: Create Interval Job

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Scheduler tab.

**Steps:**
1. Click "Settings" in the sidebar.
2. Click "Scheduler" in the left settings nav.
3. Observe the "Scheduled Jobs" heading and the empty state message.
4. Click the "+ Add Job" button.
5. In the creation form that appears:
   - **Name**: Enter "Test Interval Job".
   - **Schedule**: Select the "Interval" radio button (should be selected by default).
   - **Every (secs)**: Enter `60`.
   - **Action**: Select "Run Heartbeat checklist" from the dropdown.
6. Click "Create Job".

**Expected Results:**
- Clicking "+ Add Job" reveals an inline creation form with a "New Scheduled Job" heading.
- The form shows: Name input, Schedule type radio buttons (Interval / Cron), interval seconds input, Action dropdown.
- After filling in the fields and clicking "Create Job", the button shows "Creating..." while saving.
- The form disappears after successful creation.
- A new row appears in the jobs table showing:
  - Name: "Test Interval Job"
  - Schedule: "every 1m" (converted from 60 seconds)
  - Action: "Heartbeat"
  - Next run: a date/time value
  - Errors: (empty)
  - Active: a toggle switch (enabled by default)
  - A "Delete" button

**Notes:**
- The "Create Job" button is disabled if the Name field is empty.

---

### TC-5.2: Create Cron Job

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Scheduler tab.

**Steps:**
1. Click "+ Add Job".
2. Enter "Daily Summary" as the name.
3. Select the "Cron" radio button for schedule type.
4. In the Cron Builder UI, configure a daily schedule (e.g., `0 9 * * *` for 9 AM daily).
5. Select "Publish notification" as the Action.
6. Enter "Time for daily review" in the Message field that appears.
7. Click "Create Job".

**Expected Results:**
- Selecting "Cron" reveals the CronBuilder component instead of the interval seconds input.
- The Cron Builder allows building a cron expression through a user-friendly interface.
- After creation, the job table shows the cron expression in the Schedule column (e.g., `0 9 * * *`).
- The Action column shows "Notify: Time for daily review".

**Notes:**
- The CronBuilder component provides a guided interface for cron expressions.

---

### TC-5.3: Create Heartbeat Job

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Scheduler tab.

**Steps:**
1. Click "+ Add Job".
2. Enter "Heartbeat Check" as the name.
3. Select "Interval" schedule type with 300 seconds (5 minutes).
4. Select "Run Heartbeat checklist" as the Action.
5. Click "Create Job".

**Expected Results:**
- The job is created with Action showing "Heartbeat".
- The schedule shows "every 5m".

**Notes:**
- See section 6 (Heartbeat Monitoring) for testing the heartbeat functionality itself.

---

### TC-5.4: Create Agent Turn Job

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Scheduler tab.

**Steps:**
1. Click "+ Add Job".
2. Enter "AI Summary" as the name.
3. Configure any schedule (interval or cron).
4. Select "Agent Turn (custom prompt)" as the Action.
5. Observe that a "Prompt" text input appears.
6. Enter "Summarise today's work" as the prompt.
7. Click "Create Job".

**Expected Results:**
- When "Agent Turn (custom prompt)" is selected, a Prompt input field appears below the Action dropdown.
- After creation, the Action column shows "Agent: Summarise today's work..." (truncated to ~40 characters).

**Notes:**
- None.

---

### TC-5.5: Create Notify Job

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Scheduler tab.

**Steps:**
1. Click "+ Add Job".
2. Enter "Break Reminder" as the name.
3. Configure an interval of 3600 seconds (1 hour).
4. Select "Publish notification" as the Action.
5. Observe that a "Message" text input appears.
6. Enter "Time for a break!" as the message.
7. Click "Create Job".

**Expected Results:**
- When "Publish notification" is selected, a Message input field appears below the Action dropdown.
- After creation, the Action column shows "Notify: Time for a break!".
- The Schedule column shows "every 1h".

**Notes:**
- None.

---

### TC-5.6: Toggle Job Enable/Disable

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- At least one scheduled job exists (TC-5.1 through TC-5.5).

**Steps:**
1. In the jobs table, locate a job with the Active toggle switch turned on (enabled).
2. Click the toggle switch to disable the job.
3. Click the toggle switch again to re-enable the job.

**Expected Results:**
- The toggle switch visually changes state (on/off).
- When disabled, the job should not execute on its schedule.
- When re-enabled, the job resumes its schedule.
- The Next run column should update accordingly.

**Notes:**
- The toggle has an aria-label of "Disable job" (when enabled) or "Enable job" (when disabled) for accessibility.

---

### TC-5.7: Delete Job

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- At least one scheduled job exists.

**Steps:**
1. In the jobs table, click the "Delete" button (red text) on any job row.

**Expected Results:**
- The job is immediately removed from the table.
- If it was the last job, the empty state message reappears: "No scheduled jobs yet."

**Notes:**
- There is currently no confirmation dialog before deletion. Be cautious when deleting jobs.

---

### TC-5.8: View Execution History

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- At least one scheduled job has executed at least once.

**Steps:**
1. In the jobs table, observe the "Errors" column for each job.

**Expected Results:**
- If a job has encountered errors, a red badge shows the error count (e.g., "2 err").
- Jobs with no errors show no badge in the Errors column.

**Notes:**
- Detailed execution history may be viewable in the Log Viewer (Section 13).

---

### TC-5.9: Job Persistence (Restart App)

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- At least one scheduled job has been created.

**Steps:**
1. Note the current jobs in the Scheduler tab.
2. Completely close and restart the MesoClaw application.
3. Navigate back to Settings > Scheduler.

**Expected Results:**
- All previously created jobs are still listed in the table.
- Job enable/disable states are preserved.
- Job schedules and configurations are unchanged.

**Notes:**
- None.

---

### TC-5.10: Error Backoff Behavior

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- A scheduled job configured to run frequently (e.g., every 10 seconds).
- The job is configured in a way that will fail (e.g., heartbeat with missing HEARTBEAT.md).

**Steps:**
1. Create a heartbeat job with a short interval.
2. Allow it to fail multiple times.
3. Observe the error count in the Errors column.

**Expected Results:**
- The error count badge increments with each failure.
- The system implements error backoff (the job may reduce its execution frequency after repeated failures).

**Notes:**
- Check the Log Viewer (Section 13) for detailed error messages related to failed job executions.

---

## 6. Heartbeat Monitoring (P1)

### TC-6.1: Create HEARTBEAT.md

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Access to the file system where the app expects the HEARTBEAT.md file.

**Steps:**
1. Create a `HEARTBEAT.md` file in the expected location (typically the app's config directory or a monitored directory).
2. Add heartbeat check content to the file.

**Expected Results:**
- The file is created and accessible by the application.

**Notes:**
- The exact location for HEARTBEAT.md depends on the app configuration. Check the app documentation or logs for the expected path.

---

### TC-6.2: Schedule Heartbeat Job

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- HEARTBEAT.md exists (TC-6.1).

**Steps:**
1. Navigate to Settings > Scheduler.
2. Create a new job with Action set to "Run Heartbeat checklist".
3. Set the interval to 300 seconds (5 minutes).
4. Click "Create Job".

**Expected Results:**
- The heartbeat job is created and appears in the jobs table.
- The Action column shows "Heartbeat".
- The job runs at the configured interval.

**Notes:**
- None.

---

### TC-6.3: Verify Heartbeat OK

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- A heartbeat job is scheduled and enabled (TC-6.2).
- HEARTBEAT.md is present and valid (TC-6.1).

**Steps:**
1. Wait for the heartbeat job to execute.
2. Check the Errors column in the Scheduler tab.
3. Check the Log Viewer for heartbeat-related log entries.

**Expected Results:**
- The Errors column shows no error badge.
- The Log Viewer shows INFO-level entries indicating successful heartbeat checks.
- If notifications are enabled, a heartbeat notification may appear (if the "Heartbeat" notification category is toggled on in App Settings).

**Notes:**
- None.

---

### TC-6.4: Trigger Heartbeat Failure

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- A heartbeat job is scheduled and enabled.

**Steps:**
1. Rename or delete the HEARTBEAT.md file.
2. Wait for the next heartbeat execution.
3. Check the Errors column and Log Viewer.

**Expected Results:**
- The heartbeat job fails.
- The Errors column shows a red badge with the error count (e.g., "1 err").
- The Log Viewer shows ERROR or WARN level entries describing the heartbeat failure.

**Notes:**
- Restore the HEARTBEAT.md file after this test to resume normal heartbeat monitoring.

---

### TC-6.5: Active Hours Constraint

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- A heartbeat job is scheduled.
- DND schedule is configured in App Settings (e.g., 10 PM to 7 AM).

**Steps:**
1. Configure DND hours in Settings > App Settings > Notifications.
2. Set up a heartbeat job.
3. Verify behavior during and outside DND hours.

**Expected Results:**
- During DND hours, heartbeat notifications are suppressed (if notification integration is active).
- The heartbeat job itself still executes (DND only affects notifications, not job execution).

**Notes:**
- DND only affects notification display, not the actual execution of scheduled jobs.

---

### TC-6.6: View Heartbeat in Execution History

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- A heartbeat job has executed at least once.

**Steps:**
1. Navigate to the Log Viewer page.
2. Search for "heartbeat" in the search field.
3. Observe the filtered results.

**Expected Results:**
- Log entries related to heartbeat execution are displayed.
- Entries show the timestamp, log level (INFO for success, ERROR/WARN for failure), and message describing the heartbeat result.

**Notes:**
- None.

---

## 7. Messaging Channels (P1)

### TC-7.1: Configure Telegram Bot

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Channels tab.
- Have a Telegram bot token ready.

**How to obtain a Telegram bot token:**
1. Open Telegram and search for `@BotFather`.
2. Send `/newbot` and follow the prompts to choose a name and username for your bot.
3. BotFather will reply with your bot token (format: `110201543:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw`).
4. To find your chat ID, message `@userinfobot` on Telegram and it will reply with your user ID.

**Steps:**
1. Click "Settings" in the sidebar, then click "Channels" in the settings nav.
2. Observe the channel list. Each channel shows: a status dot, emoji icon, channel name, status text, message count badge, and a Connect/Disconnect button.
3. Click on the "Telegram" channel card to expand its configuration panel.
4. Read the "How to create a Telegram bot" guide displayed in a bordered section.
5. Enter the bot token in the "Bot Token" field (password input).
6. Enter your chat ID(s) in the "Allowed Chat IDs" field (comma-separated).
7. Optionally adjust the "Polling Timeout" (default: 30 seconds, range: 5-60).
8. Click "Test Connection".
9. After a successful test, click "Save".

**Expected Results:**
- The Channels tab shows a list of channel cards: Telegram, Discord, Slack, Matrix, and possibly tauri-ipc and webhook.
- Clicking the Telegram card expands an inline configuration panel below it.
- The configuration panel shows:
  - A step-by-step BotFather setup guide (4 steps).
  - "Bot Token" password input with placeholder `110201543:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw`.
  - "Allowed Chat IDs" text input with placeholder `123456789, -100987654321`.
  - A helper text: "Comma-separated Telegram chat IDs. Unknown senders are silently ignored."
  - "Polling Timeout (seconds)" number input (range: 5-60, default: 30).
  - "Test Connection" button (disabled until a token is entered).
  - "Save" button.
- Clicking "Test Connection":
  - Button changes to "Testing...".
  - On success: green text "Connected successfully" appears.
  - On failure: red text "Connection failed - check your token" appears.
- Clicking "Save": Button changes to "Saving..." then the configuration is persisted.

**Notes:**
- The note states: "Obtained from BotFather. Stored securely in the OS keyring."

---

### TC-7.2: Configure Discord Bot

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Channels tab.
- Have a Discord bot token ready.

**How to obtain a Discord bot token:**
1. Go to [discord.com/developers/applications](https://discord.com/developers/applications).
2. Click "New Application", give it a name, and click "Create".
3. In the left sidebar, click "Bot".
4. Click "Copy" next to the Token field to copy your bot token.
5. Under "Privileged Gateway Intents", enable "Message Content Intent".
6. Go to "OAuth2" > "URL Generator", select the "bot" scope and "Send Messages" permission.
7. Use the generated URL to invite the bot to your Discord server.
8. To get Server/Channel IDs: Enable Developer Mode in Discord (Settings > App Settings > Advanced > Developer Mode), then right-click a server or channel and select "Copy ID".

**Steps:**
1. Click the "Discord" channel card to expand its configuration.
2. Read the "How to create a Discord bot" setup guide (4 steps).
3. Enter the bot token in the "Bot Token" field.
4. Optionally enter allowed Server (Guild) IDs and Channel IDs.
5. Click "Test Connection".
6. After a successful test, click "Save".

**Expected Results:**
- The Discord configuration panel shows:
  - A 4-step setup guide referencing the Discord Developer Portal.
  - "Bot Token" password input with placeholder `MTExMjM0NTY3ODkwMTIzNDU2.Gh1234.abc...`.
  - "Allowed Server (Guild) IDs" text input (comma-separated). Helper text: "Leave empty to accept messages from all servers."
  - "Allowed Channel IDs" text input (comma-separated). Helper text: "Leave empty to accept messages from all channels."
  - "Test Connection" and "Save" buttons.
- Test Connection behavior is the same as Telegram (TC-7.1).

**Notes:**
- The setup guide mentions enabling "Message Content Intent" under Privileged Gateway Intents, which is required for the bot to read message content.

---

### TC-7.3: Configure Slack Bot

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Channels tab.
- Have Slack Bot Token and App Token ready.

**How to obtain Slack tokens:**
1. Go to [api.slack.com/apps](https://api.slack.com/apps) and click "Create New App" > "From scratch".
2. Under **Socket Mode** in the left sidebar: Enable Socket Mode and generate an App-Level Token with `connections:write` scope. Copy the `xapp-...` token.
3. Under **OAuth & Permissions**: Add Bot Token Scopes: `channels:history`, `chat:write`, `im:history`.
4. Click "Install to Workspace" and authorize. Copy the Bot User OAuth Token (`xoxb-...`).
5. Under **Event Subscriptions**: Subscribe to Bot Events: `message.channels`, `message.im`.
6. To find Channel IDs: Right-click a channel in Slack > View channel details > scroll to find the Channel ID.

**Steps:**
1. Click the "Slack" channel card to expand its configuration.
2. Read the 5-step Socket Mode setup guide.
3. Enter the Bot Token (`xoxb-...`) in the "Bot Token" field.
4. Enter the App Token (`xapp-...`) in the "App Token (Socket Mode)" field.
5. Optionally enter allowed Channel IDs.
6. Click "Test Connection".
7. After a successful test, click "Save".

**Expected Results:**
- The Slack configuration panel shows:
  - A 5-step setup guide referencing api.slack.com and Socket Mode.
  - "Bot Token" password input with placeholder `xoxb-xxxxxxxxxxxx-xxxxxxxxxxxx-xxxxxxxxxxxx`. Helper: "Bot User OAuth Token from OAuth & Permissions."
  - "App Token (Socket Mode)" password input with placeholder `xapp-1-XXXXXXXXX-0000000000000-abc...`. Helper: "App-Level Token for Socket Mode. Starts with xapp-."
  - "Allowed Channel IDs" text input with placeholder `C01234567AB, C09876543ZZ`. Helper: "Right-click a channel in Slack > View channel details > Copy channel ID."
  - "Test Connection" and "Save" buttons.

**Notes:**
- Slack uses Socket Mode (WebSocket), so no public HTTPS endpoint is required.

---

### TC-7.4: Configure Matrix

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Channels tab.
- Have Matrix homeserver URL, username, and access token ready.

**How to obtain Matrix credentials:**
1. Create a bot account on any Matrix homeserver (e.g., register at [matrix.org](https://matrix.org)).
2. Log in with Element (web or desktop client).
3. Go to Element > Settings > Help & About > scroll down to "Access Token" and copy it.
4. Alternatively, use the Matrix client-server API (`POST /_matrix/client/v3/login`) to obtain an access token programmatically.
5. Invite the bot account to the rooms you want to monitor.
6. To find Room IDs: In Element, go to Room Settings > Advanced > "Internal room ID" (format: `!abc123:matrix.org`).

**Steps:**
1. Click the "Matrix" channel card to expand its configuration.
2. Read the 4-step setup guide.
3. Enter the Homeserver URL (e.g., `https://matrix.org`) in the "Homeserver URL" field.
4. Enter the Username / MXID (e.g., `@mybot:matrix.org`) in the "Username (MXID)" field.
5. Enter the Access Token in the "Access Token" field.
6. Optionally enter allowed Room IDs (comma-separated).
7. Click "Test Connection".
8. After a successful test, click "Save".

**Expected Results:**
- The Matrix configuration panel shows:
  - A 4-step setup guide mentioning Element, access tokens, room invitations, and bridge tips.
  - "Homeserver URL" URL input with placeholder `https://matrix.org`.
  - "Username (MXID)" text input with placeholder `@mybot:matrix.org`. Helper: "Full Matrix ID including the server part."
  - "Access Token" password input. Helper: "Obtained from Element > Settings > Help & About."
  - "Allowed Room IDs" text input with placeholder `!abc123:matrix.org, !xyz789:matrix.org`. Helper: "Leave empty to receive messages from all joined rooms."
  - "Test Connection" and "Save" buttons.
- The setup guide includes a "Bridge tip" about using protocol bridges (mautrix-whatsapp, etc.) to relay messages from other platforms through Matrix.

**Notes:**
- Matrix is strategic because it supports bridging to WhatsApp, Signal, Slack, IRC, and more.

---

### TC-7.5: Test Connection (Each Platform)

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- At least one channel is configured with valid credentials (TC-7.1 through TC-7.4).

**Steps:**
1. For each configured channel, expand its configuration panel.
2. Click the "Test Connection" button.
3. Observe the result.

**Expected Results:**
- **Success**: Green text appears: "Connected successfully" (Telegram) or similar message.
- **Failure**: Red text appears: "Connection failed - check your token" (or similar message mentioning the relevant credential).
- The "Test Connection" button is disabled while the test is in progress, showing "Testing..." text.
- The button is disabled when the required credential fields are empty.

**Notes:**
- Test each platform independently: Telegram, Discord, Slack, Matrix.

---

### TC-7.6: Connect Channel

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- At least one channel is configured and saved (TC-7.1 through TC-7.4).

**Steps:**
1. Navigate to Settings > Channels tab.
2. Find a configured but disconnected channel card (status shows gray dot and "Disconnected").
3. Click the "Connect" button on the card.

**Expected Results:**
- The button briefly shows "..." during the connection attempt.
- On success:
  - The status dot changes from gray to green.
  - The status text changes to "Connected".
  - The button text changes to "Disconnect".
- On failure:
  - The status dot changes to red.
  - The status text changes to "Error" with the error message appended.

**Notes:**
- The "tauri-ipc" channel (Desktop IPC) is always connected and its Connect button is disabled with the title "Desktop IPC is always connected".

---

### TC-7.7: View Messages on Channels Page

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- At least one channel is connected and has received messages.

**Steps:**
1. Navigate to the Channels page by clicking "Channels" in the sidebar.
2. Observe the channel list on the left side.
3. Click on a channel name in the left sidebar.
4. Observe the messages displayed in the main area.

**Expected Results:**
- The left sidebar lists all configured channels, each showing the channel name and a message count badge (red) if there are messages.
- If no channels are configured, the sidebar shows: "No channels configured. Configure." with a link to the Settings page.
- Clicking a channel name highlights it in the sidebar.
- The main area header shows `#channelname`.
- Messages are displayed in chronological order, each showing:
  - Sender name (bold, small text).
  - Timestamp (muted text, formatted as local time).
  - Message content in a message bubble.
  - A "Reply to [sender]" button below each message.
- If no messages exist for the selected channel, it shows an empty state message.

**Notes:**
- The context panel (xl+ screens) shows: the selected channel name, message count, a list of senders (up to 5), and the current reply recipient.

---

### TC-7.8: Send Message via Channel

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- On the Channels page with a connected channel selected.

**Steps:**
1. Select a connected channel in the left sidebar.
2. Type a message in the composer input at the bottom of the message area.
3. Click the submit button to send.

**Expected Results:**
- The composer input at the bottom of the message area is functional.
- The submit button is disabled when the input is empty.
- After sending, the input is cleared.
- The message is sent via the selected channel's platform (e.g., Telegram bot sends the message).
- If sending fails, an error message appears below the composer in red text.

**Notes:**
- None.

---

### TC-7.9: Reply to Message

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Channels page with messages visible.

**Steps:**
1. Click the "Reply to [sender name]" button below a message.
2. Observe the reply indicator that appears above the composer.
3. Type a reply message.
4. Click submit to send.
5. Click the "X" button on the reply indicator to cancel the reply.

**Expected Results:**
- Clicking "Reply to [name]" sets the reply recipient.
- A reply indicator appears above the composer showing "Replying to: [recipient name]" with an "X" button to clear it.
- The context panel (xl+ screens) shows the current "Replying To" recipient.
- After sending, the reply indicator is cleared and the input is emptied.
- Clicking "X" on the reply indicator clears the recipient without sending.

**Notes:**
- None.

---

### TC-7.10: Disconnect Channel

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- A channel is currently connected (Settings > Channels).

**Steps:**
1. Navigate to Settings > Channels tab.
2. Locate a connected channel (green dot, "Connected" text, "Disconnect" button).
3. Click the "Disconnect" button.

**Expected Results:**
- The status dot changes from green to gray.
- The status text changes to "Disconnected".
- The button text changes back to "Connect".
- The channel stops receiving messages.

**Notes:**
- None.

---

### TC-7.11: Channel Health Status Indicators

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Channels tab.

**Steps:**
1. Observe the status dots on each channel card.
2. Note the different states.

**Expected Results:**
- **Green dot (static)**: "Connected" - the channel is connected and active.
- **Yellow dot (pulsing/animated)**: "Reconnecting..." - the channel is attempting to reconnect.
- **Red dot**: "Error" - the channel encountered an error. The error message is appended to the status text.
- **Gray dot**: "Disconnected" - the channel is not connected.
- Each channel card also shows a message count badge (secondary variant) if there are messages.

**Notes:**
- The status dot animations are CSS-based (pulse animation for reconnecting state).

---

## 8. Memory (P2)

### TC-8.1: Semantic Search

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to the Memory page.
- The agent has some stored memory/observations (from previous chat interactions).

**Steps:**
1. Click "Memory" in the sidebar to navigate to the Memory page.
2. Observe the two tabs: "Search" and "Daily Timeline". The "Search" tab should be active by default.
3. In the MemorySearch component, enter a search query (e.g., "project setup").
4. Submit the search.
5. Observe the results.

**Expected Results:**
- The Memory page header reads "Memory" with description "Search the agent's semantic memory or browse daily journals."
- Two tabs are displayed in a rounded pill-style tab bar: "Search" (active by default) and "Daily Timeline".
- The MemorySearch component provides a text input for semantic queries.
- Search results are ranked by semantic similarity.
- Each result shows the matched memory content.
- If no results are found, an appropriate empty state message is displayed.

**Notes:**
- Semantic search requires that the agent has previously stored observations/memories. If the memory store is empty, no results will be returned.

---

### TC-8.2: Daily Timeline Navigation

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to the Memory page.
- The agent has daily journal entries.

**Steps:**
1. Click the "Daily Timeline" tab.
2. Observe the DailyTimeline component.
3. Navigate between dates.

**Expected Results:**
- Switching to the "Daily Timeline" tab shows the DailyTimeline component.
- The context panel (xl+ screens) switches to show "Today" with the current date and a hint: "Browse journal entries by day in the timeline."
- The timeline allows navigating between dates to view daily entries.

**Notes:**
- None.

---

### TC-8.3: View Daily Entry

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Memory page, "Daily Timeline" tab selected.
- At least one daily entry exists.

**Steps:**
1. Select a date that has a journal entry.
2. View the entry content.

**Expected Results:**
- The daily entry content is displayed for the selected date.
- If no entry exists for a selected date, an appropriate message is shown.

**Notes:**
- None.

---

### TC-8.4: Search Tips Display

**Priority:** P3
**Status:** [ ] Not Tested

**Preconditions:**
- On the Memory page with the "Search" tab active.
- Context panel visible (xl+ screen width).

**Steps:**
1. Observe the context panel on the right side while on the "Search" tab.

**Expected Results:**
- The context panel shows a "Search Tips" section with three tips:
  - "Use natural language to query semantic memory."
  - "Try searching for topics, events, or entities."
  - "Results are ranked by semantic similarity."

**Notes:**
- Switching to the "Daily Timeline" tab changes the context panel to show today's date and timeline browsing hints.

---

## 9. Identity (P2)

### TC-9.1: List Identity Files

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Identity tab.
- The identity files directory exists with at least one file.

**Steps:**
1. Click "Settings" in the sidebar.
2. Click "Identity" in the settings nav.
3. Observe the file list on the left side.

**Expected Results:**
- The Identity Editor shows a two-pane layout: a file list on the left (fixed width ~176px) and an editor area on the right.
- The file list heading reads "Identity Files" in uppercase muted text.
- Each identity file is listed as a clickable button showing the filename in monospace text.
- If loading, an animated "Loading files..." text appears.
- If no identity files exist, the text "No identity files found." is shown.

**Notes:**
- The entire component has a fixed height of 480px.

---

### TC-9.2: Select and View File

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- At least one identity file exists in the list (TC-9.1).

**Steps:**
1. Click on a file name in the file list.
2. Observe the editor pane on the right.

**Expected Results:**
- The selected file is highlighted with an accent background and primary border.
- The editor pane shows:
  - A toolbar with the filename (monospace, bold), an "unsaved" badge (only if changes are pending), a "Preview" button, and a "Save" button (disabled when no changes have been made).
  - A large textarea filled with the file's content in monospace font.
- Before selecting any file, the editor area shows "Select a file to edit" centered.

**Notes:**
- None.

---

### TC-9.3: Edit Identity File

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- A file is selected in the Identity Editor (TC-9.2).

**Steps:**
1. Click into the textarea and make changes to the content.
2. Observe the "unsaved" badge and the Save button state.

**Expected Results:**
- The textarea is editable with spell-check disabled.
- After making any change, an "unsaved" badge appears in the toolbar.
- The "Save" button becomes enabled (was disabled when content matched the original).

**Notes:**
- None.

---

### TC-9.4: Save with Status Indicator

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- An identity file has been modified (TC-9.3).

**Steps:**
1. Click the "Save" button.
2. Observe the button state changes.

**Expected Results:**
- The "Save" button text changes to "Saving..." while the save operation is in progress.
- After a successful save, the button text briefly changes to "Saved (checkmark)" for 2 seconds, then returns to "Save".
- The "unsaved" badge disappears.
- The "Save" button becomes disabled again (content now matches the saved state).
- If saving fails, an error message appears below the toolbar in red text.

**Notes:**
- None.

---

### TC-9.5: Preview Mode Toggle

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- A file is selected with content visible in the editor.

**Steps:**
1. Click the "Preview" button in the toolbar.
2. Observe the editor area change.
3. Click the "Edit" button (which replaced "Preview") to switch back.

**Expected Results:**
- Clicking "Preview" replaces the textarea with a read-only preview pane.
- The preview shows the content in a pre-formatted block with sans-serif font, wrapped text, and a muted background.
- The button text changes from "Preview" to "Edit".
- Clicking "Edit" returns to the editable textarea with the current content.

**Notes:**
- Changes made in edit mode are preserved when switching to preview and back.

---

### TC-9.6: Hot-Reload Verification

**Priority:** P3
**Status:** [ ] Not Tested

**Preconditions:**
- An identity file has been modified externally (e.g., using a text editor outside of MesoClaw).

**Steps:**
1. Open an identity file in MesoClaw.
2. Without closing MesoClaw, edit the same file using an external text editor and save.
3. Select a different file in MesoClaw, then re-select the original file.

**Expected Results:**
- When re-selecting the file, the latest content from disk is loaded.
- Any external changes are reflected in the editor.

**Notes:**
- This tests the file reloading behavior when switching between files, not real-time file watching.

---

## 10. Skills (P2)

### TC-10.1: List Available Skills

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Skills tab.
- At least one skill file exists in the skills directory (`~/.config/<appDir>/skills/`).

**Steps:**
1. Click "Settings" in the sidebar.
2. Click "Skills" in the settings nav.
3. Observe the skills list.

**Expected Results:**
- The page header shows "AI Skills" with description "Configure which AI capabilities are available for this workspace."
- A "Refresh" button is available in the header area.
- Skills are organized into categories (e.g., Performance, Understanding, Security, Documentation, General) with section headers and descriptions.
- Each skill row shows:
  - Skill name (bold text).
  - Source badge ("Local" for filesystem-loaded skills).
  - A sparkle icon if the skill is "enabled by default" (with tooltip "Enabled by default").
  - Skill description text (up to 2 lines, clamped).
  - A toggle switch to enable/disable the skill.
- If no skills are available, an empty state is shown with a Sparkles icon, "No skills available." text, and a "Load Skills" button.

**Notes:**
- Skills are markdown files with YAML frontmatter stored in the filesystem.

---

### TC-10.2: Enable/Disable Skill

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Skills are listed (TC-10.1).

**Steps:**
1. Find a skill that is currently disabled (toggle switch off).
2. Click the toggle switch to enable it.
3. Click the toggle switch again to disable it.

**Expected Results:**
- The toggle switch visually changes state.
- The skill's enabled/disabled state persists (verified by navigating away and returning).
- The toggle is disabled while loading.

**Notes:**
- None.

---

### TC-10.3: Configure Skill Parameters

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Skills are listed.

**Steps:**
1. Observe the "Selection Mode" section at the top of the Skills settings.
2. Toggle the "Automatic Skill Selection" switch.

**Expected Results:**
- The "Selection Mode" section shows a setting called "Automatic Skill Selection" with description: "Let AI choose the best skill for each request. When disabled, you'll select skills manually."
- The toggle switch enables or disables automatic skill selection.
- The current state is persisted.

**Notes:**
- None.

---

### TC-10.4: Reload Skills from Filesystem

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Skills settings tab.

**Steps:**
1. Click the "Refresh" button in the header area (has a rotating arrows icon).
2. Observe the loading state.

**Expected Results:**
- The Refresh button icon spins while loading.
- After completion, the skill list is refreshed with the latest skills from the filesystem.
- Any newly added skill files appear in the list.
- Any deleted skill files are removed from the list.

**Notes:**
- To fully test, manually add or remove a `.md` skill file from the skills directory before refreshing.

---

### TC-10.5: Delete Skill

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Skills are listed. This test is performed from the Prompt Generator page library section.

**Steps:**
1. Navigate to the Prompt Generator page.
2. In the Library section, find a filesystem skill under "Installed Skills".
3. Click the trash icon button next to the skill.
4. In the confirmation dialog, click "Delete".

**Expected Results:**
- A confirmation dialog appears: "Delete Skill" with the message "Are you sure you want to delete '[skill name]'? This will remove the file from disk and cannot be undone."
- Clicking "Cancel" closes the dialog without deleting.
- Clicking "Delete" (red button) deletes the skill file from disk.
- A success toast appears: "Deleted '[skill name]'".
- The skill disappears from the library list.

**Notes:**
- This action is irreversible. The skill file is permanently deleted from the filesystem.

---

### TC-10.6: Skill Auto-Selection

**Priority:** P3
**Status:** [ ] Not Tested

**Preconditions:**
- At least one skill is enabled.
- Automatic Skill Selection is enabled (TC-10.3).

**Steps:**
1. Navigate to the Chat page.
2. Send a message that matches a skill's domain (e.g., a security-related question if a security skill is enabled).
3. Observe if the response uses the skill's prompt template.

**Expected Results:**
- When auto-selection is enabled, the system automatically selects the most relevant skill for each request.
- The response may reflect the skill's specialized prompting.

**Notes:**
- This is difficult to verify visually. Check logs for skill selection events.

---

## 11. Prompt Generator (P2)

### TC-11.1: Generate Skill Artifact

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to the Prompt Generator page.
- At least one AI provider is configured with a valid API key.

**Steps:**
1. Click "Prompt Generator" in the sidebar.
2. Observe the artifact type selector pills at the top.
3. Click the "Skill" pill (should be selected by default).
4. Enter "code-reviewer" in the "Name" field.
5. Enter "A skill that reviews code for best practices, potential bugs, and performance issues" in the "Describe what you want" textarea.
6. Click the "Generate" button.
7. Wait for generation to complete.

**Expected Results:**
- The page header reads "Generate Prompt" with description "Generate AI prompt templates for skills, agents, souls, and more."
- Five artifact type pills are displayed: Skill, Agent, Soul, Claude Skill, Generic.
- The "Skill" pill is highlighted with a primary border and background.
- After filling in the form and clicking "Generate":
  - The button changes to show a spinning loader and "Generating..." text.
  - The "Generate" button is disabled during generation.
  - In the context panel, a pulsing dot with "Generating..." text appears.
- After generation completes:
  - An output panel appears with heading "Generated Prompt".
  - The generated content is displayed in an editable textarea (monospace font, ~16 rows).
  - Action buttons appear: "Save", "Copy", "Clear", and "Regenerate".
  - The "Regenerate" button allows re-generating with the same inputs.

**Notes:**
- Generation requires a working AI provider. If none is configured, an error will appear.

---

### TC-11.2: Generate Agent Artifact

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Prompt Generator page.

**Steps:**
1. Click the "Agent" pill in the artifact type selector.
2. Enter "research-assistant" in the Name field.
3. Enter "An autonomous research agent that can search, summarize, and cite sources" in the description.
4. Click "Generate".

**Expected Results:**
- The "Agent" pill is highlighted.
- Generation proceeds and produces an agent-style system prompt.
- The context panel shows "Type: Agent" with description "Autonomous agent system prompt."

**Notes:**
- None.

---

### TC-11.3: Generate Soul Artifact

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Prompt Generator page.

**Steps:**
1. Click the "Soul" pill in the artifact type selector.
2. Enter "friendly-mentor" in the Name field.
3. Enter "A warm, encouraging personality that guides users through learning" in the description.
4. Click "Generate".

**Expected Results:**
- The "Soul" pill is highlighted.
- Generation produces a character/personality definition.
- The context panel shows "Type: Soul" with description "Character and personality definition."

**Notes:**
- None.

---

### TC-11.4: Edit Generated Content

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- A prompt has been generated (TC-11.1 through TC-11.3).

**Steps:**
1. In the output panel, click into the generated content textarea.
2. Make edits to the content.
3. Observe the "Save" button state.

**Expected Results:**
- The textarea is editable.
- After making changes, a "Save" button with a checkmark icon appears (indicating unsaved changes / dirty state).
- The content can be freely modified.

**Notes:**
- None.

---

### TC-11.5: Save to Disk

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- A prompt has been generated and optionally edited (TC-11.4).

**Steps:**
1. Click the "Save" button in the output panel.
2. Observe the confirmation.

**Expected Results:**
- The content is saved.
- A confirmation line appears below the textarea showing a green checkmark icon and "Saved to [file path]" indicating the disk location.
- The "Save" button is no longer shown (content matches saved state).
- The context panel shows "Last Generated" with the artifact name and disk path.
- The saved artifact appears in the Library section below.

**Notes:**
- None.

---

### TC-11.6: Library Browse and Filter

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- At least one artifact has been saved, or filesystem skills exist.

**Steps:**
1. Scroll down to the "Library" section on the Prompt Generator page.
2. Observe the tab bar: All, Skill, Agent, Soul, Claude Skill, Generic.
3. Click different tabs to filter.
4. Observe the "Installed Skills" and "Generated" sub-sections.

**Expected Results:**
- The Library section shows a tab bar for filtering by artifact type.
- The "All" tab shows all artifacts and installed skills.
- Filtering tabs show only artifacts of that type.
- "Installed Skills" sub-section shows filesystem-loaded skills with edit (pencil) and delete (trash) buttons.
- "Generated" sub-section shows previously generated and saved artifacts with edit and delete buttons.
- An "Add [Type]" button is shown at the right end of the tab bar.
- Empty state shows "No [type]s yet." when no artifacts exist for a filtered type.

**Notes:**
- None.

---

### TC-11.7: Copy Artifact

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- A prompt has been generated.

**Steps:**
1. In the output panel, click the "Copy" button.
2. Paste the clipboard content somewhere to verify.

**Expected Results:**
- The generated content is copied to the system clipboard.
- The content can be pasted into any text editor or field.

**Notes:**
- The Copy button is disabled if there is no generated content.

---

### TC-11.8: Delete Artifact

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- At least one generated artifact exists in the Library.

**Steps:**
1. In the Library section, find a generated artifact (under "Generated" sub-section).
2. Click the trash icon button next to it.

**Expected Results:**
- The artifact is deleted from the library.
- The list updates to remove the deleted item.

**Notes:**
- For filesystem skills, a confirmation dialog is shown before deletion (see TC-10.5). For generated prompt artifacts, deletion may be immediate.

---

## 12. Modules (P2)

### TC-12.1: List Modules

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Modules tab.

**Steps:**
1. Click "Settings" in the sidebar.
2. Click "Modules" in the settings nav.
3. Observe the modules list.

**Expected Results:**
- The heading shows "Modules" with a "+ New Module" button on the right.
- If loading, an animated "Loading modules..." text appears.
- If no modules are installed, the message "No modules installed yet." is displayed.
- If modules exist, they are displayed as card-style buttons in a fixed-width column (~256px).
- Each module card shows:
  - A status dot (green for running, yellow pulsing for starting, red for error, gray for stopped).
  - Module name (bold text).
  - Type badge (e.g., "mcp" for MCP servers, "service" for services).
  - Runtime type badge (e.g., "node", "python").
  - The runtime command in monospace text (truncated).
  - A toggle switch (Start/Stop).

**Notes:**
- None.

---

### TC-12.2: Start Module

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- At least one module exists in the Modules list (TC-12.1).

**Steps:**
1. Find a module that is currently stopped (gray status dot, toggle off).
2. Click the toggle switch to start the module.

**Expected Results:**
- The toggle switch turns on.
- The status dot changes to yellow (pulsing) indicating "starting".
- After the module starts successfully, the status dot turns green.
- The module is now running its configured command.

**Notes:**
- The toggle switch stops propagation of click events to the card button, so clicking the toggle does not select/deselect the card.

---

### TC-12.3: Stop Module

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- A module is currently running (TC-12.2).

**Steps:**
1. Find the running module (green status dot, toggle on).
2. Click the toggle switch to stop the module.

**Expected Results:**
- The toggle switch turns off.
- The status dot changes to gray, indicating the module is stopped.

**Notes:**
- None.

---

### TC-12.4: Create New Module

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Modules settings tab.

**Steps:**
1. Click the "+ New Module" button.
2. Fill in the module scaffold form that appears (ModuleScaffold component).
3. Complete the creation process.

**Expected Results:**
- Clicking "+ New Module" opens a module scaffold dialog/form.
- The form allows configuring a new module with name, type, runtime settings, and command.
- After creation, the new module appears in the module list.
- The "+ New Module" button is disabled while the scaffold form is open.

**Notes:**
- The exact fields in the ModuleScaffold component may vary.

---

### TC-12.5: Module Status Indicators

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- At least one module exists.

**Steps:**
1. Observe the status dots across different module states.

**Expected Results:**
- **Green dot (static)**: Module is "running".
- **Yellow dot (pulsing)**: Module is "starting".
- **Red dot**: Module has "error".
- **Gray dot**: Module is "stopped".

**Notes:**
- None.

---

### TC-12.6: MCP Server Module

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- A module of type "mcp" exists.

**Steps:**
1. Click on an MCP server module card to select it.
2. Observe the detail panel that opens on the right.

**Expected Results:**
- Clicking a module card selects it (highlighted with primary border).
- A detail panel opens to the right showing "Module Details" heading with a close button.
- The ModuleDetail component displays extended information about the module.
- Clicking the same card again deselects it and closes the detail panel.

**Notes:**
- The detail panel shows additional module information provided by the ModuleDetail component.

---

## 13. Log Viewer (P1)

### TC-13.1: View Logs

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- The application has been running and generating log entries.

**Steps:**
1. Click "Logs" in the sidebar.
2. Observe the log viewer page.

**Expected Results:**
- The page header reads "Logs" with description "Application log viewer - recent entries from the current session".
- A toolbar is displayed with: level filter buttons, search input, clear button, auto-refresh toggle, and manual refresh button.
- Below the toolbar, a table displays log entries with columns:
  - **Time**: Shortened timestamp (time portion only, e.g., "12:34:56.789").
  - **Level**: Colored badge (red for ERROR, yellow outline for WARN, gray for INFO/DEBUG/TRACE).
  - **Target**: The log target/module (hidden on small screens).
  - **Message**: The log message text, colored by level.
- Entries are sorted in ascending order (newest at the bottom).
- A footer shows "[filtered count] / [total count] entries" with a "live" indicator if auto-refresh is on.
- The table headers are sticky (visible when scrolling).

**Notes:**
- The log viewer loads up to 5,000 recent entries from the current session.

---

### TC-13.2: Filter by Level

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- On the Logs page with entries visible.

**Steps:**
1. Observe the level filter buttons in the toolbar: ALL, TRACE, DEBUG, INFO, WARN, ERROR.
2. Note the count numbers next to each level (except ALL).
3. Click "ERROR" to filter to only error entries.
4. Click "WARN" to filter to only warning entries.
5. Click "ALL" to reset the filter.

**Expected Results:**
- The currently active level button is highlighted with a primary background color.
- Each level button shows the count of entries at that level (e.g., "ERROR 3").
- Clicking a level button filters the table to show only entries of that level.
- Clicking "ALL" shows all entries regardless of level.
- The footer updates the filtered count (e.g., "3 / 150 entries").
- ERROR rows have a subtle red background tint.
- WARN rows have a subtle yellow background tint.

**Notes:**
- Filter buttons have `aria-pressed` attributes for accessibility.

---

### TC-13.3: Search Logs

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- On the Logs page with entries visible.

**Steps:**
1. Click into the search input field (has a search icon and placeholder "Search messages...").
2. Type a search term (e.g., "heartbeat" or "error").
3. Observe the filtered results.
4. Click the "X" button inside the search field to clear the search.

**Expected Results:**
- As you type, the log table filters in real-time to show only entries where the message, target, or timestamp contains the search term (case-insensitive).
- The footer updates the filtered count.
- An "X" clear button appears inside the search input when text is present.
- Clicking "X" clears the search and shows all entries again.
- Search filtering works in combination with the level filter (both are applied simultaneously).

**Notes:**
- The clear button has an aria-label of "Clear search" for accessibility.

---

### TC-13.4: Live Tail Mode

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- On the Logs page.

**Steps:**
1. Observe the auto-refresh toggle button in the toolbar (Pause/Play icon).
2. The button should show a Pause icon (meaning auto-refresh is active by default).
3. Observe the footer which shows "live" text when auto-refresh is on.
4. Wait a few seconds and observe new entries appearing automatically.

**Expected Results:**
- Auto-refresh is enabled by default.
- The toggle button shows a Pause icon with a primary background when auto-refresh is active.
- The footer shows "[count] entries . live" text.
- New log entries appear automatically every 2 seconds (the refresh interval).
- When scrolled to the bottom, the view auto-scrolls to keep showing the latest entries.

**Notes:**
- Auto-refresh polls the backend every 2 seconds (`AUTO_REFRESH_INTERVAL_MS = 2000`).

---

### TC-13.5: Pause/Resume Auto-Refresh

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- On the Logs page with auto-refresh active.

**Steps:**
1. Click the auto-refresh toggle button (Pause icon) to pause.
2. Observe the button and footer changes.
3. Click the button again (Play icon) to resume.

**Expected Results:**
- Clicking Pause:
  - The button changes to show a Play icon with a ghost (outline) style.
  - The "live" indicator disappears from the footer.
  - New entries stop appearing automatically.
  - The button tooltip/aria-label changes to "Resume auto-refresh".
- Clicking Play:
  - The button changes back to Pause icon with primary background.
  - The "live" indicator reappears in the footer.
  - Auto-refresh resumes.
  - The button tooltip/aria-label changes to "Pause auto-refresh".

**Notes:**
- Pausing is useful when you want to examine a specific log entry without the view updating.

---

### TC-13.6: Scroll-to-Bottom

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Logs page with enough entries to require scrolling.

**Steps:**
1. Scroll up in the log table away from the bottom.
2. Observe the scroll-to-bottom button that appears.
3. Click the scroll-to-bottom button.

**Expected Results:**
- When not scrolled to the bottom (more than 80px from the bottom), a floating circular button appears in the bottom-right of the log table.
- The button shows a down-arrow icon with a semi-transparent background and blur effect.
- Clicking the button smoothly scrolls to the bottom of the log table.
- Once at the bottom, the button disappears.
- The button has an aria-label of "Scroll to latest logs".

**Notes:**
- Auto-scroll only triggers when you are already at the bottom. Scrolling up pauses auto-scroll until the button is clicked.

---

### TC-13.7: Clear Logs

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Logs page with entries visible.

**Steps:**
1. Click the clear button (trash icon) in the toolbar.

**Expected Results:**
- All log entries are removed from the display.
- The table shows the empty state: "No log entries match the current filter."
- The footer shows "0 / 0 entries".
- The clear button becomes disabled (no entries to clear).
- New entries will appear again on the next auto-refresh cycle (if auto-refresh is active).

**Notes:**
- This only clears the in-memory display. It does not delete log files from disk. The logs will repopulate on the next refresh.

---

### TC-13.8: Manual Refresh

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On the Logs page.

**Steps:**
1. Click the manual refresh button (circular arrows icon) in the toolbar.

**Expected Results:**
- The refresh icon spins while loading.
- The log table is refreshed with the latest entries from the backend.
- The button is disabled during the loading process to prevent double-clicks.

**Notes:**
- None.

---

## 14. App Settings (P2)

### TC-14.1: Change Theme (Light/Dark/Auto)

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > App Settings tab.

**Steps:**
1. Click "Settings" in the sidebar.
2. Click "App Settings" in the settings nav.
3. Locate the "Appearance" section.
4. Find the "Theme" setting row with a dropdown.
5. Select "Light" from the dropdown.
6. Observe the UI change.
7. Select "Dark" from the dropdown.
8. Observe the UI change.
9. Select "Auto" (or "System") from the dropdown.

**Expected Results:**
- The "Appearance" section has a heading and description.
- The Theme dropdown shows options (typically: Light, Dark, Auto/System).
- Selecting "Light" immediately switches the entire app to light theme (white backgrounds, dark text).
- Selecting "Dark" immediately switches to dark theme (dark backgrounds, light text).
- Selecting "Auto" follows the operating system's theme preference.
- Changes auto-save (no explicit save button needed).

**Notes:**
- The settings tip in the context panel reads "Changes auto-save as you update settings."

---

### TC-14.2: Change Language

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On Settings > App Settings tab.

**Steps:**
1. Locate the "Language" setting row in the Appearance section.
2. Select a different language from the dropdown.

**Expected Results:**
- The language dropdown lists available language options.
- Selecting a language changes the app's interface language.
- Labels, descriptions, and other translatable text update to the selected language.
- The language preference is persisted to the backend settings.

**Notes:**
- Not all UI strings may be translated. Some may remain in English.

---

### TC-14.3: Toggle System Tray

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On Settings > App Settings tab.

**Steps:**
1. Locate the "Behavior" section.
2. Find the "Show in System Tray" toggle switch.
3. Toggle it on.
4. Observe the system tray area.
5. Toggle it off.

**Expected Results:**
- The "Show in Tray" toggle controls whether the app icon appears in the OS system tray/menu bar.
- When enabled, the app icon appears in the system tray.
- When disabled, the app icon is removed from the system tray.
- The setting persists across restarts.

**Notes:**
- System tray behavior is platform-specific (macOS menu bar, Windows system tray, Linux tray).

---

### TC-14.4: Toggle Launch at Login

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On Settings > App Settings tab.

**Steps:**
1. Locate the "Launch at Login" toggle switch in the Behavior section.
2. Toggle it on.
3. Toggle it off.

**Expected Results:**
- The toggle controls whether MesoClaw starts automatically when the user logs in.
- Toggling on registers the app for autostart.
- Toggling off removes the autostart registration.

**Notes:**
- Autostart behavior is OS-dependent. On some platforms, it may require additional permissions.

---

### TC-14.5: Configure Notifications

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- On Settings > App Settings tab.

**Steps:**
1. Locate the "Notifications" section.
2. Find the "Enable Notifications" toggle switch.
3. Toggle it on.
4. Observe the additional notification settings that become enabled.
5. Toggle it off.
6. Observe the additional settings become disabled.

**Expected Results:**
- The "Notifications" section contains a master toggle "Enable Notifications" with description "Enable or disable all notifications".
- When the master toggle is OFF, all sub-toggles (DND, per-category) are disabled (grayed out, not clickable).
- When the master toggle is ON, sub-toggles become interactive.

**Notes:**
- None.

---

### TC-14.6: Set DND Schedule

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Notifications are enabled (TC-14.5).

**Steps:**
1. In the Notifications section, find the "Enable DND schedule" toggle.
2. Toggle it on.
3. Set "DND start hour" to 22 (10 PM).
4. Set "DND end hour" to 7 (7 AM).

**Expected Results:**
- The "Enable DND schedule" toggle has description: "Automatically suppress notifications during the hours below."
- Below it, two number inputs appear:
  - "DND start hour" (range 0-23, default: 22). Description: "Hour (0-23) when Do Not Disturb begins. Default: 22 (10 pm)."
  - "DND end hour" (range 0-23, default: 7). Description: "Hour (0-23) when Do Not Disturb ends. Default: 7 (7 am)."
- These inputs are disabled when DND schedule is toggled off, or when notifications are disabled entirely.
- Changes auto-save.

**Notes:**
- None.

---

### TC-14.7: Per-Category Notification Toggles

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- Notifications are enabled (TC-14.5).

**Steps:**
1. In the Notifications section, locate the per-category toggles.
2. Toggle each category on and off.

**Expected Results:**
- Four per-category notification toggles are available:
  - **Heartbeat**: "Show a notification on each heartbeat tick."
  - **Cron reminders**: "Show a notification when a scheduled job fires."
  - **Agent complete**: "Show a notification when an agent task finishes."
  - **Approval requests**: "Show a notification when an action requires your approval."
- Each toggle can be independently enabled/disabled.
- All per-category toggles are disabled when the master "Enable Notifications" toggle is off.
- Changes auto-save.

**Notes:**
- None.

---

### TC-14.8: Toggle Sidebar Expanded

**Priority:** P3
**Status:** [ ] Not Tested

**Preconditions:**
- On Settings > App Settings tab.

**Steps:**
1. In the Appearance section, find the "Sidebar expanded" toggle.
2. Toggle it on and off.
3. Observe the sidebar behavior.

**Expected Results:**
- The toggle controls whether the sidebar starts in expanded or collapsed mode.
- When toggled, the sidebar immediately expands or collapses.

**Notes:**
- None.

---

### TC-14.9: Developer Settings (Logging)

**Priority:** P3
**Status:** [ ] Not Tested

**Preconditions:**
- On Settings > App Settings tab.

**Steps:**
1. Locate the "Developer" section.
2. Toggle "Enable Logging" on.
3. Change the "Log Level" dropdown.

**Expected Results:**
- The "Developer" section shows "Enable Logging" toggle and a "Log Level" dropdown.
- The Log Level dropdown offers options like TRACE, DEBUG, INFO, WARN, ERROR.
- Changing the log level affects the verbosity of logs displayed in the Log Viewer.
- The Log Level dropdown is disabled when logging is turned off.

**Notes:**
- None.

---

## 15. Advanced Settings (P3)

### TC-15.1: Toggle Caching

**Priority:** P3
**Status:** [ ] Not Tested

**Preconditions:**
- Navigate to Settings > Advanced tab.

**Steps:**
1. Click "Settings" in the sidebar.
2. Click "Advanced" in the settings nav.
3. Locate the "Caching" section.
4. Toggle the "Enable Caching" switch off.
5. Toggle it back on.
6. Adjust the "Cache Duration" number input.

**Expected Results:**
- The "Caching" section has heading "Caching" with description "Configure AI response caching to reduce API calls and improve performance".
- "Enable Caching" toggle with description "Cache AI responses to avoid redundant API calls".
- "Cache Duration" number input (range: 1-168, in hours) with "hours" label.
- Changes auto-save with a 500ms debounce.

**Notes:**
- None.

---

### TC-15.2: Adjust Temperature

**Priority:** P3
**Status:** [ ] Not Tested

**Preconditions:**
- On the Advanced settings tab.

**Steps:**
1. Locate the "Request Settings" section.
2. Find the "Temperature" number input.
3. Change the value (range: 0.0 to 1.0, step: 0.1).

**Expected Results:**
- The temperature input accepts values from 0 to 1 in 0.1 increments.
- Description: "Controls randomness in responses (0.0 - 1.0)".
- Changes auto-save with a 500ms debounce.

**Notes:**
- Lower temperature = more deterministic responses. Higher = more creative/random.

---

### TC-15.3: Set Max Tokens

**Priority:** P3
**Status:** [ ] Not Tested

**Preconditions:**
- On the Advanced settings tab.

**Steps:**
1. Find the "Max Tokens" number input in the Request Settings section.
2. Change the value (range: 256-32768, step: 256).

**Expected Results:**
- The input accepts values from 256 to 32768 in increments of 256.
- Description: "Maximum number of tokens in the response".
- Changes auto-save with debounce.

**Notes:**
- None.

---

### TC-15.4: Toggle Streaming

**Priority:** P3
**Status:** [ ] Not Tested

**Preconditions:**
- On the Advanced settings tab.

**Steps:**
1. Locate the "Advanced Options" section.
2. Find the "Stream Responses" toggle.
3. Toggle it off.
4. Navigate to Chat and send a message.
5. Observe the response behavior.
6. Toggle streaming back on.

**Expected Results:**
- "Stream Responses" toggle with description "Enable streaming for real-time response generation".
- When OFF, chat responses arrive all at once instead of streaming token-by-token.
- When ON, responses stream in real-time.
- Changes save immediately (toggle change).

**Notes:**
- None.

---

### TC-15.5: Enable Debug Mode

**Priority:** P3
**Status:** [ ] Not Tested

**Preconditions:**
- On the Advanced settings tab.

**Steps:**
1. Find the "Debug Mode" toggle in the Advanced Options section.
2. Toggle it on.
3. Send a chat message and check the Log Viewer.
4. Toggle it off.

**Expected Results:**
- "Debug Mode" toggle with description "Enable detailed logging for AI requests and responses".
- When enabled, additional debug-level log entries appear in the Log Viewer showing AI request/response details.
- Changes save immediately.

**Notes:**
- None.

---

### TC-15.6: Set Custom Base URL

**Priority:** P3
**Status:** [ ] Not Tested

**Preconditions:**
- On the Advanced settings tab.

**Steps:**
1. Find the "Custom Base URL" text input in the Advanced Options section.
2. Enter a custom URL (e.g., `https://api.proxy.example.com`).
3. Clear the input to reset.

**Expected Results:**
- "Custom Base URL" text input with placeholder "https://api.example.com".
- Description: "Use a custom base URL for API requests (overrides provider default)".
- Changes auto-save with debounce.
- When empty, the default provider base URL is used.

**Notes:**
- None.

---

### TC-15.7: Set Timeout

**Priority:** P3
**Status:** [ ] Not Tested

**Preconditions:**
- On the Advanced settings tab.

**Steps:**
1. Find the "Timeout" number input in the Request Settings section.
2. Change the value (range: 5-300 seconds).

**Expected Results:**
- "Timeout" number input (range: 5-300) with "seconds" label.
- Description: "Request timeout in seconds".
- Changes auto-save with debounce.

**Notes:**
- None.

---

## 16. Cross-Cutting Concerns

### TC-16.1: Dark Mode Consistency

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- The app is set to Dark theme (TC-14.1).

**Steps:**
1. Navigate through every page: Home, Chat, Memory, Channels, Prompt Generator, Logs, Settings (all tabs).
2. On each page, verify that all text is readable and all UI elements have appropriate dark-mode styling.

**Expected Results:**
- All pages use dark backgrounds with light text.
- No white/bright flash when navigating between pages.
- All form inputs, buttons, cards, badges, and dialogs use dark-mode colors.
- Status indicators (dots, badges) maintain their semantic colors (green, yellow, red) in dark mode.
- Code/monospace text is readable.
- Borders and dividers use appropriate muted colors.

**Notes:**
- Pay special attention to: chat message bubbles, log viewer table, settings sections, channel cards, module cards.

---

### TC-16.2: Responsive Layout (Resize Window)

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- The app is running.

**Steps:**
1. Start with the app window at full desktop width (1280px+).
2. Observe the three-column layout: Sidebar | Main Content | Context Panel.
3. Resize the window to below 1280px (xl breakpoint).
4. Observe the context panel behavior.
5. Resize the window to below 768px (md breakpoint).
6. Observe the sidebar and navigation changes.
7. Resize back to full width.

**Expected Results:**
- **xl+ (1280px+)**: Three-column layout — Sidebar (256px) | Main Content (flex) | Context Panel (320px).
- **md-xl (768-1279px)**: Two-column layout — Sidebar (256px) | Main Content (flex). Context panel is hidden.
- **Below md (<768px)**: Single-column layout. Sidebar becomes a floating drawer. A bottom MobileNav bar appears with navigation icons. Content takes full width.
- The layout transitions smoothly without content jumping or overlapping.

**Notes:**
- The sidebar supports swipe gestures on mobile (swipe right from left edge to open, swipe left to close).

---

### TC-16.3: Navigation Between All Pages

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- The app is running with onboarding completed.

**Steps:**
1. Using the sidebar (desktop) or bottom nav (mobile), navigate to each page in sequence:
   - Home (/)
   - Chat (/chat)
   - Memory (/memory)
   - Channels (/channels)
   - Prompt Generator (/prompt-generator)
   - Logs (/logs)
   - Settings (/settings)
2. On the Settings page, click through all 9 tabs in the left nav.

**Expected Results:**
- All pages load without errors.
- The sidebar highlights the currently active page.
- Each page shows its header with the correct title and description.
- Navigation is fast (no noticeable loading delays for route changes).
- The URL updates correctly for each page.
- The back/forward browser history works correctly.

**Notes:**
- Check the browser console for any JavaScript errors during navigation.

---

### TC-16.4: Sidebar Collapse/Expand

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- App is at desktop width (md+ breakpoint).

**Steps:**
1. Locate the sidebar on the left side.
2. Find the collapse/expand trigger.
3. Click to collapse the sidebar.
4. Click to expand the sidebar.

**Expected Results:**
- When collapsed, the sidebar shows only icons (no text labels).
- When expanded, the sidebar shows icons with text labels.
- The main content area adjusts its width accordingly.
- The collapse/expand state is smooth (animated transition).
- Navigation still works correctly in collapsed mode.

**Notes:**
- The sidebar expanded state can also be controlled via Settings > App Settings > "Sidebar expanded" toggle.

---

### TC-16.5: Context Panel Updates

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- App window is at xl+ width (1280px+) so the context panel is visible.

**Steps:**
1. Navigate to each page and observe the context panel content on the right side.

**Expected Results:**
- **Home**: No specific context panel content (may show default or empty).
- **Chat**: Shows Active Model info, Session message counts (You/AI), streaming indicator, and "Clear conversation" button.
- **Memory**: Shows "Search Tips" when on Search tab, or "Today" date and browsing hint when on Timeline tab.
- **Channels**: Shows selected Channel name, message count, sender list, and "Replying To" recipient.
- **Prompt Generator**: Shows artifact Type and description, Library artifact count, generating indicator, and last generated artifact name/path.
- **Settings**: Shows "Current Section" label and description, plus a tip: "Changes auto-save as you update settings."
- Context panel content updates when switching tabs or interacting with the page.

**Notes:**
- The context panel is only visible on xl+ screens (1280px or wider).

---

### TC-16.6: Toast Notifications

**Priority:** P2
**Status:** [ ] Not Tested

**Preconditions:**
- The app is running.

**Steps:**
1. Trigger various actions that produce toast notifications:
   - Save an API key (success toast).
   - Test a connection with an invalid key (error toast).
   - Switch models in Chat (success toast).
   - Generate a prompt with no AI provider (error toast).

**Expected Results:**
- Success toasts appear with green/positive styling.
- Error toasts appear with red/destructive styling and include a description with details.
- Toasts appear in a consistent position (typically top-right or bottom-right).
- Success toasts auto-dismiss after ~3 seconds.
- Error toasts auto-dismiss after ~5 seconds.
- Multiple toasts stack without overlapping.

**Notes:**
- The app uses the `sonner` toast library.

---

### TC-16.7: Error Messages

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- The app is running.

**Steps:**
1. Trigger various error conditions:
   - Attempt to send a chat message with no API key configured.
   - Attempt to test a channel connection with an invalid token.
   - Navigate to Settings > AI Provider and observe behavior with no providers loaded.

**Expected Results:**
- Error messages are user-friendly (not raw stack traces or technical jargon).
- Error toasts include a main message and a descriptive sub-message.
- In-page errors use red/destructive coloring.
- Error states do not crash the application.
- The user can recover from errors (e.g., fix the configuration and retry).

**Notes:**
- None.

---

### TC-16.8: App Restart Persistence

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- The app has been configured with: a theme, an AI provider with API key, at least one channel, and at least one scheduled job.

**Steps:**
1. Note the current state of all settings.
2. Completely close the application.
3. Reopen the application.
4. Navigate through Settings and verify all configurations.

**Expected Results:**
- Onboarding is not shown again (onboarding completed state is persisted).
- Theme setting persists.
- AI provider configuration persists (provider shows "Configured" status).
- API keys persist in the OS keyring.
- Channel configurations persist.
- Scheduled jobs persist with their enabled/disabled states.
- Advanced settings (temperature, max tokens, etc.) persist.
- Sidebar expanded/collapsed state persists.
- Language preference persists.

**Notes:**
- None.

---

## 17. Security Testing

### TC-17.1: API Keys Not in Logs

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- At least one AI provider is configured with an API key.
- Debug mode is enabled (TC-15.5).

**Steps:**
1. Enable debug mode in Advanced Settings.
2. Send a chat message to trigger an API call.
3. Navigate to the Log Viewer page.
4. Search for parts of your API key (e.g., first 8 characters after "sk-").
5. Search for common key patterns: "sk-", "xoxb-", "xapp-", "Bearer".

**Expected Results:**
- No full API key values appear in any log entries.
- Log entries may reference the provider name but should not contain the actual key.
- Searching for key prefixes should not reveal the complete key.

**Notes:**
- This is a critical security test. API keys must never be written to log files.

---

### TC-17.2: Credentials in OS Keyring

**Priority:** P0
**Status:** [ ] Not Tested

**Preconditions:**
- At least one API key has been saved.

**Steps:**
1. Search the app's local data directory for files containing API key values.
2. Check the SQLite database file for API key values.
3. Verify that keys are stored in the OS keyring:
   - **macOS**: Open "Keychain Access" app and search for "mesoclaw" or the app's keychain service name.
   - **Linux**: Run `secret-tool search --all service mesoclaw` or check with Seahorse.
   - **Windows**: Open "Credential Manager" and look under "Generic Credentials" for mesoclaw entries.

**Expected Results:**
- No API key values are found in any local files, databases, or configuration files.
- API keys are found in the OS keyring under the app's service name.
- The keyring entries are properly associated with the provider IDs.

**Notes:**
- The app uses the `keyring` Rust crate for OS keyring integration and `zeroize` crate for sensitive data cleanup.

---

### TC-17.3: Channel Tokens Not Exposed in UI

**Priority:** P1
**Status:** [ ] Not Tested

**Preconditions:**
- At least one channel is configured with a token (TC-7.1 through TC-7.4).

**Steps:**
1. Navigate to Settings > Channels.
2. Expand each configured channel's configuration panel.
3. Observe the token/key fields.
4. Right-click the page and select "Inspect" to open browser dev tools.
5. Check the DOM for any exposed token values.

**Expected Results:**
- All token/key fields use `type="password"` input fields, displaying dots instead of the actual value.
- In the DOM, the input element's value is the actual token (this is expected for password fields) but it is not visible in the rendered UI.
- Token values are not displayed in any status text, error messages, or tooltips.
- The helper text for each token field mentions "Stored securely in the OS keyring."

**Notes:**
- Password input fields inherently store the value in the DOM. The key point is that the value is not visually displayed to the user without explicitly toggling the eye icon (where available).

---

## Test Results Summary

### Overall Progress

| Section | Total | Passed | Failed | Partial | Not Tested |
|---------|-------|--------|--------|---------|------------|
| 1. Onboarding Flow | 4 | | | | 4 |
| 2. Home Dashboard | 4 | | | | 4 |
| 3. AI Chat | 7 | | | | 7 |
| 4. AI Provider Configuration | 10 | | | | 10 |
| 5. Scheduler | 10 | | | | 10 |
| 6. Heartbeat Monitoring | 6 | | | | 6 |
| 7. Messaging Channels | 11 | | | | 11 |
| 8. Memory | 4 | | | | 4 |
| 9. Identity | 6 | | | | 6 |
| 10. Skills | 6 | | | | 6 |
| 11. Prompt Generator | 8 | | | | 8 |
| 12. Modules | 6 | | | | 6 |
| 13. Log Viewer | 8 | | | | 8 |
| 14. App Settings | 9 | | | | 9 |
| 15. Advanced Settings | 7 | | | | 7 |
| 16. Cross-Cutting | 8 | | | | 8 |
| 17. Security | 3 | | | | 3 |
| **TOTAL** | **117** | | | | **117** |

### Test Run Information

| Field | Value |
|-------|-------|
| **Tester Name** | |
| **Date** | |
| **App Version** | |
| **Build Type** | Dev / Production |
| **Operating System** | |
| **Screen Resolution** | |

### P0 Test Summary (Critical)

| Test ID | Title | Status | Notes |
|---------|-------|--------|-------|
| TC-1.1 | Fresh Install Welcome Screen | [ ] | |
| TC-1.2 | AI Provider Setup | [ ] | |
| TC-1.3 | Channel Setup | [ ] | |
| TC-1.4 | Complete Onboarding | [ ] | |
| TC-3.1 | Send Message and Receive Response | [ ] | |
| TC-3.6 | Error Handling (No API Key) | [ ] | |
| TC-4.1 | Add OpenAI Provider | [ ] | |
| TC-4.7 | Test Connection | [ ] | |
| TC-4.8 | Set Default Model | [ ] | |
| TC-4.10 | API Key Security | [ ] | |
| TC-17.1 | API Keys Not in Logs | [ ] | |
| TC-17.2 | Credentials in OS Keyring | [ ] | |

### Blockers and Critical Issues

| Issue # | Test ID | Description | Severity | Status |
|---------|---------|-------------|----------|--------|
| | | | | |

### Notes and Observations

_Record any general observations, usability concerns, or improvement suggestions here._
