import { Loader2 } from "@/lib/icons";
import { cn } from "@/lib/utils";

interface LoadingStateProps {
  message?: string;
  size?: "sm" | "md" | "lg";
  className?: string;
}

export function LoadingState({
  message,
  size = "md",
  className,
}: LoadingStateProps) {
  const sizes = {
    sm: { icon: "h-4 w-4", text: "text-sm" },
    md: { icon: "h-8 w-8", text: "text-base" },
    lg: { icon: "h-12 w-12", text: "text-lg" },
  };

  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center py-8 text-center",
        className
      )}
      role="status"
      aria-live="polite"
      aria-busy="true"
    >
      <Loader2
        className={cn("animate-spin text-muted-foreground", sizes[size].icon)}
      />
      {message && (
        <p className={cn("mt-4 font-medium text-foreground", sizes[size].text)}>
          {message}
        </p>
      )}
    </div>
  );
}
