//! The Grid-Box spatial file graph: a native navigator that lays files and
//! their relationships out on a canvas.

pub mod grid_box_view;
pub mod indexer;

use gpui::{App, actions};
use workspace::Workspace;

pub use grid_box_view::{
    Camera2D, EdgeKind, GridBoxView, GridEdge, GridLayoutMode, GridNode, NodeId,
};

actions!(
    grid_box,
    [
        /// Opens the Grid-Box spatial file graph in the active pane.
        ToggleGridBox
    ]
);

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };
        GridBoxView::register(workspace, window, cx);
    })
    .detach();
}
