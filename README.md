# Palisade (Current MVP State)

Palisade is a split-process nftables firewall GUI:

- `palisade-daemon` (privileged, system D-Bus, nft JSON API)
- `palisade-gui-tauri` (unprivileged desktop app)

The sections below describe source-present MVP behavior. Live firewall, rollback, remote-access and desktop verification are separate gates. Use a disposable VM or controlled test host with console access before applying rules; an anti-lockout check and rollback timer are not a guarantee that connectivity will be preserved.

## Implemented Features

### Daemon
- System D-Bus service on `org.palisade.Daemon1`
- nftables JSON read/write via `nft -j` / `nft -j -f` / `nft -c -j -f`
- Ruleset methods:
  - `ListRuleset`, `ListTable`, `GetRuleSummaries`
  - `ValidateChangeset`, `ApplyChangeset`, `ConfirmApply`, `RollbackApply`
  - `ListSnapshots`, `CreateSnapshot`, `GetSnapshot`, `DeleteSnapshot`, `RestoreSnapshot`
  - `MigrateFirewalldZones`, `SwitchFirewalldToCompat`
- Safety pipeline on apply:
  - dry-run validation
  - anti-lockout check
  - pre-apply snapshot
  - dead-man rollback timer
  - audit log append
- Service detection and table ownership methods
- Service registration interface on `org.palisade.Daemon1.Services`:
  - `RegisterServicePort`, `RegisterServicePortRange`, `RegisterServiceRule`
  - `DeregisterServiceRule`, `ListServiceRules`, `ListAllServiceRules`
  - `ServiceRuleChanged` signal
  - SQLite persistence at `/var/lib/palisade/service-rules.db`
  - automatic cleanup for temporary registrations on D-Bus owner disconnect
- Monitor socket server at `/run/palisade/monitor.sock` (MessagePack frames, 1s updates)

### GUI
- Rules view:
  - table/chain tree
  - rule table and summaries
  - service-managed rule badges (`Managed by <service>`)
  - read-only enforcement for service-managed rules
  - service-rule summary panel
  - inline rule editor and apply flow
  - dead-man countdown controls (`Keep` / `Rollback`)
- Traffic view:
  - live totals (bytes/s, packets/s)
  - bandwidth history chart
  - live flow feed, top talkers, rule hit rates
  - refresh interval control
  - linger control (`1-99s`) with reset-on-repeat behavior
  - per-item timers (`Last Seen`, `TTL`)
- Snapshots view:
  - list/refresh/create now
  - load selected snapshot contents
  - side-by-side snapshot vs current ruleset view
  - export selected snapshot and current ruleset
  - restore selected snapshot
  - delete selected snapshot with typed confirmation (`DELETE`)
- Templates view:
  - list available templates
  - parameter input form
  - rendered preview
  - firewalld migration wizard:
    - zone detection
    - side-by-side preview (zone config vs generated nft preview)
    - warnings panel for partial translations
    - apply via standard validate/apply/dead-man flow
    - optional post-apply firewalld disable + compat enable

### firewalld Compatibility Shim
- New optional binary crate: `palisade-firewalld-compat`
- Owns `org.fedoraproject.FirewallD1` D-Bus name
- Implements practical firewalld API subset used by common tooling:
  - zones (`getZones`, `getDefaultZone`, `getActiveZones`)
  - port/service/rich-rule add/remove
  - interface zone assignment
  - runtime lifecycle no-op methods (`reload`, `runtimeToPermanent`, `completeReload`)
- Translates requests to Palisade daemon service registration API (does not call `nft` directly)

## Included Templates

Located in `gui/src/public/templates/`:

- `basic-stateful.json`
- `ssh-hardening.json`
- `web-server.json`
- `docker-coexistence.json`
- `tailscale-integration.json`

## Prerequisites

Linux with nftables, system D-Bus/systemd, a Rust toolchain, Node/pnpm, and the native Tauri 2 system dependencies are required for the corresponding components. The workspace includes the daemon, shared library, GUI backend and optional firewalld compatibility binary. Running daemon or firewall integration tests can affect networking; do not use a remote production host as a test fixture.

## Build & Check

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cd gui/src && pnpm install && pnpm build && pnpm lint
```

## Run (Dev)

1. Build binaries:

```bash
cargo build --workspace
```

2. Start daemon (root required):

```bash
sudo ./target/debug/palisade-daemon
```

3. The current frontend manifest does not declare a `tauri` script or install the Tauri CLI. `pnpm tauri dev` therefore is not a self-contained launch command. Native development requires a separately installed Tauri 2 CLI and verification of the working directory against `gui/src-tauri/tauri.conf.json` (whose hooks use `cd ../src`).

For the frontend development server alone, from the repository root:

```bash
cd gui/src
pnpm dev
```

That server does not replace the native Rust/D-Bus bridge. The native launch/setup gap needs a separately tested code/tooling change before this can be advertised as a clean desktop quickstart.

## D-Bus Policy Setup (if `AccessDenied` on daemon start)

The supplied activation file expects `/usr/libexec/palisade-daemon` and a matching `palisade-daemon.service`. A development binary under `target/debug/` does not satisfy that installed path. Inspect the binary, unit, D-Bus policy and Polkit policy as one installation set; copying the D-Bus files alone is insufficient.

Changing system-bus policy or restarting the system bus is a host-level operation that may interrupt the desktop and other services. Diagnose the exact error and use the distribution's supported installation/reload procedure from a local recovery-capable session. Do not restart D-Bus blindly as a generic `AccessDenied` fix.

## Packaging Files Present

- `packaging/systemd/palisade-daemon.service`
- `packaging/dbus/org.palisade.Daemon1.conf`
- `packaging/systemd/palisade-firewalld-compat.service`
- `packaging/dbus/org.fedoraproject.FirewallD1.conf`
- `packaging/dbus/org.fedoraproject.FirewallD1.service`
- `packaging/polkit/org.palisade.daemon1.policy`

## Notes

- The project rule is to avoid global ruleset flushes and restrict changes to owned tables. Preserve that invariant when changing apply, snapshot or compatibility paths, and verify it with tests rather than relying on this sentence.
- The firewalld compatibility service implements a subset, including no-op lifecycle methods; it is not a drop-in claim of complete firewalld behavior.
- Workspace manifests declare GPL-3.0.
