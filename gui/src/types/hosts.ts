export interface RemoteHost {
  id: string;
  name: string;
  hostname: string;
  port: number;
  username: string;
  authMethod: "key" | "agent" | "tailscale-ssh";
  status:
    | { type: "connected"; latencyMs: number }
    | { type: "disconnected"; reason: string }
    | { type: "drifted"; diff: string }
    | { type: "error"; message: string };
  lastSync?: string;
  rulesetHash?: string;
  detectedServices: string[];
}
