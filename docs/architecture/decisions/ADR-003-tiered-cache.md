# ADR-003: Tiered Scene and Accelerator Cache

**Status:** Accepted

## Decision

3DMk will use a disposable four-tier cache:

1. **NVMe cache** — decoded canonical chunks, reusable indexes, and regenerated previews.
2. **RAM cache** — memory-mapped or read-only canonical chunks and CPU-reference structures.
3. **Pinned host pool** — bounded reusable staging memory for asynchronous CUDA transfers.
4. **VRAM cache** — broker-accounted least-recently-used device buffers, acceleration structures, and validated pipeline intermediates.

No cache entry is authoritative. Losing or rebuilding a cache must not change an accepted project result.

## Cache identity

```text
SHA256(
  input asset digests
  + normalized parameters
  + kernel or engine identity
  + precision mode
  + device architecture
  + cache schema version
)
```

A cache hit is valid only when all identity inputs and payload integrity checks match.

## Policies

- Use structure-of-arrays canonical chunks with stable source IDs.
- Select chunk size from the granted GPU lease, device limits, and measured throughput.
- Pin only bounded reusable staging allocations; never pin arbitrary user buffers indefinitely.
- Use stream-ordered CUDA memory pools where supported and covered by lifetime tests.
- Use CUDA graphs only after shapes, buffers, launch order, and cancellation behavior are stable and verified.
- Keep related intermediates resident across adjacent pipeline stages when this reduces meaningful transfer or launch overhead.
- Record cache hit/miss, transferred bytes, copy count, allocation high-water mark, eviction reason, and rebuild cost.
- Verify cache payloads before use and quarantine malformed entries.
- Use backpressure and queueing rather than allocating outside the GPU resource broker.
- Recover or discard cache state independently of project recovery.
- Do not export device caches or regenerable cache payloads as canonical project content.

## Precision and source identity

- Project datum, units, transforms, measurements, and quality thresholds remain authoritative CPU metadata, normally `f64`.
- Local-origin `f32` chunks are permitted only when the error bound is measured below the operation contract's tolerance.
- Reduced CUDA precision is recorded in accelerator provenance and accepted only after differential validation.
- Stable point, vertex, face, primitive, and instance IDs survive cache conversion and same-topology operations.

## Current implementation boundary

The complete cache hierarchy, pinned host pool, VRAM LRU, broker integration, and cache telemetry are not implemented. Existing browser, Rust, operating-system, and CUDA/Mistral caches are not yet coordinated by this ADR. The CUDA-foundation plan implements this decision before production heavy-compute capabilities are marked available.
