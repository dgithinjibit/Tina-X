# Scaling analysis — what we take from "How To Scale Your Model"

Source: [jax-ml/scaling-book](https://github.com/jax-ml/scaling-book) (DeepMind, MIT-licensed).
It is a textbook on **scaling LLMs on TPUs** (roofline, sharding, parallelism, JAX).

## Honest scope decision (senior-dev call)
Nzi's thesis is the **opposite regime** from the book: tiny edge agents, symbolic reasoning,
<13 ms reflex loops — not datacenter LLM training. So we deliberately adopt only the parts
that transfer, and explicitly skip the rest. Implementing TPU sharding/parallelism here would
be building infrastructure for a problem we don't have.

## Chapter-by-chapter applicability to Nzi
| Chapter | Applies to Nzi? | What we do |
|---|---|---|
| Roofline analysis | ✅ **Yes** | Implemented in `rust-core/src/roofline.rs` |
| Profiling | ✅ **Yes** | The book's "measure, don't guess" ethos = our Phase-0 discipline |
| TPU hardware | ❌ No | Nzi runs on CPUs/microcontrollers, not TPUs |
| GPU hardware | 🟡 Later | Only if we train large weather/vision models (Phase 3/5) |
| Sharding | ❌ No (now) | Our "sharding" analogue is the partitioned Atomspace (ADR 0002) |
| Transformers | 🟡 Later | If/when we add a learned nowcasting/vision model |
| Training / Applied training | 🟡 Phase 3/5 | Revisit for the weed/pest + weather models |
| Inference / Applied inference | 🟡 Phase 3/5 | Edge-inference budgeting for on-agent models |
| JAX | ❌ No (now) | Our ML prototyping is Python; JAX only if we scale a model |

## The transferable idea we implemented: roofline
Every operation is limited by either **compute** or **data movement**, decided by:

```
arithmetic_intensity = FLOPs / bytes_moved
ridge_point          = peak_FLOP/s / peak_bytes/s
intensity < ridge  -> MEMORY-bound   (moving data less helps; more CPU doesn't)
intensity >= ridge -> COMPUTE-bound  (doing less math helps; faster memory doesn't)
```

### Our Phase-0 findings, re-read as roofline results
- **MeTTa `space.query()` is O(n)** (docs/benchmarks/metta-baseline.md): it scans the whole
  space, so it is **memory/scan-bound** — throwing CPU at it does nothing. Correct fix = move
  less data → partition the Atomspace (ADR 0002). Roofline predicted the fix.
- **Agent tool latency is ~88% CPU** (limitations-edge-cases/, arXiv 2511.00739): the pipeline
  is bound by CPU-side tool execution, not the model — a roofline-style "where's the bottleneck"
  result at the system level.
- **The reflex loop** does ~tens of FLOPs on ~tens of bytes per step: neither bound dominates;
  runtime is fixed per-step overhead. That's why it's ~14 µs regardless — and why `efficiency()`
  vs the roofline is low (overhead-bound), which is fine at this tiny scale.

## When the rest of the book becomes relevant
If Nzi ever trains/serves a **large** learned model (a big weather nowcaster or vision model),
re-open: GPU/TPU hardware, sharding, transformers, training/inference chapters. Until then they
are out of scope, and we say so rather than gold-plating.
