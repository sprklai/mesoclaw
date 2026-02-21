/**
 * PromptInput Component
 *
 * Allows a user to send a message with file attachments to a large language model.
 * Includes a textarea, submit button, and optional toolbar for custom actions.
 *
 * Based on: https://ai-sdk.dev/elements/components/prompt-input
 */

import {
  type ComponentProps,
  type FormEvent,
  type KeyboardEvent,
  forwardRef,
  useCallback,
  useEffect,
  useState,
} from "react";

import { Button } from "@/components/ui/button";
import { Loader2, Send, Square } from "@/lib/icons";
import { cn } from "@/lib/utils";

interface PromptInputMessage {
  /** The text content of the message. */
  text: string;
  /** File attachments (optional, for future use). */
  files?: File[];
}

interface PromptInputProps {
  /** Callback when the form is submitted. */
  onSubmit: (message: PromptInputMessage, event?: FormEvent) => void;
  /** Current input value (controlled). */
  value?: string;
  /** Callback when input value changes. */
  onChange?: (value: string) => void;
  /** Placeholder text for the textarea. */
  placeholder?: string;
  /** Whether the input is disabled. */
  disabled?: boolean;
  /** Whether submission is in progress (shows loading state). */
  isLoading?: boolean;
  /** Whether streaming is in progress (shows stop button). */
  isStreaming?: boolean;
  /** Callback when stop button is clicked. */
  onStop?: () => void;
  /** Minimum height of the textarea. */
  minHeight?: string;
  /** Maximum height of the textarea. */
  maxHeight?: string;
  /** File types to accept (for future attachment support). */
  accept?: string;
  /** Whether multiple files are allowed. */
  multiple?: boolean;
  /** Whether to support global drag and drop. */
  globalDrop?: boolean;
  /** Custom submit button content. */
  submitButton?: React.ReactNode;
  /** Children for compositional usage. */
  children?: React.ReactNode;
  /** Additional CSS classes to apply. */
  className?: string;
}

/**
 * Main PromptInput component - a controlled input with auto-resizing textarea.
 */
export const PromptInput = forwardRef<HTMLFormElement, PromptInputProps>(
  function PromptInput(
    {
      onSubmit,
      value: controlledValue,
      onChange,
      placeholder = "Type a message...",
      disabled = false,
      isLoading = false,
      isStreaming = false,
      onStop,
      minHeight = "56px",
      maxHeight = "200px",
      submitButton,
      children,
      className,
    },
    ref
  ) {
    const [uncontrolledValue, setUncontrolledValue] = useState("");
    const [textareaHeight, setTextareaHeight] = useState(minHeight);

    // Use controlled or uncontrolled value
    const value =
      controlledValue !== undefined ? controlledValue : uncontrolledValue;

    const setValue = useCallback(
      (newValue: string) => {
        if (controlledValue === undefined) {
          setUncontrolledValue(newValue);
        }
        onChange?.(newValue);
      },
      [controlledValue, onChange]
    );

    // Auto-resize textarea based on content
    const textareaRef = useCallback(
      (node: HTMLTextAreaElement | null) => {
        if (node) {
          // Reset height to calculate correctly
          node.style.height = minHeight;
          const newHeight = Math.min(
            Math.max(node.scrollHeight, parseInt(minHeight)),
            parseInt(maxHeight)
          );
          setTextareaHeight(`${newHeight}px`);
          node.style.height = `${newHeight}px`;
        }
      },
      [minHeight, maxHeight]
    );

    // Reset height when value changes significantly
    useEffect(() => {
      // Will be handled by the ref callback
    }, [value]);

    const handleSubmit = useCallback(
      (event?: FormEvent) => {
        event?.preventDefault();
        const trimmedValue = value.trim();
        if (!trimmedValue || disabled || isLoading) {
          return;
        }
        onSubmit({ text: trimmedValue }, event);
        if (controlledValue === undefined) {
          setUncontrolledValue("");
          setTextareaHeight(minHeight);
        }
      },
      [value, disabled, isLoading, onSubmit, controlledValue, minHeight]
    );

    const handleKeyDown = useCallback(
      (event: KeyboardEvent<HTMLTextAreaElement>) => {
        if (event.key === "Enter" && !event.shiftKey) {
          event.preventDefault();
          handleSubmit(event as unknown as FormEvent);
        }
      },
      [handleSubmit]
    );

    const defaultSubmitButton =
      submitButton ??
      (isStreaming || isLoading ? (
        <Button
          type="button"
          size="icon"
          variant="destructive"
          className="h-9 w-9 shrink-0 rounded-lg"
          onClick={onStop}
          title="Stop generating"
        >
          <Square className="h-4 w-4" />
        </Button>
      ) : (
        <Button
          type="submit"
          size="icon"
          variant="default"
          className="h-9 w-9 shrink-0 rounded-lg"
          disabled={!value.trim() || disabled}
        >
          <Send className="h-4 w-4" />
        </Button>
      ));

    // If children are provided, use compositional rendering
    if (children) {
      return (
        <form
          ref={ref}
          onSubmit={handleSubmit}
          className={cn("flex flex-col gap-2 rounded-lg border border-border bg-background p-2", className)}
        >
          {children}
        </form>
      );
    }

    // Default rendering (legacy behavior)
    return (
      <form
        ref={ref}
        onSubmit={handleSubmit}
        className={cn("flex items-center gap-2", className)}
      >
        <div className="relative flex-1">
          <textarea
            ref={textareaRef}
            value={value}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={handleKeyDown}
            disabled={disabled}
            placeholder={placeholder}
            className={cn(
              "w-full resize-none rounded-lg border border-border bg-background px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/50 disabled:cursor-not-allowed disabled:opacity-50",
              "transition-all duration-200"
            )}
            style={{
              minHeight,
              maxHeight,
              height: textareaHeight,
            }}
            rows={1}
          />
        </div>
        {defaultSubmitButton}
      </form>
    );
  }
);

interface PromptInputTextareaProps extends ComponentProps<"textarea"> {
  /** Reference to the textarea element. */
  textareaRef?: (node: HTMLTextAreaElement | null) => void;
}

/**
 * Textarea component for use inside PromptInput (for custom compositions).
 */
export const PromptInputTextarea = forwardRef<
  HTMLTextAreaElement,
  PromptInputTextareaProps
>(function PromptInputTextarea({ textareaRef, className, ...props }, ref) {
  return (
    <textarea
      ref={(node) => {
        // Handle both refs
        if (typeof ref === "function") {
          ref(node);
        } else if (ref) {
          ref.current = node;
        }
        if (textareaRef) {
          textareaRef(node);
        }
      }}
      className={cn(
        "w-full resize-none rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary/50",
        className
      )}
      {...props}
    />
  );
});

interface PromptInputSubmitProps extends ComponentProps<typeof Button> {
  /** Loading state - shows spinner when true. */
  status?: "ready" | "streaming" | "loading";
}

/**
 * Submit button for use inside PromptInput.
 */
export const PromptInputSubmit = forwardRef<
  HTMLButtonElement,
  PromptInputSubmitProps
>(function PromptInputSubmit(
  { status = "ready", children, disabled, className, ...props },
  ref
) {
  const isStreamingOrLoading = status === "streaming" || status === "loading";

  return (
    <Button
      ref={ref}
      type="submit"
      size="icon"
      variant="default"
      disabled={disabled || isStreamingOrLoading}
      className={cn("h-9 w-9 shrink-0 rounded-lg", className)}
      {...props}
    >
      {isStreamingOrLoading ? (
        <Loader2 className="h-4 w-4 animate-spin" />
      ) : (
        children || <Send className="h-4 w-4" />
      )}
    </Button>
  );
});

// Additional compositional components for advanced usage

export function PromptInputHeader({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={cn("px-2 pt-2", className)}>{children}</div>;
}

export function PromptInputBody({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={cn("flex-1", className)}>{children}</div>;
}

export function PromptInputFooter({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={cn("flex items-center justify-end gap-2 px-2 pb-2", className)}>{children}</div>;
}

export function PromptInputTools({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={cn("flex items-center gap-1", className)}>{children}</div>;
}

export function PromptInputButton({ children, className, ...props }: ComponentProps<typeof Button>) {
  return (
    <Button variant="ghost" size="sm" className={className} {...props}>
      {children}
    </Button>
  );
}

// Action menu components (placeholder)
export function PromptInputActionMenu({ children }: { children: React.ReactNode }) {
  return <div className="relative inline-block">{children}</div>;
}

export function PromptInputActionMenuTrigger({ children }: { children?: React.ReactNode }) {
  return (
    <Button variant="ghost" size="sm">
      {children || "+"}
    </Button>
  );
}

export function PromptInputActionMenuContent({ children }: { children: React.ReactNode }) {
  return <div className="absolute bottom-full mb-2 rounded-md border bg-popover p-1 shadow-md">{children}</div>;
}

export function PromptInputActionAddAttachments() {
  return <button type="button" className="rounded px-3 py-2 text-sm hover:bg-muted">Add attachments</button>;
}

// Attachments hook
export function usePromptInputAttachments() {
  const [files, setFiles] = useState<{ id: string; name: string; type: string; url: string }[]>([]);

  return {
    files,
    add: (file: { id: string; name: string; type: string; url: string }) => {
      setFiles((prev) => [...prev, file]);
    },
    remove: (id: string) => {
      setFiles((prev) => prev.filter((f) => f.id !== id));
    },
    clear: () => {
      setFiles([]);
    },
  };
}
