# S8_2 — CI-Enforced Performance Benchmarks

## Goal

Create CI-enforced performance benchmarks for cold start, IPC RTT, and terminal scrollback FPS.

## Context

Performance targets from `ui_ux_manifesto.md` section 6: cold start < 400 ms, 95p IPC RTT < 5 ms, 60 fps scrollback at 100k lines, idle RSS < 200 MiB. These are measured on a reference VM in CI and enforced as regressions.

## Docs to Read

- `for_dev/ui_ux_manifesto.md` — section 6 (Performance Posture — all targets)
- `for_dev/tech_stack_2026.md` — section 9 (Testing — criterion, bench harness)

## Reference Code

- Internet: `criterion` crate documentation
- Internet: `tauri-driver` for headless benchmarks
- Internet: GitHub Actions benchmark action (`benchmark-action/github-action-benchmark`)

## Deliverables

```
tests/benchmarks/
  cold_start.rs            # Time from process start to interactive terminal
  ipc_rtt.rs               # Ping command round-trip: p50/p95/p99
  scroll_fps.rs            # Push 100k lines, measure frame rate
  idle_rss.rs              # Measure idle memory footprint

.github/workflows/
  benchmarks.yml           # CI job on reference runner, fails on regression

benches/
  baselines/               # criterion baselines checked into repo
```

## Checklist

- [ ] Cold start measured and < 400 ms on reference machine
- [ ] IPC RTT p95 < 5 ms
- [ ] Scrollback 100k lines at 60 fps
- [ ] Idle RSS < 200 MiB
- [ ] CI job runs benchmarks on every push to main
- [ ] Regression beyond 10% threshold fails the build
- [ ] Baselines checked into repo
- [ ] Benchmark results posted as CI artifacts

## Acceptance Criteria

```bash
# Run benchmarks locally
cargo bench --bench cold_start
cargo bench --bench ipc_rtt
cargo bench --bench scroll_fps

# Verify baselines exist
ls benches/baselines/

# CI workflow validates (dry run)
act -j benchmarks --dryrun
```

## Anti-Patterns

- **Don't run benchmarks on random CI runners** — use a dedicated reference machine or pinned runner type for reproducibility.
- **Don't measure in debug mode** — release builds only (`--release`).
- **Don't set thresholds so tight that noise causes flakes** — the 10% regression threshold accounts for normal variance.
