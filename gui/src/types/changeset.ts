import type { RuleAction, RuleMatch } from "./nftables";

export interface Changeset {
  version: number;
  description: string;
  operations: Operation[];
}

export type Position =
  | { type: "first" }
  | { type: "last" }
  | { type: "before"; handle: number }
  | { type: "after"; handle: number };

export type Operation =
  | { type: "add_rule"; family: string; table: string; chain: string; position: Position; rule: RuleSpec }
  | { type: "replace_rule"; family: string; table: string; chain: string; handle: number; rule: RuleSpec }
  | { type: "delete_rule"; family: string; table: string; chain: string; handle: number };

export interface RuleSpec {
  matches: RuleMatch[];
  action: RuleAction;
  comment?: string;
}
