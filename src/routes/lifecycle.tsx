import { createFileRoute } from "@tanstack/react-router";

import { LifecycleStatus } from "@/components/lifecycle/LifecycleStatus";
import { PageHeader } from "@/components/layout/PageHeader";

export const Route = createFileRoute("/lifecycle")({
  component: LifecyclePage,
});

function LifecyclePage() {
  return (
    <div className="space-y-6">
      <PageHeader
        title="Lifecycle Management"
        description="Monitor and manage application resources"
      />

      <LifecycleStatus showControls={true} />
    </div>
  );
}
