/**
 * useMobileSwipe — detect horizontal swipe gestures for mobile sidebar control.
 *
 * Returns an object with event handlers to attach to the root element:
 *
 * - Swipe right from the left edge (< 30 px) → call `onSwipeRight`
 * - Swipe left anywhere → call `onSwipeLeft`
 *
 * The gesture fires only when the horizontal delta exceeds `threshold` (50 px)
 * and the vertical drift stays below `maxVertical` (30 px).
 *
 * Phase 7.3.1 implementation.
 */

import { useCallback, useRef } from "react";

interface UseMobileSwipeOptions {
  /** Minimum horizontal travel (px) before the gesture fires. Default: 50. */
  threshold?: number;
  /** Maximum vertical drift (px) allowed. Default: 30. */
  maxVertical?: number;
  /** Left-edge region (px) that triggers an open swipe. Default: 30. */
  edgeWidth?: number;
  onSwipeRight?: () => void;
  onSwipeLeft?: () => void;
}

interface SwipeHandlers {
  onTouchStart: (e: React.TouchEvent) => void;
  onTouchEnd: (e: React.TouchEvent) => void;
}

export function useMobileSwipe({
  threshold = 50,
  maxVertical = 30,
  edgeWidth = 30,
  onSwipeRight,
  onSwipeLeft,
}: UseMobileSwipeOptions = {}): SwipeHandlers {
  const touchStart = useRef<{ x: number; y: number; fromEdge: boolean } | null>(
    null,
  );

  const onTouchStart = useCallback((e: React.TouchEvent) => {
    const touch = e.touches[0];
    touchStart.current = {
      x: touch.clientX,
      y: touch.clientY,
      fromEdge: touch.clientX < edgeWidth,
    };
  }, [edgeWidth]);

  const onTouchEnd = useCallback(
    (e: React.TouchEvent) => {
      if (!touchStart.current) return;
      const touch = e.changedTouches[0];
      const dx = touch.clientX - touchStart.current.x;
      const dy = Math.abs(touch.clientY - touchStart.current.y);

      // Reject if vertical drift exceeds limit (likely a scroll gesture).
      if (dy > maxVertical) {
        touchStart.current = null;
        return;
      }

      if (dx > threshold && touchStart.current.fromEdge) {
        onSwipeRight?.();
      } else if (dx < -threshold) {
        onSwipeLeft?.();
      }

      touchStart.current = null;
    },
    [threshold, maxVertical, onSwipeRight, onSwipeLeft],
  );

  return { onTouchStart, onTouchEnd };
}
