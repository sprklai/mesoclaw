# Generated Diagrams for Mesoclaw

> This document contains programmatically generated diagrams based on the project's documentation, including user journeys and updated architecture visuals.

---

## 1. User Journey

This diagram illustrates the two primary user paths: the developer-focused CLI path and the user-friendly GUI path. It shows how users onboard, discover features, and progress to power usage.

```mermaid
graph TD
    subgraph Legend
        direction LR
        cli_path[CLI Path]:::cli
        gui_path[GUI Path]:::gui
        shared_path[Shared Experience]:::shared
    end

    subgraph "Stage 0: Installation"
        A0(Start) --> A1{Install Binary};
        A1 --> A2("CLI: `mesoclaw`"):::cli;
        A1 --> A3("GUI: Launch App"):::gui;
    end

    subgraph "Stage 1: First Use & Onboarding"
        A2 --> B1("CLI: First command & REPL"):::cli;
        A3 --> B2(GUI: Setup Wizard) --> B3(First Chat):::gui;
    end

    subgraph "Stage 2: Core Experience"
        B1 --> C1(REPL Sessions & Piped Commands):::cli;
        B3 --> C2(Basic Q&A):::gui;
        C1 --> C3(Discover Agent Loop):::shared;
        C2 --> C3;
    end

    subgraph "Stage 3: Becoming a Power User"
        C3 --> D1(Personalize Agent via Identity Files):::shared;
        D1 --> D2(Configure Proactive Behavior via Scheduler):::shared;
    end

    subgraph "Stage 4: Advanced Usage & Extension"
        D2 --> E1(Multi-Channel Interaction e.g., Telegram):::shared;
        D2 --> E2(Use on Mobile/Tablet):::shared;
        D2 --> E3(Extend with Sidecar Modules):::shared;
        D2 --> E4(Contribute to Project):::shared;
    end

    classDef cli fill:#D6EAF8,stroke:#3498DB,stroke-width:2px,color:#212F3D;
    classDef gui fill:#D4EFDF,stroke:#27AE60,stroke-width:2px,color:#212F3D;
    classDef shared fill:#E8DAEF,stroke:#8E44AD,stroke-width:2px,color:#212F3D;
```

---

## 2. High-Level System Architecture

This is a refined version of the system overview, emphasizing the separation between the frontend, the gateway, and the backend core.

```mermaid
graph TD
    subgraph "Clients"
        direction LR
        Client_CLI[CLI Interface]:::client
        Client_GUI[Tauri GUI]:::client
        Client_External[External Scripts via curl]:::client
    end

    subgraph "Backend Daemon"
        Gateway[Local Gateway API<br>HTTP REST + WebSocket<br>127.0.0.1:18790]:::gateway
        EventBus[Event Bus<br>tokio::broadcast]:::core

        subgraph "Core Subsystems"
            direction TB
            Agent[Agent Loop]:::core
            Tools[Tool System]:::core
            Memory[Memory System]:::core
            Scheduler[Scheduler]:::core
            Identity[Identity System]:::core
            Security[Security Policy]:::core
        end

        CoreServices[Core Services<br>LLM Providers, Config, etc.]:::core
        Storage[Storage Layer<br>SQLite, Filesystem, OS Keyring]:::storage
    end

    subgraph "External Services"
        direction LR
        LLM_APIs[LLM APIs<br>OpenAI, Anthropic, Ollama]:::external
        Webhook_Sources[Webhook Sources]:::external
        Messaging_APIs[Messaging APIs<br>Telegram, WhatsApp]:::external
    end

    Client_CLI -- HTTP/WebSocket --> Gateway;
    Client_GUI -- HTTP/WebSocket --> Gateway;
    Client_External -- HTTP/WebSocket --> Gateway;

    Gateway --> EventBus;
    EventBus --> Agent;
    EventBus --> Scheduler;
    EventBus --> Tools;
    EventBus --> Memory;

    Agent --> Tools;
    Agent --> Memory;
    Agent --> Security;
    Agent --> Identity;
    Agent --> CoreServices;

    CoreServices --> Storage;
    CoreServices --> LLM_APIs;

    Scheduler --> Agent;

    classDef gateway fill:#A3E4D7,stroke:#16A085,stroke-width:2px,color:#212F3D;
    classDef client fill:#D6EAF8,stroke:#3498DB,stroke-width:2px,color:#212F3D;
    classDef core fill:#FDEBD0,stroke:#E67E22,stroke-width:2px,color:#212F3D;
    classDef storage fill:#E8DAEF,stroke:#8E44AD,stroke-width:2px,color:#212F3D;
    classDef external fill:#FADBD8,stroke:#C0392B,stroke-width:2px,color:#212F3D;
```

---

## 3. Sidecar Modularity Architecture

This diagram shows how Mesoclaw extends its toolset using Sidecar Modules, supporting native scripts, containerized environments, and the MCP protocol.

```mermaid
graph TD
    subgraph "Mesoclaw Agent"
        ToolRegistry[Tool Registry]:::core
    end

    subgraph "Module Types"
        SidecarTool[SidecarTool<br>On-demand process<br>Protocol: Stdin/Stdout JSON]:::module
        SidecarService[SidecarService<br>Long-lived HTTP server<br>Protocol: HTTP REST]:::module
        McpServer[McpServer<br>Long-lived MCP server<br>Protocol: JSON-RPC over Stdin/Stdout]:::module
    end

    subgraph "Runtimes"
        Native[Native Process]:::runtime
        Container[Container Runtime<br>Podman / Docker]:::runtime
    end

    BuiltInTools(Built-in Rust Tools) --> ToolRegistry;

    ToolRegistry -- discovers & registers --> SidecarTool;
    ToolRegistry -- discovers & registers --> SidecarService;
    ToolRegistry -- discovers & registers --> McpServer;

    SidecarTool -- executed by --> Native;
    SidecarTool -- or executed by --> Container;
    SidecarService -- executed by --> Native;
    SidecarService -- or executed by --> Container;
    McpServer -- executed by --> Native;
    McpServer -- or executed by --> Container;

    classDef core fill:#FDEBD0,stroke:#E67E22,stroke-width:2px,color:#212F3D;
    classDef module fill:#D6EAF8,stroke:#3498DB,stroke-width:2px,color:#212F3D;
    classDef runtime fill:#D1F2EB,stroke:#138D75,stroke-width:2px,color:#212F3D;
```

---

## 4. CLI-First Gateway Architecture

This diagram details the CLI-first approach, where both the CLI and the GUI are clients to a central, local daemon.

```mermaid
graph TD
    subgraph "User Interfaces"
        CLI[CLI (mesoclaw)]:::client
        GUI[Tauri GUI (mesoclaw-desktop)]:::client
    end

    subgraph "Daemon & Gateway"
        Daemon[Daemon Process]:::daemon

        subgraph "Gateway"
            REST[HTTP REST API]:::gateway_api
            WS[WebSocket API]:::gateway_api
        end
    end

    subgraph "Core Logic (lib.rs)"
        Core[All Business Logic<br>Agent, Memory, Providers, etc.]:::core
    end

    CLI -- starts or connects to --> Daemon;
    GUI -- embeds and starts --> Daemon;

    Daemon -- exposes --> REST;
    Daemon -- exposes --> WS;
    Daemon -- contains --> Core;

    CLI -- communicates via --> REST & WS;
    GUI -- communicates via --> REST & WS;

    classDef client fill:#D6EAF8,stroke:#3498DB,stroke-width:2px,color:#212F3D;
    classDef daemon fill:#FDEBD0,stroke:#E67E22,stroke-width:2px,color:#212F3D;
    classDef gateway_api fill:#A3E4D7,stroke:#16A085,stroke-width:2px,color:#212F3D;
    classDef core fill:#E8DAEF,stroke:#8E44AD,stroke-width:2px,color:#212F3D;
```

---

## 5. Data Flow: Multi-Turn Agent Loop

This diagram shows the iterative process the agent follows to understand a request, use tools, and arrive at a final answer.

```mermaid
graph TD
    A[User Request] --> B{1. Build Context};
    B -- Identity, Memory, History --> C{2. Call LLM};
    C --> D{3. Parse Response};
    D -- Tool Call? --> E{4. Has Tool Call?};
    E -- No --> F[Done. Return Final Response];
    E -- Yes --> G{5. Security Check};

    subgraph "Security"
        G -- Validate Command & Path --> H{Approval Needed?};
    end

    H -- No / Approved --> I{6. Execute Tool};
    H -- Denied --> J[Inform LLM of Denial];
    I -- Result --> K{7. Append to History};
    J --> K;
    K --> C;

    classDef process fill:#FDEBD0,stroke:#E67E22,stroke-width:2px,color:#212F3D;
    classDef decision fill:#D6EAF8,stroke:#3498DB,stroke-width:2px,color:#212F3D;
    classDef io fill:#D4EFDF,stroke:#27AE60,stroke-width:2px,color:#212F3D;
    classDef security fill:#FADBD8,stroke:#C0392B,stroke-width:2px,color:#212F3D;

    class A,F io;
    class B,C,D,G,I,J,K process;
    class E,H decision;
```

---

## 6. Event Bus Architecture

The Event Bus is the central nervous system of the backend, allowing subsystems to communicate asynchronously.

```mermaid
graph TD
    subgraph "Event Producers"
        AgentLoop[Agent Loop]
        Scheduler
        ChannelManager[Channel Manager]
        MemorySystem[Memory System]
    end

    subgraph "Central Bus"
        EventBus[Event Bus<br>(tokio::broadcast)]
    end

    subgraph "Event Consumers"
        TauriBridge[Tauri Bridge<br>to Frontend]
        AgentLoopConsumer[Agent Loop]
        NotificationService[Notification Service]
        AuditLogger[Audit Logger]
    end

    AgentLoop -- Publishes: AgentToolStart, AgentToolResult, ApprovalNeeded --> EventBus;
    Scheduler -- Publishes: HeartbeatTick, CronFired --> EventBus;
    ChannelManager -- Publishes: ChannelMessage --> EventBus;
    MemorySystem -- Publishes: MemoryStored, MemoryRecalled --> EventBus;

    EventBus -- Subscribes & Forwards --> TauriBridge;
    EventBus -- Subscribes to Responses --> AgentLoopConsumer;
    EventBus -- Subscribes to Alerts --> NotificationService;
    EventBus -- Subscribes to All --> AuditLogger;

    style EventBus fill:#E8DAEF,stroke:#8E44AD,stroke-width:4px,color:#212F3D;
```

---

## 7. Security Architecture Layers

This diagram breaks down the six layers of defense that protect the user and their system from unintended actions.

```mermaid
graph LR
    subgraph "Security Layers"
        direction TB
        L1[<b>Layer 1: Credential Security</b><br>OS Keyring (keyring crate)<br>API keys never touch disk]:::l1
        L2[<b>Layer 2: Command Validation</b><br>3 Autonomy Levels<br>Risk Classification (Low/Med/High)]:::l2
        L3[<b>Layer 3: Filesystem Sandboxing</b><br>Workspace-only access<br>Path traversal prevention]:::l3
        L4[<b>Layer 4: Injection Protection</b><br>Block backticks, redirects, pipes]:::l4
        L5[<b>Layer 5: Rate Limiting</b><br>Sliding window (e.g., 20 actions/hour)]:::l5
        L6[<b>Layer 6: Audit Trail</b><br>All tool executions logged]:::l6
    end

    L1 --> L2 --> L3 --> L4 --> L5 --> L6

    classDef l1 fill:#FADBD8,stroke:#C0392B,stroke-width:2px,color:#000;
    classDef l2 fill:#F5B7B1,stroke:#C0392B,stroke-width:2px,color:#000;
    classDef l3 fill:#EC7063,stroke:#C0392B,stroke-width:2px,color:#000;
    classDef l4 fill:#E74C3C,stroke:#C0392B,stroke-width:2px,color:#FFF;
    classDef l5 fill:#CB4335,stroke:#C0392B,stroke-width:2px,color:#FFF;
    classDef l6 fill:#943126,stroke:#C0392B,stroke-width:2px,color:#FFF;
```

---

## 8. CI/CD Release Pipeline

This illustrates the automated pipeline for building, testing, and releasing Mesoclaw across all supported platforms.

```mermaid
graph TD
    subgraph "Trigger"
        A[Manual Trigger on GitHub]:::trigger
    end

    subgraph "CI & Test (Pre-flight on PRs)"
        B1[Lint & Format]:::ci
        B2[Unit & Integration Tests]:::ci
        B1 & B2 --> C{All Checks Pass?}:::decision
    end

    subgraph "Release Process"
        D[Create Draft Release]:::release
        E[Parallel Build Matrix]:::release
        F[Upload Artifacts to Release]:::release
    end

    subgraph "Build Matrix (8 Configurations)"
        direction LR
        M1[macOS aarch64<br>sign & notarize]:::build
        M2[macOS x86_64<br>sign & notarize]:::build
        M3[Windows x64<br>Azure sign]:::build
        M4[Windows ARM64<br>Azure sign]:::build
        M5[Linux x64 .deb]:::build
        M6[Linux x64 .AppImage]:::build
        M7[...]:::build
    end

    A -- on release branch --> C
    C -- Yes --> D
    D --> E
    E --> M1 & M2 & M3 & M4 & M5 & M6 & M7
    M1 & M2 & M3 & M4 & M5 & M6 & M7 --> F

    classDef trigger fill:#D6EAF8,stroke:#3498DB,stroke-width:2px,color:#212F3D;
    classDef ci fill:#D1F2EB,stroke:#138D75,stroke-width:2px,color:#212F3D;
    classDef decision fill:#F9E79F,stroke:#F1C40F,stroke-width:2px,color:#212F3D;
    classDef release fill:#E8DAEF,stroke:#8E44AD,stroke-width:2px,color:#212F3D;
    classDef build fill:#FDEBD0,stroke:#E67E22,stroke-width:2px,color:#212F3D;
```
