import type { ReactNode } from "react";
import { create } from "zustand";

interface ContextPanelState {
  content: ReactNode | null;
  setContent: (content: ReactNode) => void;
  clearContent: () => void;
}

export const useContextPanelStore = create<ContextPanelState>((set) => ({
  content: null,
  setContent: (content) => set({ content }),
  clearContent: () => set({ content: null }),
}));
