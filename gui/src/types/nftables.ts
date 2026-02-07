export interface NftTable {
  family: "ip" | "ip6" | "inet" | "arp" | "bridge" | "netdev";
  name: string;
  owner: {
    type:
      | "palisade"
      | "docker"
      | "tailscale"
      | "fail2ban"
      | "firewalld"
      | "wireguard"
      | "unmanaged";
    label?: string;
  };
  chains: NftChain[];
  sets: NftSet[];
  counters: NftCounter[];
}

export interface NftChain {
  name: string;
  type?: "filter" | "nat" | "route";
  hook?:
    | "prerouting"
    | "input"
    | "forward"
    | "output"
    | "postrouting"
    | "ingress"
    | "egress";
  priority?: number;
  policy?: "accept" | "drop";
  rules: NftRule[];
  isBaseChain: boolean;
}

export interface NftRule {
  handle: number;
  summary: { matches: RuleMatch[]; action: RuleAction; description: string };
  rawExpressions: unknown[];
  comment?: string;
  counter?: { packets: number; bytes: number };
  counterDelta?: { packetsPerSec: number; bytesPerSec: number };
  position: number;
  tag?: string;
  isDisabled: boolean;
  managedByService?: string;
  managedRuleId?: string;
  readOnly?: boolean;
}

export interface RuleMatch {
  field: string;
  operator: string;
  value: string;
  negated: boolean;
}

export interface RuleAction {
  type:
    | "accept"
    | "drop"
    | "reject"
    | "jump"
    | "goto"
    | "return"
    | "masquerade"
    | "snat"
    | "dnat"
    | "redirect"
    | "log"
    | "counter"
    | "limit";
  target?: string;
  rejectType?: string;
  logPrefix?: string;
  natAddress?: string;
  natPort?: string;
}

export interface NftSet {
  name: string;
  type: string;
  flags: string[];
  timeout?: number;
  elements: SetElement[];
  size?: number;
}

export interface SetElement {
  value: string;
}

export interface NftCounter {
  name: string;
  packets: number;
  bytes: number;
}
