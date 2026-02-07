interface Props {
  params: Record<string, string>;
  onChange: (params: Record<string, string>) => void;
}

export function ParameterForm({ params, onChange }: Props) {
  return (
    <div className="space-y-2">
      {Object.entries(params).map(([key, value]) => (
        <label key={key} className="block text-xs">
          <span className="mb-1 block text-slate-300">{key}</span>
          <input
            value={value}
            onChange={(event) => onChange({ ...params, [key]: event.target.value })}
            className="w-full rounded border border-slate-700 bg-slate-900 px-2 py-1 text-sm"
          />
        </label>
      ))}
    </div>
  );
}
