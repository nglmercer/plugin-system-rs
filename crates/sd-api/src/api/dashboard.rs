use serde::{Deserialize, Serialize};

pub(crate) const DASHBOARD_CONFIG_PATH: &str = "data/dashboard.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DashboardWidget {
    pub id: String,
    #[serde(rename = "type")]
    pub widget_type: String,
    pub title: String,
    /// Footprint in grid cells.
    #[serde(rename = "colSpan")]
    pub col_span: u32,
    #[serde(rename = "rowSpan")]
    pub row_span: u32,
    /// Explicit position, in cell units.
    ///
    /// Optional because layouts written before the deck existed have no
    /// coordinates at all; the client assigns them on first load and saves
    /// them back. Skipped when absent so an unplaced widget stays unplaced
    /// rather than being pinned to (0, 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(default)]
    pub settings: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DashboardLayout {
    pub widgets: Vec<DashboardWidget>,
    /// Cells across one page.
    pub columns: u32,
    /// Cells down one page. Absent in pre-deck layouts, where the grid had no
    /// fixed height and widgets simply flowed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u32>,
    /// Cell width / height. 1 gives square keys like deck hardware.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect: Option<f32>,
}

// The custom-CSS field is gone. Letting a dashboard carry a stylesheet meant
// any saved layout could restyle — or hide — the whole app, including the
// controls needed to undo it, and a layout is synced between devices. Existing
// files keep loading: an unknown `customCss` key is ignored on read and simply
// not written back.

impl Default for DashboardLayout {
    fn default() -> Self {
        Self {
            widgets: Vec::new(),
            // Matches DEFAULT_GRID in web/src/lib/deckLayout.ts.
            columns: 4,
            rows: Some(3),
            aspect: Some(1.35),
        }
    }
}

pub fn load_dashboard_config() -> DashboardLayout {
    std::fs::read_to_string(DASHBOARD_CONFIG_PATH)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub(crate) fn save_dashboard_config(layout: &DashboardLayout) -> bool {
    if let Some(parent) = std::path::Path::new(DASHBOARD_CONFIG_PATH).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match serde_json::to_string_pretty(layout) {
        Ok(json) => std::fs::write(DASHBOARD_CONFIG_PATH, json).is_ok(),
        Err(_) => false,
    }
}
