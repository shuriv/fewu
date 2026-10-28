use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement,
    Pixels, Point, Render, SharedString, Styled, WeakEntity, Window, px,
};
use project::{Project, ProjectPath};
use ui::prelude::*;
use workspace::Workspace;
use workspace::item::{Item, ItemEvent};

use crate::ToggleGridBox;

/// Identifies a node in the Grid-Box graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u64);

/// A file (or directory) rendered as a box in the Grid-Box graph.
#[derive(Clone, Debug, PartialEq)]
pub struct GridNode {
    pub id: NodeId,
    pub path: ProjectPath,
    pub label: SharedString,
}

/// The kind of relationship a [`GridEdge`] represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// A syntactic import or module dependency between two files.
    Import,
    /// Two files that frequently change together in the same commit.
    GitCoChange,
}

/// A directed relationship between two Grid-Box nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridEdge {
    pub source: NodeId,
    pub target: NodeId,
    pub kind: EdgeKind,
}

/// The layout solver Grid-Box uses to position its nodes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridLayoutMode {
    /// Files grouped into labelled directory clusters on a grid.
    #[default]
    StructuredGrid,
    /// A force-directed graph.
    ForceDirectedGraph,
}

/// Pan/zoom state for the Grid-Box canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera2D {
    pub pan: Point<Pixels>,
    pub zoom: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            pan: Point::new(px(0.0), px(0.0)),
            zoom: 1.0,
        }
    }
}

/// The Grid-Box spatial file graph, shown as a tab in a central pane.
pub struct GridBoxView {
    project: Entity<Project>,
    workspace: WeakEntity<Workspace>,
    focus_handle: FocusHandle,
    nodes: Vec<GridNode>,
    edges: Vec<GridEdge>,
    camera: Camera2D,
    layout_mode: GridLayoutMode,
}

impl GridBoxView {
    fn new(
        project: Entity<Project>,
        workspace: WeakEntity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            project,
            workspace,
            focus_handle: cx.focus_handle(),
            nodes: Vec::new(),
            edges: Vec::new(),
            camera: Camera2D::default(),
            layout_mode: GridLayoutMode::default(),
        }
    }

    /// Opens the Grid-Box graph in the active pane.
    pub fn open(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
        let project = workspace.project().clone();
        let workspace_handle = workspace.weak_handle();
        let view = cx.new(|cx| GridBoxView::new(project, workspace_handle, cx));
        workspace.active_pane().update(cx, |pane, cx| {
            pane.add_item(Box::new(view), true, true, None, window, cx);
        });
        cx.notify();
    }

    pub fn register(workspace: &mut Workspace, _window: &mut Window, cx: &mut Context<Workspace>) {
        workspace.register_action(
            |workspace, _: &ToggleGridBox, window, cx| Self::open(workspace, window, cx),
        );
    }
}

impl Render for GridBoxView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        let worktree_count = self.project.read(cx).worktrees(cx).count();
        v_flex()
            .id("GridBox")
            .key_context("GridBox")
            .track_focus(&self.focus_handle(cx))
            .size_full()
            .bg(colors.editor_background)
            .items_center()
            .justify_center()
            .child(
                Label::new(format!(
                    "Grid-Box — {} worktrees · {} files · {} edges",
                    worktree_count,
                    self.nodes.len(),
                    self.edges.len(),
                ))
                .color(Color::Muted),
            )
    }
}

impl Focusable for GridBoxView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<ItemEvent> for GridBoxView {}

impl Item for GridBoxView {
    type Event = ItemEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        SharedString::from("Grid-Box")
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(ItemEvent)) {
        f(*event);
    }
}
