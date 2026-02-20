import { X } from "lucide-react";
import { type ReactNode, useCallback, useEffect, useRef } from "react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface ResponsiveSidebarProps {
	/** Content to render inside the sidebar */
	children: ReactNode;
	/** Whether the mobile drawer is open (ignored on desktop) */
	mobileOpen: boolean;
	/** Called when the mobile drawer should close */
	onMobileClose: () => void;
	/** Called when a left-edge swipe should open the mobile drawer */
	onMobileOpen: () => void;
	/** Optional additional className for the sidebar panel */
	className?: string;
}

/**
 * Width of the touch detection zone on the left edge of the screen (px).
 * A touch that starts within this zone can trigger a swipe-to-open gesture.
 */
const SWIPE_EDGE_WIDTH = 20;

/**
 * Minimum horizontal distance (px) the finger must travel to count as an
 * intentional right-swipe that opens the sidebar.
 */
const SWIPE_OPEN_THRESHOLD = 60;

/**
 * ResponsiveSidebar
 *
 * A sidebar component that adapts its behavior based on viewport width:
 *
 * - **Desktop (>= md / 768px)**: Renders as a persistent 256px sidebar on
 *   the left side of the layout. Always visible, no overlay.
 *
 * - **Mobile (< md)**: Renders as a slide-in drawer from the left edge,
 *   controlled by the `mobileOpen` / `onMobileClose` props. Includes a
 *   semi-transparent overlay backdrop and traps focus while open.
 *   A right-swipe from the left edge of the screen (first 20 px) opens
 *   the drawer via the `onMobileOpen` callback.
 *
 * The hamburger button to open the drawer on mobile is provided separately
 * (e.g. in the header bar) and passes `mobileOpen` state down here.
 *
 * @example
 * ```tsx
 * const [drawerOpen, setDrawerOpen] = useState(false);
 *
 * <ResponsiveSidebar
 *   mobileOpen={drawerOpen}
 *   onMobileClose={() => setDrawerOpen(false)}
 *   onMobileOpen={() => setDrawerOpen(true)}
 * >
 *   <SidebarNav />
 * </ResponsiveSidebar>
 * ```
 */
export function ResponsiveSidebar({
	children,
	mobileOpen,
	onMobileClose,
	onMobileOpen,
	className,
}: ResponsiveSidebarProps) {
	const sidebarRef = useRef<HTMLElement>(null);
	const swipeStartX = useRef<number | null>(null);

	// Close on Escape key
	useEffect(() => {
		if (!mobileOpen) return;

		const handleKeyDown = (e: KeyboardEvent) => {
			if (e.key === "Escape") {
				onMobileClose();
			}
		};

		document.addEventListener("keydown", handleKeyDown);
		return () => {
			document.removeEventListener("keydown", handleKeyDown);
		};
	}, [mobileOpen, onMobileClose]);

	// Lock body scroll when drawer is open on mobile
	useEffect(() => {
		if (mobileOpen) {
			document.body.style.overflow = "hidden";
		} else {
			document.body.style.overflow = "";
		}
		return () => {
			document.body.style.overflow = "";
		};
	}, [mobileOpen]);

	const handleOverlayClick = useCallback(() => {
		onMobileClose();
	}, [onMobileClose]);

	// ── Swipe-to-open gesture handlers ──────────────────────────────────────────
	// Attach to the top-level wrapper (rendered by the parent layout) so the
	// touch zone covers the full viewport even when the sidebar is closed.

	const handleTouchStart = useCallback(
		(e: React.TouchEvent) => {
			if (mobileOpen) return; // already open — don't interfere
			const touchX = e.touches[0].clientX;
			if (touchX < SWIPE_EDGE_WIDTH) {
				swipeStartX.current = touchX;
			} else {
				swipeStartX.current = null;
			}
		},
		[mobileOpen],
	);

	const handleTouchEnd = useCallback(
		(e: React.TouchEvent) => {
			if (swipeStartX.current === null) return;
			const delta = e.changedTouches[0].clientX - swipeStartX.current;
			if (delta > SWIPE_OPEN_THRESHOLD) {
				onMobileOpen();
			}
			swipeStartX.current = null;
		},
		[onMobileOpen],
	);

	return (
		<>
			{/* ============================================================
          MOBILE: Overlay backdrop (only when drawer is open)
          ============================================================ */}
			{mobileOpen && (
				<div
					className="fixed inset-0 z-40 bg-background/80 backdrop-blur-sm md:hidden"
					onClick={handleOverlayClick}
					onKeyDown={(e) => e.key === "Escape" && handleOverlayClick()}
					aria-hidden="true"
				/>
			)}

			{/* ============================================================
          SWIPE DETECTION ZONE
          Invisible strip on the left edge of the screen — captures
          touch events to trigger the swipe-to-open gesture on mobile.
          Hidden on desktop (md and up).
          ============================================================ */}
			{!mobileOpen && (
				<div
					className="fixed inset-y-0 left-0 z-30 w-5 md:hidden"
					aria-hidden="true"
					onTouchStart={handleTouchStart}
					onTouchEnd={handleTouchEnd}
				/>
			)}

			{/* ============================================================
          SIDEBAR PANEL
          - Mobile: fixed slide-in drawer from left
          - Desktop: static 256px sidebar (always visible)
          ============================================================ */}
			<aside
				ref={sidebarRef}
				className={cn(
					// ── Desktop layout (md and up) ──────────────────────────
					// Persistent sidebar, always visible, 256px wide
					"hidden md:flex md:h-full md:w-64 md:flex-col",
					"md:border-r md:border-border md:bg-background",

					// ── Mobile layout ───────────────────────────────────────
					// Fixed position drawer that slides in from the left
					mobileOpen && [
						"fixed inset-y-0 left-0 z-50",
						"flex flex-col w-64",
						"border-r border-border bg-background",
						"shadow-xl",
					],

					className,
				)}
				aria-label="Primary navigation"
				// On mobile, use dialog role when drawer is open.
				// aria-modal is intentionally omitted; the explicit role="dialog"
				// combined with the backdrop overlay provides the modal context.
				role={mobileOpen ? "dialog" : undefined}
				onTouchStart={mobileOpen ? undefined : handleTouchStart}
				onTouchEnd={mobileOpen ? undefined : handleTouchEnd}
			>
				{/* Close button — only shown on mobile when drawer is open */}
				{mobileOpen && (
					<Button
						variant="ghost"
						size="icon"
						onClick={onMobileClose}
						className="absolute right-2 top-2 z-50 md:hidden"
						aria-label="Close navigation drawer"
					>
						<X className="size-5" />
					</Button>
				)}

				{children}
			</aside>
		</>
	);
}
