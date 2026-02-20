import type { UseChatSessionStore } from "@/stores/chatSessionStore";

export interface ChatCommand {
  name: string;
  description: string;
  usage: string;
  execute: (args: string, store: UseChatSessionStore) => Promise<void> | void;
}

/**
 * Available chat commands that can be invoked with "/" prefix.
 */
export const chatCommands: ChatCommand[] = [
  {
    name: "new",
    description: "Start a new chat session",
    usage: "/new [provider] [model]",
    execute: async (_args, store) => {
      const parts = _args.trim().split(/\s+/);
      const providerId = parts[0] || "default";
      const modelId = parts[1] || "default";
      await store.createSession(providerId, modelId);
    },
  },
  {
    name: "clear",
    description: "Clear the current chat messages",
    usage: "/clear",
    execute: (_args, store) => {
      store.clearMessages();
    },
  },
  {
    name: "status",
    description: "Show current session status",
    usage: "/status",
    execute: (_args, store) => {
      const session = store.getCurrentSession();
      if (session) {
        console.log(`Session: ${session.id}`);
        console.log(`Key: ${session.sessionKey}`);
        console.log(`Provider: ${session.agent}`);
        console.log(`Messages: ${store.getMessages().length}`);
      } else {
        console.log("No active session");
      }
    },
  },
  {
    name: "help",
    description: "Show available commands",
    usage: "/help",
    execute: () => {
      console.log("Available commands:");
      for (const cmd of chatCommands) {
        console.log(`  ${cmd.usage} - ${cmd.description}`);
      }
    },
  },
  {
    name: "export",
    description: "Export current chat as JSON",
    usage: "/export",
    execute: (_args, store) => {
      const messages = store.getMessages();
      const session = store.getCurrentSession();
      const exportData = {
        session: session
          ? {
              id: session.id,
              key: session.sessionKey,
              agent: session.agent,
              scope: session.scope,
              channel: session.channel,
              peer: session.peer,
            }
          : null,
        messages: messages.map((m) => ({
          role: m.role,
          content: m.content,
          createdAt: m.createdAt,
        })),
        exportedAt: new Date().toISOString(),
      };
      console.log(JSON.stringify(exportData, null, 2));
    },
  },
];

/**
 * Parse a message and execute command if it starts with "/".
 * Returns true if a command was executed, false otherwise.
 */
export function tryExecuteCommand(
  message: string,
  store: UseChatSessionStore,
): boolean {
  const trimmed = message.trim();
  if (!trimmed.startsWith("/")) {
    return false;
  }

  const [commandPart, ...argParts] = trimmed.slice(1).split(/\s+/);
  const commandName = commandPart?.toLowerCase();
  const args = argParts.join(" ");

  const command = chatCommands.find((c) => c.name === commandName);
  if (command) {
    command.execute(args, store);
    return true;
  }

  console.log(`Unknown command: /${commandName}. Type /help for available commands.`);
  return true;
}

/**
 * Get command suggestions for autocomplete.
 */
export function getCommandSuggestions(input: string): ChatCommand[] {
  if (!input.startsWith("/")) {
    return [];
  }

  const partial = input.slice(1).toLowerCase();
  return chatCommands.filter((c) => c.name.startsWith(partial));
}
