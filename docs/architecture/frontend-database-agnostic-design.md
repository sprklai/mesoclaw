# Frontend Database-Agnostic Design

**Status:** Design Document
**Last Updated:** 2025-01-28
**Related:** [Connection Method Refactoring](../implementation/connection-method-refactoring.md)

---

## Overview

This document outlines the frontend architecture changes needed to support the new database-agnostic backend and make the application future-proof for adding new database types.

### Current Problem

The frontend currently has:

- **Hardcoded database types** - SQLite, PostgreSQL, MySQL are strings scattered throughout
- **Tightly coupled connection methods** - SSH tunnel is embedded in each database config
- **Type-specific UI components** - Separate forms for each database type
- **Duplicated code** - SSH tunnel checkboxes for PostgreSQL and MySQL

### Solution: Layered Architecture

Separate **database type** from **connection method** in the frontend, matching the backend architecture.

---

## Proposed Frontend Architecture

### Type System Changes

#### 1. Database Type Registry

Create a central registry of supported database types with metadata:

```typescript
// src/types/database-registry.ts

export interface DatabaseTypeMetadata {
  id: DatabaseType;
  displayName: string;
  description: string;
  icon: string;
  defaultPort: number;
  connectionMode: "file" | "network";
  supportedConnectionMethods: ConnectionMethodType[];
  fields: ConnectionField[];
  capabilities: DatabaseCapabilities;
}

export enum DatabaseType {
  SQLite = "SQLite",
  PostgreSQL = "PostgreSQL",
  MySQL = "MySQL",
  // Future databases can be added here without modifying UI code
}

export enum ConnectionMethodType {
  DirectTcp = "DirectTcp",
  SshTunnel = "SshTunnel",
  LocalFile = "LocalFile",
  ConnectionPooler = "ConnectionPooler", // Future
  CloudProxy = "CloudProxy", // Future
}
```

#### 2. Registry Implementation

```typescript
// src/constants/database-registry.ts

export const DATABASE_REGISTRY: Record<DatabaseType, DatabaseTypeMetadata> = {
  [DatabaseType.SQLite]: {
    id: DatabaseType.SQLite,
    displayName: "SQLite",
    description: "Local file-based database",
    icon: "database",
    defaultPort: 0,
    connectionMode: "file",
    supportedConnectionMethods: [ConnectionMethodType.LocalFile],
    fields: [
      {
        name: "path",
        label: "Database File",
        type: "file",
        required: true,
        accept: ".sqlite,.db,.sqlite3",
      },
      {
        name: "read_only",
        label: "Read-Only Mode",
        type: "boolean",
        default: false,
      },
    ],
    capabilities: {
      supportsSsh: false,
      supportsSsl: false,
      supportsMultipleConnections: false,
    },
  },

  [DatabaseType.PostgreSQL]: {
    id: DatabaseType.PostgreSQL,
    displayName: "PostgreSQL",
    description: "Enterprise-grade relational database",
    icon: "database",
    defaultPort: 5432,
    connectionMode: "network",
    supportedConnectionMethods: [
      ConnectionMethodType.DirectTcp,
      ConnectionMethodType.SshTunnel,
      ConnectionMethodType.ConnectionPooler,
    ],
    fields: [
      {
        name: "host",
        label: "Host",
        type: "text",
        required: true,
        placeholder: "localhost",
      },
      {
        name: "port",
        label: "Port",
        type: "number",
        default: 5432,
      },
      {
        name: "database",
        label: "Database",
        type: "text",
        required: true,
      },
      {
        name: "username",
        label: "Username",
        type: "text",
        required: true,
      },
      {
        name: "password",
        label: "Password",
        type: "password",
        required: false,
      },
      {
        name: "use_ssl",
        label: "Use SSL/TLS",
        type: "boolean",
        default: false,
      },
    ],
    capabilities: {
      supportsSsh: true,
      supportsSsl: true,
      supportsMultipleConnections: true,
    },
  },

  [DatabaseType.MySQL]: {
    id: DatabaseType.MySQL,
    displayName: "MySQL",
    description: "Popular open-source relational database",
    icon: "database",
    defaultPort: 3306,
    connectionMode: "network",
    supportedConnectionMethods: [
      ConnectionMethodType.DirectTcp,
      ConnectionMethodType.SshTunnel,
    ],
    fields: [
      // Similar to PostgreSQL
    ],
    capabilities: {
      supportsSsh: true,
      supportsSsl: true,
      supportsMultipleConnections: true,
    },
  },

  // Example: Adding a new database type
  // [DatabaseType.MongoDB]: {
  //   id: DatabaseType.MongoDB,
  //   displayName: 'MongoDB',
  //   description: 'NoSQL document database',
  //   icon: 'database',
  //   defaultPort: 27017,
  //   connectionMode: 'network',
  //   supportedConnectionMethods: [
  //     ConnectionMethodType.DirectTcp,
  //     ConnectionMethodType.SshTunnel,
  //   ],
  //   fields: [ /* ... */ ],
  //   capabilities: { /* ... */ },
  // },
};
```

### Configuration Structure Changes

#### Current Structure (Tightly Coupled)

```typescript
// Current: ConnectionConfig has optional properties for each DB type
interface ConnectionConfig {
  SQLite?: { path: string; read_only: boolean };
  PostgreSQL?: {
    host: string;
    port: number;
    database: string;
    username: string;
    ssh_tunnel?: SshConfig; // ← Embedded in DB config
  };
  MySQL?: {
    host: string;
    port: number;
    database: string;
    username: string;
    ssh_tunnel?: SshConfig; // ← Duplicated!
  };
}
```

#### New Structure (Layered)

```typescript
// New: Separated database config from connection method

interface DatabaseConnectionConfig {
  databaseType: DatabaseType;
  database: DatabaseSpecificConfig;
  connectionMethod: ConnectionMethodConfig;
}

// Database-specific configuration (what to connect to)
interface DatabaseSpecificConfig {
  SQLite: {
    path: string;
    read_only?: boolean;
  };
  PostgreSQL: {
    host?: string;
    port?: number;
    database: string;
    username: string;
    password?: string; // Retrieved from keyring
    use_ssl?: boolean;
  };
  MySQL: {
    host?: string;
    port?: number;
    database: string;
    username: string;
    password?: string;
    use_ssl?: boolean;
  };
}

// Connection method configuration (how to reach it)
interface ConnectionMethodConfig {
  DirectTcp: {
    host: string;
    port: number;
  };
  SshTunnel: {
    bastion: SshConfig;
    targetHost: string;
    targetPort: number;
  };
  LocalFile: {
    path: string;
    read_only?: boolean;
  };
}
```

---

## Component Architecture

### 1. Database Connection Wizard

Replace the current tab-based form with a multi-step wizard:

```
Step 1: Select Database Type
  → Cards showing supported databases from registry
  → Filter by capability (file vs network)

Step 2: Configure Database
  → Dynamic form based on selected database type fields
  → Only shows fields relevant to that database

Step 3: Choose Connection Method
  → Shows available connection methods for selected database
  → Direct TCP (default), SSH Tunnel, Connection Pooler, etc.

Step 4: Configure Connection Method
  → Shows configuration for selected method
  → SSH tunnel form is reusable across all database types

Step 5: Review & Connect
  → Summary of configuration
  → Test connection before saving
```

### 2. Reusable Connection Method Components

```typescript
// src/components/connection-methods/SshTunnelForm.tsx

export function SshTunnelForm({ config, onChange }: {
  config: SshConfig;
  onChange: (config: SshConfig) => void;
}) {
  return (
    <div className="ssh-tunnel-form">
      <FormField label="SSH Host">
        <Input
          value={config.host}
          onChange={(e) => onChange({...config, host: e.target.value})}
        />
      </FormField>
      {/* ... SSH-specific fields */}
    </div>
  );
}

// This component is reused for PostgreSQL, MySQL, and any future database
```

### 3. Dynamic Database Form

```typescript
// src/components/database/DynamicDatabaseForm.tsx

export function DynamicDatabaseForm({
  databaseType,
  config,
  onChange
}: {
  databaseType: DatabaseType;
  config: DatabaseSpecificConfig;
  onChange: (config: DatabaseSpecificConfig) => void;
}) {
  const metadata = DATABASE_REGISTRY[databaseType];

  return (
    <div className="database-form">
      {metadata.fields.map(field => (
        <FormField
          key={field.name}
          label={field.label}
          type={field.type}
          value={config[field.name]}
          onChange={(value) => onChange({...config, [field.name]: value})}
          required={field.required}
        />
      ))}
    </div>
  );
}
```

---

## Adding a New Database Type

With the new architecture, adding a new database type requires only:

### Backend (Rust)

1. Create a new provider (e.g., `MongoDBProvider`)
2. Implement `DatabaseProvider` trait
3. Add `DatabaseType::MongoDB` variant
4. Add connection resolver if needed

### Frontend (TypeScript)

1. Add to `DatabaseType` enum:

```typescript
export enum DatabaseType {
  SQLite = "SQLite",
  PostgreSQL = "PostgreSQL",
  MySQL = "MySQL",
  MongoDB = "MongoDB", // ← Just add this line
}
```

2. Add to registry:

```typescript
export const DATABASE_REGISTRY: Record<DatabaseType, DatabaseTypeMetadata> = {
  // ... existing databases
  [DatabaseType.MongoDB]: {
    id: DatabaseType.MongoDB,
    displayName: "MongoDB",
    // ... metadata
  },
};
```

3. Update store types (add discriminated union case):

```typescript
export interface DatabaseSpecificConfig {
  // ... existing configs
  MongoDB?: {
    host?: string;
    port?: number;
    database: string;
    username: string;
    // ...
  };
}
```

**No UI changes needed!** The dynamic form and connection method components automatically work with the new database type.

---

## Migration Path

### Phase 1: Add Registry (Backward Compatible)

1. Create `database-registry.ts` alongside existing code
2. Add `SUPPORTED_DATABASE_TYPES` constant
3. Replace hardcoded strings with registry lookups
4. No breaking changes - existing UI still works

### Phase 2: Refactor Forms

1. Extract SSH tunnel form into reusable component
2. Create dynamic form renderer
3. Add multi-step wizard for new connections
4. Keep old forms as fallback

### Phase 3: Migrate Data Structures

1. Update TypeScript types to use discriminated unions
2. Update store to use new config format
3. Add transformation layer for backward compatibility
4. Migrate existing workspace data

### Phase 4: Backend Discovery (Optional)

1. Add Tauri command: `get_supported_databases()`
2. Frontend queries backend on startup
3. UI renders based on backend response
4. Adding new database requires only backend changes

---

## Implementation Priority

### High Priority

| Task                           | Effort | Impact |
| ------------------------------ | ------ | ------ |
| Create database registry       | Low    | High   |
| Add dynamic form rendering     | Medium | High   |
| Extract SSH tunnel component   | Low    | Medium |
| Add connection method selector | Medium | High   |

### Medium Priority

| Task                        | Effort | Impact |
| --------------------------- | ------ | ------ |
| Update TypeScript types     | Low    | Medium |
| Add multi-step wizard       | High   | Medium |
| Update store for new format | Medium | High   |

### Low Priority (Future)

| Task                     | Effort | Impact |
| ------------------------ | ------ | ------ |
| Backend-driven discovery | High   | High   |
| Add test database types  | Low    | Low    |

---

## File Changes Summary

### New Files

| File                                      | Purpose                               |
| ----------------------------------------- | ------------------------------------- |
| `src/types/database-registry.ts`          | Database type metadata and registry   |
| `src/types/connection-methods.ts`         | Connection method types and configs   |
| `src/components/connection-methods/`      | Reusable connection method components |
| `src/components/database/DynamicForm.tsx` | Dynamic form renderer                 |
| `src/components/connection/Wizard.tsx`    | Multi-step connection wizard          |

### Modified Files

| File                                        | Changes                         |
| ------------------------------------------- | ------------------------------- |
| `src/types/workspace.ts`                    | Add discriminated union types   |
| `src/stores/workspace-store.ts`             | Support new config format       |
| `src/components/ConnectionDialogSimple.tsx` | Use dynamic forms               |
| `src/routes/index.tsx`                      | Update connection creation flow |

---

## Design Principles

### 1. Data-Driven UI

UI is generated from metadata, not hardcoded. Adding a database type is a data change, not a code change.

### 2. Separation of Concerns

- **What to connect** = Database-specific configuration
- **How to reach it** = Connection method configuration
- **Who can access** = Authentication configuration

### 3. Reusability

Connection methods (SSH, SSL, etc.) are implemented once and reused across all database types.

### 4. Extensibility

New databases can be added by:

1. Backend: Implement provider + add to registry
2. Frontend: Add metadata to registry (or query backend)

No UI code changes needed for basic cases.

---

## Example: Adding MongoDB

### Backend

```rust
// src-tauri/src/database/providers/mongodb.rs

#[async_trait]
impl DatabaseProvider for MongoDBProvider {
    async fn connect_resolved(
        &mut self,
        target: DatabaseTarget,
        connection: ResolvedConnection,
    ) -> DbResult<()> {
        // MongoDB-specific connection logic
        // Works with any TCP-based connection method!
    }
    // ... other trait methods
}
```

### Frontend

```typescript
// Just add to registry
export const DATABASE_REGISTRY: Record<DatabaseType, DatabaseTypeMetadata> = {
  // ... existing
  [DatabaseType.MongoDB]: {
    id: DatabaseType.MongoDB,
    displayName: "MongoDB",
    icon: "database",
    defaultPort: 27017,
    connectionMode: "network",
    supportedConnectionMethods: [
      ConnectionMethodType.DirectTcp,
      ConnectionMethodType.SshTunnel,
    ],
    fields: [
      { name: "host", label: "Host", type: "text", required: true },
      { name: "port", label: "Port", type: "number", default: 27017 },
      { name: "database", label: "Database", type: "text", required: true },
      { name: "authDatabase", label: "Auth Database", type: "text" },
      { name: "username", label: "Username", type: "text", required: true },
      {
        name: "password",
        label: "Password",
        type: "password",
        required: false,
      },
      {
        name: "authMechanism",
        label: "Auth Mechanism",
        type: "select",
        options: ["SCRAM-SHA-256", "SCRAM-SHA-1", "MONGODB-X509"],
      },
    ],
    capabilities: { supportsSsh: true, supportsSsl: true },
  },
};
```

**Result:** MongoDB immediately available in UI with:

- Dynamic form
- SSH tunnel support
- Connection testing
- No UI code changes beyond adding metadata

---

## Conclusion

The proposed frontend architecture:

1. **Separates concerns** - Database type from connection method
2. **Data-driven UI** - Metadata generates forms automatically
3. **Future-proof** - Add databases by adding metadata, not rewriting UI
4. **Backward compatible** - Can migrate gradually

This aligns with the backend database-agnostic architecture and makes adding new database types a configuration task rather than a development task.
