/**
 * Centralized Toast Utility
 *
 * This module provides a standardized API for showing toast notifications
 * throughout the application. It wraps Sonner's toast functions with
 * consistent styling and behavior.
 *
 * Usage:
 * ```tsx
 * import { showSuccess, showError, showInfo, showWarning } from "@/lib/toast";
 *
 * showSuccess("Operation completed");
 * showError("Something went wrong", "Please try again");
 * showInfo("Tip: Use keyboard shortcuts");
 * showWarning("This action cannot be undone");
 * ```
 */

import { toast, type ExternalToast } from "sonner";

// Re-export toast for advanced use cases that need direct access
export { toast };

// Types for toast options
export interface ToastOptions extends Omit<ExternalToast, "description"> {
  description?: string;
}

export interface LoadingToastOptions extends ToastOptions {
  /** Custom toast ID for later updates */
  id?: string | number;
}

/**
 * Show a success toast with green styling
 * Use for: completed operations, saved changes, successful connections
 */
export function showSuccess(
  message: string,
  options?: ToastOptions
): string | number {
  const { description, ...rest } = options ?? {};
  return toast.success(message, { description, ...rest });
}

/**
 * Show an error toast with red styling
 * Use for: failed operations, validation errors, connection failures
 */
export function showError(
  message: string,
  options?: ToastOptions
): string | number {
  const { description, ...rest } = options ?? {};
  return toast.error(message, { description, ...rest });
}

/**
 * Show an info toast with blue styling
 * Use for: helpful tips, status updates, non-critical notifications
 */
export function showInfo(
  message: string,
  options?: ToastOptions
): string | number {
  const { description, ...rest } = options ?? {};
  return toast.info(message, { description, ...rest });
}

/**
 * Show a warning toast with amber/yellow styling
 * Use for: caution notices, deprecation warnings, irreversible actions
 */
export function showWarning(
  message: string,
  options?: ToastOptions
): string | number {
  const { description, ...rest } = options ?? {};
  return toast.warning(message, { description, ...rest });
}

/**
 * Show a loading toast with spinner
 * Returns the toast ID for later updates or dismissal
 *
 * Example:
 * ```tsx
 * const toastId = showLoading("Saving...");
 * try {
 *   await save();
 *   updateToast(toastId, { type: "success", message: "Saved!" });
 * } catch {
 *   updateToast(toastId, { type: "error", message: "Failed to save" });
 * }
 * ```
 */
export function showLoading(
  message: string,
  options?: LoadingToastOptions
): string | number {
  const { description, id, ...rest } = options ?? {};
  return toast.loading(message, { description, id, ...rest });
}

/**
 * Dismiss a specific toast by ID
 */
export function dismissToast(toastId?: string | number): void {
  toast.dismiss(toastId);
}

/**
 * Dismiss all toasts
 */
export function dismissAllToasts(): void {
  toast.dismiss();
}

/**
 * Update an existing toast (useful for loading → success/error transitions)
 */
export function updateToast(
  toastId: string | number,
  options: {
    type?: "success" | "error" | "info" | "warning" | "loading";
    message: string;
    description?: string;
    duration?: number;
  }
): void {
  const { type = "success", message, description, duration } = options;

  switch (type) {
    case "success":
      toast.success(message, { id: toastId, description, duration });
      break;
    case "error":
      toast.error(message, { id: toastId, description, duration });
      break;
    case "info":
      toast.info(message, { id: toastId, description, duration });
      break;
    case "warning":
      toast.warning(message, { id: toastId, description, duration });
      break;
    case "loading":
      toast.loading(message, { id: toastId, description, duration });
      break;
  }
}

/**
 * Show a promise-based toast that updates automatically
 *
 * Example:
 * ```tsx
 * await showPromise(saveData(), {
 *   loading: "Saving...",
 *   success: "Saved successfully!",
 *   error: "Failed to save",
 * });
 * ```
 */
export function showPromise<T>(
  promise: Promise<T>,
  options: {
    loading: string;
    success: string | ((data: T) => string);
    error: string | ((error: unknown) => string);
    description?: string;
  }
): string | number {
  return toast.promise(promise, options) as unknown as string | number;
}
