import { create } from "zustand";
import type { RemoteHost } from "../types/hosts";

interface HostsState {
  hosts: RemoteHost[];
  activeHostId?: string;
  setHosts: (hosts: RemoteHost[]) => void;
  setActiveHost: (id: string) => void;
}

export const useHostsStore = create<HostsState>((set) => ({
  hosts: [],
  setHosts: (hosts) => set({ hosts }),
  setActiveHost: (activeHostId) => set({ activeHostId }),
}));
