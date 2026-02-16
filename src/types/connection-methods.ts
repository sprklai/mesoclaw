/**
 * Connection Method Types
 *
 * This module defines types for database-agnostic connection methods.
 * Connection methods are now separate from database types, making them
 * reusable across all supported databases.
 */

import { z } from "zod";

import type { SshConfig } from "./workspace";

import { ConnectionMethodType, DatabaseType } from "./database-registry";

/**
 * Direct TCP connection configuration
 */
export interface DirectTcpConfig {
  host: string;
  port: number;
}

/**
 * SSH Tunnel connection configuration
 */
export interface SshTunnelConfig {
  bastion: SshConfig;
  targetHost: string;
  targetPort: number;
}

/**
 * Local file connection configuration
 */
export interface LocalFileConfig {
  path: string;
  readOnly?: boolean;
}

/**
 * Connection Pooler configuration (future)
 */
export interface ConnectionPoolerConfig {
  url: string;
}

/**
 * Connection method configuration (discriminated union)
 */
export type ConnectionMethodConfig =
  | { method: ConnectionMethodType.DirectTcp; config: DirectTcpConfig }
  | { method: ConnectionMethodType.SshTunnel; config: SshTunnelConfig }
  | { method: ConnectionMethodType.LocalFile; config: LocalFileConfig };

/**
 * Database-specific configuration
 */
export interface DatabaseSpecificConfig {
  SQLite?: {
    path: string;
    readOnly?: boolean;
  };
  PostgreSQL?: {
    host?: string;
    port?: number;
    database: string;
    username: string;
    password?: string;
    useSsl?: boolean;
  };
  MySQL?: {
    host?: string;
    port?: number;
    database: string;
    username: string;
    password?: string;
    useSsl?: boolean;
  };
}

/**
 * Complete database connection configuration (new format)
 */
export interface DatabaseConnectionConfig {
  databaseType: DatabaseType;
  database: DatabaseSpecificConfig;
  connectionMethod: ConnectionMethodConfig;
}

/**
 * Legacy connection configuration format (for backward compatibility)
 * @deprecated Use DatabaseConnectionConfig instead
 */
export interface LegacyConnectionConfig {
  SQLite?: {
    path: string;
    read_only?: boolean;
  };
  PostgreSQL?: {
    host: string;
    port: number;
    database: string;
    username: string;
    password?: string;
    use_ssl?: boolean;
    ssh_tunnel?: SshConfig;
  };
  MySQL?: {
    host: string;
    port: number;
    database: string;
    username: string;
    password?: string;
    use_ssl?: boolean;
    ssh_tunnel?: SshConfig;
  };
}

/**
 * Zod schema for Direct TCP configuration
 */
export const directTcpConfigSchema = z.object({
  host: z.string().min(1, "Host is required"),
  port: z.number().int().positive().default(5432),
});

/**
 * Zod schema for SSH Tunnel configuration
 */
export const sshTunnelConfigSchema = z.object({
  bastion: z.object({
    host: z.string().min(1, "SSH host is required"),
    port: z.number().int().positive().default(22),
    username: z.string().min(1, "SSH username is required"),
    auth: z.object({
      type: z.enum(["Password", "PrivateKey"]),
      // Password fields are handled separately (keyring)
    }),
  }),
  targetHost: z.string().min(1, "Target host is required"),
  targetPort: z.number().int().positive(),
});

/**
 * Zod schema for Local File configuration
 */
export const localFileConfigSchema = z.object({
  path: z.string().min(1, "File path is required"),
  readOnly: z.boolean().default(false),
});

/**
 * Zod schema for database connection configuration (new format)
 */
export function createDatabaseConnectionConfigSchema(dbType: DatabaseType) {
  const baseSchema = z.object({
    databaseType: z.nativeEnum(DatabaseType),
  });

  // Add database-specific fields
  let databaseSchema;
  switch (dbType) {
    case "SQLite":
      databaseSchema = z.object({
        path: z.string().min(1, "Database path is required"),
        readOnly: z.boolean().default(false),
      });
      break;
    case "PostgreSQL":
      databaseSchema = z.object({
        host: z.string().optional(),
        port: z.number().int().positive().optional(),
        database: z.string().min(1, "Database name is required"),
        username: z.string().min(1, "Username is required"),
        useSsl: z.boolean().default(false),
      });
      break;
    case "MySQL":
      databaseSchema = z.object({
        host: z.string().optional(),
        port: z.number().int().positive().optional(),
        database: z.string().min(1, "Database name is required"),
        username: z.string().min(1, "Username is required"),
        useSsl: z.boolean().default(false),
      });
      break;
  }

  // Add connection method
  const connectionMethodSchema = z.object({
    method: z.nativeEnum(ConnectionMethodType),
    // Config is validated based on method type
  });

  return baseSchema.and(
    z.object({
      database: databaseSchema,
      connectionMethod: connectionMethodSchema,
    })
  );
}

/**
 * Convert legacy config to new format
 */
export function convertLegacyToNewConfig(
  legacy: LegacyConnectionConfig
): DatabaseConnectionConfig | null {
  if (legacy.SQLite) {
    return {
      databaseType: DatabaseType.SQLite,
      database: {
        SQLite: {
          path: legacy.SQLite.path,
          readOnly: legacy.SQLite.read_only ?? false,
        },
      } as DatabaseSpecificConfig,
      connectionMethod: {
        method: ConnectionMethodType.LocalFile,
        config: {
          path: legacy.SQLite.path,
          readOnly: legacy.SQLite.read_only ?? false,
        },
      },
    };
  }

  if (legacy.PostgreSQL) {
    const pg = legacy.PostgreSQL;
    const method = pg.ssh_tunnel
      ? ConnectionMethodType.SshTunnel
      : ConnectionMethodType.DirectTcp;

    let config: ConnectionMethodConfig;
    if (method === ConnectionMethodType.SshTunnel && pg.ssh_tunnel) {
      config = {
        method: ConnectionMethodType.SshTunnel,
        config: {
          bastion: pg.ssh_tunnel,
          targetHost: pg.host,
          targetPort: pg.port,
        },
      };
    } else {
      config = {
        method: ConnectionMethodType.DirectTcp,
        config: {
          host: pg.host,
          port: pg.port,
        },
      };
    }

    return {
      databaseType: DatabaseType.PostgreSQL,
      database: {
        PostgreSQL: {
          host: pg.host,
          port: pg.port,
          database: pg.database,
          username: pg.username,
          password: pg.password,
          useSsl: pg.use_ssl ?? false,
        },
      } as DatabaseSpecificConfig,
      connectionMethod: config,
    };
  }

  if (legacy.MySQL) {
    const mysql = legacy.MySQL;
    const method = mysql.ssh_tunnel
      ? ConnectionMethodType.SshTunnel
      : ConnectionMethodType.DirectTcp;

    let config: ConnectionMethodConfig;
    if (method === ConnectionMethodType.SshTunnel && mysql.ssh_tunnel) {
      config = {
        method: ConnectionMethodType.SshTunnel,
        config: {
          bastion: mysql.ssh_tunnel,
          targetHost: mysql.host,
          targetPort: mysql.port,
        },
      };
    } else {
      config = {
        method: ConnectionMethodType.DirectTcp,
        config: {
          host: mysql.host,
          port: mysql.port,
        },
      };
    }

    return {
      databaseType: DatabaseType.MySQL,
      database: {
        MySQL: {
          host: mysql.host,
          port: mysql.port,
          database: mysql.database,
          username: mysql.username,
          password: mysql.password,
          useSsl: mysql.use_ssl ?? false,
        },
      } as DatabaseSpecificConfig,
      connectionMethod: config,
    };
  }

  return null;
}

/**
 * Convert new config to legacy format
 */
export function convertNewToLegacyConfig(
  config: DatabaseConnectionConfig
): LegacyConnectionConfig {
  if (config.databaseType === DatabaseType.SQLite) {
    const db = config.database.SQLite!;
    return {
      SQLite: {
        path: db.path,
        read_only: db.readOnly ?? false,
      },
    };
  }

  // PostgreSQL and PostgreSQL-compatible variants (Supabase, Neon, CockroachDB)
  if (
    config.databaseType === DatabaseType.PostgreSQL ||
    config.databaseType === DatabaseType.Supabase ||
    config.databaseType === DatabaseType.Neon ||
    config.databaseType === DatabaseType.CockroachDB
  ) {
    const db = config.database.PostgreSQL!;
    const method = config.connectionMethod;

    let ssh_tunnel: SshConfig | undefined;
    if (method.method === ConnectionMethodType.SshTunnel) {
      ssh_tunnel = method.config.bastion;
    }

    const host =
      method.method === ConnectionMethodType.SshTunnel
        ? method.config.targetHost
        : (db.host ?? "localhost");

    const port =
      method.method === ConnectionMethodType.SshTunnel
        ? method.config.targetPort
        : (db.port ?? 5432);

    return {
      PostgreSQL: {
        host,
        port,
        database: db.database,
        username: db.username,
        password: db.password,
        use_ssl: db.useSsl ?? false,
        ssh_tunnel,
      },
    };
  }

  // MySQL and MySQL-compatible variants (MariaDB, PlanetScale)
  if (
    config.databaseType === DatabaseType.MySQL ||
    config.databaseType === DatabaseType.MariaDB ||
    config.databaseType === DatabaseType.PlanetScale
  ) {
    const db = config.database.MySQL!;
    const method = config.connectionMethod;

    let ssh_tunnel: SshConfig | undefined;
    if (method.method === ConnectionMethodType.SshTunnel) {
      ssh_tunnel = method.config.bastion;
    }

    const host =
      method.method === ConnectionMethodType.SshTunnel
        ? method.config.targetHost
        : (db.host ?? "localhost");

    const port =
      method.method === ConnectionMethodType.SshTunnel
        ? method.config.targetPort
        : (db.port ?? 3306);

    return {
      MySQL: {
        host,
        port,
        database: db.database,
        username: db.username,
        password: db.password,
        use_ssl: db.useSsl ?? false,
        ssh_tunnel,
      },
    };
  }

  throw new Error(`Unsupported database type: ${config.databaseType}`);
}
