/**
 * sidebarStore — thin Zustand store for mobile sidebar open/close state.
 *
 * Keeping this state outside the `SidebarMobile` component allows external
 * actors (e.g. the root layout's swipe gesture) to open and close the drawer
 * without prop-drilling through the component tree.
 */

import { create } from "zustand";

interface SidebarStore {
  mobileOpen: boolean;
  openMobile: () => void;
  closeMobile: () => void;
}

export const useSidebarStore = create<SidebarStore>()((set) => ({
  mobileOpen: false,
  openMobile: () => set({ mobileOpen: true }),
  closeMobile: () => set({ mobileOpen: false }),
}));
