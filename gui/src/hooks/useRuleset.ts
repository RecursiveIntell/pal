import { useEffect } from "react";
import { daemonCall } from "./useDaemon";
import { useRulesetStore } from "../stores/ruleset";
import type { NftRule, NftTable } from "../types/nftables";

function parseRulesetJson(raw: string): NftTable[] {
  const value: unknown = JSON.parse(raw);
  if (typeof value !== "object" || value === null) {
    return [];
  }
  const arr = (value as { nftables?: unknown }).nftables;
  if (!Array.isArray(arr)) {
    return [];
  }

  const tables: NftTable[] = [];

  for (const entry of arr) {
    if (!isRecord(entry)) {
      continue;
    }
    const table = entry.table;
    if (isRecord(table) && typeof table.family === "string" && typeof table.name === "string") {
      tables.push({
        family: asFamily(table.family),
        name: table.name,
        owner: { type: table.name === "palisade" ? "palisade" : "unmanaged" },
        chains: [],
        sets: [],
        counters: [],
      });
    }
  }

  for (const entry of arr) {
    if (!isRecord(entry)) {
      continue;
    }

    const chain = entry.chain;
    if (isRecord(chain) && typeof chain.family === "string" && typeof chain.table === "string" && typeof chain.name === "string") {
      const chainFamily = chain.family;
      const chainTable = chain.table;
      const chainName = chain.name;
      const table = tables.find((t) => t.family === asFamily(chainFamily) && t.name === chainTable);
      if (!table) {
        continue;
      }
      table.chains.push({
        name: chainName,
        type: typeof chain.type === "string" ? asChainType(chain.type) : undefined,
        hook: typeof chain.hook === "string" ? asHook(chain.hook) : undefined,
        priority: typeof chain.prio === "number" ? chain.prio : undefined,
        policy: chain.policy === "accept" || chain.policy === "drop" ? chain.policy : undefined,
        rules: [],
        isBaseChain: typeof chain.hook === "string",
      });
    }

    const rule = entry.rule;
    if (isRecord(rule) && typeof rule.family === "string" && typeof rule.table === "string" && typeof rule.chain === "string") {
      const ruleFamily = rule.family;
      const ruleTable = rule.table;
      const ruleChain = rule.chain;
      const table = tables.find((t) => t.family === asFamily(ruleFamily) && t.name === ruleTable);
      const chain = table?.chains.find((c) => c.name === ruleChain);
      if (!chain) {
        continue;
      }
      const handle = typeof rule.handle === "number" ? rule.handle : 0;
      const action = detectAction(rule.expr);
      const nftRule: NftRule = {
        handle,
        summary: {
          matches: [],
          action: { type: action },
          description: typeof rule.comment === "string" ? rule.comment : `${action} rule`,
        },
        rawExpressions: Array.isArray(rule.expr) ? rule.expr : [],
        comment: typeof rule.comment === "string" ? rule.comment : undefined,
        position: chain.rules.length,
        isDisabled: false,
      };
      chain.rules.push(nftRule);
    }
  }

  return tables;
}

export function useRuleset() {
  const setTables = useRulesetStore((s) => s.setTables);

  useEffect(() => {
    void (async () => {
      const raw = await daemonCall<string>("list_ruleset");
      const tables = parseRulesetJson(raw);
      setTables(tables);
    })();
  }, [setTables]);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asFamily(family: string): NftTable["family"] {
  if (family === "ip" || family === "ip6" || family === "inet" || family === "arp" || family === "bridge" || family === "netdev") {
    return family;
  }
  return "inet";
}

function asChainType(value: string): "filter" | "nat" | "route" {
  if (value === "nat" || value === "route") {
    return value;
  }
  return "filter";
}

function asHook(value: string): "prerouting" | "input" | "forward" | "output" | "postrouting" | "ingress" | "egress" {
  if (
    value === "prerouting" ||
    value === "input" ||
    value === "forward" ||
    value === "output" ||
    value === "postrouting" ||
    value === "ingress" ||
    value === "egress"
  ) {
    return value;
  }
  return "input";
}

function detectAction(expr: unknown): NftRule["summary"]["action"]["type"] {
  if (!Array.isArray(expr)) {
    return "counter";
  }
  for (const item of expr) {
    if (!isRecord(item)) {
      continue;
    }
    if ("accept" in item) return "accept";
    if ("drop" in item) return "drop";
    if ("reject" in item) return "reject";
    if ("jump" in item) return "jump";
    if ("goto" in item) return "goto";
    if ("return" in item) return "return";
    if ("masquerade" in item) return "masquerade";
    if ("snat" in item) return "snat";
    if ("dnat" in item) return "dnat";
    if ("redirect" in item) return "redirect";
    if ("log" in item) return "log";
    if ("limit" in item) return "limit";
    if ("counter" in item) return "counter";
  }
  return "counter";
}
