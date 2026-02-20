import { useNavigate } from "@tanstack/react-router";
import { useHotkeys } from "react-hotkeys-hook";
import { useCallback, useState } from "react";

import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from "@/components/ui/command";
import { useChatSessionStore } from "@/stores/chatSessionStore";

interface CommandAction {
  id: string;
  label: string;
  shortcut?: string;
  icon?: React.ReactNode;
  action: () => void;
}

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const navigate = useNavigate();
  const clearMessages = useChatSessionStore((s) => s.clearMessages);
  const createSession = useChatSessionStore((s) => s.createSession);

  const togglePalette = useCallback(() => {
    setOpen((prev) => !prev);
  }, []);

  // Cmd+K (Mac) or Ctrl+K (Windows/Linux) to open command palette
  useHotkeys("mod+k", togglePalette, { preventDefault: true });

  const navigationCommands: CommandAction[] = [
    {
      id: "nav-chat",
      label: "Go to Chat",
      shortcut: "G C",
      action: () => {
        navigate({ to: "/chat" });
        setOpen(false);
      },
    },
    {
      id: "nav-agents",
      label: "Go to Agents",
      shortcut: "G A",
      action: () => {
        navigate({ to: "/agents" });
        setOpen(false);
      },
    },
    {
      id: "nav-settings",
      label: "Go to Settings",
      shortcut: "G S",
      action: () => {
        navigate({ to: "/settings", search: { tab: "ai" } });
        setOpen(false);
      },
    },
    {
      id: "nav-channels",
      label: "Go to Channels",
      shortcut: "G H",
      action: () => {
        navigate({ to: "/channels" });
        setOpen(false);
      },
    },
    {
      id: "nav-memory",
      label: "Go to Memory",
      shortcut: "G M",
      action: () => {
        navigate({ to: "/memory" });
        setOpen(false);
      },
    },
    {
      id: "nav-logs",
      label: "Go to Logs",
      action: () => {
        navigate({ to: "/logs" });
        setOpen(false);
      },
    },
  ];

  const chatCommands: CommandAction[] = [
    {
      id: "chat-new",
      label: "New Chat Session",
      shortcut: "N",
      action: async () => {
        await createSession("default", "default");
        setOpen(false);
      },
    },
    {
      id: "chat-clear",
      label: "Clear Current Chat",
      shortcut: "C",
      action: () => {
        clearMessages();
        setOpen(false);
      },
    },
  ];

  const handleSelect = (action: () => void) => {
    action();
  };

  return (
    <CommandDialog open={open} onOpenChange={setOpen}>
      <CommandInput placeholder="Type a command or search..." />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>

        <CommandGroup heading="Navigation">
          {navigationCommands.map((cmd) => (
            <CommandItem
              key={cmd.id}
              onSelect={() => handleSelect(cmd.action)}
            >
              <span>{cmd.label}</span>
              {cmd.shortcut && <CommandShortcut>{cmd.shortcut}</CommandShortcut>}
            </CommandItem>
          ))}
        </CommandGroup>

        <CommandSeparator />

        <CommandGroup heading="Chat">
          {chatCommands.map((cmd) => (
            <CommandItem
              key={cmd.id}
              onSelect={() => handleSelect(cmd.action)}
            >
              <span>{cmd.label}</span>
              {cmd.shortcut && <CommandShortcut>{cmd.shortcut}</CommandShortcut>}
            </CommandItem>
          ))}
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  );
}
