import { AlertCircle, RefreshCw } from "@/lib/icons";
import { cn } from "@/lib/utils";

import { Button } from "./button";

interface ErrorStateProps {
  title?: string;
  message: string;
  onRetry?: () => void;
  retryLabel?: string;
  variant?: "inline" | "card" | "banner";
  className?: string;
}

export function ErrorState({
  title = "An error occurred",
  message,
  onRetry,
  retryLabel = "Try Again",
  variant = "card",
  className,
}: ErrorStateProps) {
  if (variant === "inline") {
    return (
      <div
        className={cn("flex items-center gap-2 text-sm", className)}
        role="alert"
        aria-live="assertive"
      >
        <AlertCircle
          className="h-4 w-4 shrink-0 text-destructive"
          aria-hidden="true"
        />
        <span className="text-destructive">{message}</span>
      </div>
    );
  }

  if (variant === "banner") {
    return (
      <div
        className={cn(
          "flex items-center gap-3 rounded-md border border-destructive/50 bg-destructive/10 p-3",
          className
        )}
        role="alert"
        aria-live="assertive"
      >
        <AlertCircle
          className="h-5 w-5 shrink-0 text-destructive"
          aria-hidden="true"
        />
        <div className="flex-1">
          <p className="text-sm font-medium text-destructive">{title}</p>
          <p className="text-sm text-destructive/80">{message}</p>
        </div>
        {onRetry && (
          <Button
            size="sm"
            variant="outline"
            onClick={onRetry}
            aria-label={retryLabel}
          >
            <RefreshCw className="mr-2 h-4 w-4" aria-hidden="true" />
            {retryLabel}
          </Button>
        )}
      </div>
    );
  }

  return (
    <div
      className={cn(
        "rounded-lg border border-destructive/50 bg-destructive/10 p-6",
        className
      )}
      role="alert"
      aria-live="assertive"
    >
      <div className="flex items-start gap-3">
        <AlertCircle
          className="h-5 w-5 shrink-0 mt-0.5 text-destructive"
          aria-hidden="true"
        />
        <div className="flex-1">
          <p className="font-medium text-destructive">{title}</p>
          <p className="mt-1 text-sm text-destructive/80">{message}</p>
          {onRetry && (
            <Button
              className="mt-4"
              size="sm"
              variant="outline"
              onClick={onRetry}
              aria-label={retryLabel}
            >
              <RefreshCw className="mr-2 h-4 w-4" aria-hidden="true" />
              {retryLabel}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
