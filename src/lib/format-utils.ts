/**
 * Number and byte formatting utilities.
 *
 * This module provides consistent formatting functions for numbers,
 * file sizes, and percentages used throughout the application.
 */

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
