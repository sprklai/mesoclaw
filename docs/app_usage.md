# MesoClaw User Guide

A comprehensive guide to using MesoClaw, your AI-powered desktop application for multi-provider LLM chat, scheduled agent tasks, messaging channel integrations, semantic memory, and more.

---

## Table of Contents

- [1. Getting Started](#1-getting-started)
  - [1.1 System Requirements](#11-system-requirements)
  - [1.2 Installation](#12-installation)
  - [1.3 First Launch & Onboarding](#13-first-launch--onboarding)
- [2. Home Dashboard](#2-home-dashboard)
- [3. AI Chat](#3-ai-chat)
  - [3.1 Starting a Conversation](#31-starting-a-conversation)
  - [3.2 Selecting an AI Model](#32-selecting-an-ai-model)
  - [3.3 Streaming Responses](#33-streaming-responses)
  - [3.4 Skill Suggestions](#34-skill-suggestions)
- [4. AI Provider Configuration](#4-ai-provider-configuration)
  - [4.1 Supported Providers](#41-supported-providers)
  - [4.2 Adding an API Key](#42-adding-an-api-key)
  - [4.3 Testing Connection](#43-testing-connection)
  - [4.4 Setting Default Model](#44-setting-default-model)
  - [4.5 Custom / User-Defined Providers](#45-custom--user-defined-providers)
  - [4.6 Ollama (Local Models)](#46-ollama-local-models)
- [5. Scheduler](#5-scheduler)
  - [5.1 Creating a Scheduled Job](#51-creating-a-scheduled-job)
  - [5.2 Schedule Types](#52-schedule-types)
  - [5.3 Job Payload Types](#53-job-payload-types)
  - [5.4 Managing Jobs](#54-managing-jobs)
  - [5.5 Viewing Execution History](#55-viewing-execution-history)
  - [5.6 Error Handling & Backoff](#56-error-handling--backoff)
- [6. Heartbeat Monitoring](#6-heartbeat-monitoring)
  - [6.1 What is Heartbeat?](#61-what-is-heartbeat)
  - [6.2 Creating HEARTBEAT.md](#62-creating-heartbeatmd)
  - [6.3 Setting Up a Heartbeat Job](#63-setting-up-a-heartbeat-job)
  - [6.4 Active Hours](#64-active-hours)
  - [6.5 Viewing Heartbeat Results](#65-viewing-heartbeat-results)
  - [6.6 Troubleshooting Heartbeat Failures](#66-troubleshooting-heartbeat-failures)
- [7. Messaging Channels](#7-messaging-channels)
  - [7.1 Supported Platforms](#71-supported-platforms)
  - [7.2 Setting Up Telegram](#72-setting-up-telegram)
  - [7.3 Setting Up Discord](#73-setting-up-discord)
  - [7.4 Setting Up Slack](#74-setting-up-slack)
  - [7.5 Setting Up Matrix](#75-setting-up-matrix)
  - [7.6 Testing Connection](#76-testing-connection)
  - [7.7 Using the Channels Page](#77-using-the-channels-page)
  - [7.8 Sending & Receiving Messages](#78-sending--receiving-messages)
- [8. Memory](#8-memory)
  - [8.1 Semantic Search](#81-semantic-search)
  - [8.2 Daily Timeline](#82-daily-timeline)
  - [8.3 Memory Categories](#83-memory-categories)
- [9. Identity (Agent Personality)](#9-identity-agent-personality)
  - [9.1 Understanding Identity Files](#91-understanding-identity-files)
  - [9.2 Editing Identity](#92-editing-identity)
  - [9.3 Hot-Reload](#93-hot-reload)
- [10. Skills (Prompt Templates)](#10-skills-prompt-templates)
  - [10.1 What are Skills?](#101-what-are-skills)
  - [10.2 Managing Skills](#102-managing-skills)
  - [10.3 Skill Auto-Selection](#103-skill-auto-selection)
  - [10.4 Creating Custom Skills](#104-creating-custom-skills)
- [11. Prompt Generator](#11-prompt-generator)
  - [11.1 Artifact Types](#111-artifact-types)
  - [11.2 Generating Content](#112-generating-content)
  - [11.3 Library Management](#113-library-management)
- [12. Modules (Extensions)](#12-modules-extensions)
  - [12.1 What are Modules?](#121-what-are-modules)
  - [12.2 Module Types](#122-module-types)
  - [12.3 Creating a Module](#123-creating-a-module)
  - [12.4 Starting & Stopping Modules](#124-starting--stopping-modules)
  - [12.5 MCP Server Modules](#125-mcp-server-modules)
- [13. Logs](#13-logs)
  - [13.1 Viewing Logs](#131-viewing-logs)
  - [13.2 Filtering by Level](#132-filtering-by-level)
  - [13.3 Searching Logs](#133-searching-logs)
  - [13.4 Live Tail Mode](#134-live-tail-mode)
- [14. App Settings](#14-app-settings)
  - [14.1 Appearance](#141-appearance)
  - [14.2 Behavior](#142-behavior)
  - [14.3 Notifications & DND](#143-notifications--dnd)
  - [14.4 Developer Options](#144-developer-options)
- [15. Advanced Settings](#15-advanced-settings)
- [16. Security & Privacy](#16-security--privacy)
  - [16.1 API Key Storage](#161-api-key-storage)
  - [16.2 Channel Credential Storage](#162-channel-credential-storage)
  - [16.3 Data Locations](#163-data-locations)
- [17. Troubleshooting](#17-troubleshooting)
  - [17.1 Common Issues](#171-common-issues)
  - [17.2 Gateway Status](#172-gateway-status)
  - [17.3 Channel Connection Issues](#173-channel-connection-issues)

---

## 1. Getting Started

### 1.1 System Requirements

MesoClaw runs as a native desktop application on the following platforms:

| Platform | Minimum Version |
|----------|----------------|
| **macOS** | macOS 11 (Big Sur) or later |
| **Windows** | Windows 10 (version 1803) or later |
| **Linux** | Ubuntu 20.04+, Fedora 36+, or equivalent with WebKitGTK |

**Additional requirements:**

- An active internet connection for cloud AI providers (OpenAI, Anthropic, Google AI, Groq)
- An API key from at least one supported AI provider (or a local Ollama installation)
- Approximately 150 MB of free disk space

### 1.2 Installation

1. Download the installer for your platform from the official release page.
2. Run the installer:
   - **macOS**: Open the `.dmg` file and drag MesoClaw to your Applications folder.
   - **Windows**: Run the `.msi` installer and follow the setup wizard.
   - **Linux**: Install the `.deb` or `.AppImage` package for your distribution.
3. Launch MesoClaw from your applications menu or desktop shortcut.

### 1.3 First Launch & Onboarding

When you launch MesoClaw for the first time, you are guided through a four-step onboarding wizard. A progress bar at the top of the screen shows which step you are on.

**Step 1: Welcome**

1. You see a welcome screen with the heading "Welcome to MesoClaw" and a brief description: "Your AI-powered desktop assistant."
2. The screen summarizes what will be set up: connecting an AI provider and adding a messaging channel.
3. Click the **Get started** button to proceed.

**Step 2: AI Provider Setup**

1. You see the heading "Set up AI" with a **Provider** dropdown listing all available providers (OpenAI, Anthropic, Google AI, Groq, Ollama, and others).
2. Select your preferred provider from the dropdown.
3. If the selected provider requires an API key, an **API Key** field appears with a password input. Enter your key (e.g., `sk-...` for OpenAI).
4. A note below the field confirms: "API key is stored securely in your OS keyring."
5. Click **Save & continue** to store your key and move on.

> **Tip:** If you do not have an API key yet, click **Skip for now** to configure it later in Settings.

**Step 3: Channels (Optional)**

1. You see the heading "Connect a channel (optional)" with cards for each messaging platform: Telegram, Discord, Matrix, and Slack.
2. Click a platform card to select it. The card highlights with a border.
3. If you select Telegram, a configuration panel expands below with:
   - **Bot Token** field (password input, placeholder `123456:ABC-DEF...`)
   - **Allowed Chat IDs** field (comma-separated list of Telegram chat IDs)
   - **Test Connection** button to verify your credentials
   - **Save** button to persist the configuration
4. Click **Skip for now** if you do not want to set up a channel yet.
5. Click **Continue** to proceed to the final step.

> **Note:** WhatsApp appears as a card with a "Use Matrix bridge" label, indicating it is available through the Matrix protocol.

**Step 4: Done**

1. A success screen displays "You're all set!" with a green check mark.
2. A message reminds you: "MesoClaw is ready. You can adjust all settings anytime from the Settings page."
3. Click **Open MesoClaw** to enter the main application.

After completing onboarding, you are taken to the Home dashboard.

---

## 2. Home Dashboard

The Home page is the first screen you see after onboarding. It provides a quick overview and shortcuts to the most-used features.

**What you see:**

- **Time-based greeting**: "Good morning," "Good afternoon," or "Good evening" followed by the app name, and today's date below it.
- **Gateway Status indicator** (top right): Shows whether the backend HTTP gateway is running. A green dot means the gateway is active; a gray or red dot means it is offline.
- **Quick Actions** grid with four cards:

| Card | Description | Navigates To |
|------|-------------|-------------|
| **New Chat** | Start a conversation | `/chat` |
| **Memory** | Search agent memory | `/memory` |
| **Channels** | View messaging inbox | `/channels` |
| **Settings** | Configure providers | `/settings` |

- **System Status** section at the bottom showing:
  - **AI Provider**: The currently configured provider name, or "Not configured" with a gray dot.
  - **Model**: The currently selected model name, or "Not selected" with a gray dot.

Click any Quick Action card to navigate to that page.

---

## 3. AI Chat

The Chat page (`/chat`) is the primary interface for conversing with AI models. It supports streaming responses, model switching, and contextual suggestions.

### 3.1 Starting a Conversation

1. Navigate to **Chat** from the sidebar or the Home dashboard "New Chat" card.
2. You see a centered empty state with a sparkle icon and the text "Start a conversation / Ask me anything!"
3. Below the conversation area, you see **suggestion pills** -- clickable prompts such as:
   - "Explain how to build a desktop app with Tauri"
   - "What are the benefits of using React 19?"
   - "How does TypeScript improve code quality?"
   - "Best practices for state management in React"
   - "Explain the concept of hooks in React"
4. Click a suggestion to populate the input field, or type your own message in the text area at the bottom.
5. Press **Enter** or click the **Send** button (arrow icon) to submit your message.

> **Tip:** Suggestions disappear after you send your first message to keep the conversation area clean.

### 3.2 Selecting an AI Model

The model selector is located in the footer of the prompt input area, to the left of the send button.

1. Click the **model button** (shows the provider logo and current model name, e.g., "gpt-4o").
2. A dialog opens with a **search field** ("Search models...") and a list of all configured models grouped by provider.
3. Type in the search field to filter models by name.
4. Click a model to select it. A check mark appears next to the active model.
5. The dialog closes and a toast notification confirms the switch (e.g., "Switched to gpt-4o").

> **Note:** Models are loaded from your configured providers in **Settings > AI Provider**. If no models appear, add models to your providers first.

### 3.3 Streaming Responses

When you send a message:

1. Your message appears immediately in the conversation as a user bubble.
2. An assistant message placeholder appears below with "..." while the stream initializes.
3. As tokens arrive from the AI provider, the assistant's response fills in progressively in real time.
4. A pulsing indicator appears in the context panel showing "Streaming..." while the response is being generated.
5. Once the response is complete, the streaming indicator disappears.

If streaming is interrupted by an error, a toast notification appears with the error message and the incomplete assistant message is removed.

> **Tip:** If the provider requires an API key and none is configured, you will see an error toast: "No API key found for [provider]. Please add an API key in Settings > AI Providers."

### 3.4 Skill Suggestions

Skills are prompt templates that enhance AI responses for specific tasks. When the context of your conversation matches a skill, suggestions may appear to help you select the right template.

The **context panel** (right side on wide screens) shows:
- **Active Model**: Provider logo, model name, and provider name.
- **Session stats**: Count of your messages and AI responses.
- **Clear conversation** button to reset the chat.

---

## 4. AI Provider Configuration

Access provider settings at **Settings > AI Provider**.

### 4.1 Supported Providers

MesoClaw supports the following provider categories:

| Category | Providers |
|----------|-----------|
| **AI Gateway** | Vercel AI Gateway, OpenRouter |
| **AI Providers** | OpenAI, Anthropic, Google AI, Groq |
| **Local** | Ollama |
| **User-Defined** | Any OpenAI-compatible API endpoint |

Each provider card shows its name, base URL, active status, and whether it requires an API key.

### 4.2 Adding an API Key

1. Go to **Settings > AI Provider**.
2. Locate the provider you want to configure (e.g., OpenAI).
3. Click the provider card to expand its configuration panel.
4. Enter your API key in the **API Key** field (password input).
5. The key is automatically saved to your operating system's secure keyring.

> **Important:** API keys are never stored in plain text. They are kept in your OS keyring:
> - **macOS**: Keychain
> - **Windows**: Windows Credential Manager
> - **Linux**: Secret Service (GNOME Keyring or KWallet)

### 4.3 Testing Connection

After entering an API key:

1. Click the **Test Connection** button on the provider configuration panel.
2. MesoClaw calls the provider's health-check API to verify the key is valid.
3. A success or failure message appears:
   - Green: "Connected successfully"
   - Red: "Connection failed" with error details

### 4.4 Setting Default Model

1. In the provider configuration panel, use the **Model** dropdown to select from available models.
2. The selected model becomes the default for new chat sessions.
3. You can override the default on a per-conversation basis using the model selector in the Chat page.

### 4.5 Custom / User-Defined Providers

To add a custom OpenAI-compatible provider:

1. Go to **Settings > AI Provider**.
2. Click **Add Provider** or the equivalent button.
3. Enter the provider details:
   - **Name**: A display name for the provider
   - **Base URL**: The API endpoint (e.g., `https://api.example.com/v1`)
   - **API Key**: Your authentication key
4. Add models manually by clicking **Add Custom Model** and entering the model ID and display name.
5. Save the configuration.

### 4.6 Ollama (Local Models)

Ollama lets you run AI models locally on your machine without an API key.

1. Install Ollama on your system (see [ollama.com](https://ollama.com)).
2. Pull a model: `ollama pull llama3` (or any supported model).
3. In MesoClaw, go to **Settings > AI Provider** and locate the **Ollama** provider.
4. Ollama does not require an API key. Its base URL defaults to `http://localhost:11434`.
5. Click **Refresh Models** to detect locally available models.
6. Select a model from the dropdown.

> **Tip:** Make sure the Ollama service is running before attempting to use local models. MesoClaw cannot start Ollama for you.

---

## 5. Scheduler

The Scheduler allows you to create automated jobs that run on a recurring schedule. Access it at **Settings > Scheduler**.

### 5.1 Creating a Scheduled Job

1. Navigate to **Settings > Scheduler**.
2. Click the **+ Add Job** button in the top right.
3. A creation form appears with the following fields:
   - **Name**: Enter a descriptive name (e.g., "Morning health check").
   - **Schedule**: Choose between **Interval** or **Cron** (radio buttons).
   - **Action**: Select the payload type from the dropdown.
   - Additional fields depending on the action type.
4. Click **Create Job** to submit.

### 5.2 Schedule Types

| Type | Description | Example |
|------|-------------|---------|
| **Interval** | Runs every N seconds | Every 300 seconds (5 minutes) |
| **Cron** | Standard 5-field cron expression | `0 * * * *` (every hour at minute 0) |

**Interval configuration:**

- Enter the number of seconds in the "Every (secs)" input field.
- The display shows a human-readable label (e.g., "every 5m", "every 1h").

**Cron configuration:**

- Use the visual **Cron Builder** to set minute, hour, day-of-month, month, and day-of-week fields.
- Or type a raw cron expression directly.

> **Tip:** Common cron examples:
> - `*/5 * * * *` -- every 5 minutes
> - `0 9 * * 1-5` -- 9 AM on weekdays
> - `0 0 * * *` -- midnight daily

### 5.3 Job Payload Types

When creating a job, select one of three action types from the **Action** dropdown:

| Payload Type | Label | Description |
|-------------|-------|-------------|
| **Heartbeat** | "Run Heartbeat checklist" | Reads items from your `HEARTBEAT.md` file and verifies each one using an AI agent. See [Section 6](#6-heartbeat-monitoring) for details. |
| **Agent Turn** | "Agent Turn (custom prompt)" | Runs an AI agent with a custom prompt you provide. A **Prompt** text field appears where you enter the instruction (e.g., "Summarise today's work"). |
| **Notify** | "Publish notification" | Publishes an event or notification message. A **Message** text field appears where you enter the notification text (e.g., "Reminder: check progress"). |

### 5.4 Managing Jobs

The Scheduler tab displays a table of all configured jobs with the following columns:

| Column | Description |
|--------|-------------|
| **Name** | The job's display name |
| **Schedule** | Human-readable schedule (e.g., "every 5m" or the cron expression) |
| **Action** | Payload summary (e.g., "Heartbeat", "Agent: Summarise today's...") |
| **Next run** | The next scheduled execution time |
| **Errors** | Red badge showing consecutive error count, if any |
| **Active** | Toggle switch to enable or disable the job |
| *(Actions)* | **Delete** button to remove the job permanently |

**To enable or disable a job:** Toggle the **Active** switch in the job's row.

**To delete a job:** Click the **Delete** button. The job is removed immediately.

### 5.5 Viewing Execution History

For each job, you can view the last 100 execution runs. Click on a job row to expand the execution history, which shows:

- Timestamp of each run
- Status (success or failure)
- Output or error message

### 5.6 Error Handling & Backoff

When a job fails, MesoClaw applies an exponential backoff strategy to avoid hammering failing services:

| Consecutive Failures | Backoff Delay |
|---------------------|---------------|
| 1 | 30 seconds |
| 2 | 60 seconds |
| 3 | 5 minutes |
| 4 | 15 minutes |
| 5+ | 1 hour |

**Stuck job detection:** Jobs that have been running for more than 120 seconds are flagged as stuck.

Jobs persist in the SQLite database and reload automatically when the application restarts.

---

## 6. Heartbeat Monitoring

### 6.1 What is Heartbeat?

Heartbeat is a special scheduler job type that performs automated health checks on your system. It reads a checklist from a `HEARTBEAT.md` file, passes each item to an AI agent for verification, and reports the results.

### 6.2 Creating HEARTBEAT.md

Create a file named `HEARTBEAT.md` in your MesoClaw configuration directory with checklist items using standard Markdown task syntax:

```markdown
- [ ] Check API is responding
- [ ] Verify database connection
- [ ] Confirm webhook endpoint is reachable
- [ ] Check disk space is above 10%
- [ ] Validate SSL certificate is not expiring soon
```

Each `- [ ]` item represents a health check that the AI agent will attempt to verify.

### 6.3 Setting Up a Heartbeat Job

1. Go to **Settings > Scheduler**.
2. Click **+ Add Job**.
3. Enter a name (e.g., "System Health Check").
4. Select a schedule:
   - **Interval**: e.g., every 3600 seconds (1 hour)
   - **Cron**: e.g., `0 */2 * * *` (every 2 hours)
5. Under **Action**, select **"Run Heartbeat checklist"** from the dropdown.
6. Click **Create Job**.

### 6.4 Active Hours

Heartbeat supports an `active_hours` configuration (e.g., 9-17) to restrict checks to working hours only. Outside of active hours, heartbeat jobs are silently skipped.

### 6.5 Viewing Heartbeat Results

1. Go to **Settings > Scheduler**.
2. Locate your heartbeat job in the jobs table.
3. Click the job to expand its execution history.
4. Each run shows:
   - **HEARTBEAT_OK**: All checklist items passed (silent success).
   - **HeartbeatAlert**: One or more items failed, with details about which checks did not pass.

### 6.6 Troubleshooting Heartbeat Failures

If a heartbeat check fails:

1. Review the execution history to identify which checklist item(s) failed.
2. Verify that the `HEARTBEAT.md` file exists and contains valid checklist items.
3. Ensure the AI provider is configured and has a valid API key (the heartbeat agent needs an AI provider to evaluate checks).
4. Check the Logs page for detailed error messages.
5. If failures persist, simplify the checklist items to isolate the issue.

---

## 7. Messaging Channels

Channels allow your AI agent to communicate through external messaging platforms. Configure channels at **Settings > Channels** and view messages at the **Channels** page (`/channels`).

### 7.1 Supported Platforms

| Platform | Status | Description |
|----------|--------|-------------|
| **Telegram** | Available | Receive and send messages via a Telegram bot |
| **Discord** | Available | Connect via a Discord bot to receive and send messages |
| **Slack** | Available | Integrate with a Slack workspace via Socket Mode |
| **Matrix** | Available | Bridges WhatsApp, Slack, IRC, Signal, and more through the Matrix protocol |
| **WhatsApp** | Via Matrix bridge | Use the Matrix channel with a WhatsApp bridge |

### 7.2 Setting Up Telegram

1. Create a Telegram bot through [BotFather](https://t.me/BotFather):
   - Send `/newbot` to BotFather
   - Follow the prompts to name your bot
   - Copy the bot token provided
2. In MesoClaw, go to **Settings > Channels**.
3. Click the **Telegram** channel card to expand the configuration panel.
4. Enter your credentials:
   - **Bot Token**: Paste the token from BotFather (e.g., `123456:ABC-DEF...`)
   - **Allowed Chat IDs**: Comma-separated list of Telegram chat IDs that are authorized to interact with the bot (e.g., `123456789, -1001234567890`)
5. Click **Test Connection** to verify the token works.
6. Click **Connect** to activate the channel.

> **Tip:** To find your Telegram chat ID, send a message to your bot and visit `https://api.telegram.org/bot<YOUR_TOKEN>/getUpdates` in a browser.

### 7.3 Setting Up Discord

1. Create a Discord application and bot at the [Discord Developer Portal](https://discord.com/developers/applications).
2. Copy the bot token from the Bot section.
3. Invite the bot to your server with the necessary permissions.
4. In MesoClaw, go to **Settings > Channels**.
5. Click the **Discord** channel card to expand the configuration panel.
6. Enter your credentials:
   - **Bot Token**: The Discord bot token
   - **Allowed Guild IDs**: Comma-separated server (guild) IDs
   - **Allowed Channel IDs**: Comma-separated channel IDs where the bot should listen
7. Click **Test Connection** to verify. Discord health checks call the `get_current_user()` API.
8. Click **Connect** to activate.

### 7.4 Setting Up Slack

1. Create a Slack app at [api.slack.com/apps](https://api.slack.com/apps).
2. Enable **Socket Mode** in your app settings.
3. Generate a **Bot Token** (starts with `xoxb-`) and an **App-Level Token** (starts with `xapp-`).
4. In MesoClaw, go to **Settings > Channels**.
5. Click the **Slack** channel card to expand the configuration panel.
6. Enter your credentials:
   - **Bot Token**: Your `xoxb-...` token
   - **App Token**: Your `xapp-...` token
   - **Allowed Channel IDs**: Comma-separated Slack channel IDs
7. Click **Test Connection** to verify. Slack health checks call the `auth.test` API.
8. Click **Connect** to activate.

### 7.5 Setting Up Matrix

1. Create an account on your Matrix homeserver (or use an existing one).
2. Generate an access token for the account.
3. In MesoClaw, go to **Settings > Channels**.
4. Click the **Matrix** channel card to expand the configuration panel.
5. Enter your credentials:
   - **Homeserver URL**: Your Matrix server URL (e.g., `https://matrix.org`)
   - **Username**: Your Matrix username
   - **Access Token**: The access token for authentication
   - **Allowed Room IDs**: Comma-separated Matrix room IDs the bot should join
6. Click **Test Connection** to verify. Matrix health checks call the `whoami` endpoint.
7. Click **Connect** to activate.

> **Tip:** Matrix is particularly powerful because it supports bridges to WhatsApp, Slack, IRC, Signal, and other protocols. Configure bridges on your homeserver to extend MesoClaw's reach.

### 7.6 Testing Connection

Each channel has a **Test Connection** button in its configuration panel. When clicked:

1. MesoClaw calls the platform's health-check API:
   - **Telegram**: Calls `getMe` to verify the bot token
   - **Discord**: Calls `get_current_user()` to verify the bot token
   - **Slack**: Calls `auth.test` to verify the bot and app tokens
   - **Matrix**: Calls the `whoami` endpoint to verify the access token
2. Results appear inline:
   - Green text: "Connected successfully"
   - Red text: "Connection failed"

> **Important:** Always test your connection before saving and connecting. This helps catch typos in tokens or IDs early.

### 7.7 Using the Channels Page

Navigate to the **Channels** page from the sidebar to view and interact with messages.

**Layout:**

- **Left sidebar**: Lists all configured channels with status indicators and message counts.
- **Right panel**: Shows messages for the selected channel.

**Channel list details:**

| Indicator | Meaning |
|-----------|---------|
| Green dot | Connected |
| Yellow pulsing dot | Reconnecting |
| Red dot | Error |
| Gray dot | Disconnected |

Each channel card shows:
- Channel icon and display name
- Status label (e.g., "Connected", "Disconnected")
- Message count badge (if messages exist)
- **Connect** / **Disconnect** button

Click a channel name in the sidebar to view its messages in the right panel.

> **Note:** If no channels are configured, the sidebar displays a message with a link to configure channels in Settings.

### 7.8 Sending & Receiving Messages

**Viewing messages:**

1. Select a channel from the left sidebar.
2. The header shows the channel name (e.g., "#telegram").
3. Messages appear in a scrollable conversation view, each showing:
   - **Sender name** (bold)
   - **Timestamp**
   - **Message content**
4. A "Reply to [name]" link appears below each message.

**Replying to a message:**

1. Click the **"Reply to [name]"** link below any message.
2. A recipient indicator appears above the input area showing "Replying to: [name]" with an **X** button to cancel.
3. Type your reply in the message composer at the bottom.
4. Click the **Send** button to send the reply.

**Sending a new message:**

1. Type your message in the composer textarea at the bottom of the message panel.
2. Click the **Send** button.
3. If no recipient is set, the message is sent to the channel directly.

> **Tip:** The Desktop IPC channel (tauri-ipc) is always connected and cannot be disconnected. It provides the internal communication layer for the application.

---

## 8. Memory

The Memory page (`/memory`) provides access to the agent's semantic memory system. Navigate to it from the sidebar or the Home dashboard "Memory" card.

### 8.1 Semantic Search

1. Select the **Search** tab (active by default).
2. Type a natural language query into the search field (e.g., "What did we discuss about authentication?").
3. Press Enter or click the search button.
4. Results appear ranked by **semantic similarity** -- the most relevant memories are listed first.
5. Each result shows the memory content, category, and relevance score.

The context panel on the right provides search tips:
- Use natural language to query semantic memory
- Try searching for topics, events, or entities
- Results are ranked by semantic similarity

### 8.2 Daily Timeline

1. Select the **Daily Timeline** tab.
2. You see a date-based browser for journal entries.
3. Select a specific date to view the daily summary for that day.
4. Each entry shows the events, conversations, and key takeaways from that day.

The context panel shows today's date and a hint to "Browse journal entries by day in the timeline."

### 8.3 Memory Categories

Memory entries are organized into categories:

| Category | Description |
|----------|-------------|
| **Core** | Fundamental facts and permanent knowledge |
| **Daily** | Daily summaries and journal entries |
| **Conversation** | Contextual information from past conversations |
| **Custom** | User-defined memory categories |

Memory data is stored and searched via the gateway REST API.

---

## 9. Identity (Agent Personality)

The Identity editor lets you customize the AI agent's personality, values, and behavior. Access it at **Settings > Identity**.

### 9.1 Understanding Identity Files

Identity is defined through a collection of Markdown files that shape the agent's system prompt. Common identity files include:

- **SOUL.md** -- Core personality and character traits
- **OPERATING_INSTRUCTIONS.md** -- Behavioral guidelines and operating procedures
- **VALUES.md** -- Principles and priorities
- Additional files as defined by your configuration

These files collectively determine how the AI agent responds, what tone it uses, and what principles it follows.

### 9.2 Editing Identity

1. Go to **Settings > Identity**.
2. The left sidebar shows a list of all identity files in monospace font under the heading "Identity Files."
3. Click a file name to load it into the editor.
4. The right pane displays:
   - **File name** in the toolbar
   - **"unsaved"** badge when you have pending changes
   - **Preview / Edit** toggle button to switch between raw editing and preview modes
   - **Save** button (enabled only when changes are pending)
5. Edit the file content in the text area.
6. Click **Save** to persist your changes. The button briefly shows "Saved" with a check mark on success.

> **Note:** If no file is selected, the editor area shows "Select a file to edit."

### 9.3 Hot-Reload

Changes to identity files are hot-reloaded into the system prompt. After saving a file:

1. The updated content is immediately available to the AI agent.
2. You do not need to restart the application.
3. New conversations will use the updated identity.

Identity files are also accessible via the gateway REST API:
- `GET /api/v1/identity/{file}` -- Read a file
- `PUT /api/v1/identity/{file}` -- Update a file

---

## 10. Skills (Prompt Templates)

### 10.1 What are Skills?

Skills are reusable prompt templates that enhance AI responses for specific tasks. They are Markdown files with YAML frontmatter, stored in `~/.config/mesoclaw/skills/`.

Example skill file:

```markdown
---
id: code-reviewer
name: Code Reviewer
description: Reviews code for quality, security, and best practices
category: development
defaultEnabled: true
---

You are an expert code reviewer. Analyze the following code for:
- Code quality and readability
- Security vulnerabilities
- Performance issues
- Best practice violations

{{code}}

Provide actionable feedback with specific line references.
```

### 10.2 Managing Skills

Access skill management at **Settings > Skills**.

From this tab you can:
- **View** all installed skills organized by category
- **Enable/disable** individual skills using toggle switches
- **Reload** skills from the filesystem to pick up new or modified files
- **Delete** skills you no longer need

### 10.3 Skill Auto-Selection

MesoClaw can automatically suggest or select skills based on the context of your conversation. When the AI detects that a particular skill matches your request, it may:

- Apply the skill's prompt template automatically
- Suggest relevant skills for you to choose from

### 10.4 Creating Custom Skills

**Method 1: Manual file creation**

1. Create a new `.md` file in `~/.config/mesoclaw/skills/`.
2. Add YAML frontmatter with the required fields (`id`, `name`, `description`, `category`, `defaultEnabled`).
3. Write your prompt template below the frontmatter, using `{{parameter_name}}` placeholders.
4. Go to **Settings > Skills** and click **Reload** to discover the new skill.

**Method 2: Using the Prompt Generator**

1. Navigate to **Prompt Generator** from the sidebar.
2. Select **Skill** as the artifact type.
3. Enter a name and description.
4. Click **Generate** and review the result.
5. Click **Save** to write the skill to disk.

See [Section 11](#11-prompt-generator) for full details on the Prompt Generator.

---

## 11. Prompt Generator

The Prompt Generator (`/prompt-generator`) is a tool for creating AI prompt templates for various purposes. It generates content using AI and saves it to your library.

### 11.1 Artifact Types

Select the type of artifact you want to generate using the pill buttons at the top:

| Type | Description |
|------|-------------|
| **Skill** | Reusable prompt template for the skills system |
| **Agent** | Autonomous agent system prompt |
| **Soul** | Character and personality definition |
| **Claude Skill** | Claude Code skill file |
| **Generic** | General-purpose prompt template |

### 11.2 Generating Content

1. Select an artifact type from the pill buttons.
2. Fill in the form:
   - **Name**: A short identifier (e.g., "code-reviewer")
   - **Describe what you want**: A detailed description of the prompt you need (e.g., "A code review prompt that checks for security, performance, and maintainability")
3. Click **Generate**. A loading spinner appears while the AI creates the content.
4. When complete, the generated prompt appears in an editable text area.
5. Review and modify the output as needed.
6. Use the action buttons:
   - **Save**: Write the artifact to disk (shows the saved file path below)
   - **Copy**: Copy the content to your clipboard
   - **Clear**: Reset the form
   - **Regenerate**: Generate a new version with the same inputs

### 11.3 Library Management

Below the generator form, a **Library** section shows all previously generated artifacts and installed skills.

**Library tabs:**

| Tab | Shows |
|-----|-------|
| **All** | Every artifact and skill |
| **Skill** | Installed filesystem skills + generated skill artifacts |
| **Agent** | Generated agent prompts |
| **Soul** | Generated soul definitions |
| **Claude Skill** | Generated Claude skill files |
| **Generic** | Generated generic prompts |

**Actions on each artifact:**

- **Edit** (pencil icon): Opens a dialog with a full-height text area where you can modify the content and click **Save**.
- **Delete** (trash icon): Removes the artifact. For filesystem skills, a confirmation dialog appears: "Are you sure you want to delete [name]? This will remove the file from disk and cannot be undone."

**Adding new artifacts:**

- Click the **+ Add [Type]** button in the library tab bar to scroll to the generator form with the appropriate type pre-selected.

---

## 12. Modules (Extensions)

Modules are sidecar processes that extend MesoClaw's capabilities. Access module management at **Settings > Modules**.

### 12.1 What are Modules?

Modules are external tools, services, or MCP (Model Context Protocol) servers that run alongside MesoClaw. They can provide additional AI capabilities, integrations with external services, or custom tools for the agent.

### 12.2 Module Types

| Type | Badge Color | Description |
|------|-------------|-------------|
| **mcp** | Primary (blue) | Model Context Protocol servers that provide tools and resources to AI models |
| **service** | Secondary (gray) | Background services (APIs, databases, etc.) |
| **tool** | Outline | Command-line tools or utilities |

Each module also has a **runtime type**:

| Runtime | Description |
|---------|-------------|
| **native** | Runs directly on the host system |
| **docker** | Runs in a Docker container |
| **podman** | Runs in a Podman container |

### 12.3 Creating a Module

1. Go to **Settings > Modules**.
2. Click the **+ New Module** button.
3. A scaffold form appears where you enter:
   - **Name**: Display name for the module
   - **Type**: mcp, service, or tool
   - **Runtime**: native, docker, or podman
   - **Command**: The command to execute (e.g., `npx @modelcontextprotocol/server-filesystem`)
   - **Description**: What the module does
4. Submit the form to create the module.

Module manifests are stored as TOML files at `~/.config/mesoclaw/modules/{id}/manifest.toml`.

### 12.4 Starting & Stopping Modules

Each module card in the grid shows:

- **Status dot**: Green (running), yellow pulsing (starting), red (error), or gray (stopped)
- **Name** and type/runtime badges
- **Command** preview in monospace
- **Toggle switch**: Click to start or stop the module

**To start a module:** Toggle the switch to the on position. The status dot changes to yellow (starting) then green (running).

**To stop a module:** Toggle the switch to the off position. The status dot returns to gray (stopped).

**To view module details:** Click the module card to select it. A detail panel expands to the right showing the full module configuration and status information. Click the **X** button in the detail panel to close it.

### 12.5 MCP Server Modules

MCP (Model Context Protocol) server modules are a special category that allows AI models to access external tools and resources. When an MCP module is running:

- The AI agent can discover and use the tools provided by the MCP server.
- Tools appear as callable functions in the agent's context.
- Resources are available for the agent to read and use in responses.

---

## 13. Logs

The Logs page (`/logs`) provides a real-time view of application log entries from the current session.

### 13.1 Viewing Logs

1. Navigate to **Logs** from the sidebar.
2. You see a table with the following columns:

| Column | Description |
|--------|-------------|
| **Time** | Shortened timestamp (HH:MM:SS.mmm format) |
| **Level** | Color-coded badge (TRACE, DEBUG, INFO, WARN, ERROR) |
| **Target** | The source module that generated the log (hidden on small screens) |
| **Message** | The log message content, color-coded by level |

3. A footer shows the count of displayed entries vs. total entries (e.g., "142 / 5000 entries") and a "live" indicator when auto-refresh is active.

**Color coding:**

| Level | Text Color | Row Highlight |
|-------|-----------|---------------|
| ERROR | Red | Light red background |
| WARN | Yellow | Light yellow background |
| INFO | Blue | None |
| DEBUG | Green | None |
| TRACE | Gray | None |

### 13.2 Filtering by Level

The toolbar at the top contains level filter buttons: **ALL**, **TRACE**, **DEBUG**, **INFO**, **WARN**, **ERROR**.

1. Click a level button to show only entries of that level.
2. The active button is highlighted with the primary color.
3. Each level button shows a count of matching entries (e.g., "ERROR 3").
4. Click **ALL** to show entries of every level.

### 13.3 Searching Logs

1. Use the **search field** in the toolbar (magnifying glass icon, placeholder "Search messages...").
2. Type a search term to filter log entries.
3. The filter applies to the message text, target module, and timestamp.
4. Click the **X** button inside the search field to clear the search.

> **Tip:** Combine level filtering with search for precise results. For example, select "ERROR" and search for "database" to find all database-related errors.

### 13.4 Live Tail Mode

Live tail mode automatically refreshes the log view every 2 seconds to show the latest entries.

**Controls:**

- **Pause/Play button** (in the toolbar): Toggle auto-refresh on or off.
  - When active (playing): The button shows a **Pause** icon and is styled with the primary color.
  - When paused: The button shows a **Play** icon in ghost style.
- **Refresh button** (circular arrow icon): Manually trigger a log refresh at any time.
- **Scroll-to-bottom button**: A floating button appears in the bottom-right corner of the log table when you scroll away from the latest entries. Click it to jump back to the newest logs.

**Auto-scroll behavior:** When you are already scrolled to the bottom, new entries automatically keep the view scrolled to the latest. If you scroll up to read older entries, auto-scroll pauses so your position is preserved.

---

## 14. App Settings

Access application settings at **Settings > App Settings**. Changes auto-save as you update them.

### 14.1 Appearance

| Setting | Description | Options |
|---------|-------------|---------|
| **Theme** | Application color scheme | Light, Dark, Auto (follows system) |
| **Language** | Interface language | Available language options |
| **Sidebar Expanded** | Whether the sidebar starts expanded or collapsed | Toggle switch |

### 14.2 Behavior

| Setting | Description | Default |
|---------|-------------|---------|
| **Show in Tray** | Keep MesoClaw in the system tray when the window is closed | Off |
| **Launch at Login** | Automatically start MesoClaw when you log in to your computer | Off |

### 14.3 Notifications & DND

**Master notification toggle:**

| Setting | Description |
|---------|-------------|
| **Enable Notifications** | Master switch for all desktop notifications. When off, all notification categories below are disabled. |

**Do Not Disturb (DND) schedule:**

| Setting | Description | Default |
|---------|-------------|---------|
| **Enable DND schedule** | Automatically suppress notifications during specified hours | Off |
| **DND start hour** | Hour (0-23) when DND begins | 22 (10 PM) |
| **DND end hour** | Hour (0-23) when DND ends | 7 (7 AM) |

**Per-category notification toggles:**

| Category | Description |
|----------|-------------|
| **Heartbeat** | Show a notification on each heartbeat tick |
| **Cron reminders** | Show a notification when a scheduled job fires |
| **Agent complete** | Show a notification when an agent task finishes |
| **Approval requests** | Show a notification when an action requires your approval |

> **Note:** Per-category toggles are disabled when the master notification switch is off.

### 14.4 Developer Options

| Setting | Description |
|---------|-------------|
| **Enable Logging** | Turn application logging on or off |
| **Log Level** | Set the minimum log level to capture (TRACE, DEBUG, INFO, WARN, ERROR) |

> **Tip:** Set the log level to DEBUG or TRACE when troubleshooting issues, then switch back to INFO for normal operation to reduce noise.

---

## 15. Advanced Settings

Access advanced configuration at **Settings > Advanced**. These settings are intended for power users.

**Caching:**

| Setting | Description | Range |
|---------|-------------|-------|
| **Enable Caching** | Cache AI responses to avoid redundant API calls | Toggle |
| **Cache Duration** | How long to keep cached responses | 1-168 hours |

**Request Settings (AI Parameters):**

| Setting | Description | Range | Default |
|---------|-------------|-------|---------|
| **Temperature** | Controls randomness in AI responses. Lower values produce more deterministic output. | 0.0 - 1.0 | 0.7 |
| **Max Tokens** | Maximum number of tokens in the AI response | 256 - 32768 | 4096 |
| **Timeout** | Request timeout before giving up | 5 - 300 seconds | 30 |

**Advanced Options:**

| Setting | Description |
|---------|-------------|
| **Stream Responses** | Enable streaming for real-time token-by-token response display |
| **Debug Mode** | Enable detailed logging for AI requests and responses |
| **Custom Base URL** | Override the provider's default API endpoint with a custom URL (e.g., for proxies or self-hosted instances) |

> **Tip:** If you are using a corporate proxy or a self-hosted API, enter the proxy URL in the **Custom Base URL** field. This overrides the default endpoint for all providers.

---

## 16. Security & Privacy

### 16.1 API Key Storage

MesoClaw stores all API keys in your operating system's secure credential store:

| Platform | Storage Mechanism |
|----------|-------------------|
| **macOS** | Keychain (via `keyring` crate) |
| **Windows** | Windows Credential Manager |
| **Linux** | Secret Service API (GNOME Keyring or KWallet) |

API keys are:
- Never stored in plain text on disk
- Never written to configuration files
- Encrypted at rest by the OS credential manager
- Zeroized in memory after use (using the `zeroize` crate)

### 16.2 Channel Credential Storage

Channel credentials (bot tokens, access tokens, API keys) follow the same security model as AI provider API keys. They are stored in the OS keyring under the service identifier `com.sprklai.mesoclaw`.

### 16.3 Data Locations

| Data | Location |
|------|----------|
| **Application database** | Tauri app-local data directory (SQLite) |
| **Skills** | `~/.config/mesoclaw/skills/` |
| **Module manifests** | `~/.config/mesoclaw/modules/{id}/manifest.toml` |
| **Identity files** | Managed through the gateway; stored in the app configuration directory |
| **Logs** | In-memory for the current session; viewable at `/logs` |
| **API keys** | OS keyring (never on disk) |

---

## 17. Troubleshooting

### 17.1 Common Issues

**No AI models appear in the Chat model selector:**

1. Go to **Settings > AI Provider**.
2. Verify at least one provider is configured with a valid API key.
3. Ensure the provider has models added (some providers require manual model addition).
4. For Ollama, ensure the service is running and click **Refresh Models**.

**Chat messages fail to send:**

1. Check that an API key is set for the selected provider (**Settings > AI Provider**).
2. Test the provider connection using the **Test Connection** button.
3. Verify your internet connection (for cloud providers).
4. Check the **Logs** page for detailed error messages.

**Settings are not saving:**

1. Settings auto-save on change. If a save fails, check the Logs page for errors.
2. Ensure the application has write permissions to its data directory.
3. Try restarting MesoClaw.

**Application appears blank or fails to load:**

1. Check the system tray -- MesoClaw may be minimized.
2. Restart the application.
3. Check the system logs for crash reports.

### 17.2 Gateway Status

The Gateway Status indicator appears on the Home dashboard in the top-right corner.

| Status | Meaning | Action |
|--------|---------|--------|
| Green dot | Gateway is running and healthy | No action needed |
| Gray dot | Gateway is offline or unreachable | Check if the backend process is running; restart MesoClaw if needed |
| Red dot | Gateway encountered an error | Check the Logs page for details |

The gateway provides the HTTP REST API that the Memory system, Identity system, and other features depend on. If the gateway is offline, these features will not function.

### 17.3 Channel Connection Issues

**Telegram:**

- Verify the bot token is correct (from BotFather).
- Ensure the bot has been started (send `/start` to the bot in Telegram).
- Check that the allowed chat IDs match the chats you are trying to use.
- The health check calls `getMe` -- if this fails, the token is likely invalid.

**Discord:**

- Verify the bot token is correct (from the Discord Developer Portal).
- Ensure the bot has been invited to your server with appropriate permissions.
- Check that the guild IDs and channel IDs match your server configuration.
- The health check calls `get_current_user()` -- if this fails, the token is invalid or the bot is not properly configured.

**Slack:**

- Verify both the Bot Token (`xoxb-...`) and App Token (`xapp-...`) are correct.
- Ensure Socket Mode is enabled in your Slack app settings.
- Check that the bot has been invited to the channels listed in allowed channel IDs.
- The health check calls `auth.test` -- if this fails, check your token scopes.

**Matrix:**

- Verify the homeserver URL is accessible.
- Ensure the access token is valid and not expired.
- Check that the bot user has been invited to and joined the allowed rooms.
- The health check calls the `whoami` endpoint -- if this fails, the access token may be revoked.

**General channel troubleshooting:**

1. Always use the **Test Connection** button before attempting to connect.
2. Check the **Logs** page (filter by "ERROR") for detailed connection failure messages.
3. Verify that your network allows outbound connections to the platform's API servers.
4. If a channel shows "Reconnecting..." for an extended period, try disconnecting and reconnecting manually.
