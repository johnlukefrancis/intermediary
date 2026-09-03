# Verify the WSL Agent Process-Tree Kill End-to-End
Updated on: 2026-09-03
Owners: JL · Agents
Depends on: ADR-012, ADR-013

Manually exercises the emergency stop's in-distro sweep
(`build_wsl_kill_agent_process_trees_command_line`) through a real `wsl.exe`, against a stand-in
agent that ignores SIGTERM and owns the three descendant shapes the sweep has to tell apart:
two process groups of its own (`setsid`) and one child inside the agent's own group. Run this
after touching `wsl_process_tree_commands.rs`, `wsl_agent_termination.rs`, or the bash-over-stdin
route they use.

The script is printed by the real builder, never transcribed, so what runs here is what ships.

## Start the stand-in agent and capture its pid

```bash
SCRATCH="${TMPDIR:-/tmp}/wsl_tree_kill_witness"
rm -rf "$SCRATCH"; mkdir -p "$SCRATCH"
cat > "$SCRATCH/fake_agent.sh" <<'EOF'
#!/bin/bash
# `setsid` forks, so the shell's $! is setsid's pid, not this script's. The
# stand-in agent reports its own pid or the sweep is aimed at a pid that is
# already gone and proves nothing.
echo $$ > "$1/agent.pid"
trap "" TERM; setsid sleep 600 & setsid sleep 600 & sleep 600 & wait
EOF
chmod +x "$SCRATCH/fake_agent.sh"
setsid "$SCRATCH/fake_agent.sh" "$SCRATCH" >/dev/null 2>&1 </dev/null &
sleep 1
AGENT=$(cat "$SCRATCH/agent.pid")
echo "fake agent pid=$AGENT"
ps -e -o pid=,ppid=,pgid=,args= | grep -E "(^| )$AGENT |sleep 600" | grep -v grep
```

Expect four rows: the agent, two `sleep 600` with a `pgid` of their own, and one `sleep 600`
sharing the agent's `pgid`.

## Print the real kill script from the builder

```bash
INTERMEDIARY_TEST_AGENT_PIDS="$AGENT" cargo test -p intermediary --lib -- --ignored --nocapture \
  agent::wsl_process_tree_commands::tests::print_wsl_kill_agent_process_trees_script \
  | sed -n '/----SCRIPT----/,/----END----/p' | sed '1d;$d' > "$SCRATCH/kill_script.sh"
```

## Run it through the real WSL bash-over-stdin route

```bash
wsl.exe -d Ubuntu -- bash --noprofile --norc -s < "$SCRATCH/kill_script.sh"
```

Expect exactly four `signalled …` lines — `signalled group <pgid>` twice, `signalled pid <pid>`
once, `signalled agent <pid>` once — and then nothing left:

```bash
ps -e -o pid=,ppid=,pgid=,args= | grep -E "(^| )$AGENT |sleep 600" | grep -v grep || echo "(nothing left)"
```

A missing `signalled group` line means the descendant walk lost a process group and the sweep is
back to killing the agent alone, which is the defect this route exists to close. A `signalled`
line for anything outside the stand-in agent's tree means the walk over-reached and the change
must be reverted rather than tuned.

## Verify the agent's stdin-EOF shutdown owner

Builds the Linux agent and drives the case that matters: a supervisor handle that closes while the
agent is idle. The `sleep` on the left of the pipe stands in for the supervisor holding the write end
of the pipe `spawn_wsl_agent_process` hands the backend; when it exits, the agent sees EOF. The waits are generous
because a **debug** agent hashes its own ~100 MB binary before it binds (`runtime_binary_sha256`),
which can take ten seconds on a cold page cache.

```bash
cargo build -p im_agent
LOGS="$SCRATCH/eof_logs"; rm -rf "$LOGS"; mkdir -p "$LOGS"
( sleep 20 ) | ( INTERMEDIARY_AGENT_PORT=3193 INTERMEDIARY_WSL_WS_TOKEN=witness \
  INTERMEDIARY_AGENT_LOG_DIR="$LOGS" INTERMEDIARY_AGENT_STDIO_LOGGING=0 \
  ./target/debug/im_agent >/dev/null 2>&1 ) &
JOB=$!
sleep 30
cat "$LOGS/agent_latest.log"
kill -0 "$JOB" 2>/dev/null && echo "STILL RUNNING (fail)" || echo "exited after stdin EOF"
```

Expect a `Watching the supervisor's stdin pipe for EOF` line with `"stdin":"pipe"`, and
`exited after stdin EOF`. The agent's log writer is a separate task that `main` returning does not
flush, so the final drain line may or may not reach the file — the process exiting is the oracle,
not the tail of the log.

Now the launch shape that must be unaffected — `/dev/null` stdin, which reaches EOF immediately and
must never be claimed:

```bash
LOGS2="$SCRATCH/null_logs"; rm -rf "$LOGS2"; mkdir -p "$LOGS2"
INTERMEDIARY_AGENT_PORT=3194 INTERMEDIARY_WSL_WS_TOKEN=witness \
  INTERMEDIARY_AGENT_LOG_DIR="$LOGS2" INTERMEDIARY_AGENT_STDIO_LOGGING=0 \
  ./target/debug/im_agent < /dev/null >/dev/null 2>&1 &
NULL_PID=$!
sleep 20
grep -c "Stdin is not a supervisor pipe" "$LOGS2/agent_latest.log"
kill -0 "$NULL_PID" 2>/dev/null && echo "still running (correct)" || echo "EXITED (fail)"
kill -TERM "$NULL_PID"
```

Expect `1` and `still running (correct)`. A failure here means a terminal or script launch is being
claimed as a supervisor handle, which would drain-and-exit an agent nobody asked to stop.
