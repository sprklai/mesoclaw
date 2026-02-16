export function SidebarSkeleton() {
  return (
    <aside className="hidden h-screen w-64 flex-col border-r border-sidebar-border bg-sidebar md:flex">
      <SidebarHeaderSkeleton />
      <SidebarNavSkeleton />
      <SidebarBottomNavSkeleton />
    </aside>
  );
}

function SidebarHeaderSkeleton() {
  return (
    <div className="flex h-14 items-center justify-between border-b border-sidebar-border px-3">
      <div className="h-5 w-24 animate-pulse rounded bg-sidebar-foreground/10" />
      <div className="h-9 w-9 animate-pulse rounded-md bg-sidebar-foreground/10" />
    </div>
  );
}

function SidebarNavSkeleton() {
  return (
    <div className="flex-1 space-y-1 p-3">
      {[...Array(3)].map((_, i) => (
        <div key={i} className="flex h-10 items-center gap-3 px-3">
          <div className="size-5 animate-pulse rounded bg-sidebar-foreground/10" />
          <div className="h-4 flex-1 animate-pulse rounded bg-sidebar-foreground/10" />
        </div>
      ))}
    </div>
  );
}

function SidebarBottomNavSkeleton() {
  return (
    <div className="space-y-1 border-t border-sidebar-border p-3">
      <div className="flex h-10 items-center gap-3 px-3">
        <div className="size-5 animate-pulse rounded bg-sidebar-foreground/10" />
        <div className="h-4 flex-1 animate-pulse rounded bg-sidebar-foreground/10" />
      </div>
    </div>
  );
}
