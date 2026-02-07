import type { NftChain } from "../../types/nftables";

interface Props {
  chain?: NftChain;
  onExportJson?: () => void;
  onExportText?: () => void;
  serviceRuleSummary?: Record<string, number>;
}

export function ChainHeader({ chain, onExportJson, onExportText, serviceRuleSummary }: Props) {
  if (!chain) {
    return <div className="border-b border-slate-700 px-4 py-3 text-sm text-slate-400">Select a chain</div>;
  }

  return (
    <div className="border-b border-slate-700 px-4 py-3">
      <div className="flex items-center justify-between">
        <div>
          <div className="text-lg font-semibold">{chain.name}</div>
          <div className="mt-1 text-xs text-slate-400">
            type={chain.type ?? "filter"} hook={chain.hook ?? "none"} policy={chain.policy ?? "none"}
          </div>
        </div>
        <div className="flex gap-2 text-xs">
          <button type="button" onClick={onExportJson} className="rounded bg-slate-700 px-2 py-1">
            Export JSON
          </button>
          <button type="button" onClick={onExportText} className="rounded bg-slate-700 px-2 py-1">
            Export nft
          </button>
        </div>
      </div>
      {serviceRuleSummary && Object.keys(serviceRuleSummary).length > 0 && (
        <div className="mt-3 rounded border border-slate-700 bg-slate-900/50 p-2">
          <div className="mb-1 text-xs font-semibold text-slate-300">Service Rules</div>
          <div className="flex flex-wrap gap-2 text-xs">
            {Object.entries(serviceRuleSummary).map(([service, count]) => (
              <span
                key={service}
                className="rounded border border-slate-600 bg-slate-800 px-2 py-1 text-slate-200"
                title={`Managed by ${service}`}
              >
                {service}: {count}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
