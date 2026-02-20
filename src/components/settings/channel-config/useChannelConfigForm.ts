/**
 * useChannelConfigForm — shared state management for channel configuration panels.
 *
 * Extracts the common draft/test/save pattern used by TelegramConfig, DiscordConfig,
 * MatrixConfig, and SlackConfig into a single reusable hook.
 */

import { useState } from "react";

import { useChannelStore } from "@/stores/channelStore";

interface UseChannelConfigFormOptions<C extends object> {
  config: C;
  channelType: string;
  updateFn: (config: C) => Promise<void>;
  fieldTransforms?: Partial<Record<keyof C, (val: string) => unknown>>;
  /** Custom test function. If provided, overrides the default store testConnection. */
  testFn?: (draft: C) => Promise<boolean>;
}

export function useChannelConfigForm<C extends object>(
  options: UseChannelConfigFormOptions<C>,
) {
  const { config, channelType, updateFn, fieldTransforms, testFn } = options;
  const { testConnection } = useChannelStore();
  const [draft, setDraft] = useState<C>(config);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<"ok" | "fail" | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const handleChange =
    (field: keyof C) => (e: React.ChangeEvent<HTMLInputElement>) => {
      const transform = fieldTransforms?.[field];
      const value = transform ? transform(e.target.value) : e.target.value;
      setDraft((prev) => ({ ...prev, [field]: value }));
      setTestResult(null);
    };

  const handleTest = async () => {
    setIsTesting(true);
    setTestResult(null);
    try {
      const ok = testFn
        ? await testFn(draft)
        : await testConnection(channelType);
      setTestResult(ok ? "ok" : "fail");
    } catch {
      setTestResult("fail");
    }
    setIsTesting(false);
  };

  const handleSave = async () => {
    setIsSaving(true);
    await updateFn(draft);
    setIsSaving(false);
  };

  return {
    draft,
    isTesting,
    testResult,
    isSaving,
    handleChange,
    handleTest,
    handleSave,
  };
}
