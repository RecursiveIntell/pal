import { ParameterForm } from "./ParameterForm";

interface Props {
  name?: string;
  rendered: string;
  params: Record<string, string>;
  onChangeParams: (params: Record<string, string>) => void;
}

export function TemplateDetail({ name, rendered, params, onChangeParams }: Props) {
  if (!name) {
    return <section className="p-4 text-sm text-slate-400">Select a template</section>;
  }

  return (
    <section className="flex-1 space-y-4 p-4">
      <h3 className="text-lg font-semibold">{name}</h3>
      <ParameterForm params={params} onChange={onChangeParams} />
      <div>
        <h4 className="mb-2 text-sm font-semibold">Live Preview</h4>
        <pre className="max-h-[420px] overflow-auto rounded border border-slate-700 bg-slate-950 p-3 text-xs">
          {rendered}
        </pre>
      </div>
    </section>
  );
}
