/**
 * AgentRunDialog - Dialog for executing an agent with a message.
 *
 * Features:
 * - Message input for the agent task
 * - Real-time streaming output display
 * - Cancel execution support
 * - Shows agent name and model info
 */
import { Loader2, Play, Square, Bot, CheckCircle2, XCircle } from "@/lib/icons";
import type { AgentConfig } from "@/lib/agent-config";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useState, useEffect } from "react";
import { toast } from "sonner";

// ─── Types ────────────────────────────────────────────────────────

interface AgentRunDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  agent: AgentConfig | null;
  onRunComplete?: (result: string) => void;
}

interface StreamEvent {
  type: "start" | "token" | "done" | "error";
  content?: string;
}

// ─── Component ─────────────────────────────────────────────────────

export function AgentRunDialog({
  open,
  onOpenChange,
  agent,
  onRunComplete,
}: AgentRunDialogProps) {
  const [message, setMessage] = useState("");
  const [isRunning, setIsRunning] = useState(false);
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);

  // Reset state when dialog opens/closes
  useEffect(() => {
    if (!open) {
      setMessage("");
      setOutput("");
      setError(null);
      setIsRunning(false);
      setSessionId(null);
    }
  }, [open]);

  // Listen for streaming events
  useEffect(() => {
    if (!sessionId) return;

    const unlisten = listen<StreamEvent>(`agent-stream-${sessionId}`, (event) => {
      const { type, content } = event.payload;

      switch (type) {
        case "start":
          setOutput("");
          setError(null);
          break;
        case "token":
          if (content) {
            setOutput((prev) => prev + content);
          }
          break;
        case "done":
          setIsRunning(false);
          if (content) {
            onRunComplete?.(content);
          }
          break;
        case "error":
          setError(content ?? "Unknown error");
          setIsRunning(false);
          break;
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [sessionId, onRunComplete]);

  // ─── Handlers ─────────────────────────────────────────────────────

  const handleRun = async () => {
    if (!agent || !message.trim()) {
      toast.error("Please enter a message for the agent");
      return;
    }

    setIsRunning(true);
    setOutput("");
    setError(null);

    try {
      const result = await invoke<string>("run_agent_command", {
        agentId: agent.id,
        message: message.trim(),
      });

      // If streaming isn't implemented, show the result directly
      if (result && !sessionId) {
        setOutput(result);
        onRunComplete?.(result);
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setError(errorMsg);
      toast.error(`Agent execution failed: ${errorMsg}`);
    } finally {
      setIsRunning(false);
    }
  };

  const handleCancel = async () => {
    if (!sessionId) return;

    try {
      await invoke("cancel_agent_session_command", { sessionId });
      setIsRunning(false);
      toast.info("Agent execution cancelled");
    } catch (err) {
      console.error("Failed to cancel agent:", err);
    }
  };

  const handleClose = () => {
    if (isRunning) {
      handleCancel();
    }
    onOpenChange(false);
  };

  // ─── Render ───────────────────────────────────────────────────────

  if (!agent) return null;

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Bot className="h-5 w-5" />
            Run Agent: {agent.name}
          </DialogTitle>
          <DialogDescription>
            Send a task to the agent and watch it execute in real-time.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4 py-4">
          {/* Agent Info */}
          <div className="flex items-center gap-2 flex-wrap">
            <Badge variant="outline">{agent.role}</Badge>
            <Badge variant="secondary">{agent.modelId}</Badge>
            <Badge variant={agent.isEnabled ? "success" : "destructive"}>
              {agent.isEnabled ? "Active" : "Disabled"}
            </Badge>
          </div>

          {/* Input */}
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium">Task Message</label>
            <Textarea
              placeholder="Describe the task for the agent..."
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              disabled={isRunning}
              className="min-h-[100px]"
            />
          </div>

          {/* Output */}
          {(output || error) && (
            <div className="flex flex-col gap-2">
              <label className="text-sm font-medium flex items-center gap-2">
                {isRunning ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : error ? (
                  <XCircle className="h-4 w-4 text-destructive" />
                ) : (
                  <CheckCircle2 className="h-4 w-4 text-green-500" />
                )}
                {error ? "Error" : "Output"}
              </label>
              <div
                className={cn(
                  "min-h-[150px] max-h-[300px] overflow-y-auto rounded-md border p-3 font-mono text-sm whitespace-pre-wrap",
                  error ? "border-destructive bg-destructive/10" : "border-border bg-muted/50"
                )}
              >
                {error || output || "Waiting for output..."}
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={handleClose} disabled={isRunning}>
            Close
          </Button>
          {isRunning ? (
            <Button variant="destructive" onClick={handleCancel}>
              <Square className="mr-2 h-4 w-4" />
              Cancel
            </Button>
          ) : (
            <Button onClick={handleRun} disabled={!message.trim() || !agent.isEnabled}>
              <Play className="mr-2 h-4 w-4" />
              Run Agent
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
