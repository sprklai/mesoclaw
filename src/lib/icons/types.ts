/**
 * Icon Types
 *
 * Shared types for the centralized icon system.
 * Provides type-safe abstractions for icons from different libraries.
 */

import type { LucideIcon } from "lucide-react";
import type { IconType } from "react-icons";

// Re-export LucideIcon for external use
export type { LucideIcon } from "lucide-react";

// Re-export IconType from react-icons
export type { IconType } from "react-icons";

/**
 * Props interface for consistent icon sizing across the app
 */
export interface IconProps {
  /** Icon size in pixels or CSS value */
  size?: number | string;
  /** Additional CSS classes */
  className?: string;
}

/**
 * Union type for any icon component in the system
 * Covers both Lucide icons and react-icons
 */
export type AnyIcon = LucideIcon | IconType;
