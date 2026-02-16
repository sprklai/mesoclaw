import { RouterProvider, createRouter } from "@tanstack/react-router";
import { attachConsole } from "@tauri-apps/plugin-log";
import React from "react";
import ReactDOM from "react-dom/client";
import { Toaster } from "sonner";

import { StoreInitializer } from "@/components/store-initializer";
import { ThemeProvider } from "@/components/ThemeProvider";

import { routeTree } from "./routeTree.gen";
import "./styles/globals.css";

// Attach console to forward Rust logs to browser devtools
attachConsole();

const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <StoreInitializer />
    <ThemeProvider>
      <RouterProvider router={router} />
      <Toaster
        position="top-right"
        richColors
        closeButton
        toastOptions={{
          classNames: {
            toast: "font-sans",
            title: "text-sm font-medium",
            description: "text-xs",
          },
        }}
      />
    </ThemeProvider>
  </React.StrictMode>
);
