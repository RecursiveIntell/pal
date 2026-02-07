import { useState } from "react";
import type { NftTable } from "../../types/nftables";

interface Props {
  tables: NftTable[];
  selected?: { family: string; table: string; chain: string };
  onSelectChain: (family: string, table: string, chain: string) => void;
}

export function TableTree({ tables, selected, onSelectChain }: Props) {
  const [openFamilies, setOpenFamilies] = useState<Record<string, boolean>>({ inet: true });

  const families = new Map<string, NftTable[]>();
  for (const table of tables) {
    const group = families.get(table.family) ?? [];
    group.push(table);
    families.set(table.family, group);
  }

  return (
    <aside className="flex h-full min-h-0 w-72 flex-col overflow-hidden border-r border-slate-700 bg-slate-900/40 p-3">
      <h2 className="mb-2 text-sm font-semibold text-slate-300">Ruleset</h2>
      <div className="min-h-0 flex-1 overflow-y-auto pr-1">
        {[...families.entries()].map(([family, familyTables]) => (
          <section key={family} className="mb-2">
            <button
              type="button"
              className="w-full rounded bg-slate-800/70 px-2 py-1 text-left text-xs uppercase tracking-wide"
              onClick={() => setOpenFamilies((prev) => ({ ...prev, [family]: !prev[family] }))}
            >
              {family}
            </button>
            {openFamilies[family] && (
              <div className="mt-1 space-y-1">
                {familyTables.map((table) => (
                  <div key={`${table.family}/${table.name}`} className="rounded border border-slate-700 p-2">
                    <div className="mb-1 flex items-center justify-between text-xs">
                      <span>{table.name}</span>
                      <span className="rounded bg-slate-700 px-1 py-0.5 text-[10px] text-slate-200">{table.owner.type}</span>
                    </div>
                    <div className="space-y-1">
                      {table.chains.map((chain) => {
                        const active =
                          selected?.family === table.family && selected?.table === table.name && selected?.chain === chain.name;
                        return (
                          <button
                            key={chain.name}
                            type="button"
                            className={`w-full rounded px-2 py-1 text-left text-xs ${
                              active ? "bg-blue-700/70" : "bg-slate-800/70 hover:bg-slate-700/70"
                            }`}
                            onClick={() => onSelectChain(table.family, table.name, chain.name)}
                          >
                            {chain.name}
                          </button>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>
        ))}
      </div>
    </aside>
  );
}
