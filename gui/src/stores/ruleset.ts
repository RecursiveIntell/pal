import { create } from "zustand";
import type { Changeset, RuleSpec } from "../types/changeset";
import type { NftChain, NftRule, NftTable } from "../types/nftables";

interface RulesetState {
  tables: NftTable[];
  selected?: { family: string; table: string; chain: string; handle?: number };
  draftRule?: RuleSpec;
  pendingApplyId?: string;
  setTables: (tables: NftTable[]) => void;
  selectChain: (family: string, table: string, chain: string) => void;
  selectRule: (handle: number) => void;
  setDraftRule: (rule: RuleSpec | undefined) => void;
  reorderRules: (family: string, table: string, chain: string, from: number, to: number) => void;
  buildChangesetFromDraft: () => Changeset | undefined;
}

export const useRulesetStore = create<RulesetState>((set, get) => ({
  tables: [],
  setTables: (tables) => set({ tables }),
  selectChain: (family, table, chain) => set({ selected: { family, table, chain } }),
  selectRule: (handle) => {
    const selected = get().selected;
    if (!selected) {
      return;
    }
    set({ selected: { ...selected, handle } });
  },
  setDraftRule: (draftRule) => set({ draftRule }),
  reorderRules: (family, table, chain, from, to) =>
    set((state) => {
      const tables = state.tables.map((t) => {
        if (t.family !== family || t.name !== table) {
          return t;
        }
        const chains = t.chains.map((c) => {
          if (c.name !== chain) {
            return c;
          }
          const rules = [...c.rules];
          const [moved] = rules.splice(from, 1);
          if (!moved) {
            return c;
          }
          rules.splice(to, 0, moved);
          return { ...c, rules: rules.map((r, index) => ({ ...r, position: index })) };
        });
        return { ...t, chains };
      });
      return { tables };
    }),
  buildChangesetFromDraft: () => {
    const state = get();
    if (!state.selected || !state.draftRule) {
      return undefined;
    }
    return {
      version: 1,
      description: "Inline rule edit",
      operations: [
        {
          type: "add_rule",
          family: state.selected.family,
          table: state.selected.table,
          chain: state.selected.chain,
          position: { type: "last" },
          rule: state.draftRule,
        },
      ],
    };
  },
}));

export function selectedChain(tables: NftTable[], selected?: { family: string; table: string; chain: string }): NftChain | undefined {
  if (!selected) {
    return undefined;
  }
  return tables
    .find((t) => t.family === selected.family && t.name === selected.table)
    ?.chains.find((c) => c.name === selected.chain);
}

export function selectedRule(chain: NftChain | undefined, selectedHandle?: number): NftRule | undefined {
  if (!chain || selectedHandle === undefined) {
    return undefined;
  }
  return chain.rules.find((r) => r.handle === selectedHandle);
}
