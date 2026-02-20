/**
 * AgentWorkspaceEditor - Editor for workspace files (SOUL.md, AGENTS.md, etc.)
 *
 * Features:
 * - Tabbed interface for different file types
 * - Markdown editor with preview
 * - Auto-save functionality
 * - File status indicators
 */
import { Eye, EyeOff, FileText, Loader2, Save, ScrollText } from "@/lib/icons";
import type { AgentConfig, WorkspaceFileType, WorkspaceFile } from "@/lib/agent-config";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";

import { useState, useEffect } from "react";

// ─── Types ────────────────────────────────────────────────────────────────

interface AgentWorkspaceEditorProps {
  agent: AgentConfig | null;
  workspaceFiles: Map<string, WorkspaceFile>;
  isLoading?: boolean;
  onUpdateFile: (agentId: string, type: WorkspaceFileType, content: string) => Promise<void>;
  onLoadFiles: (agentId: string) => Promise<void>;
  className?: string;
}

interface FileTab {
  type: WorkspaceFileType;
  label: string;
  description: string;
  icon: typeof FileText;
}

// ─── File Tab Definitions ──────────────────────────────────────────────────

const FILE_TABS: FileTab[] = [
  {
    type: "soul",
    label: "SOUL.md",
    description: "Core agent identity, personality, and behavioral traits",
    icon: FileText,
  },
  {
    type: "agents",
    label: "AGENTS.md",
    description: "Instructions for multi-agent collaboration",
    icon: FileText,
  },
  {
    type: "scratchpad",
    label: "SCRATCHPAD.md",
    description: "Working notes, plans, and temporary context",
    icon: ScrollText,
  },
  {
    type: "instructions",
    label: "INSTRUCTIONS.md",
    description: "Additional task-specific instructions",
    icon: FileText,
  },
];

// ─── Component ────────────────────────────────────────────────────────────

export function AgentWorkspaceEditor({
  agent,
  workspaceFiles,
  isLoading = false,
  onUpdateFile,
  onLoadFiles,
  className,
}: AgentWorkspaceEditorProps) {
  const [activeTab, setActiveTab] = useState<WorkspaceFileType>("soul");
  const [content, setContent] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);

  // Load files when agent changes
  useEffect(() => {
    if (agent) {
      onLoadFiles(agent.id);
    }
  }, [agent, onLoadFiles]);

  // Update content when tab or files change
  useEffect(() => {
    if (agent) {
      const file = workspaceFiles.get(`${agent.id}:${activeTab}`);
      setContent(file?.content ?? getDefaultContent(activeTab));
      setHasUnsavedChanges(false);
    }
  }, [agent, activeTab, workspaceFiles]);

  // ─── Handlers ────────────────────────────────────────────────────────────

  const handleContentChange = (value: string) => {
    setContent(value);
    setHasUnsavedChanges(true);
  };

  const handleSave = async () => {
    if (!agent || !hasUnsavedChanges) return;

    setIsSaving(true);
    try {
      await onUpdateFile(agent.id, activeTab, content);
      setHasUnsavedChanges(false);
    } finally {
      setIsSaving(false);
    }
  };

  const handleTabChange = (tab: string) => {
    if (hasUnsavedChanges) {
      // Prompt to save before switching
      if (confirm("You have unsaved changes. Save before switching?")) {
        handleSave();
      }
    }
    setActiveTab(tab as WorkspaceFileType);
  };

  // ─── Render ──────────────────────────────────────────────────────────────

  if (!agent) {
    return (
      <EmptyState
        icon={FileText}
        title="No agent selected"
        description="Select an agent to edit their workspace files."
        className={className}
      />
    );
  }

  if (isLoading) {
    return (
      <div className={cn("flex items-center justify-center p-8", className)}>
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className={cn("flex flex-col h-full", className)}>
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="flex items-center gap-2">
          <FileText className="h-4 w-4 text-muted-foreground" />
          <span className="font-medium">{agent.name}</span>
          <span className="text-muted-foreground">/</span>
          <span className="text-sm text-muted-foreground">Workspace</span>
        </div>
        <div className="flex items-center gap-2">
          {hasUnsavedChanges && (
            <Badge variant="warning" className="text-xs">
              Unsaved
            </Badge>
          )}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setShowPreview(!showPreview)}
            disabled={!content}
          >
            {showPreview ? (
              <>
                <EyeOff className="mr-2 h-4 w-4" />
                Hide Preview
              </>
            ) : (
              <>
                <Eye className="mr-2 h-4 w-4" />
                Preview
              </>
            )}
          </Button>
          <Button
            size="sm"
            onClick={handleSave}
            disabled={!hasUnsavedChanges || isSaving}
          >
            {isSaving ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Saving...
              </>
            ) : (
              <>
                <Save className="mr-2 h-4 w-4" />
                Save
              </>
            )}
          </Button>
        </div>
      </div>

      {/* Tabs */}
      <Tabs
        value={activeTab}
        onValueChange={handleTabChange}
        className="flex-1 flex flex-col"
      >
        <div className="border-b border-border px-4">
          <TabsList className="h-10">
            {FILE_TABS.map((tab) => {
              const file = workspaceFiles.get(`${agent.id}:${tab.type}`);
              const hasContent = !!file?.content;

              return (
                <TabsTrigger
                  key={tab.type}
                  value={tab.type}
                  className="text-xs gap-1.5"
                >
                  <tab.icon className="h-3.5 w-3.5" />
                  {tab.label.split(".")[0]}
                  {hasContent && (
                    <span className="w-1.5 h-1.5 rounded-full bg-primary" />
                  )}
                </TabsTrigger>
              );
            })}
          </TabsList>
        </div>

        {/* Tab Content */}
        {FILE_TABS.map((tab) => (
          <TabsContent
            key={tab.type}
            value={tab.type}
            className="flex-1 mt-0 border-0"
          >
            <div className="flex flex-col h-full">
              {/* Tab Description */}
              <div className="px-4 py-2 border-b border-border bg-muted/30">
                <p className="text-xs text-muted-foreground">{tab.description}</p>
              </div>

              {/* Editor / Preview */}
              <div className="flex-1 flex flex-col">
                {showPreview ? (
                  <div className="flex-1 flex">
                    {/* Editor */}
                    <div className="flex-1 border-r border-border">
                      <Textarea
                        value={content}
                        onChange={(e) => handleContentChange(e.target.value)}
                        placeholder={`Enter content for ${tab.label}...`}
                        className="w-full h-full min-h-[300px] border-0 rounded-none resize-none font-mono text-sm"
                      />
                    </div>

                    {/* Preview */}
                    <div className="flex-1 p-4 overflow-auto">
                      <div className="prose prose-sm dark:prose-invert max-w-none">
                        {content ? (
                          <MarkdownPreview content={content} />
                        ) : (
                          <p className="text-muted-foreground italic">
                            No content to preview
                          </p>
                        )}
                      </div>
                    </div>
                  </div>
                ) : (
                  <Textarea
                    value={content}
                    onChange={(e) => handleContentChange(e.target.value)}
                    placeholder={`Enter content for ${tab.label}...`}
                    className="flex-1 min-h-[300px] border-0 rounded-none resize-none font-mono text-sm p-4"
                  />
                )}
              </div>
            </div>
          </TabsContent>
        ))}
      </Tabs>
    </div>
  );
}

// ─── Helper Components ──────────────────────────────────────────────────────

function MarkdownPreview({ content }: { content: string }) {
  // Simple markdown preview - just render as preformatted for now
  // ## TODO: Add proper markdown rendering with a library like react-markdown
  return (
    <pre className="whitespace-pre-wrap text-sm">{content}</pre>
  );
}

// ─── Helper Functions ────────────────────────────────────────────────────────

function getDefaultContent(type: WorkspaceFileType): string {
  switch (type) {
    case "soul":
      return `# Agent Soul

This document defines the core identity, personality, and behavioral traits of this agent.

## Core Identity

[Describe the agent's fundamental nature and purpose]

## Behavioral Traits

- [Trait 1]
- [Trait 2]
- [Trait 3]

## Communication Style

[Describe how the agent communicates]

## Values

- [Value 1]
- [Value 2]
`;

    case "agents":
      return `# Multi-Agent Collaboration

This document contains instructions for how this agent collaborates with other agents.

## Collaboration Protocol

[Describe how this agent works with others]

## Message Format

[Define expected message formats]

## Handoff Procedures

[Describe how to hand off work to other agents]
`;

    case "scratchpad":
      return `# Scratchpad

Working notes, plans, and temporary context.

## Current Task

[Current task description]

## Notes

- [Note 1]
- [Note 2]

## Questions

- [Question 1]
`;

    case "instructions":
      return `# Additional Instructions

Task-specific instructions and guidelines.

## Task Guidelines

[Guidelines for specific tasks]

## Constraints

[Constraints and limitations]

## Best Practices

[Best practices for this agent]
`;

    default:
      return "";
  }
}
