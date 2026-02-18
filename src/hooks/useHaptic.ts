/**
 * useHaptic — platform-appropriate haptic feedback for mobile.
 *
 * Uses the Web Vibration API as the cross-platform fallback.  On Tauri Mobile
 * targets, a native haptics plugin could be substituted here in a later phase.
 *
 * Usage:
 * ```tsx
 * const { haptic } = useHaptic();
 * <button onClick={() => haptic("light")}>Press me</button>
 * ```
 *
 * Phase 7.3.4 implementation.
 */

export type HapticStyle = "light" | "medium" | "heavy";

interface UseHapticReturn {
  /**
   * Fire a haptic impulse.  Safe to call even when vibration is not supported.
   */
  haptic: (style?: HapticStyle) => void;
  /** Whether any haptic capability is available on this device. */
  isSupported: boolean;
}

/** Vibration durations (ms) for each haptic style. */
const VIBRATION_MS: Record<HapticStyle, number> = {
  light: 10,
  medium: 25,
  heavy: 50,
};

export function useHaptic(): UseHapticReturn {
  const isSupported =
    typeof navigator !== "undefined" && "vibrate" in navigator;

  const haptic = (style: HapticStyle = "light") => {
    if (!isSupported) return;
    try {
      navigator.vibrate(VIBRATION_MS[style]);
    } catch {
      // Silently swallow — not all browsers/platforms honour the API.
    }
  };

  return { haptic, isSupported };
}
