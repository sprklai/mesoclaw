/**
 * Agents page - Main route for agent management and monitoring.
 *
 * Features:
 * - Agent list with CRUD operations
 * - Workspace editor for agent files
 * - Session history viewer
 * - Execution monitor for active runs
 */
import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import {
  AgentList,
  AgentCreateDialog,
  AgentWorkspaceEditor,
  SessionHistoryViewer,
  ExecutionMonitor,
} from "@/components/agents";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import { LoadingState } from "@/components/ui/loading-state";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAgentConfigStore } from "@/stores/agentConfigStore";
import { useLLMStore } from "@/stores/llm";
import type { AgentConfig, CreateAgentRequest, UpdateAgentRequest } from "@/lib/agent-config";

import { Bot, History, Plus, Settings, Zap } from "@/lib/icons";

// ─── Route Definition ──────────────────────────────────────────────────────

export const Route = createFileRoute("/agents")({
  component: AgentsPage,
});

// ─── Main Component ────────────────────────────────────────────────────────

function AgentsPage() {
  // Store state
  const {
    agents,
    selectedAgentId,
    workspaceFiles,
    sessions,
    activeRuns,
    isLoadingAgents,
    loadAgents,
    selectAgent,
    createAgent,
    updateAgent,
    deleteAgent,
    duplicateAgent,
    toggleAgentEnabled,
    loadWorkspaceFiles,
    updateWorkspaceFile,
    loadRecentSessions,
    clearSessionHistory,
    loadActiveRuns,
    cancelRun,
  } = useAgentConfigStore();

  const { providersWithModels, loadProvidersAndModels } = useLLMStore();

  // Local state
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [editingAgent, setEditingAgent] = useState<AgentConfig | null>(null);
  const [activeTab, setActiveTab] = useState("agents");

  // Load initial data
  useEffect(() => {
    loadAgents();
    loadProvidersAndModels();
    loadRecentSessions();
    loadActiveRuns();
  }, [loadAgents, loadProvidersAndModels, loadRecentSessions, loadActiveRuns]);

  // ─── Handlers ────────────────────────────────────────────────────────────

  const handleSelectAgent = (agent: AgentConfig) => {
    selectAgent(agent.id);
    setEditingAgent(agent);
    setCreateDialogOpen(true);
  };

  const handleCreateAgent = () => {
    setEditingAgent(null);
    setCreateDialogOpen(true);
  };

  const handleSubmitAgent = async (request: CreateAgentRequest | UpdateAgentRequest) => {
    if ("id" in request) {
      await updateAgent(request as UpdateAgentRequest);
    } else {
      await createAgent(request as CreateAgentRequest);
    }
  };

  const handleDeleteAgent = async (agent: AgentConfig) => {
    if (confirm(`Are you sure you want to delete "${agent.name}"?`)) {
      await deleteAgent(agent.id);
    }
  };

  const handleDuplicateAgent = async (agent: AgentConfig) => {
    await duplicateAgent(agent.id);
  };

  const handleToggleEnabled = async (agent: AgentConfig) => {
    await toggleAgentEnabled(agent.id);
  };

  const handleViewSession = (sessionId: string) => {
    // ## TODO: Navigate to session detail view
    console.log("View session:", sessionId);
  };

  const handleViewRun = (runId: string) => {
    // ## TODO: Navigate to run detail view
    console.log("View run:", runId);
  };

  // ─── Render ──────────────────────────────────────────────────────────────

  if (isLoadingAgents && agents.length === 0) {
    return <LoadingState message="Loading agents..." />;
  }

  const selectedAgent = agents.find((a) => a.id === selectedAgentId) ?? null;

  return (
    <div className="mx-auto w-full max-w-6xl">
      <PageHeader
        title="Agents"
        description="Manage multi-agent configurations and monitor executions"
      />

      <div className="flex flex-col gap-6">
        {/* Main content tabs */}
        <Tabs value={activeTab} onValueChange={setActiveTab}>
          <div className="flex items-center justify-between border-b border-border">
            <TabsList className="h-10">
              <TabsTrigger value="agents" className="gap-2">
                <Bot className="h-4 w-4" />
                Agents
              </TabsTrigger>
              <TabsTrigger value="workspace" className="gap-2">
                <Settings className="h-4 w-4" />
                Workspace
              </TabsTrigger>
              <TabsTrigger value="history" className="gap-2">
                <History className="h-4 w-4" />
                History
              </TabsTrigger>
              <TabsTrigger value="monitor" className="gap-2">
                <Zap className="h-4 w-4" />
                Monitor
                {activeRuns.length > 0 && (
                  <span className="ml-1 px-1.5 py-0.5 rounded-full bg-primary text-primary-foreground text-xs">
                    {activeRuns.length}
                  </span>
                )}
              </TabsTrigger>
            </TabsList>

            <Button onClick={handleCreateAgent} size="sm">
              <Plus className="mr-2 h-4 w-4" />
              New Agent
            </Button>
          </div>

          {/* Agents tab */}
          <TabsContent value="agents" className="mt-6">
            <AgentList
              agents={agents}
              isLoading={isLoadingAgents}
              selectedAgentId={selectedAgentId}
              onSelectAgent={handleSelectAgent}
              onDeleteAgent={handleDeleteAgent}
              onDuplicateAgent={handleDuplicateAgent}
              onToggleEnabled={handleToggleEnabled}
              onCreateAgent={handleCreateAgent}
            />
          </TabsContent>

          {/* Workspace tab */}
          <TabsContent value="workspace" className="mt-6">
            <AgentWorkspaceEditor
              agent={selectedAgent}
              workspaceFiles={workspaceFiles}
              onUpdateFile={updateWorkspaceFile}
              onLoadFiles={loadWorkspaceFiles}
              className="h-[600px] border border-border rounded-lg"
            />
          </TabsContent>

          {/* History tab */}
          <TabsContent value="history" className="mt-6">
            <SessionHistoryViewer
              sessions={sessions}
              agents={agents}
              onLoadSessions={loadRecentSessions}
              onClearHistory={clearSessionHistory}
              onViewSession={handleViewSession}
              className="h-[600px] border border-border rounded-lg"
            />
          </TabsContent>

          {/* Monitor tab */}
          <TabsContent value="monitor" className="mt-6">
            <ExecutionMonitor
              activeRuns={activeRuns}
              onLoadRuns={loadActiveRuns}
              onCancelRun={cancelRun}
              onViewRun={handleViewRun}
              className="h-[600px] border border-border rounded-lg"
            />
          </TabsContent>
        </Tabs>
      </div>

      {/* Create/Edit Dialog */}
      <AgentCreateDialog
        open={createDialogOpen}
        onOpenChange={(open) => {
          setCreateDialogOpen(open);
          if (!open) {
            setEditingAgent(null);
            selectAgent(null);
          }
        }}
        onSubmit={handleSubmitAgent}
        agent={editingAgent}
        providers={providersWithModels}
      />
    </div>
  );
}
