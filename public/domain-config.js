(() => {
    const byId = id => document.getElementById(id);
    const PANEL_NAMES = {
        import: 'Import', scene_tree: 'Scene tree', analysis: 'Analysis and conversion',
        measure: 'Measurement', export: 'Export'
    };
    let state = null;
    let previewProfile = null;
    let stagedProfileSource = null;
    let previewRevision = 0;
    let configToken = null;

    function setStatus(message, error = false) {
        const target = byId('domainStatus');
        target.textContent = message;
        target.classList.toggle('error', error);
    }

    async function requestJson(url, options = {}) {
        const response = await fetch(url, options);
        const body = await response.json().catch(() => null);
        if (!response.ok) throw new Error(body?.error?.message || `Request failed (${response.status}).`);
        return body;
    }

    function selectedIdentity() {
        const selected = byId('domainSelect').selectedOptions[0];
        return selected ? { id: selected.dataset.domainId, version: selected.dataset.version } : null;
    }

    function setSelection(identity) {
        const select = byId('domainSelect');
        for (const option of select.options) {
            if (option.dataset.domainId === identity.domain_id && option.dataset.version === identity.domain_version) {
                select.value = option.value;
                return;
            }
        }
    }

    function renderCatalog() {
        const select = byId('domainSelect');
        select.replaceChildren();
        for (const profile of state.profiles) {
            const option = document.createElement('option');
            option.value = `${profile.id}@${profile.version}`;
            option.dataset.domainId = profile.id;
            option.dataset.version = profile.version;
            option.textContent = `${profile.label} · ${profile.version}`;
            select.append(option);
        }
        setSelection(state.effective);
        select.disabled = state.locked;
        byId('domainBrowseBtn').disabled = state.locked;
        byId('domainApplyBtn').disabled = state.locked;
        byId('domainCancelBtn').disabled = state.recovery_required;
        byId('domainCloseBtn').disabled = state.recovery_required;
        byId('domainConfigBtn').title = `Domain: ${state.profiles.find(profile => profile.id === state.effective.domain_id && profile.version === state.effective.domain_version)?.label || state.effective.domain_id}`;
    }

    function appendTextRow(container, title, detail) {
        const row = document.createElement('div');
        row.className = 'domain-feature';
        const name = document.createElement('span');
        name.textContent = title;
        const value = document.createElement('span');
        value.textContent = detail;
        row.append(name, value);
        container.append(row);
    }

    function renderProfile(profile) {
        const classes = byId('domainClassList');
        const features = byId('domainFeatureList');
        classes.replaceChildren();
        features.replaceChildren();
        for (const definition of profile.classes) {
            const details = document.createElement('details');
            const summary = document.createElement('summary');
            summary.textContent = definition.label;
            details.append(summary);
            const classId = document.createElement('div');
            classId.className = 'domain-property';
            classId.textContent = `Class ID: ${definition.id}${definition.parent_class_id ? ` · Extends ${definition.parent_class_id}` : ''}`;
            details.append(classId);
            const children = document.createElement('div');
            children.className = 'domain-property';
            children.textContent = `Can contain: ${definition.allowed_child_class_ids.join(', ') || 'none'}`;
            details.append(children);
            for (const property of definition.properties) {
                const row = document.createElement('div');
                row.className = 'domain-property';
                const sources = property.allowed_sources?.length ? ` · ${property.allowed_sources.join(' or ')}` : '';
                row.textContent = `${property.label} (${property.kind}${sources})${property.required ? ' · required' : ''}`;
                details.append(row);
            }
            classes.append(details);
        }
        if (!profile.classes.length) appendTextRow(classes, 'No classes defined', '—');
        for (const panel of profile.features.workstation_panels) {
            appendTextRow(features, PANEL_NAMES[panel] || panel,
                profile.features.default_visible_panels.includes(panel) ? 'Configured · shown by default' : 'Configured · hidden by default');
        }
        if (!profile.features.workstation_panels.length) appendTextRow(features, 'No panels selected', '—');
        if (profile.features.capability_ids.length) {
            appendTextRow(features, 'Capability requests', profile.features.capability_ids.join(', '));
        }
    }

    async function preview(identity) {
        const revision = ++previewRevision;
        previewProfile = null;
        byId('domainApplyBtn').disabled = true;
        const staged = stagedProfileSource && byId('domainSelect').selectedOptions[0]?.dataset.staged === 'true';
        const path = `/api/v1/domains/${encodeURIComponent(identity.id)}/${encodeURIComponent(identity.version)}`;
        const profile = staged ? JSON.parse(stagedProfileSource) : await requestJson(path);
        if (revision !== previewRevision) return;
        previewProfile = profile;
        renderProfile(profile);
        byId('domainApplyBtn').disabled = !!state.locked;
    }

    function setFeatureVisible(element, visible) {
        element?.classList.toggle('domain-feature-hidden', !visible);
        if (element) element.setAttribute('aria-hidden', String(!visible));
    }

    function accordion(key) {
        return document.querySelector(`.accordion-item[data-accordion-key="${key}"]`);
    }

    function applyPresentation(profile) {
        const panels = new Set(profile.features.default_visible_panels);
        setFeatureVisible(accordion('load'), panels.has('import') || panels.has('scene_tree'));
        setFeatureVisible(accordion('geometry'), panels.has('import'));
        setFeatureVisible(accordion('reference'), panels.has('import'));
        setFeatureVisible(accordion('alignment'), panels.has('import') || panels.has('scene_tree'));
        setFeatureVisible(byId('sceneTreePanel'), panels.has('scene_tree'));
        setFeatureVisible(accordion('clean'), panels.has('analysis'));
        setFeatureVisible(accordion('derive'), panels.has('analysis') || panels.has('measure'));
        setFeatureVisible(accordion('convert'), panels.has('analysis'));
        setFeatureVisible(accordion('room-plan'), panels.has('analysis'));
        setFeatureVisible(accordion('measure'), panels.has('measure'));
        setFeatureVisible(accordion('export'), panels.has('export'));
    }

    async function refresh() {
        configToken = (await requestJson('/api/v1/config-session')).token;
        state = await requestJson('/api/v1/domains');
        renderCatalog();
        const identity = { id: state.effective.domain_id, version: state.effective.domain_version };
        await preview(identity);
        if (!state.recovery_required) applyPresentation(previewProfile);
        if (state.diagnostic) {
            setStatus(state.diagnostic, true);
            byId('domainDialog').showModal();
        } else {
            setStatus(state.locked ? 'Domain selection is locked by deployment policy.' : 'Choose a domain, then Apply to use it across the application.');
        }
    }

    async function importFile(file) {
        if (!file || !/\.json$/i.test(file.name)) throw new Error('Choose a JSON domain profile.');
        if (file.size > 1024 * 1024) throw new Error('Domain profiles must be 1 MB or smaller.');
        const profile = await requestJson('/api/v1/domains', {
            method: 'POST', headers: { 'Content-Type': 'text/plain; charset=utf-8', 'X-3DMK-Config-Action': '1', 'X-3DMK-Session': configToken },
            body: await file.text()
        });
        stagedProfileSource = JSON.stringify(profile);
        renderCatalog();
        const option = document.createElement('option');
        option.value = `${profile.domain.id}@${profile.domain.version}`;
        option.dataset.domainId = profile.domain.id;
        option.dataset.version = profile.domain.version;
        option.dataset.staged = 'true';
        option.textContent = `${profile.domain.label} · ${profile.domain.version} (preview)`;
        byId('domainSelect').append(option);
        byId('domainSelect').value = option.value;
        await preview({ id: profile.domain.id, version: profile.domain.version });
        setStatus(`${profile.domain.label} validated. Press Apply to install and select it; Cancel discards it.`);
    }

    document.addEventListener('DOMContentLoaded', () => {
        const dialog = byId('domainDialog');
        const trigger = byId('domainConfigBtn');
        trigger.addEventListener('click', async () => {
            if (!state) {
                try { await refresh(); } catch (error) { setStatus(error.message, true); }
            }
            if (!dialog.open) dialog.showModal();
            byId('domainSelect').focus();
        });
        byId('domainCloseBtn').addEventListener('click', () => dialog.close());
        byId('domainCancelBtn').addEventListener('click', () => dialog.close());
        dialog.addEventListener('cancel', event => { if (state?.recovery_required) event.preventDefault(); });
        dialog.addEventListener('close', async () => {
            if (state) {
                stagedProfileSource = null;
                ++previewRevision;
                renderCatalog();
                setSelection(state.effective);
                const identity = { id: state.effective.domain_id, version: state.effective.domain_version };
                if (previewProfile?.domain.id !== identity.id || previewProfile?.domain.version !== identity.version) {
                    try { await preview(identity); } catch (error) { setStatus(error.message, true); }
                }
                if (!state.diagnostic) setStatus(state.locked ? 'Domain selection is locked by deployment policy.' : 'Choose a domain, then Apply to use it across the application.');
            }
            trigger.focus();
        });
        byId('domainSelect').addEventListener('change', async () => {
            const identity = selectedIdentity();
            if (!identity) return;
            try { await preview(identity); setStatus('Preview only. Press Apply to save this application preference.'); }
            catch (error) { setStatus(error.message, true); }
        });
        byId('domainApplyBtn').addEventListener('click', async () => {
            const identity = selectedIdentity();
            if (!identity || !previewProfile || previewProfile.domain.id !== identity.id || previewProfile.domain.version !== identity.version) return;
            try {
                state = await requestJson('/api/v1/domain-preference', {
                    method: 'PUT', headers: { 'Content-Type': 'application/json', 'X-3DMK-Config-Action': '1', 'X-3DMK-Session': configToken },
                    body: JSON.stringify({ domain_id: identity.id, domain_version: identity.version,
                        profile_source: byId('domainSelect').selectedOptions[0]?.dataset.staged === 'true' ? stagedProfileSource : null })
                });
                stagedProfileSource = null;
                renderCatalog();
                await preview(identity);
                applyPresentation(previewProfile);
                dialog.close();
            } catch (error) { setStatus(error.message, true); }
        });
        byId('domainBrowseBtn').addEventListener('click', () => byId('domainFileInput').click());
        byId('domainFileInput').addEventListener('change', async event => {
            try { await importFile(event.target.files?.[0]); } catch (error) { setStatus(error.message, true); }
            event.target.value = '';
        });
        const drop = byId('domainDropZone');
        drop.addEventListener('dragover', event => { event.preventDefault(); drop.classList.add('drag-over'); });
        drop.addEventListener('dragleave', () => drop.classList.remove('drag-over'));
        drop.addEventListener('drop', async event => {
            event.preventDefault(); drop.classList.remove('drag-over');
            try { await importFile(event.dataTransfer?.files?.[0]); } catch (error) { setStatus(error.message, true); }
        });
        refresh().catch(error => { setStatus(error.message, true); trigger.title = 'Domain configuration unavailable'; });
    });
})();
