# Spatial Engineering Platform Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Build a runnable, testable Rust-first foundation for traceable point-cloud processing, engineering geometry, semantic proposals, BIM entities, and uncertainty-aware measurements.

**Architecture:** A dependency-light Rust workspace owns domain logic and contracts. A Python reference runner ensures immediate execution in environments without Rust. A Tauri shell consumes the CLI/application boundary.

**Tech Stack:** Rust 2021, Python 3.11 standard library, Tauri 2 source, HTML/CSS/TypeScript.

## Global Constraints

- No Docker.
- Rust is authoritative.
- Python uses no pandas.
- AI cannot directly mutate engineering state.
- All derived engineering results preserve provenance and quality metadata.

---

### Task 1: Canonical spatial contracts
- [x] Define vectors, points, transforms, identifiers, provenance, confidence, uncertainty, and errors.
- [x] Add unit tests for transforms and validation.

### Task 2: Point-cloud ingestion and reduction
- [x] Implement XYZ/CSV and ASCII PLY parsing.
- [x] Implement summaries and deterministic voxel decimation.
- [x] Add malformed-input and reduction tests.

### Task 3: Geometry fitting
- [x] Implement plane fitting through covariance minimization.
- [x] Implement axis-aligned cylinder hypothesis fitting.
- [x] Return residuals, support count, and confidence.

### Task 4: Semantic, BIM, and metrology contracts
- [x] Define non-authoritative semantic hypotheses and explicit validation.
- [x] Define BIM entities and verification state.
- [x] Implement uncertainty-aware measurement decisions.

### Task 5: CLI and reference implementation
- [x] Add summarize, decimate, fit-plane, and measure commands.
- [x] Add Python equivalents and executable unit tests.

### Task 6: Desktop shell and repository verification
- [x] Add Tauri 2 shell source and contract-driven UI.
- [x] Add PowerShell bootstrap and verification scripts.
- [x] Package repository.
