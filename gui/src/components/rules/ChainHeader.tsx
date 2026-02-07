import type { NftChain } from "../../types/nftables";

interface Props {
  chain?: NftChain;
  onExportJson?: () => void;
  onExportText?: () => void;
}

export function ChainHeader({ chain, onExportJson, onExportText }: Props) {
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
    </div>
  );
}
