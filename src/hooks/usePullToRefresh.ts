/**
 * usePullToRefresh — pull-down gesture to trigger a refresh action.
 *
 * Attach the returned `ref` to a scrollable container.  When the user pulls
 * down (scroll position is at the top) beyond `threshold` pixels, `onRefresh`
 * is called.  The hook exposes `isPulling` and `pullDistance` so the UI can
 * render a visual indicator.
 *
 * Phase 7.3.2 implementation.
 */

import { useCallback, useRef, useState } from "react";

interface UsePullToRefreshOptions {
  /** Callback invoked when the pull gesture completes. */
  onRefresh: () => Promise<void> | void;
  /** Pull distance (px) required to trigger. Default: 64. */
  threshold?: number;
}

interface PullToRefreshHandlers {
  ref: React.RefObject<HTMLDivElement | null>;
  /** True while the user is actively pulling. */
  isPulling: boolean;
  /** True while the refresh callback is in progress. */
  isRefreshing: boolean;
  /** Current pull distance (0 → threshold). */
  pullDistance: number;
}

export function usePullToRefresh({
  onRefresh,
  threshold = 64,
}: UsePullToRefreshOptions): PullToRefreshHandlers {
  const ref = useRef<HTMLDivElement | null>(null);
  const startY = useRef<number | null>(null);
  const [isPulling, setIsPulling] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [pullDistance, setPullDistance] = useState(0);

  const handleTouchStart = useCallback((e: TouchEvent) => {
    const el = ref.current;
    // Only activate when already scrolled to the very top.
    if (!el || el.scrollTop > 0) return;
    startY.current = e.touches[0].clientY;
  }, []);

  const handleTouchMove = useCallback(
    (e: TouchEvent) => {
      if (startY.current === null || isRefreshing) return;
      const dy = e.touches[0].clientY - startY.current;
      if (dy <= 0) {
        setIsPulling(false);
        setPullDistance(0);
        return;
      }
      setIsPulling(true);
      setPullDistance(Math.min(dy, threshold));
    },
    [isRefreshing, threshold],
  );

  const handleTouchEnd = useCallback(async () => {
    if (!isPulling || isRefreshing) {
      startY.current = null;
      return;
    }
    if (pullDistance >= threshold) {
      setIsRefreshing(true);
      try {
        await onRefresh();
      } finally {
        setIsRefreshing(false);
      }
    }
    startY.current = null;
    setIsPulling(false);
    setPullDistance(0);
  }, [isPulling, isRefreshing, pullDistance, threshold, onRefresh]);

  // Attach / detach native touch listeners when ref is assigned.
  const setRef = useCallback(
    (node: HTMLDivElement | null) => {
      if (ref.current) {
        ref.current.removeEventListener("touchstart", handleTouchStart);
        ref.current.removeEventListener("touchmove", handleTouchMove);
        ref.current.removeEventListener("touchend", handleTouchEnd);
      }
      (ref as React.MutableRefObject<HTMLDivElement | null>).current = node;
      if (node) {
        node.addEventListener("touchstart", handleTouchStart, { passive: true });
        node.addEventListener("touchmove", handleTouchMove, { passive: true });
        node.addEventListener("touchend", handleTouchEnd, { passive: true });
      }
    },
    [handleTouchStart, handleTouchMove, handleTouchEnd],
  );

  // Provide a combined ref that both tracks the element and attaches listeners.
  const combinedRef = {
    get current() {
      return ref.current;
    },
    set current(node: HTMLDivElement | null) {
      setRef(node);
    },
  } as React.RefObject<HTMLDivElement | null>;

  return { ref: combinedRef, isPulling, isRefreshing, pullDistance };
}
