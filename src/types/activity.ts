/**
 * Activity Dashboard Types
 *
 * Types for tracking and displaying real-time, scheduled, and recent activities
 * in the MesoClaw home page.
 */

/**
 * Activity status indicates the current state of an activity.
 *
 * Active states: running, awaiting, pending, paused
 * Terminal states: success, error, cancelled, terminated, stuck, skipped
 */
export type ActivityStatus =
  // Active states
  | "running" // Currently executing
  | "awaiting" // Waiting for user approval
  | "pending" // Scheduled, not yet started
  | "paused" // Temporarily stopped
  // Terminal states
  | "success" // Completed successfully
  | "error" // Failed with error
  | "cancelled" // User cancelled
  | "terminated" // Forcefully stopped
  | "stuck" // Running too long (timeout)
  | "skipped"; // Skipped (e.g., outside active hours)

/**
 * Source of the activity event.
 */
export type ActivitySource = "agent" | "scheduler" | "system" | "channel";

/**
 * An individual activity entry for the dashboard.
 */
export interface Activity {
  /** Unique identifier */
  id: string;
  /** Source of the activity */
  source: ActivitySource;
  /** Short, actionable title (e.g., "Writing file") */
  title: string;
  /** Current status */
  status: ActivityStatus;
  /** ISO timestamp when activity started */
  startedAt: string;
  /** ISO timestamp when activity completed (optional for active) */
  completedAt?: string;
  /** Navigation path for related page (optional) */
  linkTo?: string;
}

/**
 * A scheduled job that hasn't started yet.
 */
export interface PlannedJob {
  /** Job identifier */
  id: string;
  /** Job name */
  name: string;
  /** ISO timestamp of next scheduled run */
  nextRun: string;
  /** Navigation path to job details */
  linkTo?: string;
}

/**
 * Configuration options for activity dashboard.
 */
export interface ActivityDashboardConfig {
  /** Refresh interval in milliseconds (default: 5000 = 5s) */
  refreshIntervalMs: number;
  /** Rolling window in milliseconds (default: 3600000 = 1h) */
  rollingWindowMs: number;
}

/**
 * Default configuration values.
 */
export const DEFAULT_ACTIVITY_CONFIG: ActivityDashboardConfig = {
  refreshIntervalMs: 5000, // 5 seconds
  rollingWindowMs: 3600000, // 1 hour
};
