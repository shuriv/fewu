use std::collections::HashMap;
use std::time::Duration;

use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement,
    Pixels, Point, Render, SharedString, Styled, Subscription, Task, WeakEntity, Window, px,
};
use project::{Project, ProjectPath};
use ui::prelude::*;
use workspace::Workspace;
use workspace::item::{Item, ItemEvent};

use crate::canvas_element::{self, LabelCache};
use crate::indexer::{self, SourceFile};
use crate::layout::{self, ClusterBounds, Vec2};
use crate::{ToggleGridBox, ToggleLayoutMode};

/// Delay before a content edit triggers a re-index.
const REINDEX_DEBOUNCE: Duration = Duration::from_secs(2);
/// Duration of the animation between two layout modes.
const LAYOUT_ANIMATION: Duration = Duration::from_millis(150);
/// Number of frames in that animation.
const LAYOUT_ANIMATION_FRAMES: u32 = 10;

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
    positions: HashMap<NodeId, Vec2>,
    clusters: Vec<ClusterBounds>,
    _project_subscription: Subscription,
    _index_task: Option<Task<()>>,
    _pending_reindex: Option<Task<()>>,
    _layout_task: Option<Task<()>>,
    _layout_animation: Option<Task<()>>,
}

impl GridBoxView {
    fn new(
        project: Entity<Project>,
        workspace: WeakEntity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        let project_subscription = cx.subscribe(
            &project,
            |this: &mut Self, _project, event: &project::Event, cx| match event {
                project::Event::WorktreeUpdatedEntries(..) => this.reindex(cx),
                project::Event::BufferEdited { .. } => this.schedule_reindex(cx),
                _ => {}
            },
        );

        let mut this = Self {
            project,
            workspace,
            focus_handle: cx.focus_handle(),
            nodes: Vec::new(),
            edges: Vec::new(),
            camera: Camera2D::default(),
            layout_mode: GridLayoutMode::default(),
            positions: HashMap::new(),
            clusters: Vec::new(),
            _project_subscription: project_subscription,
            _index_task: None,
            _pending_reindex: None,
            _layout_task: None,
            _layout_animation: None,
        };
        this.reindex(cx);
        this
    }

    /// The source files currently in the project, in a stable order.
    fn collect_source_files(&self, cx: &App) -> Vec<SourceFile> {
        let mut files = Vec::new();
        let mut next_id = 0;
        for worktree in self.project.read(cx).worktrees(cx) {
            let worktree = worktree.read(cx);
            let worktree_id = worktree.id();
            let root = worktree.abs_path().clone();
            for entry in worktree.files(false, 0) {
                let relative = entry.path.as_std_path();
                if !indexer::is_indexable_source(relative) {
                    continue;
                }
                files.push(SourceFile {
                    id: NodeId(next_id),
                    project_path: ProjectPath {
                        worktree_id,
                        path: entry.path.clone(),
                    },
                    abs_path: root.join(relative),
                });
                next_id += 1;
            }
        }
        files
    }

    /// Rebuilds the graph from the project's files on a background thread.
    fn reindex(&mut self, cx: &mut Context<Self>) {
        self._pending_reindex = None;
        let files = self.collect_source_files(cx);
        let mode = self.layout_mode;
        let indexing = cx.background_spawn(async move {
            let (nodes, edges) = indexer::index(files);
            let layout = layout_for(mode, &nodes, &edges);
            (nodes, edges, layout)
        });
        self._index_task = Some(cx.spawn(async move |this, cx| {
            let (nodes, edges, layout) = indexing.await;
            this.update(cx, |view, cx| {
                view.nodes = nodes;
                view.edges = edges;
                view.animate_to(layout, cx);
            })
            .ok();
        }));
    }

    /// Recomputes the layout for the current mode on a background thread.
    fn recompute_layout(&mut self, cx: &mut Context<Self>) {
        let mode = self.layout_mode;
        let nodes = self.nodes.clone();
        let edges = self.edges.clone();
        let computing = cx.background_spawn(async move { layout_for(mode, &nodes, &edges) });
        self._layout_task = Some(cx.spawn(async move |this, cx| {
            let layout = computing.await;
            this.update(cx, |view, cx| view.animate_to(layout, cx)).ok();
        }));
    }

    /// Animates the positions towards `target` over [`LAYOUT_ANIMATION`].
    /// Starting a new animation cancels the one already running.
    fn animate_to(&mut self, target: layout::Layout, cx: &mut Context<Self>) {
        let from = layout::Layout {
            positions: std::mem::take(&mut self.positions),
            clusters: std::mem::take(&mut self.clusters),
        };
        self._layout_animation = Some(cx.spawn(async move |this, cx| {
            for frame in 1..=LAYOUT_ANIMATION_FRAMES {
                cx.background_executor()
                    .timer(LAYOUT_ANIMATION / LAYOUT_ANIMATION_FRAMES)
                    .await;
                let progress = frame as f32 / LAYOUT_ANIMATION_FRAMES as f32;
                let interpolated = layout::lerp(&from, &target, progress);
                if this
                    .update(cx, |view, cx| {
                        view.positions = interpolated.positions.clone();
                        view.clusters = interpolated.clusters.clone();
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
        }));
    }

    fn toggle_layout_mode(&mut self, cx: &mut Context<Self>) {
        self.layout_mode = match self.layout_mode {
            GridLayoutMode::StructuredGrid => GridLayoutMode::ForceDirectedGraph,
            GridLayoutMode::ForceDirectedGraph => GridLayoutMode::StructuredGrid,
        };
        self.recompute_layout(cx);
    }

    pub(crate) fn nodes(&self) -> &[GridNode] {
        &self.nodes
    }

    pub(crate) fn edges(&self) -> &[GridEdge] {
        &self.edges
    }

    pub(crate) fn node_positions(&self) -> &HashMap<NodeId, Vec2> {
        &self.positions
    }

    pub(crate) fn cluster_bounds(&self) -> &[ClusterBounds] {
        &self.clusters
    }

    pub(crate) fn camera(&self) -> Camera2D {
        self.camera
    }

    /// Coalesces rapid edits into a single re-index.
    fn schedule_reindex(&mut self, cx: &mut Context<Self>) {
        self._pending_reindex = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(REINDEX_DEBOUNCE).await;
            this.update(cx, |view, cx| view.reindex(cx)).ok();
        }));
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let background = cx.theme().colors().editor_background;
        let labels = window.use_keyed_state("grid-box-labels", cx, |_, _| LabelCache::default());
        v_flex()
            .id("GridBox")
            .key_context("GridBox")
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(|view, _: &ToggleLayoutMode, _window, cx| {
                view.toggle_layout_mode(cx);
            }))
            .size_full()
            .bg(background)
            .child(canvas_element::render_canvas(cx.entity(), labels))
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

fn layout_for(mode: GridLayoutMode, nodes: &[GridNode], edges: &[GridEdge]) -> layout::Layout {
    match mode {
        GridLayoutMode::StructuredGrid => layout::structured_grid(nodes, edges),
        GridLayoutMode::ForceDirectedGraph => layout::force_directed(nodes, edges),
    }
}
