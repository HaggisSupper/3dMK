# Error, Status, and Job Contract

**Contract ID:** SEP-UI-007

## Status levels

1. normal state;
2. advisory;
3. warning;
4. blocking error;
5. fatal project/application error.

Status color SHALL be paired with icon and text.

## Inline validation

Validation appears adjacent to the responsible field or entity. It SHALL state the violated rule and preserve the entered value. Long remediation guidance is expandable.

## Notifications

Transient notifications are permitted only for completed background actions, external changes, and recoverable failures. They SHALL not become the primary record; processing history and jobs retain authoritative details.

## Job row

```text
[icon] Register Scan 002 → Scan 001 | ICP 62% | GPU | 00:41 | Pause | Cancel | Details
```

Details include parameters, stage timings, backend, memory, warnings, logs, inputs, and partial outputs.

## Recovery

- Failed import: preserve recognized metadata and identify unreadable region.
- Failed processing: preserve source, parameters, logs, and safe partial outputs.
- Application restart: recover autosaved project transaction log and identify uncommitted preview artifacts.
- Missing source: retain project entities as offline and offer relink.

## No false authority

The UI SHALL distinguish unsupported, unavailable, unverified, and failed. It SHALL NOT label a contract-only feature as implemented or imply metrological validity without calibration and uncertainty evidence.
