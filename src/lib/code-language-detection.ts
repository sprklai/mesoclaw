/**
 * Code Language Detection Utility
 *
 * Auto-detects code language for syntax highlighting in chat messages.
 * Supports SQL (PostgreSQL, MySQL, SQLite) and NoSQL (MongoDB) databases.
 */

export type DatabaseType = "MongoDB" | "PostgreSQL" | "MySQL" | "SQLite";
export type DetectedLanguage =
  | "sql"
  | "mongodb"
  | "javascript"
  | "json"
  | "text";

// SQL keyword patterns (case-insensitive)
const SQL_KEYWORDS = [
  "SELECT",
  "INSERT",
  "UPDATE",
  "DELETE",
  "FROM",
  "WHERE",
  "JOIN",
  "LEFT JOIN",
  "RIGHT JOIN",
  "INNER JOIN",
  "OUTER JOIN",
  "GROUP BY",
  "ORDER BY",
  "HAVING",
  "LIMIT",
  "OFFSET",
  "CREATE TABLE",
  "ALTER TABLE",
  "DROP TABLE",
  "CREATE INDEX",
  "DROP INDEX",
  "BEGIN",
  "COMMIT",
  "ROLLBACK",
  "UNION",
  "INTERSECT",
  "EXCEPT",
];

// MongoDB operator patterns
const MONGODB_PATTERNS = [
  /\$match\b/,
  /\$group\b/,
  /\$project\b/,
  /\$sort\b/,
  /\$limit\b/,
  /\$skip\b/,
  /\$unwind\b/,
  /\$lookup\b/,
  /\$aggregate\b/,
  /\$push\b/,
  /\$pull\b/,
  /\$addToSet\b/,
  /\$set\b/,
  /\$unset\b/,
  /\$inc\b/,
  /\$gt\b/,
  /\$gte\b/,
  /\$lt\b/,
  /\$lte\b/,
  /\$eq\b/,
  /\$ne\b/,
  /\$in\b/,
  /\$nin\b/,
  /\$and\b/,
  /\$or\b/,
  /\$not\b/,
  /\$nor\b/,
  /\$exists\b/,
  /\$regex\b/,
  /ObjectId\s*\(/,
  /db\.\w+\.(find|insert|update|delete|aggregate)/,
];

/**
 * Checks if code contains SQL keywords.
 * Uses word boundary matching to avoid false positives.
 */
function containsSqlKeywords(code: string): boolean {
  const upperCode = code.toUpperCase();
  return SQL_KEYWORDS.some((keyword) => {
    const regex = new RegExp(`\\b${keyword}\\b`);
    return regex.test(upperCode);
  });
}

/**
 * Checks if code contains MongoDB patterns.
 */
function containsMongoPatterns(code: string): boolean {
  return MONGODB_PATTERNS.some((pattern) => pattern.test(code));
}

/**
 * Checks if code is valid JSON structure.
 */
function isJsonStructure(code: string): boolean {
  const trimmed = code.trim();
  if (
    (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
    (trimmed.startsWith("[") && trimmed.endsWith("]"))
  ) {
    try {
      JSON.parse(trimmed);
      return true;
    } catch {
      return false;
    }
  }
  return false;
}

/**
 * Infers code language from content, with optional hints.
 *
 * Priority:
 * 1. If language explicitly declared in markdown (e.g., ```sql), use that
 * 2. If workspace has active database, use its type as hint
 * 3. Otherwise, auto-detect from code content patterns
 *
 * @param code - The code content to analyze
 * @param declaredLanguage - Language declared in markdown fence (e.g., "sql", "javascript")
 * @param databaseType - Optional database type hint from workspace
 * @returns Detected language for syntax highlighting
 */
export function inferCodeLanguage(
  code: string,
  declaredLanguage: string,
  databaseType?: DatabaseType
): DetectedLanguage {
  // Normalize declared language
  const normalizedDeclared = declaredLanguage.toLowerCase().trim();

  // Priority 1: Explicit language declaration
  if (normalizedDeclared && normalizedDeclared !== "text") {
    // Map common aliases
    if (["sql", "postgresql", "mysql", "sqlite"].includes(normalizedDeclared)) {
      return "sql";
    }
    if (["mongodb", "mongo"].includes(normalizedDeclared)) {
      return "mongodb";
    }
    if (["js", "javascript", "typescript", "ts"].includes(normalizedDeclared)) {
      return "javascript";
    }
    if (normalizedDeclared === "json") {
      return "json";
    }
  }

  // Priority 2: Database type hint
  if (databaseType) {
    if (databaseType === "MongoDB") {
      // Still check if it looks like MongoDB syntax
      if (containsMongoPatterns(code) || isJsonStructure(code)) {
        return containsMongoPatterns(code) ? "mongodb" : "json";
      }
    } else {
      // SQL databases
      if (containsSqlKeywords(code)) {
        return "sql";
      }
    }
  }

  // Priority 3: Content-based detection
  if (containsMongoPatterns(code)) {
    return "mongodb";
  }

  if (containsSqlKeywords(code)) {
    return "sql";
  }

  if (isJsonStructure(code)) {
    return "json";
  }

  // Default to text
  return "text";
}

/**
 * Language display colors for code blocks.
 * Consistent with the design system.
 */
export const languageColors: Record<DetectedLanguage, string> = {
  sql: "text-blue-400",
  mongodb: "text-emerald-400",
  javascript: "text-yellow-300",
  json: "text-green-300",
  text: "text-foreground",
};

/**
 * Gets display name for a language.
 */
export function getLanguageDisplayName(language: DetectedLanguage): string {
  const names: Record<DetectedLanguage, string> = {
    sql: "SQL",
    mongodb: "MongoDB",
    javascript: "JavaScript",
    json: "JSON",
    text: "Text",
  };
  return names[language] || language.toUpperCase();
}
