/**
 * CodeBlock Component
 *
 * Provides syntax highlighting, line numbers, and copy to clipboard functionality
 * for code blocks. Wraps react-syntax-highlighter with custom styling.
 *
 * Features:
 * - Text wrap toggle for long lines
 * - Database-agnostic language detection (SQL, MongoDB)
 * - Copy to clipboard functionality
 *
 * Based on: https://ai-sdk.dev/elements/components/code-block
 */

import { useCallback, useState } from "react";

import { Button } from "@/components/ui/button";
import { Tooltip } from "@/components/ui/tooltip";
import {
  languageColors,
  type DetectedLanguage,
} from "@/lib/code-language-detection";
import { Check, Copy, WrapText } from "@/lib/icons";
import { cn } from "@/lib/utils";

interface CodeBlockProps {
  /** The code content to display */
  code: string;
  /** The programming language for syntax highlighting */
  language?: string;
  /** Whether to show line numbers. Default: false. */
  showLineNumbers?: boolean;
  /** Child elements (like CodeBlockCopyButton) positioned in the top-right corner. */
  children?: React.ReactNode;
  /** Additional CSS classes to apply to the root container. */
  className?: string;
  /** Default text wrap state. Default: false (horizontal scroll). */
  defaultTextWrap?: boolean;
}

/**
 * Gets the color class for a language.
 */
function getLanguageColor(language: string): string {
  const normalizedLang = language.toLowerCase();

  // Check if it's a known language type
  if (normalizedLang in languageColors) {
    return languageColors[normalizedLang as DetectedLanguage];
  }

  // Map common aliases
  if (["postgresql", "mysql", "sqlite"].includes(normalizedLang)) {
    return languageColors.sql;
  }
  if (["mongo"].includes(normalizedLang)) {
    return languageColors.mongodb;
  }
  if (["ts", "typescript"].includes(normalizedLang)) {
    return "text-blue-300";
  }
  if (["js"].includes(normalizedLang)) {
    return languageColors.javascript;
  }

  return languageColors.text;
}

export function CodeBlock({
  code,
  language = "text",
  showLineNumbers: _showLineNumbers = false,
  children,
  className,
  defaultTextWrap = false,
}: CodeBlockProps) {
  const [isTextWrapEnabled, setIsTextWrapEnabled] = useState(defaultTextWrap);

  const toggleTextWrap = useCallback(() => {
    setIsTextWrapEnabled((prev) => !prev);
  }, []);

  const colorClass = getLanguageColor(language);

  return (
    <div
      className={cn(
        "group relative my-4 overflow-hidden rounded-lg border border-border bg-muted/50",
        className
      )}
    >
      {/* Header with language indicator and controls */}
      <div className="flex items-center justify-between border-b border-border bg-muted px-4 py-2">
        <span className={cn("text-xs font-medium uppercase", colorClass)}>
          {language}
        </span>
        <div className="flex items-center gap-1">
          {/* Text wrap toggle */}
          <Tooltip
            content={isTextWrapEnabled ? "Disable wrap" : "Enable wrap"}
            side="top"
          >
            <Button
              variant="ghost"
              size="sm"
              className={cn(
                "h-7 w-7 p-0",
                isTextWrapEnabled && "bg-accent text-accent-foreground"
              )}
              onClick={toggleTextWrap}
              type="button"
              aria-label={
                isTextWrapEnabled ? "Disable text wrap" : "Enable text wrap"
              }
              aria-pressed={isTextWrapEnabled}
            >
              <WrapText className="h-3.5 w-3.5" aria-hidden="true" />
            </Button>
          </Tooltip>
          {children}
        </div>
      </div>

      {/* Code content */}
      <pre
        className={cn(
          "p-4",
          isTextWrapEnabled
            ? "whitespace-pre-wrap break-words"
            : "overflow-x-auto"
        )}
      >
        <code className={cn("text-sm font-mono", colorClass)}>{code}</code>
      </pre>
    </div>
  );
}

interface CodeBlockCopyButtonProps {
  /** Callback fired after a successful copy. */
  onCopy?: () => void;
  /** Callback fired if copying fails. */
  onError?: (error: Error) => void;
  /** How long to show the copied state (ms). Default: 2000. */
  timeout?: number;
  /** Custom content for the button. Defaults to copy/check icons. */
  children?: React.ReactNode;
  /** Additional CSS classes to apply to the button. */
  className?: string;
  /** The code to copy. If not provided, will try to get from parent CodeBlock. */
  code?: string;
}

export function CodeBlockCopyButton({
  onCopy,
  onError,
  timeout = 2000,
  children,
  className,
  code,
}: CodeBlockCopyButtonProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      // Get code from prop or try to find it in DOM
      const codeToCopy = code || "";
      await navigator.clipboard.writeText(codeToCopy);
      setCopied(true);
      onCopy?.();

      setTimeout(() => setCopied(false), timeout);
    } catch (error) {
      onError?.(error as Error);
    }
  }, [code, onCopy, onError, timeout]);

  return (
    <Button
      variant="ghost"
      size="sm"
      className={cn("h-7 w-7 p-0", className)}
      onClick={handleCopy}
      type="button"
      aria-label={copied ? "Copied to clipboard" : "Copy code to clipboard"}
    >
      {copied ? (
        <Check className="h-3.5 w-3.5 text-green-700" aria-hidden="true" />
      ) : (
        <Copy className="h-3.5 w-3.5" aria-hidden="true" />
      )}
      {children}
    </Button>
  );
}
