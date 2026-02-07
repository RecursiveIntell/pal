# Palisade (Current MVP State)

Palisade is a split-process nftables firewall GUI:

- `palisade-daemon` (privileged, system D-Bus, nft JSON API)
- `palisade-gui-tauri` (unprivileged desktop app)

This README documents **current implemented behavior only**.

## Implemented Features

### Daemon
- System D-Bus service on `org.palisade.Daemon1`
- nftables JSON read/write via `nft -j` / `nft -j -f` / `nft -c -j -f`
- Ruleset methods:
  - `ListRuleset`, `ListTable`, `GetRuleSummaries`
  - `ValidateChangeset`, `ApplyChangeset`, `ConfirmApply`, `RollbackApply`
  - `ListSnapshots`, `CreateSnapshot`, `GetSnapshot`, `DeleteSnapshot`, `RestoreSnapshot`
- Safety pipeline on apply:
  - dry-run validation
  - anti-lockout check
  - pre-apply snapshot
  - dead-man rollback timer
  - audit log append
- Service detection and table ownership methods
- Monitor socket server at `/run/palisade/monitor.sock` (MessagePack frames, 1s updates)

### GUI
- Rules view:
  - table/chain tree
  - rule table and summaries
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

## Included Templates

Located in `gui/src/public/templates/`:

- `basic-stateful.json`
- `ssh-hardening.json`
- `web-server.json`
- `docker-coexistence.json`
- `tailscale-integration.json`

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

3. Start GUI (separate terminal):

```bash
cd gui/src
pnpm tauri dev
```

## D-Bus Policy Setup (if `AccessDenied` on daemon start)

Install policy and service files:

```bash
sudo install -D -m 644 packaging/dbus/org.palisade.Daemon1.conf /etc/dbus-1/system.d/org.palisade.Daemon1.conf
sudo install -D -m 644 packaging/dbus/org.palisade.Daemon1.service /usr/share/dbus-1/system-services/org.palisade.Daemon1.service
```

Then restart D-Bus (pick whichever exists on your distro):

```bash
sudo systemctl restart dbus-broker.service || sudo systemctl restart dbus.service
```

## Packaging Files Present

- `packaging/systemd/palisade-daemon.service`
- `packaging/dbus/org.palisade.Daemon1.conf`
- `packaging/polkit/org.palisade.daemon1.policy`

## Notes

- Non-negotiable safety rule enforced: Palisade does **not** flush global ruleset.
- Some planned firewalld-compat and migration features are not yet implemented in this tree.
