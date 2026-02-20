import { useEffect, useRef, useState } from "react";

import { cn } from "@/lib/utils";

interface BottomSheetProps {
	open: boolean;
	onClose: () => void;
	children: React.ReactNode;
	className?: string;
}

/** Threshold in pixels — drag this far down to dismiss the sheet. */
const DISMISS_THRESHOLD = 120;

export function BottomSheet({
	open,
	onClose,
	children,
	className,
}: BottomSheetProps) {
	const [isDragging, setIsDragging] = useState(false);
	const [dragY, setDragY] = useState(0);
	const startY = useRef(0);

	// Reset drag offset whenever the sheet is closed from outside.
	useEffect(() => {
		if (!open) setDragY(0);
	}, [open]);

	const handleTouchStart = (e: React.TouchEvent) => {
		startY.current = e.touches[0].clientY;
		setIsDragging(true);
	};

	const handleTouchMove = (e: React.TouchEvent) => {
		if (!isDragging) return;
		const delta = e.touches[0].clientY - startY.current;
		// Only allow dragging downward.
		if (delta > 0) setDragY(delta);
	};

	const handleTouchEnd = () => {
		setIsDragging(false);
		if (dragY > DISMISS_THRESHOLD) {
			onClose();
		} else {
			setDragY(0);
		}
	};

	if (!open) return null;

	return (
		<>
			{/* Backdrop — a full-screen button that dismisses the sheet */}
			<button
				type="button"
				className="fixed inset-0 z-40 w-full bg-black/50 cursor-default"
				onClick={onClose}
				onKeyDown={(e) => e.key === "Escape" && onClose()}
				aria-label="Close sheet"
			/>

			{/* Sheet panel */}
			<div
				className={cn(
					"fixed bottom-0 left-0 right-0 z-50 rounded-t-2xl bg-background shadow-xl",
					"transition-transform duration-200",
					className,
				)}
				style={{ transform: `translateY(${dragY}px)` }}
				onTouchStart={handleTouchStart}
				onTouchMove={handleTouchMove}
				onTouchEnd={handleTouchEnd}
				role="dialog"
				aria-modal="true"
			>
				{/* Drag handle */}
				<div className="flex justify-center pt-3 pb-2">
					<div
						className="h-1 w-10 rounded-full bg-muted-foreground/30"
						aria-hidden="true"
					/>
				</div>

				{children}
			</div>
		</>
	);
}
