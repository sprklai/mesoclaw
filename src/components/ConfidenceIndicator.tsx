import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

interface ConfidenceIndicatorProps {
  /** Confidence value between 0 and 1 */
  confidence: number;
  /** Whether to show the percentage label */
  showLabel?: boolean;
  /** Size variant */
  size?: "sm" | "md" | "lg";
  /** Additional CSS classes */
  className?: string;
}

/**
 * Visual indicator showing confidence level of AI-generated explanations.
 * Uses color coding to convey confidence:
 * - High (>= 0.8): Green
 * - Medium (>= 0.5): Yellow/Amber
 * - Low (< 0.5): Red
 */
export function ConfidenceIndicator({
  confidence,
  showLabel = true,
  size = "md",
  className,
}: ConfidenceIndicatorProps) {
  const percentage = Math.round(confidence * 100);

  const getConfidenceLevel = () => {
    if (confidence >= 0.8) return "high";
    if (confidence >= 0.5) return "medium";
    return "low";
  };

  const getConfidenceLabel = () => {
    const level = getConfidenceLevel();
    switch (level) {
      case "high":
        return "High confidence";
      case "medium":
        return "Medium confidence";
      case "low":
        return "Low confidence";
    }
  };

  const getConfidenceDescription = () => {
    const level = getConfidenceLevel();
    switch (level) {
      case "high":
        return "This explanation is well-supported by schema evidence.";
      case "medium":
        return "This explanation has moderate support from schema evidence.";
      case "low":
        return "This explanation is inferred with limited evidence.";
    }
  };

  const getConfidenceColor = () => {
    const level = getConfidenceLevel();
    switch (level) {
      case "high":
        return "bg-green-500";
      case "medium":
        return "bg-amber-500";
      case "low":
        return "bg-red-500";
    }
  };

  const getTrackColor = () => {
    const level = getConfidenceLevel();
    switch (level) {
      case "high":
        return "bg-green-100 dark:bg-green-950";
      case "medium":
        return "bg-amber-100 dark:bg-amber-950";
      case "low":
        return "bg-red-100 dark:bg-red-950";
    }
  };

  const getTextColor = () => {
    const level = getConfidenceLevel();
    switch (level) {
      case "high":
        return "text-green-700 dark:text-green-400";
      case "medium":
        return "text-amber-700 dark:text-amber-400";
      case "low":
        return "text-red-700 dark:text-red-400";
    }
  };

  const sizeClasses = {
    sm: { bar: "h-1", text: "text-xs" },
    md: { bar: "h-1.5", text: "text-sm" },
    lg: { bar: "h-2", text: "text-base" },
  };

  const tooltipContent = (
    <div>
      <p className="font-medium">
        {getConfidenceLabel()} ({percentage}%)
      </p>
      <p className="text-xs opacity-80 mt-1">{getConfidenceDescription()}</p>
    </div>
  );

  return (
    <Tooltip content={tooltipContent}>
      <div className={cn("flex items-center gap-2 cursor-help", className)}>
        <div
          className={cn(
            "flex-1 min-w-[60px] max-w-[100px] rounded-full overflow-hidden",
            getTrackColor(),
            sizeClasses[size].bar
          )}
        >
          <div
            className={cn(
              "h-full rounded-full transition-all duration-300",
              getConfidenceColor()
            )}
            style={{ width: `${percentage}%` }}
          />
        </div>
        {showLabel && (
          <span
            className={cn(
              "font-medium",
              getTextColor(),
              sizeClasses[size].text
            )}
          >
            {percentage}%
          </span>
        )}
      </div>
    </Tooltip>
  );
}
