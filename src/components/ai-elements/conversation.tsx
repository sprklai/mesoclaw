/**
 * Conversation Component
 *
 * Wraps messages and automatically scrolls to the bottom. Also includes
 * a scroll button that appears when not at the bottom.
 *
 * Based on: https://ai-sdk.dev/elements/components/conversation
 */

import {
  type ReactNode,
  type RefObject,
  type UIEvent,
  type CSSProperties,
  useCallback,
  useEffect,
  useRef,
  useState,
  createContext,
  useContext,
  Children,
} from "react";

import { Button } from "@/components/ui/button";
import { ChevronDown } from "@/lib/icons";
import { cn } from "@/lib/utils";

interface ConversationProps {
  /** Child elements to render inside the conversation. */
  children: ReactNode;
  /** Additional CSS classes to apply to the root container. */
  className?: string;
  /** Style prop for setting height explicitly. */
  style?: CSSProperties;
}

interface ConversationContentProps {
  /** Child elements (messages) to render. */
  children: ReactNode;
  /** Additional CSS classes to apply. */
  className?: string;
}

type ConversationEmptyStateProps = {
  /** The title text to display. Defaults to "No messages yet". */
  title?: string;
  /** The description text to display. */
  description?: string;
  /** Optional icon to display above the text. */
  icon?: ReactNode;
  /** Optional additional content to render below the text. */
  children?: ReactNode;
  /** Additional CSS classes to apply. */
  className?: string;
};

interface ConversationScrollButtonProps {
  /** Additional props to pass to the underlying Button component. */
  className?: string;
}

// Context for scroll management
interface ScrollContextValue {
  scrollToBottom: (behavior?: ScrollBehavior) => void;
  isAtBottom: boolean;
  scrollContainerRef: RefObject<HTMLDivElement | null>;
}

const ScrollContext = createContext<ScrollContextValue | null>(null);

function useScrollContext(): ScrollContextValue {
  const context = useContext(ScrollContext);
  if (!context) {
    throw new Error(
      "Conversation components must be used within a Conversation component"
    );
  }
  return context;
}

/**
 * Main Conversation component that provides scroll context and automatic
 * scrolling behavior.
 */
export function Conversation({
  children,
  className,
  style,
}: ConversationProps) {
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const [isAtBottom, setIsAtBottom] = useState(true);

  const scrollToBottom = useCallback((behavior: ScrollBehavior = "smooth") => {
    scrollContainerRef.current?.scrollTo({
      top: scrollContainerRef.current.scrollHeight,
      behavior,
    });
  }, []);

  const handleScroll = useCallback((e: UIEvent<HTMLDivElement>) => {
    const target = e.target as HTMLDivElement;
    const threshold = 50;
    const position = target.scrollTop + target.clientHeight;
    const height = target.scrollHeight;
    setIsAtBottom(height - position < threshold);
  }, []);

  // Auto-scroll to bottom on mount
  useEffect(() => {
    scrollToBottom("instant");
  }, [scrollToBottom]);

  const contextValue: ScrollContextValue = {
    scrollToBottom,
    isAtBottom,
    scrollContainerRef,
  };

  return (
    <ScrollContext.Provider value={contextValue}>
      <div
        ref={scrollContainerRef}
        className={cn("relative h-full overflow-y-auto", className)}
        style={style}
        onScroll={handleScroll}
      >
        {children}
      </div>
    </ScrollContext.Provider>
  );
}

/**
 * Container for message content. Handles automatic scrolling when
 * new messages are added.
 */
export function ConversationContent({
  children,
  className,
}: ConversationContentProps) {
  const { scrollToBottom } = useScrollContext();
  const prevChildrenLength = useRef(0);

  // Scroll to bottom when children change
  useEffect(() => {
    const currentLength = Children.count(children);
    if (currentLength !== prevChildrenLength.current) {
      scrollToBottom();
      prevChildrenLength.current = currentLength;
    }
  }, [children, scrollToBottom]);

  return <div className={cn("flex flex-col", className)}>{children}</div>;
}

/**
 * Empty state displayed when there are no messages.
 */
export function ConversationEmptyState({
  title = "No messages yet",
  description,
  icon,
  children,
  className,
}: ConversationEmptyStateProps) {
  return (
    <div
      className={cn(
        "flex h-full flex-col items-center justify-center p-8 text-center",
        className
      )}
    >
      {icon && (
        <div className="mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-primary/10">
          {icon}
        </div>
      )}
      <h3 className="mb-2 text-lg font-semibold">{title}</h3>
      {description && (
        <p className="max-w-md text-sm text-muted-foreground">{description}</p>
      )}
      {children}
    </div>
  );
}

/**
 * Scroll-to-bottom button that appears when not at the bottom.
 */
export function ConversationScrollButton({
  className,
}: ConversationScrollButtonProps) {
  const { isAtBottom, scrollToBottom } = useScrollContext();

  if (isAtBottom) {
    return null;
  }

  return (
    <div className="absolute bottom-4 left-1/2 -translate-x-1/2">
      <Button
        variant="default"
        size="icon"
        className={cn("h-8 w-8 rounded-full shadow-lg", className)}
        onClick={() => scrollToBottom()}
        type="button"
      >
        <ChevronDown className="h-4 w-4" />
      </Button>
    </div>
  );
}
