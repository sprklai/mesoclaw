/**
 * Store utilities for Zustand state management.
 *
 * This module provides helper functions to reduce boilerplate in Zustand stores,
 * particularly for async operations with loading/error state management.
 */

import { extractErrorMessage } from "@/lib/error-utils";

/**
 * Standard loading state fields used by most stores.
 */
export interface LoadingState {
  isLoading: boolean;
  error: string | null;
}

/**
 * Extended loading state with refresh indicator.
 */
export interface RefreshableLoadingState extends LoadingState {
  isRefreshing?: boolean;
}

/**
 * Type for Zustand's set function.
 */
// biome-ignore lint/suspicious/noExplicitAny: Zustand set accepts broad partial types
export type StoreSetFn = (...args: any[]) => void;

/**
 * Options for withStoreLoading helper.
 */
export interface WithStoreLoadingOptions<TResult> {
  /** Called when the action succeeds */
  onSuccess?: (result: TResult) => void;
  /** Called when the action fails */
  onError?: (error: Error) => void;
  /** Whether to rethrow the error after handling */
  rethrow?: boolean;
  /** Use isRefreshing instead of isLoading (for refresh actions) */
  isRefresh?: boolean;
  /** Custom name for the loading field (default: "isLoading"). */
  loadingKey?: string;
  /** Custom loading state to merge before action */
  beforeState?: Record<string, unknown>;
  /** Custom success state to merge after action (in addition to loading: false) */
  afterState?: Record<string, unknown>;
}

/**
 * Wraps an async store action with standard loading/error state management.
 *
 * This reduces boilerplate by automatically:
 * - Setting the loading key to true, error: null before the action
 * - Setting the loading key to false on success or failure
 * - Setting error message on failure
 *
 * @example
 * ```ts
 * // Before (repeated ~15-20 lines per action)
 * fetchData: async () => {
 *   set({ isLoading: true, error: null });
 *   try {
 *     const result = await invoke<Data>("get_data");
 *     set({ data: result, isLoading: false });
 *   } catch (error) {
 *     set({
 *       error: error instanceof Error ? error.message : String(error),
 *       isLoading: false,
 *     });
 *   }
 * }
 *
 * // After (3-5 lines)
 * fetchData: async () => {
 *   await withStoreLoading(set, async () => {
 *     const result = await invoke<Data>("get_data");
 *     set({ data: result });
 *     return result;
 *   });
 * }
 *
 * // With custom loading key (e.g. "loading" instead of "isLoading")
 * fetchData: async () => {
 *   await withStoreLoading(set, async () => {
 *     const result = await invoke<Data>("get_data");
 *     set({ data: result });
 *     return result;
 *   }, { loadingKey: "loading" });
 * }
 * ```
 */
export async function withStoreLoading<TResult>(
  set: StoreSetFn,
  action: () => Promise<TResult>,
  options: WithStoreLoadingOptions<TResult> = {}
): Promise<TResult | undefined> {
  const {
    onSuccess,
    onError,
    rethrow = false,
    isRefresh = false,
    loadingKey: customLoadingKey,
    beforeState = {},
    afterState = {},
  } = options;

  // Set loading state
  const loadingKey = customLoadingKey ?? (isRefresh ? "isRefreshing" : "isLoading");
  set({
    [loadingKey]: true,
    error: null,
    ...beforeState,
  });

  try {
    const result = await action();

    // Set success state
    set({
      [loadingKey]: false,
      ...afterState,
    });

    onSuccess?.(result);
    return result;
  } catch (error) {
    const errorMessage = extractErrorMessage(error);

    // Set error state
    set({
      error: errorMessage,
      [loadingKey]: false,
    });

    onError?.(error instanceof Error ? error : new Error(errorMessage));

    if (rethrow) {
      throw error;
    }

    return undefined;
  }
}

/**
 * Create initial loading state for a store.
 *
 * @returns Default loading state object
 */
export function createInitialLoadingState(): LoadingState {
  return {
    isLoading: false,
    error: null,
  };
}

/**
 * Create initial refreshable loading state for a store.
 *
 * @returns Default refreshable loading state object
 */
export function createInitialRefreshableState(): RefreshableLoadingState {
  return {
    isLoading: false,
    isRefreshing: false,
    error: null,
  };
}
