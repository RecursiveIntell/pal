import type { ComponentType } from "react";
import { Shield, FileText, History, Activity } from "lucide-react";
import { useUiStore, type ViewName } from "../../stores/ui";

const items: Array<{ key: ViewName; label: string; icon: ComponentType<{ size?: number }> }> = [
  { key: "rules", label: "Rules", icon: Shield },
  { key: "traffic", label: "Traffic", icon: Activity },
  { key: "templates", label: "Templates", icon: FileText },
  { key: "snapshots", label: "Snapshots", icon: History },
];

export function Sidebar() {
  const view = useUiStore((s) => s.view);
  const setView = useUiStore((s) => s.setView);

  return (
    <aside className="h-full min-h-0 w-52 overflow-y-auto border-r border-slate-700 bg-slate-900/80 p-3">
      <h1 className="mb-4 text-lg font-semibold">Palisade</h1>
      <nav className="space-y-2">
        {items.map((item) => {
          const Icon = item.icon;
          const active = item.key === view;
          return (
            <button
              key={item.key}
              type="button"
              onClick={() => setView(item.key)}
              className={`flex w-full items-center gap-2 rounded px-3 py-2 text-left transition ${
                active ? "bg-blue-600 text-white" : "bg-slate-800/50 hover:bg-slate-700"
              }`}
            >
              <Icon size={16} />
              {item.label}
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
