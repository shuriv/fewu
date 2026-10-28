//! The Grid-Box spatial file graph: a native navigator that lays files and
//! their relationships out on a canvas.

pub mod canvas_element;
pub mod grid_box_view;
pub mod indexer;
pub mod layout;
pub mod persistence;
pub mod skeleton;

use gpui::{App, KeyBinding, actions};
use workspace::Workspace;

pub use grid_box_view::{
    Camera2D, EdgeKind, GridBoxView, GridEdge, GridLayoutMode, GridNode, NodeId,
};
pub use layout::{ClusterBounds, Layout, LayoutRect, Vec2};

actions!(
    grid_box,
    [
        /// Opens the Grid-Box spatial file graph in the active pane.
        ToggleGridBox,
        /// Switches Grid-Box between the structured grid and the force graph.
        ToggleLayoutMode,
        /// Pins the selected Grid-Box nodes at their current positions.
        PinSelectedNodes
    ]
);

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", ToggleLayoutMode, Some("GridBox")),
        KeyBinding::new("p", PinSelectedNodes, Some("GridBox")),
    ]);
    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };
        GridBoxView::register(workspace, window, cx);
    })
    .detach();
}
