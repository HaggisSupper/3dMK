# Configurable Domain Objects and Workstation Features

**Status:** Approved product direction; Veritas contract PR and 3DMK preference/dialog candidate under review
**Date:** 2026-09-22
**Owner boundary:** Veritas owns generic domain-model and composition contracts. 3DMK owns geometry-specific reconstruction adapters, its application preference, and workstation presentation.

## Purpose and user decision

The operator SHALL be able to choose an application-wide domain from validated configurations. The chosen domain SHALL determine object classes, property editors, and available workstation features. The initial default SHALL preserve the existing 3DMK geometry/scene workflow. A later deployment profile MAY lock domain selection; the lock mechanism is outside the first implementation slice.

The active domain is an application preference, not a project setting. Object records still carry their own class identity and source evidence so switching the preference cannot reinterpret or mutate existing project data.

An object name is an editable instance label. It does not identify a class. Chair, wall, corner, room, well, casing, and formation are examples of classes supplied by domain data rather than hard-coded global object types.

## Scope and ownership

The design contains three separately testable boundaries:

1. A generic, versioned Veritas domain-object contract: class definitions, property definitions, instance hierarchy, domain identity, asset references, and dimensional definition references.
2. A 3DMK geometry specialization: accepted 3D media types, dimensional inputs, registered reconstruction capability IDs, finite-number and coordinate-frame validation, candidate revision production, and source mappings.
3. A 3DMK application preference and configuration dialog: domain selection, feature disclosure, validation diagnostics, and feature-driven presentation using existing workstation styling.

Veritas's existing composition-manifest contract already owns capability and shared-surface selection. This design SHALL extend or reference that composition boundary; 3DMK SHALL NOT create a competing generic capability registry or infer backend permission from a visible UI control.

## Generic domain contract

The Veritas-owned JSON contract SHALL have a versioned schema and fingerprint. An illustrative payload is:

    {
      "schema_version": "1.0.0",
      "domain": {"id": "built_environment", "version": "1.0.0", "label": "Built environment"},
      "classes": [
        {
          "id": "space",
          "label": "Space",
          "parent_class_id": null,
          "allowed_child_class_ids": ["wall"],
          "properties": [
            {"id": "geometry", "label": "Geometry", "kind": "geometry_source",
             "required": false, "allowed_sources": ["embedded_asset", "dimensional_definition"]}
          ]
        },
        {
          "id": "wall",
          "label": "Wall",
          "parent_class_id": null,
          "allowed_child_class_ids": [],
          "properties": [
            {"id": "geometry", "label": "Geometry", "kind": "geometry_source",
             "required": true, "allowed_sources": ["embedded_asset", "dimensional_definition"]}
          ]
        }
      ],
      "features": {
        "workstation_panels": ["import", "scene_tree", "measure", "export"],
        "capability_ids": ["geometry.measure_clearance", "export.glb"],
        "default_visible_panels": ["import", "scene_tree"]
      }
    }

Class IDs and property IDs SHALL be stable machine identifiers. Labels MAY be localized or edited without changing identity. Optional class specialization is a directed acyclic graph through parent_class_id; instance containment is a separate parent-object tree constrained by allowed_child_class_ids. A wall can therefore be contained by a space without being a subtype of space. Domain validation SHALL reject unknown parents, duplicate IDs, cycles, and child relationships that contradict class rules. The schema SHALL bound nesting and collection sizes before allocation.

Generic property kinds SHALL include scalar values (text, number, boolean, enum), structured values where a registered schema is provided, and geometry_source. A geometry_source value SHALL use exactly one of:

- embedded_asset: an immutable content-addressed asset ID, digest, media type, source filename, and optional coordinate transform. “Embedded” means included in the project/package asset set, not inline base64 inside the configuration JSON.
- dimensional_definition: an immutable input asset or typed value set, a named schema/version, units and coordinate frame, and a registered reconstruction capability ID. Domain configuration cannot carry executable code.

An object record SHALL contain a stable object ID, editable name, fully qualified class identity (domain_id, domain_version, class_id), optional parent object ID, property values, source asset references, and provenance. The selected application domain does not overwrite those fields. Geometry reconstructed from dimensions SHALL be staged and validated as a candidate revision under the existing 3DMK transaction boundary; it is never published by the browser.

## Feature configuration and UI admission

The feature list is declarative intent. The backend SHALL intersect it with the registered capability inventory, active policy, hardware/resource admission, scene kind, and project state. The resolved response SHALL provide each feature's status and reason. The UI SHALL render only the resolved features and show an explicit unavailable state for a configured feature whose dependencies are missing. Hidden controls SHALL NOT revoke or grant backend authority.

Initial workstation feature IDs SHALL map to existing panels and actions through a versioned adapter. Missing or unrecognized IDs SHALL fail validation with a path-specific diagnostic rather than silently disappear. Future domains add class and feature data without editing the global scene-object type switch; genuinely new operations still require registered backend capabilities and UI renderers.

## Application preference and configuration dialog

Rust SHALL own the active domain preference in application data. Startup SHALL load the installed domain catalog, validate the selected configuration against the published schema and capability registry, and return a resolved presentation model to the browser. When no preference exists, Rust selects the bundled default. An invalid saved preference SHALL produce an explicit recoverable diagnostic and use the bundled default only after the operator acknowledges the change.

The configuration dialog SHALL show:

- active domain and selection controls;
- the domain's class tree and property definitions in read-only form;
- configured versus available workstation features with reasons;
- import/validation feedback for a JSON domain package;
- an Apply action that persists the application preference atomically, and Cancel that changes nothing.

It SHALL use the existing 3DMK dark neutral palette, teal accents, compact controls, focus treatment, and keyboard dismissal. Selection changes SHALL not mutate an open project. An imported profile remains a validation preview until Apply; Cancel discards that preview without changing the persisted catalog. A future deployment policy can make the selection read-only with a visible policy explanation.

## Compatibility, failures, and migration

Existing projects with no domain-class records remain valid and render under the bundled default. Legacy recognition strings remain source evidence; migration to class references is explicit and reversible. On domain switch, objects from another domain remain identifiable and their geometry remains visible; unsupported class editors are disabled with a clear reason. Package export retains object class identity and referenced immutable asset bytes.

Validation SHALL reject malformed JSON, unknown schema versions, duplicate/cyclic classes, unknown feature or reconstruction IDs, non-finite dimensional values, invalid units, unsafe paths, missing or digest-mismatched assets, and unsupported media types. Applying a domain preference SHALL use a staged write and atomic replace. Failure leaves the previous selection intact.

## Verification and release sequence

1. Publish and fingerprint the generic Veritas schema with valid and negative fixtures. Validate the existing composition contract relationship.
2. Add the 3DMK adapter and bundled default domain; prove feature resolution cannot enable an unavailable backend capability.
3. Add application-wide preference persistence and the dialog. Verify restart, invalid saved state, Cancel, Apply, keyboard access, and explicit lock-state presentation.
4. Add object-record and geometry-source admission. Verify package round-trip, immutable asset identity, hierarchy limits, and candidate revision behavior for dimensional reconstruction.

Each step is independently reviewable. The current 3DMK authoritative-task and CUDA-foundation ledgers remain controlling: this specification does not claim that persistence, reconstruction, or release gates have already passed.

## Evidence and open validation

The bounded preference/dialog candidate, its red tests, independent gates, browser observations, and residual scope are recorded in `docs/agent-execution/DOMAIN_CONFIG_PROGRESS.md`.

- **Established:** 3DMK currently has hard-coded room/floorplan labels in public/index.html, src/ai_vision.rs, and src/scene.rs; src/projects.rs has project/revision records but no generic class hierarchy. Veritas has a registered composition manifest that selects capabilities and AppCore surfaces.
- **Approved product direction:** the default domain is application-wide and can be locked by a later deployment scenario.
- **Upstream candidate, not yet governing mainline:** Veritas PR #16 (`01bbb77`) defines the versioned generic schema, JSON Schema fingerprint, Rust loader, hierarchy validator, and positive/negative fixtures. The 3DMK candidate pins that exact commit; merging the upstream PR and updating the child dependency through reviewed history remain separate gates.
- **3DMK candidate evidence:** the app-wide preference and profile catalog are persisted together in one recoverable Rust-owned JSON write. Profile content digests are checked on load and bound to the active selection. The dialog previews imports without persistence, requires Apply, supports a blocking invalid-preference recovery choice, and uses the existing dark neutral/teal styling. Browser evidence covered Cancel, Apply, reload persistence, and recovery. The initial frontend lists panel intent as "configured," not backend availability.
- **Unresolved feature-admission boundary:** this first adapter rejects all non-empty capability IDs and any Veritas composition ID. Its known workstation-panel mapping is presentation-only and does not grant backend authority. It does not yet return the resolved capability status/reason model required above. A registered 3DMK child composition and capability inventory must be accepted upstream before arbitrary domain capabilities can be enabled.
- **Unimplemented object workflow:** this candidate does not yet add class-bearing project object records, class-specific property editors, embedded asset admission, dimensional reconstruction, source mappings, or package round-trip for those records. These remain verification step 4 and must not be described as working product behavior.
- **Residual uncertainty:** domain-specific dimensional representations differ substantially. The generic contract names a typed reconstruction input and registered capability; each concrete dimensional schema and engine must be separately validated against real domain fixtures.
