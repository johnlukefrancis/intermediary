# Fix Inotify Limits (WSL)
Updated on: 2026-02-03
Owners: JL · Agents
Depends on: ADR-006, ADR-012

## When to use

Use this when the agent reports watcher errors like ENOSPC (inotify watch limit) or EMFILE (file descriptor limit).

## Command (increase inotify limits)

```bash
sudo tee /etc/sysctl.d/99-intermediary-inotify.conf >/dev/null <<'EOF'
fs.inotify.max_user_watches=524288
fs.inotify.max_user_instances=8192
fs.inotify.max_queued_events=16384
EOF
sudo sysctl --system
```

## Command (check current limits)

```bash
sysctl fs.inotify.max_user_watches fs.inotify.max_user_instances fs.inotify.max_queued_events
```

## EMFILE note

If you see EMFILE after raising inotify limits, increase the open file limit in your distro and restart WSL. The exact steps depend on your distro, but typically involve raising `nofile` in `/etc/security/limits.conf` and confirming with `ulimit -n`.
