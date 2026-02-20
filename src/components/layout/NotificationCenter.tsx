/**
 * Notification Center Component
 *
 * Displays a bell icon with unread count badge and a slide-out panel
 * for viewing and managing system notifications.
 */
import { cn } from "@/lib/utils";
import { Bell, Check, CheckCheck, Trash2, X } from "@/lib/icons";
import type { AppNotification, NotificationCategory } from "@/stores/notificationStore";
import { useNotificationStore } from "@/stores/notificationStore";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

const categoryIcons: Record<NotificationCategory, string> = {
  system: "⚙️",
  agent: "🤖",
  channel: "💬",
  scheduler: "⏰",
  approval: "⚠️",
};

const priorityColors: Record<string, string> = {
  low: "border-l-muted",
  normal: "border-l-transparent",
  high: "border-l-amber-500",
  urgent: "border-l-red-500",
};

/**
 * Format a timestamp to a relative time string.
 */
function formatRelativeTime(timestamp: string): string {
  const now = new Date();
  const date = new Date(timestamp);
  const diffMs = now.getTime() - date.getTime();
  const diffSeconds = Math.floor(diffMs / 1000);
  const diffMinutes = Math.floor(diffSeconds / 60);
  const diffHours = Math.floor(diffMinutes / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffSeconds < 60) {
    return "just now";
  } else if (diffMinutes < 60) {
    return `${diffMinutes}m ago`;
  } else if (diffHours < 24) {
    return `${diffHours}h ago`;
  } else if (diffDays < 7) {
    return `${diffDays}d ago`;
  } else {
    return date.toLocaleDateString();
  }
}

interface NotificationItemProps {
  notification: AppNotification;
  onMarkRead: (id: string) => void;
  onRemove: (id: string) => void;
  onAction?: (url: string) => void;
}

function NotificationItem({
  notification,
  onMarkRead,
  onRemove,
  onAction,
}: NotificationItemProps) {
  const timeAgo = formatRelativeTime(notification.timestamp);

  const handleClick = () => {
    if (!notification.isRead) {
      onMarkRead(notification.id);
    }
    if (notification.actionUrl && onAction) {
      onAction(notification.actionUrl);
    }
  };

  return (
    <div
      className={cn(
        "group relative border-l-2 p-3 transition-colors hover:bg-muted/50",
        priorityColors[notification.priority],
        !notification.isRead && "bg-muted/30"
      )}
    >
      <div className="flex items-start gap-3">
        <span className="text-lg" role="img" aria-label={notification.category}>
          {categoryIcons[notification.category]}
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex items-start justify-between gap-2">
            <p
              className={cn(
                "text-sm font-medium leading-tight",
                !notification.isRead && "text-foreground",
                notification.isRead && "text-muted-foreground"
              )}
            >
              {notification.title}
            </p>
            <button
              type="button"
              onClick={() => onRemove(notification.id)}
              className="shrink-0 rounded p-1 opacity-0 transition-opacity hover:bg-muted group-hover:opacity-100"
              aria-label="Remove notification"
            >
              <X className="size-3 text-muted-foreground" />
            </button>
          </div>
          <p className="mt-1 text-xs text-muted-foreground line-clamp-2">
            {notification.message}
          </p>
          <div className="mt-2 flex items-center justify-between">
            <span className="text-xs text-muted-foreground">{timeAgo}</span>
            <div className="flex items-center gap-1">
              {notification.actionLabel && notification.actionUrl && (
                <button
                  type="button"
                  onClick={handleClick}
                  className="rounded px-2 py-0.5 text-xs text-primary hover:bg-primary/10"
                >
                  {notification.actionLabel}
                </button>
              )}
              {!notification.isRead && (
                <button
                  type="button"
                  onClick={() => onMarkRead(notification.id)}
                  className="rounded p-1 hover:bg-muted"
                  aria-label="Mark as read"
                >
                  <Check className="size-3 text-muted-foreground" />
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-8 text-center">
      <Bell className="size-8 text-muted-foreground/50" />
      <p className="mt-2 text-sm text-muted-foreground">No notifications</p>
      <p className="text-xs text-muted-foreground/70">
        You're all caught up!
      </p>
    </div>
  );
}

export function NotificationCenter() {
  const [isOpen, setIsOpen] = useState(false);
  const navigate = useNavigate();
  const {
    notifications,
    addNotification,
    markAsRead,
    markAllAsRead,
    removeNotification,
    clearAll,
    getUnreadCount,
  } = useNotificationStore();

  const unreadCount = getUnreadCount();

  const handleAction = (url: string) => {
    // Parse deep link and navigate
    // Format: mesoclaw://<resource>/<id>
    const parsed = url.match(/^mesoclaw:\/\/([^/]+)\/(.+)$/);
    if (parsed) {
      const [, resource, id] = parsed;
      switch (resource) {
        case "session":
          navigate({ to: "/chat", search: { sessionId: id } });
          break;
        case "channel":
          navigate({ to: "/channels" });
          break;
        case "approval":
          navigate({ to: "/settings", search: { tab: "approvals" } });
          break;
        case "scheduler":
          navigate({ to: "/settings", search: { tab: "scheduler" } });
          break;
      }
      setIsOpen(false);
    }
  };

  // Add test notification for development
  const handleAddTestNotification = () => {
    addNotification({
      title: "Test Notification",
      message: "This is a test notification to verify the notification center is working correctly.",
      category: "system",
      priority: "normal",
    });
  };

  return (
    <Popover open={isOpen} onOpenChange={setIsOpen}>
      <PopoverTrigger
        className="relative inline-flex h-9 w-9 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
        aria-label={`${unreadCount} unread notifications`}
      >
        <Bell className="size-5" />
        {unreadCount > 0 && (
          <span className="absolute -right-1 -top-1 flex size-4 items-center justify-center rounded-full bg-destructive text-[10px] font-medium text-destructive-foreground">
            {unreadCount > 9 ? "9+" : unreadCount}
          </span>
        )}
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80 p-0">
        <div className="flex items-center justify-between border-b px-4 py-3">
          <h3 className="font-semibold">Notifications</h3>
          <div className="flex items-center gap-1">
            {unreadCount > 0 && (
              <Button
                variant="ghost"
                size="sm"
                onClick={markAllAsRead}
                className="h-7 px-2 text-xs"
              >
                <CheckCheck className="mr-1 size-3" />
                Mark all read
              </Button>
            )}
            {notifications.length > 0 && (
              <Button
                variant="ghost"
                size="sm"
                onClick={clearAll}
                className="h-7 px-2 text-xs text-destructive hover:text-destructive"
              >
                <Trash2 className="mr-1 size-3" />
                Clear
              </Button>
            )}
          </div>
        </div>

        <div className="max-h-80 overflow-y-auto">
          {notifications.length === 0 ? (
            <EmptyState />
          ) : (
            <div className="divide-y">
              {notifications.slice(0, 10).map((notification) => (
                <NotificationItem
                  key={notification.id}
                  notification={notification}
                  onMarkRead={markAsRead}
                  onRemove={removeNotification}
                  onAction={handleAction}
                />
              ))}
            </div>
          )}
        </div>

        {notifications.length > 10 && (
          <div className="border-t px-4 py-2 text-center text-xs text-muted-foreground">
            Showing 10 of {notifications.length} notifications
          </div>
        )}

        {/* Dev: Add test notification button */}
        {import.meta.env.DEV && (
          <div className="border-t px-4 py-2">
            <Button
              variant="outline"
              size="sm"
              onClick={handleAddTestNotification}
              className="w-full text-xs"
            >
              Add Test Notification
            </Button>
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}
