import { useEffect, useMemo, useState } from "react";

export interface MigrationZoneSummary {
  zone: string;
  services: string[];
  ports: string[];
  interfaces: string[];
  source_addresses: string[];
  rules_generated: number;
}

export interface MigrationPreview {
  changeset: unknown;
  zone_summaries: MigrationZoneSummary[];
  warnings: string[];
  nft_preview: string;
}

interface Props {
  migration?: MigrationPreview;
  loading: boolean;
  applying: boolean;
  onGenerate: () => void;
  onApply: (changesetJson: string, enableCompatSwitch: boolean) => void;
}

export function MigrationWizard({ migration, loading, applying, onGenerate, onApply }: Props) {
  const [step, setStep] = useState<number>(1);
  const [enableCompatSwitch, setEnableCompatSwitch] = useState(false);

  useEffect(() => {
    if (migration) {
      setStep(2);
    }
  }, [migration]);

  const changesetJson = useMemo(() => {
    if (!migration) {
      return "";
    }
    return JSON.stringify((migration as { changeset: unknown }).changeset);
  }, [migration]);

  return (
    <section className="rounded border border-slate-700 bg-slate-900/40 p-4">
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-sm font-semibold">Migrate from firewalld</h3>
        <span className="rounded bg-slate-700 px-2 py-1 text-[10px] uppercase tracking-wide text-slate-200">
          Step {step} / 3
        </span>
      </div>

      {step === 1 && (
        <div className="space-y-3 text-sm">
          <p className="text-slate-300">
            Scan firewalld zones and generate a Palisade changeset preview before applying.
          </p>
          <button
            type="button"
            onClick={onGenerate}
            disabled={loading}
            className="rounded bg-blue-600 px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-60"
          >
            {loading ? "Scanning…" : "Detect and Build Preview"}
          </button>
        </div>
      )}

      {step === 2 && migration && (
        <div className="space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <div className="rounded border border-slate-700 bg-slate-950 p-3">
              <div className="mb-2 text-xs font-semibold text-slate-300">Detected firewalld Zones</div>
              <div className="max-h-64 space-y-2 overflow-auto pr-1 text-xs">
                {migration.zone_summaries.map((zone) => (
                  <div key={zone.zone} className="rounded border border-slate-700 bg-slate-900/50 p-2">
                    <div className="font-semibold text-slate-100">{zone.zone}</div>
                    <div className="mt-1 text-slate-300">Services: {zone.services.join(", ") || "none"}</div>
                    <div className="text-slate-300">Ports: {zone.ports.join(", ") || "none"}</div>
                    <div className="text-slate-300">Interfaces: {zone.interfaces.join(", ") || "none"}</div>
                    <div className="text-slate-400">Generated rules: {zone.rules_generated}</div>
                  </div>
                ))}
                {migration.zone_summaries.length === 0 && (
                  <div className="text-slate-400">No firewalld zones found.</div>
                )}
              </div>
            </div>
            <div className="rounded border border-slate-700 bg-slate-950 p-3">
              <div className="mb-2 text-xs font-semibold text-slate-300">Generated nft Preview</div>
              <pre className="max-h-64 overflow-auto whitespace-pre-wrap text-xs text-slate-200">
                {migration.nft_preview || "# No operations generated"}
              </pre>
            </div>
          </div>

          <div>
            <div className="mb-1 text-xs font-semibold text-slate-300">Warnings</div>
            <div className="max-h-32 overflow-auto rounded border border-slate-700 bg-slate-950 p-2 text-xs">
              {migration.warnings.length > 0 ? (
                <ul className="space-y-1 text-amber-300">
                  {migration.warnings.map((warning) => (
                    <li key={warning}>- {warning}</li>
                  ))}
                </ul>
              ) : (
                <div className="text-slate-400">No migration warnings.</div>
              )}
            </div>
          </div>

          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setStep(3)}
              className="rounded bg-blue-600 px-3 py-2 text-sm"
              disabled={!changesetJson}
            >
              Continue to Apply
            </button>
            <button
              type="button"
              onClick={() => {
                setStep(1);
                onGenerate();
              }}
              className="rounded bg-slate-700 px-3 py-2 text-sm"
            >
              Rebuild Preview
            </button>
          </div>
        </div>
      )}

      {step === 3 && migration && (
        <div className="space-y-3 text-sm">
          <p className="text-slate-300">
            Apply migration using the standard validate → snapshot → dead man’s switch flow.
          </p>
          <label className="flex items-center gap-2 rounded border border-slate-700 bg-slate-950 p-2 text-xs">
            <input
              type="checkbox"
              checked={enableCompatSwitch}
              onChange={(event) => setEnableCompatSwitch(event.target.checked)}
            />
            Disable firewalld and enable palisade-firewalld-compat after apply
          </label>
          <div className="flex gap-2">
            <button
              type="button"
              disabled={applying || !changesetJson}
              onClick={() => onApply(changesetJson, enableCompatSwitch)}
              className="rounded bg-blue-600 px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-60"
            >
              {applying ? "Applying…" : "Apply Migration"}
            </button>
            <button type="button" onClick={() => setStep(2)} className="rounded bg-slate-700 px-3 py-2 text-sm">
              Back
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
