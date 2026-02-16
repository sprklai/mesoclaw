/**
 * Database Icons (React Icons)
 *
 * Brand-specific icons for database types from react-icons/si (Simple Icons).
 * Provides visual recognition for different database systems.
 */

import type { LucideIcon } from "lucide-react";
import type { IconType } from "react-icons";

import {
  SiCockroachlabs,
  SiMariadb,
  SiMongodb,
  SiMysql,
  SiPlanetscale,
  SiPostgresql,
  SiSqlite,
  SiSupabase,
} from "react-icons/si";

import { DatabaseType } from "@/types/database-registry";

import { Database, Zap } from "./ui-icons";

// ============================================
// INDIVIDUAL DATABASE ICON EXPORTS
// ============================================

// Implemented databases
export { SiSqlite } from "react-icons/si";
export { SiPostgresql } from "react-icons/si";
export { SiMysql } from "react-icons/si";
export { SiMongodb } from "react-icons/si";

// Coming soon databases
export { SiSupabase } from "react-icons/si";
export { SiCockroachlabs } from "react-icons/si";
export { SiMariadb } from "react-icons/si";
export { SiPlanetscale } from "react-icons/si";

// ============================================
// DATABASE ICON REGISTRY
// ============================================

/**
 * Mapping of database types to their brand icons.
 * Uses Lucide fallbacks for databases without official brand icons.
 */
export const DATABASE_ICONS: Record<DatabaseType, IconType | LucideIcon> = {
  // Implemented
  [DatabaseType.SQLite]: SiSqlite,
  [DatabaseType.PostgreSQL]: SiPostgresql,
  [DatabaseType.MySQL]: SiMysql,
  [DatabaseType.MongoDB]: SiMongodb,

  // Coming Soon - Local
  [DatabaseType.DuckDB]: Database, // No official icon, use generic

  // Coming Soon - PostgreSQL Family
  [DatabaseType.Supabase]: SiSupabase,
  [DatabaseType.Neon]: Zap, // Neon uses a lightning bolt style logo
  [DatabaseType.CockroachDB]: SiCockroachlabs,

  // Coming Soon - MySQL Family
  [DatabaseType.MariaDB]: SiMariadb,
  [DatabaseType.PlanetScale]: SiPlanetscale,
  [DatabaseType.TiDB]: Database, // No official icon, use generic
};

/**
 * Get the icon component for a database type.
 *
 * @param dbType - The database type to get the icon for
 * @returns The icon component for the database type
 *
 * @example
 * ```tsx
 * import { getDatabaseIcon, DatabaseType } from '@/lib/icons';
 *
 * function DatabaseLabel({ type }: { type: DatabaseType }) {
 *   const Icon = getDatabaseIcon(type);
 *   return <Icon className="h-4 w-4" />;
 * }
 * ```
 */
export function getDatabaseIcon(dbType: DatabaseType): IconType | LucideIcon {
  return DATABASE_ICONS[dbType] ?? Database;
}

/**
 * Database brand colors for styling icons.
 * Can be used with inline styles or CSS variables.
 */
export const DATABASE_BRAND_COLORS: Partial<Record<DatabaseType, string>> = {
  [DatabaseType.SQLite]: "#003B57",
  [DatabaseType.PostgreSQL]: "#4169E1",
  [DatabaseType.MySQL]: "#4479A1",
  [DatabaseType.MongoDB]: "#47A248",
  [DatabaseType.Supabase]: "#3ECF8E",
  [DatabaseType.CockroachDB]: "#6933FF",
  [DatabaseType.MariaDB]: "#003545",
  [DatabaseType.PlanetScale]: "#000000",
  [DatabaseType.Neon]: "#00E699",
  [DatabaseType.DuckDB]: "#FFF000",
  [DatabaseType.TiDB]: "#FF3232",
};

/**
 * Get the brand color for a database type.
 *
 * @param dbType - The database type to get the color for
 * @returns The brand color hex code, or undefined if no color defined
 */
export function getDatabaseBrandColor(
  dbType: DatabaseType
): string | undefined {
  return DATABASE_BRAND_COLORS[dbType];
}
