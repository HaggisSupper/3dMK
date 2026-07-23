from pathlib import Path
import json, math

root = Path(__file__).resolve().parents[1]
ui = root / 'docs' / 'ui'
required = [
    'UI_CONTRACT.md',
    'NAVIGATION_CONTRACT.md',
    'WORKSPACE_CONTRACTS.md',
    'VIEWPORT_INTERACTION_CONTRACT.md',
    'DENSITY_AND_TOKENS.md',
    'CONTROL_BEHAVIOR_CONTRACT.md',
    'ERROR_AND_STATUS_CONTRACT.md',
    'KEYBOARD_AND_COMMAND_CONTRACT.md',
    'ACCESSIBILITY_CONTRACT.md',
    'UI_EVALUATION_MATRIX.md',
    'REAL_WORLD_WORKFLOW_RESEARCH.md',
    'DOCK_LAYOUT_PERSISTENCE_CONTRACT.md',
    'LAYOUT_RECOVERY_AND_MIGRATION.md',
    'ui-contract.json',
    'schemas/dock-layout.schema.json',
    'layouts/default-engineering-layout.json',
]
missing = [item for item in required if not (ui / item).is_file()]
if missing:
    raise SystemExit('Missing UI contract files: ' + ', '.join(missing))

contract = json.loads((ui / 'ui-contract.json').read_text(encoding='utf-8'))
assert contract['status'] == 'immutable-normative'
assert contract['metrics']['panel_padding_px'] <= 6
assert contract['metrics']['tree_row_px'] <= 24
assert contract['metrics']['control_height_px'] <= 26
assert contract['required_shell_regions'] == [
    'menu_command_strip', 'project_tree', 'viewport', 'inspector',
    'results_history_jobs', 'status_bar'
]
assert len(contract['workspaces']) == 7
viewport = contract['primary_viewport']
assert viewport == {
    'role': 'fixed_spatial_authority',
    'dockable': False,
    'floatable': False,
    'closable': False,
    'alignment_reference_for_child_views': True,
}
persistence = contract['dock_layout_persistence']
assert persistence['required'] is True
assert persistence['restore_before_first_interactive_frame'] is True
assert persistence['save_debounce_ms'] == 500
assert persistence['max_dirty_interval_ms'] == 10000
assert persistence['atomic_write'] is True
assert persistence['last_known_good_required'] is True
assert persistence['fallback_order'] == ['current', 'last_known_good', 'shipped_default']

schema = json.loads((ui / persistence['canonical_schema']).read_text(encoding='utf-8'))
assert schema['$schema'].endswith('2020-12/schema')
assert schema['properties']['schema_version']['const'] == persistence['schema_version']
assert schema['properties']['viewport']['properties']['fixed_center']['const'] is True
assert schema['properties']['panels']['maxItems'] <= 128

layout = json.loads((ui / persistence['default_layout']).read_text(encoding='utf-8'))
assert layout['schema_version'] == persistence['schema_version']
assert layout['viewport']['fixed_center'] is True
assert layout['active_workspace'] in contract['workspaces']
assert len(layout['panels']) == len({panel['panel_id'] for panel in layout['panels']})
assert sum(1 for monitor in layout['monitor_topology'] if monitor['primary']) == 1
for splitter in layout['splitters']:
    assert math.isfinite(splitter['ratio']) and 0.0 < splitter['ratio'] < 1.0
for panel in layout['panels']:
    assert panel['panel_id'] != 'viewport'
    assert panel['floating'] == (panel['region'] == 'floating')

allowed_regions = {
    'project_tree': {'left'},
    'properties': {'right'},
    'validation': {'right'},
    'jobs': {'bottom'},
    'processing_history': {'bottom'},
    'measurements': {'bottom'},
}
for panel in layout['panels']:
    expected = allowed_regions.get(panel['panel_id'])
    if expected is not None and panel['region'] not in expected:
        raise SystemExit(f"Illegal default dock region for {panel['panel_id']}: {panel['region']}")

all_text = '\n'.join((ui / item).read_text(encoding='utf-8') for item in required if item.endswith('.md'))
for phrase in [
    'selection', 'preview', 'provenance', 'uncertainty', 'keyboard', 'cancel',
    'units', 'viewport', 'last-known-good', 'atomic', 'debounce', 'monitor',
    'DPI', 'before the first interactive frame'
]:
    if phrase.lower() not in all_text.lower():
        raise SystemExit(f'Missing cross-contract concept: {phrase}')

rust_contract = (root / 'crates/ui-layout-contracts/src/lib.rs').read_text(encoding='utf-8')
for symbol in [
    'pub struct PersistedDockLayout',
    'pub struct PersistedPanelState',
    'pub struct PersistedViewportState',
    'pub fn validate_layout',
    'CURRENT_LAYOUT_SCHEMA_VERSION',
    'DEFAULT_SAVE_DEBOUNCE_MS',
    'LayoutViolation::ViewportNotFixed',
]:
    if symbol not in rust_contract:
        raise SystemExit(f'Missing Rust layout contract symbol: {symbol}')

print('UI and dock-layout persistence contract verification passed')
