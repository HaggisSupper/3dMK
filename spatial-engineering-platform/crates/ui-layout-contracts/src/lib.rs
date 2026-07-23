use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const CURRENT_LAYOUT_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_SAVE_DEBOUNCE_MS: u64 = 500;
pub const MAX_DIRTY_INTERVAL_MS: u64 = 10_000;
pub const MAX_PANELS: usize = 128;
pub const MIN_REACHABLE_HEADER_PX: f64 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceId {
    View,
    Clean,
    Register,
    Segment,
    Reconstruct,
    Bim,
    Inspect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockRegion {
    Left,
    Right,
    Bottom,
    Floating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LogicalRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraState {
    pub position: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    pub projection: Projection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Projection {
    Perspective,
    Orthographic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedViewportState {
    pub fixed_center: bool,
    pub camera: CameraState,
    pub coordinate_frame: String,
    #[serde(default)]
    pub selected_entities: Vec<String>,
    #[serde(default)]
    pub expanded_tree_nodes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedPanelState {
    pub panel_id: String,
    pub state_version: u32,
    pub region: DockRegion,
    pub tab_group: Option<String>,
    pub tab_order: u32,
    #[serde(default)]
    pub active: bool,
    pub visible: bool,
    pub floating: bool,
    pub pinned: bool,
    #[serde(default)]
    pub auto_hidden: bool,
    pub floating_bounds: Option<LogicalRect>,
    pub monitor_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedSplitterState {
    pub splitter_id: String,
    pub orientation: SplitOrientation,
    pub ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedMonitor {
    pub monitor_id: String,
    pub work_area: LogicalRect,
    pub scale_factor: f64,
    pub primary: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedDockLayout {
    pub schema_version: u32,
    pub app_version: String,
    pub active_workspace: WorkspaceId,
    pub active_layout_profile: Option<String>,
    pub viewport: PersistedViewportState,
    pub panels: Vec<PersistedPanelState>,
    pub splitters: Vec<PersistedSplitterState>,
    pub monitor_topology: Vec<PersistedMonitor>,
    pub saved_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelPolicy {
    pub panel_id: &'static str,
    pub allowed_regions: &'static [DockRegion],
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutViolation {
    UnsupportedSchema { found: u32 },
    ViewportNotFixed,
    TooManyPanels { found: usize },
    UnknownPanel { panel_id: String },
    MissingRequiredPanel { panel_id: String },
    DuplicatePanel { panel_id: String },
    IllegalDockRegion { panel_id: String, region: DockRegion },
    FloatingStateMismatch { panel_id: String },
    InvalidBounds { owner: String },
    InvalidSplitterRatio { splitter_id: String },
    DuplicateSplitter { splitter_id: String },
    InvalidScaleFactor { monitor_id: String },
    MissingPrimaryMonitor,
    MultiplePrimaryMonitors,
    NonFiniteNumber { owner: String },
}

pub fn validate_layout(
    layout: &PersistedDockLayout,
    policies: &[PanelPolicy],
) -> Result<(), Vec<LayoutViolation>> {
    let mut violations = Vec::new();
    if layout.schema_version != CURRENT_LAYOUT_SCHEMA_VERSION {
        violations.push(LayoutViolation::UnsupportedSchema { found: layout.schema_version });
    }
    if !layout.viewport.fixed_center {
        violations.push(LayoutViolation::ViewportNotFixed);
    }
    if layout.panels.len() > MAX_PANELS {
        violations.push(LayoutViolation::TooManyPanels { found: layout.panels.len() });
    }
    validate_vec3("viewport.camera.position", layout.viewport.camera.position, &mut violations);
    validate_vec3("viewport.camera.target", layout.viewport.camera.target, &mut violations);
    validate_vec3("viewport.camera.up", layout.viewport.camera.up, &mut violations);

    let policy_map: HashMap<&str, &PanelPolicy> = policies.iter().map(|p| (p.panel_id, p)).collect();
    let mut seen_panels = HashSet::new();
    for panel in &layout.panels {
        if !seen_panels.insert(panel.panel_id.as_str()) {
            violations.push(LayoutViolation::DuplicatePanel { panel_id: panel.panel_id.clone() });
        }
        let Some(policy) = policy_map.get(panel.panel_id.as_str()) else {
            violations.push(LayoutViolation::UnknownPanel { panel_id: panel.panel_id.clone() });
            continue;
        };
        if !policy.allowed_regions.contains(&panel.region) {
            violations.push(LayoutViolation::IllegalDockRegion {
                panel_id: panel.panel_id.clone(),
                region: panel.region,
            });
        }
        if panel.floating != (panel.region == DockRegion::Floating) {
            violations.push(LayoutViolation::FloatingStateMismatch { panel_id: panel.panel_id.clone() });
        }
        if let Some(bounds) = panel.floating_bounds {
            validate_rect(&format!("panel:{}", panel.panel_id), bounds, &mut violations);
        }
    }
    for policy in policies.iter().filter(|p| p.required) {
        if !seen_panels.contains(policy.panel_id) {
            violations.push(LayoutViolation::MissingRequiredPanel { panel_id: policy.panel_id.to_owned() });
        }
    }

    let mut seen_splitters = HashSet::new();
    for splitter in &layout.splitters {
        if !seen_splitters.insert(splitter.splitter_id.as_str()) {
            violations.push(LayoutViolation::DuplicateSplitter { splitter_id: splitter.splitter_id.clone() });
        }
        if !splitter.ratio.is_finite() || splitter.ratio <= 0.0 || splitter.ratio >= 1.0 {
            violations.push(LayoutViolation::InvalidSplitterRatio { splitter_id: splitter.splitter_id.clone() });
        }
    }

    let primary_count = layout.monitor_topology.iter().filter(|m| m.primary).count();
    if primary_count == 0 {
        violations.push(LayoutViolation::MissingPrimaryMonitor);
    } else if primary_count > 1 {
        violations.push(LayoutViolation::MultiplePrimaryMonitors);
    }
    for monitor in &layout.monitor_topology {
        validate_rect(&format!("monitor:{}", monitor.monitor_id), monitor.work_area, &mut violations);
        if !monitor.scale_factor.is_finite() || !(0.5..=8.0).contains(&monitor.scale_factor) {
            violations.push(LayoutViolation::InvalidScaleFactor { monitor_id: monitor.monitor_id.clone() });
        }
    }

    if violations.is_empty() { Ok(()) } else { Err(violations) }
}

fn validate_vec3(owner: &str, values: [f64; 3], violations: &mut Vec<LayoutViolation>) {
    if values.iter().any(|value| !value.is_finite()) {
        violations.push(LayoutViolation::NonFiniteNumber { owner: owner.to_owned() });
    }
}

fn validate_rect(owner: &str, rect: LogicalRect, violations: &mut Vec<LayoutViolation>) {
    let finite = [rect.x, rect.y, rect.width, rect.height].iter().all(|value| value.is_finite());
    if !finite {
        violations.push(LayoutViolation::NonFiniteNumber { owner: owner.to_owned() });
    }
    if rect.width < 64.0 || rect.height < 48.0 {
        violations.push(LayoutViolation::InvalidBounds { owner: owner.to_owned() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEFT: &[DockRegion] = &[DockRegion::Left];
    const RIGHT: &[DockRegion] = &[DockRegion::Right];

    fn policies() -> Vec<PanelPolicy> {
        vec![
            PanelPolicy { panel_id: "project_tree", allowed_regions: LEFT, required: true },
            PanelPolicy { panel_id: "properties", allowed_regions: RIGHT, required: true },
        ]
    }

    fn valid_layout() -> PersistedDockLayout {
        PersistedDockLayout {
            schema_version: CURRENT_LAYOUT_SCHEMA_VERSION,
            app_version: "0.5.0".into(),
            active_workspace: WorkspaceId::View,
            active_layout_profile: Some("default".into()),
            viewport: PersistedViewportState {
                fixed_center: true,
                camera: CameraState {
                    position: [4.0, 3.0, 4.0],
                    target: [0.0, 0.0, 0.0],
                    up: [0.0, 1.0, 0.0],
                    projection: Projection::Perspective,
                },
                coordinate_frame: "project.local".into(),
                selected_entities: Vec::new(),
                expanded_tree_nodes: Vec::new(),
            },
            panels: vec![
                PersistedPanelState { panel_id: "project_tree".into(), state_version: 1, region: DockRegion::Left, tab_group: None, tab_order: 0, active: true, visible: true, floating: false, pinned: true, auto_hidden: false, floating_bounds: None, monitor_id: None },
                PersistedPanelState { panel_id: "properties".into(), state_version: 1, region: DockRegion::Right, tab_group: None, tab_order: 0, active: true, visible: true, floating: false, pinned: true, auto_hidden: false, floating_bounds: None, monitor_id: None },
            ],
            splitters: vec![PersistedSplitterState { splitter_id: "left.viewport".into(), orientation: SplitOrientation::Vertical, ratio: 0.2 }],
            monitor_topology: vec![PersistedMonitor { monitor_id: "primary".into(), work_area: LogicalRect { x: 0.0, y: 0.0, width: 1920.0, height: 1040.0 }, scale_factor: 1.0, primary: true }],
            saved_at_utc: "2026-07-23T00:00:00Z".into(),
        }
    }

    #[test]
    fn accepts_valid_layout() {
        assert_eq!(validate_layout(&valid_layout(), &policies()), Ok(()));
    }

    #[test]
    fn rejects_movable_primary_viewport() {
        let mut layout = valid_layout();
        layout.viewport.fixed_center = false;
        assert!(matches!(validate_layout(&layout, &policies()), Err(v) if v.contains(&LayoutViolation::ViewportNotFixed)));
    }

    #[test]
    fn rejects_panel_in_illegal_region() {
        let mut layout = valid_layout();
        layout.panels[0].region = DockRegion::Right;
        assert!(matches!(validate_layout(&layout, &policies()), Err(v) if v.iter().any(|x| matches!(x, LayoutViolation::IllegalDockRegion { panel_id, .. } if panel_id == "project_tree"))));
    }
}
