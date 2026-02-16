/**
 * Date and time formatting utilities.
 *
 * This module provides consistent date/time formatting functions
 * used throughout the application.
 */

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
  // Unix epoch is January 1, 1970 00:00:00 UTC
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
