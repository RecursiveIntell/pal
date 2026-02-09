import { useCallback, useEffect, useMemo, useState } from "react";
import { ChainHeader } from "./components/rules/ChainHeader";
import { RuleEditor } from "./components/rules/RuleEditor";
import { RuleTable } from "./components/rules/RuleTable";
import { TableTree } from "./components/rules/TableTree";
import { Sidebar } from "./components/layout/Sidebar";
import { StatusBar } from "./components/layout/StatusBar";
import { TopBar } from "./components/layout/TopBar";
import { SnapshotDiff } from "./components/snapshots/SnapshotDiff";
import { SnapshotList } from "./components/snapshots/SnapshotList";
import { TemplateDetail } from "./components/templates/TemplateDetail";
import { TemplateList } from "./components/templates/TemplateList";
import { MigrationWizard, type MigrationPreview } from "./components/templates/MigrationWizard";
import { BandwidthChart } from "./components/traffic/BandwidthChart";
import { FlowTable } from "./components/traffic/FlowTable";
import { RuleHitRates } from "./components/traffic/RuleHitRates";
import { TopTalkers } from "./components/traffic/TopTalkers";
import { ConfirmDialog } from "./components/shared/ConfirmDialog";
import { DeadManCountdown } from "./components/shared/DeadManCountdown";
import { daemonCall, useDaemonConnection } from "./hooks/useDaemon";
import { useRuleset } from "./hooks/useRuleset";
import { useTraffic } from "./hooks/useTraffic";
import { selectedChain, selectedRule, useRulesetStore } from "./stores/ruleset";
import { useTrafficStore } from "./stores/traffic";
import { useUiStore } from "./stores/ui";
import type { Changeset, RuleSpec } from "./types/changeset";
import type { NftTable } from "./types/nftables";

export function App() {
  useDaemonConnection();
  useRuleset();
  const view = useUiStore((s) => s.view);
  const trafficRefreshMs = useUiStore((s) => s.trafficRefreshMs);
  const trafficLingerSeconds = useUiStore((s) => s.trafficLingerSeconds);
  const setTrafficRefreshMs = useUiStore((s) => s.setTrafficRefreshMs);
  const setTrafficLingerSeconds = useUiStore((s) => s.setTrafficLingerSeconds);
  useTraffic(view === "traffic", trafficRefreshMs, trafficLingerSeconds);

  const deadManSeconds = useUiStore((s) => s.deadManSeconds);
  const setDeadManSeconds = useUiStore((s) => s.setDeadManSeconds);

  const tables = useRulesetStore((s) => s.tables);
  const selected = useRulesetStore((s) => s.selected);
  const selectChain = useRulesetStore((s) => s.selectChain);
  const selectRule = useRulesetStore((s) => s.selectRule);
  const setDraftRule = useRulesetStore((s) => s.setDraftRule);
  const reorderRules = useRulesetStore((s) => s.reorderRules);

  const chain = useMemo(() => selectedChain(tables, selected), [tables, selected]);

  const [confirmOpen, setConfirmOpen] = useState(false);
  const [deleteSnapshotOpen, setDeleteSnapshotOpen] = useState(false);
  const [lastApplyId, setLastApplyId] = useState<string | undefined>(undefined);
  const [pendingRule, setPendingRule] = useState<RuleSpec | undefined>(undefined);

  const [templates, setTemplates] = useState<string[]>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<string | undefined>(undefined);
  const [templateParams, setTemplateParams] = useState<Record<string, string>>({ interface: "eth0", ssh_port: "22" });
  const [templateRendered, setTemplateRendered] = useState("");
  const [serviceRuleSummary, setServiceRuleSummary] = useState<Record<string, number>>({});
  const [migrationAvailable, setMigrationAvailable] = useState(false);
  const [migrationPreview, setMigrationPreview] = useState<MigrationPreview | undefined>(undefined);
  const [migrationLoading, setMigrationLoading] = useState(false);
  const [migrationApplying, setMigrationApplying] = useState(false);
  const [switchCompatAfterConfirm, setSwitchCompatAfterConfirm] = useState(false);

  const [snapshots, setSnapshots] = useState<string[]>([]);
  const [selectedSnapshot, setSelectedSnapshot] = useState<string | undefined>(undefined);
  const [selectedSnapshotContent, setSelectedSnapshotContent] = useState<string>("");
  const [currentRulesetContent, setCurrentRulesetContent] = useState<string>("");
  const [snapshotMessage, setSnapshotMessage] = useState<string>("");
  const trafficSummary = useTrafficStore((s) => s.summary);
  const trafficHistory = useTrafficStore((s) => s.history);
  const trafficLiveFlows = useTrafficStore((s) => s.liveFlows);
  const trafficLiveCounterDeltas = useTrafficStore((s) => s.liveCounterDeltas);
  const trafficMonitoring = useTrafficStore((s) => s.monitoring);
  const trafficError = useTrafficStore((s) => s.error);
  const pendingApply = Boolean(lastApplyId) && deadManSeconds !== undefined && deadManSeconds > 0;
  const activeRule = useMemo(() => selectedRule(chain, selected?.handle), [chain, selected?.handle]);
  const readOnlyRuleReason = useMemo(() => {
    if (chain?.name === "service-rules") {
      return "Rules in service-rules are managed by external services and are read-only.";
    }
    if (activeRule?.managedByService) {
      return `This rule is managed by ${activeRule.managedByService} and is read-only.`;
    }
    return undefined;
  }, [activeRule?.managedByService, chain?.name]);

  useEffect(() => {
    if (view !== "templates") {
      return;
    }
    void daemonCall<string[]>("list_templates").then(setTemplates).catch(() => setTemplates([]));
  }, [view]);

  useEffect(() => {
    if (view !== "rules") {
      return;
    }
    void daemonCall<string>("list_all_service_rules")
      .then((raw) => setServiceRuleSummary(extractServiceRuleSummary(raw)))
      .catch(() => setServiceRuleSummary({}));
  }, [tables, view]);

  useEffect(() => {
    if (view !== "templates") {
      return;
    }
    setMigrationPreview(undefined);
    setMigrationAvailable(false);
    void daemonCall<string>("migrate_firewalld_zones")
      .then((raw) => {
        const parsed = parseMigrationPreview(raw);
        setMigrationAvailable(parsed.zone_summaries.length > 0);
      })
      .catch(() => setMigrationAvailable(false));
  }, [view]);

  useEffect(() => {
    if (!selectedTemplate) {
      return;
    }
    void daemonCall<string>("render_template", {
      templateName: selectedTemplate,
      params: templateParams,
    })
      .then(setTemplateRendered)
      .catch((error) => setTemplateRendered(`render error: ${String(error)}`));
  }, [selectedTemplate, templateParams]);

  const refreshSnapshots = useCallback(async () => {
    try {
      const raw = await daemonCall<string>("list_snapshots");
      const parsed: unknown = JSON.parse(raw);
      const list = Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === "string") : [];
      setSnapshots(list);
      setSelectedSnapshot((current) => {
        if (current && list.includes(current)) {
          return current;
        }
        return list[0];
      });
    } catch (error) {
      setSnapshots([]);
      setSelectedSnapshot(undefined);
      setSnapshotMessage(`Failed to list snapshots: ${String(error)}`);
    }
  }, []);

  useEffect(() => {
    if (view !== "snapshots") {
      return;
    }
    setSnapshotMessage("Snapshots are created automatically before every apply. You can also create one manually.");
    void refreshSnapshots();
    void daemonCall<string>("list_ruleset")
      .then((raw) => setCurrentRulesetContent(prettyJson(raw)))
      .catch((error) => setCurrentRulesetContent(`Failed to load current ruleset: ${String(error)}`));
  }, [refreshSnapshots, view]);

  useEffect(() => {
    if (view !== "snapshots") {
      return;
    }
    if (!selectedSnapshot) {
      setSelectedSnapshotContent("");
      return;
    }
    void daemonCall<[boolean, string]>("get_snapshot", { id: selectedSnapshot })
      .then(([ok, payload]) => {
        if (!ok) {
          setSnapshotMessage(`Failed to load snapshot: ${payload}`);
          setSelectedSnapshotContent("");
          return;
        }
        setSelectedSnapshotContent(prettyJson(payload));
      })
      .catch((error) => {
        setSnapshotMessage(`Failed to load snapshot: ${String(error)}`);
        setSelectedSnapshotContent("");
      });
  }, [selectedSnapshot, view]);

  useEffect(() => {
    if (!deadManSeconds) {
      return;
    }
    const timer = window.setInterval(() => {
      setDeadManSeconds(Math.max(0, (deadManSeconds ?? 0) - 1));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [deadManSeconds, setDeadManSeconds]);

  async function startApplyFlow(rule: RuleSpec) {
    const changeset = createChangeset(selected, rule);
    if (!changeset) {
      return;
    }
    const valid = await daemonCall<[boolean, string]>("validate_changeset", { changesetJson: JSON.stringify(toDaemonChangeset(changeset)) });
    if (!valid[0]) {
      window.alert(`validate failed: ${valid[1]}`);
      return;
    }
    setConfirmOpen(true);
  }

  async function confirmApply() {
    setConfirmOpen(false);
    if (!pendingRule) {
      return;
    }
    const changeset = createChangeset(selected, pendingRule);
    if (!changeset) {
      return;
    }
    const [applyId, error] = await daemonCall<[string, string]>("apply_changeset", {
      changesetJson: JSON.stringify(toDaemonChangeset(changeset)),
      timeoutSecs: 60,
    });
    if (error) {
      window.alert(error);
      return;
    }
    setLastApplyId(applyId);
    setDeadManSeconds(60);
  }

  async function keepApply() {
    if (!lastApplyId) {
      return;
    }
    await daemonCall<boolean>("confirm_apply", { applyId: lastApplyId });
    if (switchCompatAfterConfirm) {
      const [ok, error] = await daemonCall<[boolean, string]>("switch_firewalld_to_compat");
      if (!ok) {
        window.alert(`Post-migration service switch failed: ${error}`);
      }
      setSwitchCompatAfterConfirm(false);
    }
    setDeadManSeconds(undefined);
    setLastApplyId(undefined);
  }

  async function rollbackApply() {
    if (!lastApplyId) {
      return;
    }
    await daemonCall<boolean>("rollback_apply", { applyId: lastApplyId });
    setSwitchCompatAfterConfirm(false);
    setDeadManSeconds(undefined);
    setLastApplyId(undefined);
  }

  async function buildMigrationPreview() {
    setMigrationLoading(true);
    try {
      const raw = await daemonCall<string>("migrate_firewalld_zones");
      const parsed = parseMigrationPreview(raw);
      setMigrationPreview(parsed);
      setMigrationAvailable(parsed.zone_summaries.length > 0);
    } catch (error) {
      window.alert(`Failed to build migration preview: ${String(error)}`);
      setMigrationPreview(undefined);
    } finally {
      setMigrationLoading(false);
    }
  }

  async function applyMigration(changesetJson: string, enableCompatSwitch: boolean) {
    setMigrationApplying(true);
    try {
      const [valid, validationError] = await daemonCall<[boolean, string]>("validate_changeset", { changesetJson });
      if (!valid) {
        window.alert(`Migration validation failed: ${validationError}`);
        setSwitchCompatAfterConfirm(false);
        return;
      }
      const [applyId, applyError] = await daemonCall<[string, string]>("apply_changeset", {
        changesetJson,
        timeoutSecs: 60,
      });
      if (applyError) {
        window.alert(`Migration apply failed: ${applyError}`);
        setSwitchCompatAfterConfirm(false);
        return;
      }
      setLastApplyId(applyId);
      setDeadManSeconds(60);
      setSwitchCompatAfterConfirm(enableCompatSwitch);
      window.alert("Migration staged. Confirm within 60 seconds using Keep Changes.");
    } catch (error) {
      setSwitchCompatAfterConfirm(false);
      window.alert(`Migration apply failed unexpectedly: ${String(error)}`);
    } finally {
      setMigrationApplying(false);
    }
  }

  async function createSnapshotNow() {
    const [ok, result] = await daemonCall<[boolean, string]>("create_snapshot");
    if (!ok) {
      setSnapshotMessage(`Create snapshot failed: ${result}`);
      return;
    }
    setSnapshotMessage(`Snapshot created: ${result}`);
    await refreshSnapshots();
    setSelectedSnapshot(result);
  }

  async function restoreSelectedSnapshot() {
    if (!selectedSnapshot) {
      return;
    }
    const [ok, error] = await daemonCall<[boolean, string]>("restore_snapshot", { id: selectedSnapshot });
    if (!ok) {
      setSnapshotMessage(`Restore failed: ${error}`);
      return;
    }
    setSnapshotMessage(`Restored snapshot: ${selectedSnapshot}`);
    const raw = await daemonCall<string>("list_ruleset");
    setCurrentRulesetContent(prettyJson(raw));
  }

  function exportSelectedSnapshot() {
    if (!selectedSnapshot || !selectedSnapshotContent) {
      setSnapshotMessage("Select a snapshot first, then export.");
      return;
    }
    downloadTextFile(`${selectedSnapshot}.json`, selectedSnapshotContent);
    setSnapshotMessage(`Exported ${selectedSnapshot}.json`);
  }

  function exportCurrentRulesetSnapshotView() {
    if (!currentRulesetContent) {
      setSnapshotMessage("Current ruleset not loaded yet.");
      return;
    }
    const stamp = new Date().toISOString().replaceAll(":", "-");
    downloadTextFile(`current-ruleset-${stamp}.json`, currentRulesetContent);
    setSnapshotMessage("Exported current ruleset JSON");
  }

  async function deleteSelectedSnapshot() {
    if (!selectedSnapshot) {
      setSnapshotMessage("Select a snapshot first, then delete.");
      return;
    }
    setDeleteSnapshotOpen(true);
  }

  async function confirmDeleteSelectedSnapshot() {
    setDeleteSnapshotOpen(false);
    if (!selectedSnapshot) {
      return;
    }
    const [ok, error] = await daemonCall<[boolean, string]>("delete_snapshot", { id: selectedSnapshot });
    if (!ok) {
      setSnapshotMessage(`Delete failed: ${error}`);
      return;
    }
    setSnapshotMessage(`Deleted snapshot: ${selectedSnapshot}`);
    setSelectedSnapshot(undefined);
    setSelectedSnapshotContent("");
    await refreshSnapshots();
  }

  return (
    <div className="flex h-full w-full flex-col overflow-hidden">
      <TopBar />
      <div className="flex min-h-0 flex-1 overflow-hidden">
        <Sidebar />
        {view === "rules" && (
          <>
            <TableTree tables={tables} selected={selected} onSelectChain={selectChain} />
            <main className="flex min-w-0 flex-1 flex-col overflow-hidden">
              <ChainHeader
                chain={chain}
                onExportJson={() => downloadTextFile("ruleset.json", JSON.stringify(tables, null, 2))}
                onExportText={() => downloadTextFile("ruleset.nft", toNftText(tables))}
                serviceRuleSummary={serviceRuleSummary}
              />
              <div className="min-h-0 flex-1 overflow-auto">
                <RuleTable
                  rules={chain?.rules ?? []}
                  selectedHandle={selected?.handle}
                  onSelect={selectRule}
                  onReorder={(from, to) => {
                    if (!selected) {
                      return;
                    }
                    if (selected.chain === "service-rules") {
                      return;
                    }
                    reorderRules(selected.family, selected.table, selected.chain, from, to);
                  }}
                />
              </div>
              <RuleEditor
                readOnlyReason={readOnlyRuleReason}
                onSave={(rule: RuleSpec) => {
                  setDraftRule(rule);
                  setPendingRule(rule);
                  void startApplyFlow(rule);
                }}
              />
            </main>
          </>
        )}
        {view === "templates" && (
          <>
            <TemplateList templates={templates} selected={selectedTemplate} onSelect={setSelectedTemplate} />
            <div className="flex min-w-0 flex-1 flex-col overflow-auto p-4">
              {migrationAvailable && (
                <div className="mb-4">
                  <MigrationWizard
                    migration={migrationPreview}
                    loading={migrationLoading}
                    applying={migrationApplying}
                    pendingApply={pendingApply}
                    rollbackSeconds={deadManSeconds}
                    onGenerate={() => {
                      void buildMigrationPreview();
                    }}
                    onApply={(changesetJson, enableCompatSwitch) => {
                      void applyMigration(changesetJson, enableCompatSwitch);
                    }}
                    onKeep={() => {
                      void keepApply();
                    }}
                    onRollback={() => {
                      void rollbackApply();
                    }}
                  />
                </div>
              )}
              <TemplateDetail
                name={selectedTemplate}
                rendered={templateRendered}
                params={templateParams}
                onChangeParams={setTemplateParams}
              />
            </div>
          </>
        )}
        {view === "traffic" && (
          <main className="min-w-0 flex-1 overflow-auto p-3">
            <div className="mb-3 rounded border border-slate-700 bg-slate-900/50 p-3 text-xs text-slate-300">
              <div className="flex flex-wrap items-center gap-4">
                <span className={trafficMonitoring ? "text-emerald-400" : "text-amber-400"}>
                  {trafficMonitoring ? "Monitoring active" : "Monitoring stopped"}
                </span>
                <span>{trafficSummary.totalBytesPerSec.toLocaleString()} B/s</span>
                <span>{trafficSummary.totalPacketsPerSec.toLocaleString()} pkt/s</span>
                <label className="ml-auto flex items-center gap-2">
                  <span>Refresh</span>
                  <select
                    value={trafficRefreshMs}
                    onChange={(event) => setTrafficRefreshMs(Number(event.target.value))}
                    className="rounded border border-slate-700 bg-white px-2 py-1 text-xs font-semibold text-black"
                  >
                    <option value={500} className="text-black">
                      0.5s
                    </option>
                    <option value={1000} className="text-black">
                      1.0s
                    </option>
                    <option value={2000} className="text-black">
                      2.0s
                    </option>
                    <option value={5000} className="text-black">
                      5.0s
                    </option>
                  </select>
                </label>
                <label className="flex items-center gap-2">
                  <span>Linger (s)</span>
                  <input
                    type="number"
                    min={1}
                    max={99}
                    value={trafficLingerSeconds}
                    onChange={(event) => setTrafficLingerSeconds(Number(event.target.value))}
                    className="w-16 rounded border border-slate-700 bg-white px-2 py-1 text-xs font-semibold text-black"
                  />
                </label>
              </div>
              {trafficError && <div className="mt-2 text-rose-400">{trafficError}</div>}
            </div>
            <div className="grid grid-cols-[2fr_1fr] gap-3">
              <div className="space-y-3">
                <BandwidthChart history={trafficHistory} sampleMs={trafficRefreshMs} />
                <FlowTable flows={trafficLiveFlows} />
              </div>
              <div className="space-y-3">
                <TopTalkers flows={trafficLiveFlows} />
                <RuleHitRates deltas={trafficLiveCounterDeltas} />
              </div>
            </div>
          </main>
        )}
        {view === "snapshots" && (
          <main className="flex min-h-0 min-w-0 flex-1 overflow-hidden">
            <SnapshotList
              snapshots={snapshots}
              selected={selectedSnapshot}
              onSelect={setSelectedSnapshot}
              onRefresh={() => void refreshSnapshots()}
              onCreate={() => void createSnapshotNow()}
              onRestore={() => void restoreSelectedSnapshot()}
              onExportSelected={exportSelectedSnapshot}
              onExportCurrent={exportCurrentRulesetSnapshotView}
              onDeleteSelected={() => void deleteSelectedSnapshot()}
            />
            <div className="flex min-h-0 min-w-0 flex-1 flex-col">
              <div className="border-b border-slate-700 bg-slate-900/40 px-4 py-2 text-xs text-slate-300">{snapshotMessage}</div>
              <SnapshotDiff
                left={selectedSnapshotContent}
                right={currentRulesetContent}
                leftTitle={selectedSnapshot ? `Snapshot: ${selectedSnapshot}` : "Snapshot"}
                rightTitle="Current Ruleset"
              />
            </div>
          </main>
        )}
      </div>
      {deadManSeconds !== undefined && deadManSeconds > 0 && (
        <div className="border-t border-slate-700 bg-slate-950/80 px-4 py-2">
          <DeadManCountdown seconds={deadManSeconds} onKeep={keepApply} onRollback={rollbackApply} />
        </div>
      )}
      <StatusBar />
      <ConfirmDialog
        open={confirmOpen}
        title="Apply Changeset"
        message="This will validate, snapshot, and apply with rollback timer. Continue?"
        confirmLabel="Apply"
        onCancel={() => setConfirmOpen(false)}
        onConfirm={() => {
          void confirmApply();
        }}
      />
      <ConfirmDialog
        open={deleteSnapshotOpen}
        title="Delete Snapshot"
        message={
          selectedSnapshot
            ? `Delete snapshot "${selectedSnapshot}"? This cannot be undone.`
            : "Delete selected snapshot?"
        }
        confirmLabel="Delete Snapshot"
        requireText="DELETE"
        onCancel={() => setDeleteSnapshotOpen(false)}
        onConfirm={() => {
          void confirmDeleteSelectedSnapshot();
        }}
      />
    </div>
  );
}

function toDaemonChangeset(cs: Changeset): Record<string, unknown> {
  return {
    version: cs.version,
    description: cs.description,
    operations: cs.operations.map((op) => {
      if (op.type === "add_rule") {
        return {
          type: "AddRule",
          family: op.family,
          table: op.table,
          chain: op.chain,
          position: "Last",
          rule: {
            expr: op.rule.matches.map((m) => ({ match: { left: m.field, op: m.operator, right: m.value } })),
            comment: op.rule.comment,
          },
        };
      }
      if (op.type === "replace_rule") {
        return {
          type: "ReplaceRule",
          family: op.family,
          table: op.table,
          chain: op.chain,
          handle: op.handle,
          rule: {
            expr: op.rule.matches.map((m) => ({ match: { left: m.field, op: m.operator, right: m.value } })),
            comment: op.rule.comment,
          },
        };
      }
      return {
        type: "DeleteRule",
        family: op.family,
        table: op.table,
        chain: op.chain,
        handle: op.handle,
      };
    }),
  };
}

function createChangeset(
  selected: { family: string; table: string; chain: string; handle?: number } | undefined,
  rule: RuleSpec,
): Changeset | undefined {
  if (!selected) {
    return undefined;
  }
  return {
    version: 1,
    description: "Inline rule edit",
    operations: [
      {
        type: "add_rule",
        family: selected.family,
        table: selected.table,
        chain: selected.chain,
        position: { type: "last" },
        rule,
      },
    ],
  };
}

function toNftText(tables: NftTable[]): string {
  const lines: string[] = [];
  for (const table of tables) {
    lines.push(`table ${table.family} ${table.name} {`);
    for (const chain of table.chains) {
      lines.push(`  chain ${chain.name} {`);
      if (chain.type && chain.hook) {
        lines.push(`    type ${chain.type} hook ${chain.hook} priority ${chain.priority ?? 0}; policy ${chain.policy ?? "accept"};`);
      }
      for (const rule of chain.rules) {
        lines.push(`    ${rule.summary.description};`);
      }
      lines.push("  }");
    }
    lines.push("}");
  }
  return lines.join("\n");
}

function downloadTextFile(filename: string, content: string) {
  const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

function prettyJson(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw) as unknown, null, 2);
  } catch {
    return raw;
  }
}

function extractServiceRuleSummary(raw: string): Record<string, number> {
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (typeof parsed !== "object" || parsed === null) {
      return {};
    }
    const out: Record<string, number> = {};
    for (const [service, entries] of Object.entries(parsed as Record<string, unknown>)) {
      out[service] = Array.isArray(entries) ? entries.length : 0;
    }
    return out;
  } catch {
    return {};
  }
}

function parseMigrationPreview(raw: string): MigrationPreview {
  const value = JSON.parse(raw) as Record<string, unknown>;
  return {
    changeset: value.changeset ?? {},
    zone_summaries: Array.isArray(value.zone_summaries) ? (value.zone_summaries as MigrationPreview["zone_summaries"]) : [],
    warnings: Array.isArray(value.warnings) ? value.warnings.filter((item): item is string => typeof item === "string") : [],
    nft_preview: typeof value.nft_preview === "string" ? value.nft_preview : "",
  };
}
