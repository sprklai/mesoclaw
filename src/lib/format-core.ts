/**
 * Core formatting utilities.
 *
 * Canonical implementations for number, byte, percentage, date,
 * and time formatting used throughout the application.
 * Other format modules re-export from here to avoid duplication.
 */

// ---------------------------------------------------------------------------
// Number & byte formatting
// ---------------------------------------------------------------------------

/**
 * Format bytes into a human-readable string.
 *
 * Converts bytes to the most appropriate unit (B, KB, MB, GB, TB).
 *
 * @param bytes - Number of bytes, or null/undefined
 * @param decimals - Number of decimal places (default: 2)
 * @returns Formatted string like "1.5 MB", or "N/A" if input is null/undefined
 */
export function formatBytes(
  bytes: number | null | undefined,
  decimals = 2
): string {
  if (bytes === undefined || bytes === null) return "N/A";
  if (bytes === 0) return "0 B";

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ["B", "KB", "MB", "GB", "TB"];

  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const index = Math.min(i, sizes.length - 1);

  // For bytes, show no decimals
  if (index === 0) {
    return `${bytes} B`;
  }

  return `${(bytes / k ** index).toFixed(dm)} ${sizes[index]}`;
}

/**
 * Format a number with locale-aware thousand separators.
 *
 * @param num - Number to format
 * @returns Formatted string with thousand separators (e.g., "1,234,567")
 */
export function formatNumber(num: number | null | undefined): string {
  if (num === undefined || num === null) return "N/A";
  return num.toLocaleString();
}

/**
 * Format a number as a percentage.
 *
 * @param value - Decimal value (e.g., 0.85 for 85%)
 * @param decimals - Number of decimal places (default: 1)
 * @returns Formatted percentage string (e.g., "85.0%")
 */
export function formatPercent(
  value: number | null | undefined,
  decimals = 1
): string {
  if (value === undefined || value === null) return "N/A";
  return `${(value * 100).toFixed(decimals)}%`;
}

/**
 * Format a count with appropriate singular/plural suffix.
 *
 * @param count - Number to format
 * @param singular - Singular form of the word
 * @param plural - Plural form of the word (optional, defaults to singular + "s")
 * @returns Formatted string like "1 table" or "5 tables"
 */
export function formatCount(
  count: number,
  singular: string,
  plural?: string
): string {
  const pluralForm = plural ?? `${singular}s`;
  return `${formatNumber(count)} ${count === 1 ? singular : pluralForm}`;
}

// ---------------------------------------------------------------------------
// Date & time formatting
// ---------------------------------------------------------------------------

/**
 * Format a cache timestamp for display.
 *
 * Returns a human-readable relative time string like "Just now", "5m ago", "2h ago",
 * or a date string for older timestamps. Returns empty string for invalid timestamps.
 *
 * @param timestamp - Unix timestamp in milliseconds, or null
 * @returns Formatted relative time string, or empty string if invalid
 */
export function formatCacheTimestamp(timestamp: number | null): string {
  if (!timestamp) return "";

  const date = new Date(timestamp);
  const now = new Date();

  // Check if the date is invalid or from epoch (1970)
  const epochTime = new Date(0).getTime();
  // Reject timestamps from 1970 or before January 1, 2020 (reasonable minimum)
  const minimumValidTime = new Date("2020-01-01").getTime();

  if (date.getTime() <= epochTime || date.getTime() < minimumValidTime) {
    return "";
  }

  // Also reject future timestamps
  if (date.getTime() > now.getTime()) {
    return "";
  }

  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  return date.toLocaleDateString();
}

/**
 * Format a date string for display.
 *
 * Converts ISO date strings to a localized date format.
 *
 * @param dateString - ISO date string (e.g., "2024-01-15T10:30:00Z")
 * @returns Formatted date string, or the original string if parsing fails
 */
export function formatDate(dateString: string): string {
  try {
    const date = new Date(dateString);
    if (Number.isNaN(date.getTime())) {
      return dateString;
    }
    return date.toLocaleDateString();
  } catch {
    return dateString;
  }
}

/**
 * Format a date string as a relative time.
 *
 * Converts ISO date strings to relative time like "2 hours ago", "3 days ago".
 *
 * @param dateString - ISO date string (e.g., "2024-01-15T10:30:00Z")
 * @returns Relative time string, or formatted date for old timestamps
 */
export function formatRelativeTime(dateString: string): string {
  try {
    const date = new Date(dateString);
    if (Number.isNaN(date.getTime())) {
      return dateString;
    }

    const now = new Date();
    const diffMs = now.getTime() - date.getTime();

    // Handle future dates
    if (diffMs < 0) {
      return date.toLocaleDateString();
    }

    const diffSecs = Math.floor(diffMs / 1000);
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffSecs < 60) return "Just now";
    if (diffMins < 60)
      return `${diffMins} minute${diffMins === 1 ? "" : "s"} ago`;
    if (diffHours < 24)
      return `${diffHours} hour${diffHours === 1 ? "" : "s"} ago`;
    if (diffDays < 7) return `${diffDays} day${diffDays === 1 ? "" : "s"} ago`;
    if (diffDays < 30) {
      const weeks = Math.floor(diffDays / 7);
      return `${weeks} week${weeks === 1 ? "" : "s"} ago`;
    }

    return date.toLocaleDateString();
  } catch {
    return dateString;
  }
}
