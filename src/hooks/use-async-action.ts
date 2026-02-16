import { useCallback, useState } from "react";

import { showError, showSuccess } from "@/lib/toast";

/**
 * Hook that wraps async operations with loading state, error handling, and toast notifications
 * @returns A tuple of [withSaving, isSaving] where withSaving executes the action with state management
 */
export function useAsyncAction() {
  const [isSaving, setIsSaving] = useState(false);

  const withSaving = useCallback(
    async <T>(
      action: () => Promise<T>,
      options?: {
        onError?: () => void;
        onSuccess?: () => void;
        errorMessage?: string;
        successMessage?: string;
      }
    ): Promise<T | undefined> => {
      setIsSaving(true);
      try {
        const result = await action();
        if (options?.successMessage) {
          showSuccess(options.successMessage);
        }
        options?.onSuccess?.();
        return result;
      } catch (error) {
        const message = options?.errorMessage ?? "Action failed";
        console.error(message, error);
        showError(message);
        options?.onError?.();
        return undefined;
      } finally {
        setIsSaving(false);
      }
    },
    []
  );

  return [withSaving, isSaving] as const;
}
