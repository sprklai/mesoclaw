/**
 * Dialog state management hook.
 *
 * This hook provides common dialog lifecycle management including:
 * - Loading state during async operations
 * - Test result state for connection testing
 * - Automatic reset when dialog opens
 */

import { useCallback, useEffect, useState } from "react";

/**
 * Result of a test operation (e.g., connection test).
 */
export interface TestResult {
  success: boolean;
  message: string;
}

/**
 * Options for useDialogState hook.
 */
export interface UseDialogStateOptions {
  /** Whether the dialog is open */
  open: boolean;
  /** Optional callback to run when dialog opens */
  onOpen?: () => void;
  /** Optional callback to run when dialog closes */
  onClose?: () => void;
}

/**
 * Return type for useDialogState hook.
 */
export interface DialogState {
  /** Whether an async operation is in progress */
  isLoading: boolean;
  /** Set the loading state */
  setIsLoading: (loading: boolean) => void;
  /** Whether a test operation is in progress */
  isTesting: boolean;
  /** Set the testing state */
  setIsTesting: (testing: boolean) => void;
  /** Result of the last test operation */
  testResult: TestResult | null;
  /** Set the test result */
  setTestResult: (result: TestResult | null) => void;
  /** Reset all dialog state to initial values */
  resetState: () => void;
  /** Wrap an async action with loading state management */
  withLoading: <T>(action: () => Promise<T>) => Promise<T | undefined>;
  /** Wrap a test action with testing state management */
  withTesting: <T>(action: () => Promise<T>) => Promise<T | undefined>;
}

/**
 * Hook for managing common dialog state patterns.
 *
 * @example
 * ```tsx
 * function MyDialog({ open, onOpenChange }) {
 *   const {
 *     isLoading,
 *     isTesting,
 *     testResult,
 *     setTestResult,
 *     withLoading,
 *     withTesting,
 *   } = useDialogState({ open });
 *
 *   const handleTestConnection = async () => {
 *     await withTesting(async () => {
 *       const result = await testConnection(params);
 *       setTestResult({
 *         success: result.success,
 *         message: result.message,
 *       });
 *     });
 *   };
 *
 *   const handleSubmit = async () => {
 *     await withLoading(async () => {
 *       await saveData(formData);
 *       onOpenChange(false);
 *     });
 *   };
 *
 *   return (
 *     <Dialog open={open}>
 *       {testResult && (
 *         <Alert variant={testResult.success ? "success" : "error"}>
 *           {testResult.message}
 *         </Alert>
 *       )}
 *       <Button onClick={handleTestConnection} disabled={isTesting}>
 *         {isTesting ? "Testing..." : "Test Connection"}
 *       </Button>
 *       <Button onClick={handleSubmit} disabled={isLoading}>
 *         {isLoading ? "Saving..." : "Save"}
 *       </Button>
 *     </Dialog>
 *   );
 * }
 * ```
 */
export function useDialogState(options: UseDialogStateOptions): DialogState {
  const { open, onOpen, onClose } = options;

  const [isLoading, setIsLoading] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<TestResult | null>(null);

  // Track previous open state to detect transitions
  const [wasOpen, setWasOpen] = useState(open);

  // Reset state when dialog opens
  useEffect(() => {
    if (open && !wasOpen) {
      // Dialog just opened
      setIsLoading(false);
      setIsTesting(false);
      setTestResult(null);
      onOpen?.();
    } else if (!open && wasOpen) {
      // Dialog just closed
      onClose?.();
    }
    setWasOpen(open);
  }, [open, wasOpen, onOpen, onClose]);

  const resetState = useCallback(() => {
    setIsLoading(false);
    setIsTesting(false);
    setTestResult(null);
  }, []);

  const withLoading = useCallback(
    async <T>(action: () => Promise<T>): Promise<T | undefined> => {
      setIsLoading(true);
      try {
        return await action();
      } catch (error) {
        console.error("Dialog action failed:", error);
        return undefined;
      } finally {
        setIsLoading(false);
      }
    },
    []
  );

  const withTesting = useCallback(
    async <T>(action: () => Promise<T>): Promise<T | undefined> => {
      setIsTesting(true);
      setTestResult(null);
      try {
        return await action();
      } catch (error) {
        setTestResult({
          success: false,
          message: error instanceof Error ? error.message : String(error),
        });
        return undefined;
      } finally {
        setIsTesting(false);
      }
    },
    []
  );

  return {
    isLoading,
    setIsLoading,
    isTesting,
    setIsTesting,
    testResult,
    setTestResult,
    resetState,
    withLoading,
    withTesting,
  };
}
