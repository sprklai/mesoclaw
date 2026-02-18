/**
 * useVirtualKeyboard — handle the software keyboard on mobile WebViews.
 *
 * Listens to `window.visualViewport` resize events to detect when the
 * on-screen keyboard opens or closes, then:
 *
 * 1. Exposes the current `keyboardHeight` in pixels.
 * 2. Sets `--keyboard-height` CSS custom property on `<html>` so layout
 *    elements can use it directly (e.g. `padding-bottom: var(--keyboard-height)`).
 * 3. Calls `onKeyboardOpen` / `onKeyboardClose` callbacks for imperative
 *    actions (e.g. scroll a chat container to the bottom).
 *
 * Phase 7.3.3 implementation.
 */

import { useEffect, useState } from "react";

interface UseVirtualKeyboardOptions {
  onKeyboardOpen?: () => void;
  onKeyboardClose?: () => void;
}

interface VirtualKeyboardState {
  /** Height of the software keyboard in CSS pixels (0 when closed). */
  keyboardHeight: number;
  /** True while the keyboard is visible. */
  isKeyboardOpen: boolean;
}

export function useVirtualKeyboard({
  onKeyboardOpen,
  onKeyboardClose,
}: UseVirtualKeyboardOptions = {}): VirtualKeyboardState {
  const [keyboardHeight, setKeyboardHeight] = useState(0);

  useEffect(() => {
    const vv = window.visualViewport;
    if (!vv) return;

    const handle = () => {
      // The keyboard height is the difference between the window's inner height
      // and the visual viewport height.
      const height = Math.max(0, window.innerHeight - vv.height);
      setKeyboardHeight(height);

      // Expose as CSS custom property for layout use.
      document.documentElement.style.setProperty(
        "--keyboard-height",
        `${height}px`,
      );

      if (height > 0) {
        onKeyboardOpen?.();
      } else {
        onKeyboardClose?.();
      }
    };

    vv.addEventListener("resize", handle);
    // Initialise on mount.
    handle();

    return () => {
      vv.removeEventListener("resize", handle);
      document.documentElement.style.removeProperty("--keyboard-height");
    };
  }, [onKeyboardOpen, onKeyboardClose]);

  return { keyboardHeight, isKeyboardOpen: keyboardHeight > 0 };
}
