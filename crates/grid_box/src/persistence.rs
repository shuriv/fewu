//! Persistence for Grid-Box node positions.
//!
//! Positions are stored as JSON in the app database's scoped key-value store,
//! namespaced per project (keyed by the sorted worktree roots). This avoids a
//! dedicated table and its migration while remaining durable across restarts.

use std::collections::HashMap;

use db::kvp::KeyValueStore;
use gpui::App;
use serde::{Deserialize, Serialize};

use crate::layout::Vec2;

const NAMESPACE: &str = "grid_box_layout";

/// The persisted Grid-Box layout for one project.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PersistedLayout {
    /// Node positions and pinned state, keyed by project-relative path.
    pub pins: HashMap<String, PinnedNode>,
}

/// A persisted node position.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PinnedNode {
    pub x: f32,
    pub y: f32,
    pub pinned: bool,
}

impl PersistedLayout {
    pub fn load(cx: &App, key: &str) -> Self {
        KeyValueStore::global(cx)
            .scoped(NAMESPACE)
            .read(key)
            .ok()
            .flatten()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, cx: &App, key: String) {
        let Ok(json) = serde_json::to_string(self) else {
            return;
        };
        let store = KeyValueStore::global(cx);
        db::write_and_log(cx, move || async move {
            store.scoped(NAMESPACE).write(key, json).await
        });
    }

    pub fn position(&self, path: &str) -> Option<Vec2> {
        self.pins
            .get(path)
            .map(|pin| Vec2::new(pin.x, pin.y))
    }
}
