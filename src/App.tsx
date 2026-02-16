import { Sidebar } from "@/components/ui/sidebar";

import "./styles/globals.css";

export default function App() {
  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <main className="flex flex-1 flex-col overflow-hidden">
        <div
          data-tauri-drag-region
          className="h-14 shrink-0 border-b border-border"
        />
        <div className="flex-1 overflow-auto p-6">
          <h1 className="text-2xl font-bold">Welcome</h1>
          <p className="mt-2 text-muted-foreground">
            Select a page from the sidebar to get started.
          </p>
        </div>
      </main>
    </div>
  );
}
