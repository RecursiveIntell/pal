interface Props {
  templates: string[];
  selected?: string;
  onSelect: (name: string) => void;
}

export function TemplateList({ templates, selected, onSelect }: Props) {
  return (
    <aside className="w-72 border-r border-slate-700 bg-slate-900/30 p-3">
      <h3 className="mb-3 text-sm font-semibold">Templates</h3>
      <div className="space-y-2">
        {templates.map((name) => (
          <button
            key={name}
            type="button"
            onClick={() => onSelect(name)}
            className={`w-full rounded px-2 py-2 text-left text-sm ${
              selected === name ? "bg-blue-600" : "bg-slate-800/70 hover:bg-slate-700"
            }`}
          >
            {name}
          </button>
        ))}
      </div>
    </aside>
  );
}
