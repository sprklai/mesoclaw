import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";

import { DailyTimeline } from "@/components/memory/DailyTimeline";
import { MemorySearch } from "@/components/memory/MemorySearch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

export const Route = createFileRoute("/memory")({
  component: MemoryPage,
});

function MemoryPage() {
  const [tab, setTab] = useState<"search" | "timeline">("search");

  return (
    <div className="flex h-full flex-col gap-4 overflow-hidden p-4">
      <div>
        <h1 className="text-xl font-semibold">Memory</h1>
        <p className="text-sm text-muted-foreground">
          Search the agent's semantic memory or browse daily journals.
        </p>
      </div>

      <Tabs
        value={tab}
        onValueChange={(v) => setTab(v as "search" | "timeline")}
        className="flex flex-1 flex-col overflow-hidden"
      >
        <TabsList className="shrink-0">
          <TabsTrigger value="search">Search</TabsTrigger>
          <TabsTrigger value="timeline">Daily Timeline</TabsTrigger>
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
