/**
 * Code Language Detection Utility
 *
 * Auto-detects code language for syntax highlighting in chat messages.
 * Supports SQL and common programming languages.
 */

export type DetectedLanguage = "sql" | "javascript" | "json" | "text";

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
 * Infers code language from content.
 *
 * Priority:
 * 1. If language explicitly declared in markdown (e.g., ```sql), use that
 * 2. Otherwise, auto-detect from code content patterns
 *
 * @param code - The code content to analyze
 * @param declaredLanguage - Language declared in markdown fence (e.g., "sql", "javascript")
 * @returns Detected language for syntax highlighting
 */
export function inferCodeLanguage(
  code: string,
  declaredLanguage: string
): DetectedLanguage {
  // Normalize declared language
  const normalizedDeclared = declaredLanguage.toLowerCase().trim();

  // Priority 1: Explicit language declaration
  if (normalizedDeclared && normalizedDeclared !== "text") {
    if (
      ["sql", "postgresql", "mysql", "sqlite"].includes(normalizedDeclared)
    ) {
      return "sql";
    }
    if (["js", "javascript", "typescript", "ts"].includes(normalizedDeclared)) {
      return "javascript";
    }
    if (normalizedDeclared === "json") {
      return "json";
    }
  }

  // Priority 2: Content-based detection
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
    javascript: "JavaScript",
    json: "JSON",
    text: "Text",
  };
  return names[language] || language.toUpperCase();
}
