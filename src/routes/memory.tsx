import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { PageHeader } from "@/components/layout/PageHeader";
import { useContextPanelStore } from "@/stores/contextPanelStore";

import { DailyTimeline } from "@/components/memory/DailyTimeline";
import { MemorySearch } from "@/components/memory/MemorySearch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

export const Route = createFileRoute("/memory")({
  component: MemoryPage,
});

function MemoryContextPanel({ tab }: { tab: "search" | "timeline" }) {
  const today = new Date().toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
  });

  return (
    <div className="space-y-4 p-4">
      {tab === "search" ? (
        <div>
          <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Search Tips
          </p>
          <ul className="space-y-2 text-xs text-muted-foreground">
            <li>Use natural language to query semantic memory.</li>
            <li>Try searching for topics, events, or entities.</li>
            <li>Results are ranked by semantic similarity.</li>
          </ul>
        </div>
      ) : (
        <div>
          <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Today
          </p>
          <p className="text-sm text-foreground">{today}</p>
          <p className="mt-2 text-xs text-muted-foreground">
            Browse journal entries by day in the timeline.
          </p>
        </div>
      )}
    </div>
  );
}

function MemoryPage() {
  const [tab, setTab] = useState<"search" | "timeline">("search");

  useEffect(() => {
    useContextPanelStore.getState().setContent(<MemoryContextPanel tab={tab} />);
    return () => useContextPanelStore.getState().clearContent();
  }, [tab]);

  return (
    <div className="flex h-full flex-col gap-4 overflow-hidden">
      <PageHeader
        title="Memory"
        description="Search the agent's semantic memory or browse daily journals."
      />

      <Tabs
        value={tab}
        onValueChange={(v) => setTab(v as "search" | "timeline")}
        className="flex flex-1 flex-col overflow-hidden"
      >
        <TabsList className="shrink-0 rounded-full bg-muted p-1">
          <TabsTrigger value="search" className="rounded-full">Search</TabsTrigger>
          <TabsTrigger value="timeline" className="rounded-full">Daily Timeline</TabsTrigger>
        </TabsList>

        <TabsContent value="search" className="flex-1 overflow-y-auto pt-2">
          <MemorySearch />
        </TabsContent>

        <TabsContent
          value="timeline"
          className="flex-1 overflow-y-auto pt-2"
        >
          <DailyTimeline />
        </TabsContent>
      </Tabs>
    </div>
  );
}
