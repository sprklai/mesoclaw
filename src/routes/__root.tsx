import { Outlet, createRootRoute } from "@tanstack/react-router";

import logo from "@/assets/logo.png";
import { Sidebar } from "@/components/ui/sidebar";

export const Route = createRootRoute({
  component: RootLayout,
});

function RootLayout() {
  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <main className="flex flex-1 flex-col overflow-hidden">
        <div
          data-tauri-drag-region
          className="flex h-14 shrink-0 items-center justify-between border-b border-border px-4"
        >
          <div className="flex items-center gap-3">
            <img src={logo} alt="Tauri AI Boilerplate" className="h-7 w-7" />
            <span className="text-xl font-bold">Tauri AI Boilerplate</span>
          </div>
        </div>
        <div className="flex-1 overflow-auto p-4 pt-6 md:p-6">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
