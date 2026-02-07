# Palisade — nftables Firewall GUI

Open-source Linux desktop app for managing nftables firewall rules directly via the JSON API. Split-process: privileged daemon + unprivileged GUI over D-Bus. No existing tool does this — every alternative (firewalld, UFW, Cockpit) wraps nftables behind lossy abstractions.

**License:** GPL-3.0 | **Kernel:** 5.10+ | **Distros:** Fedora 39+, Ubuntu 22.04+, Arch

## Build & Test

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cd gui/src && pnpm install && pnpm build && pnpm lint
```

## Non-Negotiable Rules

1. **NEVER `flush ruleset`.** Only flush Palisade's own table: `flush table inet palisade`.
2. **NEVER modify tables Palisade doesn't own.** Docker/Tailscale/fail2ban tables are read-only.
3. **ALL nftables interaction uses JSON API.** `nft -j list ruleset` to read, `nft -j -f` to write, `nft -c -j -f` to dry-run. Never parse text format.
4. **ALL mutations go through:** validate (dry-run) → snapshot → apply (atomic) → confirm/rollback (dead man's switch).
5. **The daemon is the single source of truth.** GUI never talks to `nft` directly.

## Architecture

```
palisade (Tauri 2.0, user session)
  ├── React 19 + TypeScript 5 frontend (WebKitGTK webview)
  └── Rust backend → D-Bus client + SSH (russh) + SQLite (rusqlite)
        ↕ D-Bus system bus (org.palisade.Daemon1)
        ↕ Unix socket /run/palisade/monitor.sock (MessagePack traffic data)
palisade-daemon (root or CAP_NET_ADMIN)
  ├── nftables engine → nft CLI + nftables-rs crate for JSON serde
  ├── monitor → counter polling 1Hz, conntrack via neli, nft monitor
  ├── safety → anti-lockout, dead man's switch, snapshots, audit log
  └── service detector → Docker, Tailscale, fail2ban, WireGuard, firewalld
```

## Stack

**Daemon:** Rust 2021, tokio, zbus 5.x, nftables-rs, neli, serde, serde_json, rmp-serde, tracing, thiserror, chrono, sha2, toml, rusqlite.

**GUI Rust:** tauri 2.x, zbus, russh, rusqlite, rmp-serde, serde.

**GUI Frontend:** React 19, TypeScript 5, Vite 6, Tailwind CSS 4, Zustand, TanStack Table v8, dnd-kit, Recharts, Lucide React.

**Do NOT use:** Electron, GTK4, libnftables C FFI, libnetfilter_conntrack C FFI, any ORM.

## Conventions

- **Rust:** `thiserror` for daemon errors, `anyhow` in GUI backend. All public types derive `Debug, Clone, Serialize, Deserialize`. `tracing` not `log`. Async everywhere in daemon.
- **TypeScript:** Strict mode. No `any` — use `unknown` + runtime validation. Functional components, Zustand stores, no prop drilling past 2 levels.
- **Tailwind:** slate (bg), blue-600 (primary), red-600 (destructive/drop), amber-500 (warnings), emerald-600 (accept), rose-600 (reject). Dark mode via `dark:` from day one.
- **Errors:** Never swallow. Daemon → D-Bus error replies. GUI → toasts (recoverable) or modals (critical). Include actionable context.
- **Tests:** Unit tests for summarizer, changeset generation, anti-lockout. Integration tests use `ip netns`. Fixture JSON files in `tests/fixtures/rulesets/`.
- **Commits:** `<type>(<scope>): <summary>` — types: feat/fix/refactor/test/docs/chore, scopes: daemon/gui/shared/packaging.

## Workspace Layout

```
palisade/
├── Cargo.toml                    # Workspace: ["shared", "daemon", "gui/src-tauri"]
├── shared/src/
│   ├── lib.rs
│   ├── changeset.rs              # Changeset, Operation, Position
│   ├── summary.rs                # RuleSummary, RuleMatch, RuleAction
│   ├── monitor.rs                # TrafficSummary, FlowEntry, CounterDelta
│   └── models.rs                 # TableOwner, AddressFamily, HostStatus
├── daemon/src/
│   ├── main.rs                   # Tokio runtime, zbus server, event loop
│   ├── config.rs                 # /etc/palisade/daemon.toml
│   ├── dbus/
│   │   ├── ruleset.rs            # Ruleset interface (list, validate, apply, confirm, rollback)
│   │   ├── services.rs           # Service detection + table ownership
│   │   └── monitor.rs            # Traffic monitor start/stop, socket path
│   ├── nftables/
│   │   ├── engine.rs             # Shell out to nft binary for all operations
│   │   ├── model.rs              # RulesetSnapshot, TableState, ChainState, RuleState
│   │   ├── changeset.rs          # Changeset → nft JSON batch, table ownership enforcement
│   │   ├── summarizer.rs         # JSON expressions → human-readable RuleSummary
│   │   └── ownership.rs          # Table name → TableOwner mapping
│   ├── monitor/
│   │   ├── counters.rs           # Poll nft -j list counters at 1Hz, compute deltas
│   │   ├── conntrack.rs          # Netlink subscription via neli (8MB SO_RCVBUFFORCE)
│   │   ├── aggregator.rs         # Flow table, 1s windows, top-K, ring buffers
│   │   ├── nft_monitor.rs        # nft monitor for external change detection
│   │   └── socket.rs             # Unix socket, MessagePack streaming 1-2Hz
│   ├── safety/
│   │   ├── anti_lockout.rs       # Parse ss output, simulate proposed ruleset
│   │   ├── dead_man.rs           # systemd-run auto-rollback timer
│   │   ├── snapshots.rs          # Capture/store/restore/prune
│   │   └── audit.rs              # Append-only log
│   └── services/
│       ├── detector.rs           # Orchestrate: systemctl, ip link, table name matching
│       ├── docker.rs, tailscale.rs, wireguard.rs, fail2ban.rs, firewalld.rs
├── gui/src-tauri/src/
│   ├── lib.rs                    # Tauri setup, register commands
│   ├── commands/
│   │   ├── ruleset.rs            # Proxy daemon D-Bus methods as #[tauri::command]
│   │   ├── hosts.rs              # SSH remote management
│   │   ├── monitor.rs            # Connect to monitor socket, parse MessagePack
│   │   └── templates.rs          # Load + resolve templates
│   ├── ssh/client.rs, remote_nft.rs
│   └── db/hosts.rs               # SQLite host inventory
├── gui/src/
│   ├── App.tsx                   # Layout + view routing
│   ├── types/nftables.ts, traffic.ts, hosts.ts
│   ├── stores/ruleset.ts, traffic.ts, hosts.ts, ui.ts
│   ├── hooks/useDaemon.ts, useRuleset.ts, useTraffic.ts, useHosts.ts
│   ├── components/
│   │   ├── layout/Sidebar, TopBar, StatusBar
│   │   ├── rules/TableTree, ChainHeader, RuleTable, RuleRow, RuleEditor,
│   │   │        MatchConditionForm, ActionSelector, RulePreview
│   │   ├── sets/SetList, SetEditor, ElementTable, BulkImport
│   │   ├── traffic/BandwidthChart, TopTalkers, FlowTable, RuleHitRates
│   │   ├── templates/TemplateList, TemplateDetail, ParameterForm
│   │   ├── hosts/HostTable, AddHostDialog, DriftViewer, BulkPushDialog
│   │   ├── snapshots/SnapshotList, SnapshotDiff
│   │   └── shared/ConfirmDialog, DeadManCountdown, ServiceBadge, ErrorBoundary
│   └── public/templates/*.json   # Shipped template files
├── packaging/
│   ├── systemd/palisade-daemon.service
│   ├── dbus/org.palisade.Daemon1.conf
│   └── polkit/org.palisade.daemon1.policy
└── tests/fixtures/rulesets/      # Real nft -j output samples
```

## D-Bus Interface

**Bus:** `org.palisade.Daemon1` | **Path:** `/org/palisade/Daemon1`

### Ruleset Methods

| Method | In | Out | Description |
|---|---|---|---|
| ListRuleset | — | s json | Full nft -j list ruleset |
| ListTable | s family, s table | s json | Single table |
| GetRuleSummaries | s family, s table, s chain | s json | Human-readable summaries |
| ValidateChangeset | s json | b valid, s error | Dry-run |
| ApplyChangeset | s json, u timeout_secs | s apply_id, s error | Full pipeline with dead man's switch |
| ConfirmApply | s apply_id | b ok | Cancel rollback |
| RollbackApply | s apply_id | b ok | Immediate rollback |
| ListSnapshots | — | s json | Available snapshots |
| RestoreSnapshot | s id | b ok, s error | Restore snapshot |

### Signals

- `RulesetChanged(s type, s details)` — after successful apply
- `ExternalModification(s description)` — non-Palisade changes detected
- `CounterUpdate(s json)` — 1Hz counter deltas

### Services: `DetectServices() → s`, `GetTableOwnership() → s`
### Monitor: `GetMonitorSocketPath() → s`, `StartMonitoring() → b`, `StopMonitoring() → b`

## Changeset Format

```rust
pub struct Changeset { pub version: u32, pub description: String, pub operations: Vec<Operation> }

pub enum Operation {
    AddTable { family, name },
    AddChain { family, table, chain: ChainSpec },
    AddRule { family, table, chain, position: Position, rule: RuleSpec },
    ReplaceRule { family, table, chain, handle: u64, rule: RuleSpec },
    DeleteRule { family, table, chain, handle: u64 },
    MoveRule { family, table, chain, handle: u64, position: Position },
    AddSet { family, table, set: SetSpec },
    AddElement { family, table, set, elements: Vec<ElementSpec> },
    DeleteElement { family, table, set, elements: Vec<ElementSpec> },
    FlushChain { family, table, chain },
    SetChainPolicy { family, table, chain, policy: String },
    DeleteChain { family, table, chain },
    DeleteSet { family, table, set },
    DeleteTable { family, name },
}

pub enum Position { First, Last, BeforeHandle { handle: u64 }, AfterHandle { handle: u64 } }
```

## TypeScript Types

```typescript
interface NftTable {
  family: "ip" | "ip6" | "inet" | "arp" | "bridge" | "netdev";
  name: string;
  owner: { type: "palisade"|"docker"|"tailscale"|"fail2ban"|"firewalld"|"wireguard"|"unmanaged"; label?: string };
  chains: NftChain[]; sets: NftSet[]; counters: NftCounter[];
}
interface NftChain {
  name: string; type?: "filter"|"nat"|"route";
  hook?: "prerouting"|"input"|"forward"|"output"|"postrouting"|"ingress"|"egress";
  priority?: number; policy?: "accept"|"drop"; rules: NftRule[]; isBaseChain: boolean;
}
interface NftRule {
  handle: number;
  summary: { matches: RuleMatch[]; action: RuleAction; description: string };
  rawExpressions: unknown[]; comment?: string;
  counter?: { packets: number; bytes: number };
  counterDelta?: { packetsPerSec: number; bytesPerSec: number };
  position: number; tag?: string; isDisabled: boolean;
}
interface RuleMatch { field: string; operator: string; value: string; negated: boolean }
interface RuleAction {
  type: "accept"|"drop"|"reject"|"jump"|"goto"|"return"|"masquerade"|"snat"|"dnat"|"redirect"|"log"|"counter"|"limit";
  target?: string; rejectType?: string; logPrefix?: string; natAddress?: string; natPort?: string;
}
interface NftSet { name: string; type: string; flags: string[]; timeout?: number; elements: SetElement[]; size?: number }
interface RemoteHost {
  id: string; name: string; hostname: string; port: number; username: string;
  authMethod: "key"|"agent"|"tailscale-ssh";
  status: { type: "connected"; latencyMs: number } | { type: "disconnected"; reason: string } | { type: "drifted"; diff: string } | { type: "error"; message: string };
  lastSync?: string; rulesetHash?: string; detectedServices: string[];
}
```

## Summarizer (Hardest Component)

`daemon/src/nftables/summarizer.rs` converts nftables JSON expressions → RuleSummary. Must handle:

- **Payload:** ip saddr/daddr, tcp/udp dport/sport, ip protocol → fields "source_ip", "dest_port", etc.
- **CT state:** established, related, new, invalid
- **Interfaces:** iif, oif, iifname, oifname
- **Sets:** @set_name references → operator "in"
- **Meta:** l4proto, nfproto
- **Operators:** ==, !=, ranges (1024-65535), lists ({80, 443})
- **Verdicts:** accept, drop, reject+ICMP, jump/goto+chain, return, masquerade, snat, dnat, redirect, log+prefix, counter, limit

Description reads naturally: "Accept TCP to ports 80, 443 from 192.168.1.0/24" or "Drop all from @blocked_ips".

## Service Detection

| Service | Check | Tables | GUI |
|---|---|---|---|
| Docker | systemctl, docker0 iface | ip docker-bridges | Read-only + badge |
| Tailscale | systemctl, tailscale0 | ip/ip6 filter, ts- chains | Read-only + badge |
| fail2ban | systemctl | inet fail2ban, addr-set-* | Read-only + badge |
| firewalld | systemctl | inet firewalld | Read-only + WARNING |
| WireGuard | ip link show type wireguard | none | Show iface refs |

## Apply Pipeline

```
1. Parse changeset → enforce table ownership (reject non-palisade tables)
2. Generate nft JSON → nft -c -j -f (dry-run) → fail? abort
3. Anti-lockout: detect SSH sessions (ss -tnp), simulate proposed rules
4. Snapshot: nft -j list ruleset → /var/lib/palisade/snapshots/
5. Dead man's switch: systemd-run --on-active=<timeout>s nft -f <snapshot>
6. Apply: nft -j -f
7. Wait for ConfirmApply or timeout → auto-rollback
```

## GUI Views

**Rules (primary):** Left panel table tree (families→tables→chains, lock icon on non-Palisade). Main area: TanStack Table with drag-drop reorder (dnd-kit), action badges, live hit rates. Inline editor below selected row — match condition form, action selector, live nft preview. Dead man countdown dialog on apply.

**Sets:** Set list, element table, bulk import (paste IPs), "used by" rule references.

**Traffic:** Bandwidth sparkline (Recharts), top sources/destinations, filterable flow table, per-rule hit rates. Flag 0-hit rules as shadowed.

**Templates:** Category sidebar, parameter form, live preview. Non-destructive apply. Service-aware.

**Hosts:** Status table, host switching (updates all views), drift diff, bulk push.

**Snapshots:** List with view/compare/restore/export/delete.

## Safety (all mandatory, none disableable)

Dry-run before every apply. Dead man's switch (30-300s, default 60). Pre-change snapshots (keep 50). Anti-lockout check. Table ownership enforcement. Never flush ruleset. Atomic transactions. Audit log. Confirmation dialogs for destructive ops.

## Critical Notes

- **Conntrack buffer:** Default 212KB overflows at ~5K events/sec → set SO_RCVBUFFORCE to 8MB+
- **Docker nftables mode:** Docker 29+ has native nftables (`ip docker-bridges` table). Older uses iptables-nft (`filter`/`nat` tables). Detect which.
- **Tailscale:** Manages rules via netlink, recreates on restart. flush ruleset kills them without recovery (issue #11926).
- **fail2ban:** One set per jail + one rule referencing it. Banning = set element add.
- **JSON round-trips:** Comments survive. define/include are resolved before JSON and lost.
- **Rule handles:** Use kernel-assigned handles (not indices) for stable identification.

## File Locations

```
/usr/libexec/palisade-daemon
/usr/share/dbus-1/system-services/org.palisade.Daemon1.service
/usr/share/polkit-1/actions/org.palisade.daemon1.policy
/etc/palisade/daemon.toml
/var/lib/palisade/snapshots/
/var/lib/palisade/audit.log
/run/palisade/monitor.sock
~/.config/palisade/config.toml
~/.config/palisade/hosts.db
```

# Palisade — Feature Additions

These three features extend the existing Palisade codebase to replace firewalld entirely. The base app (daemon, GUI, safety pipeline, templates, traffic monitor, multi-host) is already built and working.

## 1. Service Registration D-Bus API

### What it does
Lets external services (Docker, libvirt, NetworkManager, custom apps) dynamically register and deregister firewall port openings via D-Bus. Rules created this way are tagged, visible in the GUI with service badges, and automatically cleaned up on deregistration.

### D-Bus Interface Addition

Add to `org.palisade.Daemon1.Services`:

| Method | In | Out | Description |
|---|---|---|---|
| RegisterServicePort | s service_name, s protocol, u port, b temporary | s rule_id, s error | Create a tagged accept rule in inet palisade for the given protocol/port. If temporary=true, rule is removed when the calling D-Bus connection disconnects (use NameOwnerChanged signal). |
| RegisterServicePortRange | s service_name, s protocol, u port_start, u port_end, b temporary | s rule_id, s error | Same but for a port range. |
| RegisterServiceRule | s service_name, s rule_json, b temporary | s rule_id, s error | Advanced: register an arbitrary rule (still constrained to inet palisade table). |
| DeregisterServiceRule | s rule_id | b ok | Remove a previously registered rule. |
| ListServiceRules | s service_name | s json | List all rules registered by a given service. |
| ListAllServiceRules | — | s json | List all dynamically registered rules grouped by service. |

**Signal:** `ServiceRuleChanged(s service_name, s action, s rule_id)` — emitted on register/deregister so the GUI updates live.

### Implementation

**Daemon side:**

- `daemon/src/dbus/services.rs`: Add the new methods. Each registration creates a rule in a dedicated chain `inet palisade service-rules` (base chain, type filter, hook input, priority 0). Tag each rule's comment with `palisade:service:<service_name>:<rule_id>`.
- Track registrations in a `HashMap<String, Vec<ServiceRule>>` keyed by service name, persisted to `/var/lib/palisade/service-rules.db` (SQLite) so rules survive daemon restart.
- For temporary rules: subscribe to `org.freedesktop.DBus.NameOwnerChanged`. When the registering service's bus name disappears, auto-deregister all its temporary rules.
- All registrations still go through the standard changeset pipeline (validate → snapshot → apply). Skip the dead man's switch for service registrations since they're programmatic, not human-initiated.

**GUI side:**

- Service-registered rules should appear in the Rules view inside the `service-rules` chain with a distinct badge per service name and a "Managed by <service>" label.
- These rules are read-only in the GUI — users can view them but not edit or delete them (the owning service manages them).
- Add a "Service Rules" section to the existing service detection panel showing which services have registered rules and how many.

### Chain Structure

```
table inet palisade {
    chain input {
        type filter hook input priority 0; policy drop;
        # ... user-managed rules ...
        jump service-rules    # at appropriate position
    }
    chain service-rules {
        # Auto-managed by RegisterServicePort calls
        # tcp dport 8080 accept comment "palisade:service:docker-proxy:uuid1"
        # tcp dport 5432 accept comment "palisade:service:libvirt:uuid2"
    }
}
```

---

## 2. firewalld Compatibility Shim

### What it does
A separate lightweight daemon (`palisade-firewalld-compat`) that exposes firewalld's D-Bus interface so NetworkManager, Docker, libvirt, and other tools that integrate with firewalld work without modification. Users can `systemctl disable firewalld && systemctl enable palisade-firewalld-compat` and everything keeps working.

### Scope — Only the Methods That Matter

firewalld's full D-Bus API has 100+ methods. Real-world software uses maybe 15-20 of them. Implement only these:

**From `org.fedoraproject.FirewallD1.zone`:**
- `getZones()` — Return zone names mapped from Palisade chains.
- `getDefaultZone()` — Return the zone mapped to the default input chain.
- `getActiveZones()` — Return zones with their interface bindings.
- `addPort(zone, port, protocol, timeout)` — Translate to RegisterServicePort on Palisade daemon.
- `removePort(zone, port, protocol)` — Translate to DeregisterServiceRule.
- `addService(zone, service, timeout)` — Look up service in `/usr/lib/firewalld/services/*.xml`, resolve to ports, register.
- `removeService(zone, service)` — Deregister.
- `addRichRule(zone, rule, timeout)` — Parse firewalld rich rule syntax, translate to nftables rule via Palisade changeset. Best-effort — log a warning for unsupported constructs.
- `removeRichRule(zone, rule)` — Remove.
- `getServices(zone)` / `getPorts(zone)` — Query from Palisade state.
- `addInterface(zone, interface)` / `removeInterface(zone, interface)` — Map interface to Palisade chain via iifname match.
- `changeZoneOfInterface(zone, interface)` — Move interface binding.

**From `org.fedoraproject.FirewallD1`:**
- `reload()` — No-op (Palisade is always live).
- `getLogDenied()` / `setLogDenied()` — Map to Palisade log rules.
- `runtimeToPermanent()` — No-op (Palisade doesn't have the runtime/permanent split).
- `completeReload()` — No-op.

**Signals to emit:**
- `Reloaded()` — After any rule change.
- `ServiceAdded(zone, service)` / `ServiceRemoved(zone, service)` — After service add/remove.
- `PortAdded(zone, port, protocol)` / `PortRemoved(zone, port, protocol)` — After port changes.

### Implementation

**New crate:** `palisade-firewalld-compat` (separate binary, NOT part of the main daemon).

```
palisade-firewalld-compat/
├── Cargo.toml          # deps: zbus, serde, tracing, toml
└── src/
    ├── main.rs         # Register on org.fedoraproject.FirewallD1, connect to org.palisade.Daemon1
    ├── zone_map.rs     # Bidirectional mapping: firewalld zone names ↔ Palisade chains
    ├── service_db.rs   # Parse /usr/lib/firewalld/services/*.xml for port lookups
    ├── rich_rule.rs    # Parse firewalld rich rule syntax → Palisade changeset operations
    └── interface.rs    # Interface → zone/chain binding management
```

**Zone → Chain mapping:**
- `public` → `inet palisade input` (default)
- `trusted` → rules with accept for bound interfaces
- `drop` → rules with drop for bound interfaces
- Custom zones → dedicated chains in inet palisade, jumped to via iifname match

The shim talks to the Palisade daemon via D-Bus — it calls RegisterServicePort, RegisterServiceRule, etc. with service_name="firewalld-compat". It never touches nft directly.

**Packaging:** Separate systemd unit. Conflicts with firewalld.service (only one can own the bus name). Package as optional: `palisade-firewalld-compat`.

**Systemd unit should include:**
```ini
[Unit]
Description=Palisade firewalld Compatibility Shim
After=palisade-daemon.service
Requires=palisade-daemon.service
Conflicts=firewalld.service

[Service]
ExecStart=/usr/libexec/palisade-firewalld-compat
BusName=org.fedoraproject.FirewallD1
```

### Testing
- Install Palisade + compat shim, disable firewalld.
- Verify `firewall-cmd --add-port=8080/tcp` works (it calls the compat shim's D-Bus).
- Verify NetworkManager zone assignment works.
- Verify Docker port publishing creates visible rules in Palisade GUI.
- Verify `firewall-cmd --list-all` returns accurate state from Palisade.

---

## 3. firewalld Zone Migration Wizard

### What it does
Parses existing firewalld zone configurations and converts them into equivalent Palisade rules with a side-by-side preview before applying.

### Implementation

**Daemon side** — `daemon/src/services/firewalld_migrate.rs`:

Add a D-Bus method: `MigrateFirewalldZones() → s json` that:

1. Reads all zone XML files from `/etc/firewalld/zones/` and `/usr/lib/firewalld/zones/` (user overrides take precedence).
2. For each zone, parses: services (resolve via `/usr/lib/firewalld/services/*.xml`), ports, protocols, rich rules, forward ports, source addresses, interfaces, masquerade flag, ICMP blocks.
3. Generates a Palisade changeset that creates equivalent rules. Mapping:
   - Each zone with bound interfaces → a chain in inet palisade with iifname match rules jumping to it.
   - Services → accept rules for resolved ports.
   - Ports → accept rules.
   - Rich rules → best-effort translation to nftables expressions. Log untranslatable rules as warnings.
   - Forward ports → DNAT rules in a prerouting chain.
   - Masquerade → masquerade rule in postrouting chain.
   - ICMP blocks → drop rules for specific ICMP types.
   - Source-based rules → saddr match rules.
4. Returns a JSON structure containing: the generated changeset, a human-readable summary of each zone's conversion, and a list of warnings for anything that couldn't be translated.

**GUI side** — new component `components/templates/MigrationWizard.tsx`:

1. "Migrate from firewalld" button (visible only when firewalld zones are detected on the system).
2. Step 1: Show detected zones with their current configuration (services, ports, interfaces).
3. Step 2: Side-by-side preview — left panel shows the firewalld zone config, right panel shows the generated Palisade rules in nft syntax.
4. Warnings panel at the bottom for any untranslatable rich rules or unsupported features.
5. Step 3: Confirmation with standard dead man's switch apply flow.
6. Optional: "Disable firewalld after migration" checkbox that runs `systemctl disable --now firewalld` and `systemctl enable palisade-firewalld-compat` post-migration.

### Zone XML Parsing

firewalld zone XML looks like:
```xml
<zone>
  <short>Public</short>
  <service name="ssh"/>
  <service name="dhcpv6-client"/>
  <port protocol="tcp" port="443"/>
  <port protocol="tcp" port="8080-8090"/>
  <rule family="ipv4">
    <source address="10.0.0.0/8"/>
    <port protocol="tcp" port="5432"/>
    <accept/>
  </rule>
  <masquerade/>
  <forward-port port="80" protocol="tcp" to-port="8080"/>
</zone>
```

Service XML in `/usr/lib/firewalld/services/ssh.xml`:
```xml
<service>
  <port protocol="tcp" port="22"/>
</service>
```

Use `quick-xml` or `roxmltree` crate for parsing. Don't pull in a full XML framework.
