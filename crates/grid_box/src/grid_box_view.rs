use std::collections::{HashMap, HashSet};
use std::time::Duration;

use gpui::{
    App, Bounds, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, MouseButton,
    ParentElement, Pixels, Point, Render, SharedString, Styled, Subscription, Task, WeakEntity,
    Window, point, px,
};
use project::{Project, ProjectPath};
use ui::prelude::*;
use workspace::Workspace;
use workspace::item::{Item, ItemEvent};

use crate::canvas_element::{self, LabelCache};
use crate::indexer::{self, SourceFile};
use crate::layout::{self, ClusterBounds, Vec2, NODE_SIZE};
use crate::persistence::{PersistedLayout, PinnedNode};
use crate::{PinSelectedNodes, ToggleGridBox, ToggleLayoutMode};

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

/// An in-progress pointer interaction.
#[derive(Clone, Copy, Debug)]
enum DragState {
    /// Panning the camera by dragging.
    Pan { last: Point<Pixels> },
    /// Selecting nodes with a lasso rectangle, in world coordinates.
    Lasso { start: Vec2, current: Vec2 },
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
    selected_nodes: HashSet<NodeId>,
    drag: Option<DragState>,
    persisted: PersistedLayout,
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
            selected_nodes: HashSet::new(),
            drag: None,
            persisted: PersistedLayout::default(),
            _project_subscription: project_subscription,
            _index_task: None,
            _pending_reindex: None,
            _layout_task: None,
            _layout_animation: None,
        };
        let key = this.persistence_key(cx);
        this.persisted = PersistedLayout::load(cx, &key);
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
                let mut layout = layout;
                view.apply_persisted(&mut layout);
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

    fn apply_persisted(&self, layout: &mut layout::Layout) {
        for node in &self.nodes {
            if let Some(position) = self.persisted.position(&path_key(&node.path)) {
                layout.positions.insert(node.id, position);
            }
        }
    }

    fn pin_selected(&mut self, cx: &mut Context<Self>) {
        if self.selected_nodes.is_empty() {
            return;
        }
        for node in &self.nodes {
            if !self.selected_nodes.contains(&node.id) {
                continue;
            }
            if let Some(position) = self.positions.get(&node.id).copied() {
                self.persisted.pins.insert(
                    path_key(&node.path),
                    PinnedNode {
                        x: position.x,
                        y: position.y,
                        pinned: true,
                    },
                );
            }
        }
        let key = self.persistence_key(cx);
        self.persisted.save(cx, key);
        cx.notify();
    }

    fn persistence_key(&self, cx: &App) -> String {
        let mut roots: Vec<String> = self
            .project
            .read(cx)
            .worktrees(cx)
            .map(|worktree| worktree.read(cx).abs_path().to_string_lossy().into_owned())
            .collect();
        roots.sort();
        roots.join("\n")
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

    pub(crate) fn selected_nodes(&self) -> &HashSet<NodeId> {
        &self.selected_nodes
    }

    pub(crate) fn lasso_rect(&self) -> Option<(Vec2, Vec2)> {
        match self.drag {
            Some(DragState::Lasso { start, current }) => Some((start, current)),
            _ => None,
        }
    }

    fn screen_to_world(&self, bounds: Bounds<Pixels>, screen: Point<Pixels>) -> Vec2 {
        let pan = Vec2::new(f32::from(self.camera.pan.x), f32::from(self.camera.pan.y));
        let zoom = self.camera.zoom;
        Vec2::new(
            (f32::from(screen.x) - f32::from(bounds.origin.x)) / zoom + pan.x,
            (f32::from(screen.y) - f32::from(bounds.origin.y)) / zoom + pan.y,
        )
    }

    fn node_at(&self, bounds: Bounds<Pixels>, screen: Point<Pixels>) -> Option<NodeId> {
        let world = self.screen_to_world(bounds, screen);
        self.nodes
            .iter()
            .find(|node| {
                self.positions.get(&node.id).is_some_and(|position| {
                    world.x >= position.x
                        && world.x <= position.x + NODE_SIZE.x
                        && world.y >= position.y
                        && world.y <= position.y + NODE_SIZE.y
                })
            })
            .map(|node| node.id)
    }

    fn open_node(&mut self, id: NodeId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self
            .nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.path.clone())
        else {
            return;
        };
        if let Some(workspace) = self.workspace.upgrade() {
            workspace.update(cx, |workspace, cx| {
                workspace.open_path(path, None, true, window, cx).detach();
            });
        }
    }

    pub(crate) fn mouse_down(
        &mut self,
        bounds: Bounds<Pixels>,
        button: MouseButton,
        position: Point<Pixels>,
        shift: bool,
        click_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match button {
            MouseButton::Middle => {
                self.drag = Some(DragState::Pan { last: position });
            }
            MouseButton::Left if shift => {
                let world = self.screen_to_world(bounds, position);
                self.drag = Some(DragState::Lasso {
                    start: world,
                    current: world,
                });
                cx.notify();
            }
            MouseButton::Left => {
                self.selected_nodes.clear();
                if let Some(id) = self.node_at(bounds, position) {
                    self.selected_nodes.insert(id);
                    if click_count >= 2 {
                        self.open_node(id, window, cx);
                    }
                }
                cx.notify();
            }
            _ => {}
        }
    }

    pub(crate) fn mouse_move(
        &mut self,
        bounds: Bounds<Pixels>,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        match self.drag {
            Some(DragState::Pan { last }) => {
                let dx = f32::from(position.x) - f32::from(last.x);
                let dy = f32::from(position.y) - f32::from(last.y);
                self.camera.pan = point(
                    px(f32::from(self.camera.pan.x) + dx),
                    px(f32::from(self.camera.pan.y) + dy),
                );
                self.drag = Some(DragState::Pan { last: position });
                cx.notify();
            }
            Some(DragState::Lasso { start, .. }) => {
                let current = self.screen_to_world(bounds, position);
                self.drag = Some(DragState::Lasso { start, current });
                cx.notify();
            }
            None => {}
        }
    }

    pub(crate) fn mouse_up(&mut self, cx: &mut Context<Self>) {
        if let Some(DragState::Lasso { start, current }) = self.drag.take() {
            let min = Vec2::new(start.x.min(current.x), start.y.min(current.y));
            let max = Vec2::new(start.x.max(current.x), start.y.max(current.y));
            self.selected_nodes = self
                .nodes
                .iter()
                .filter(|node| {
                    self.positions.get(&node.id).is_some_and(|position| {
                        position.x <= max.x
                            && position.x + NODE_SIZE.x >= min.x
                            && position.y <= max.y
                            && position.y + NODE_SIZE.y >= min.y
                    })
                })
                .map(|node| node.id)
                .collect();
        }
        self.drag = None;
        cx.notify();
    }

    pub(crate) fn scroll(&mut self, delta_y: f32, cx: &mut Context<Self>) {
        let factor = (1.0 + delta_y * 0.0015).clamp(0.5, 2.0);
        self.camera.zoom = (self.camera.zoom * factor).clamp(0.15, 2.5);
        cx.notify();
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
            .on_action(cx.listener(|view, _: &PinSelectedNodes, _window, cx| {
                view.pin_selected(cx);
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

/// A stable, human-readable key for a project path, used for persistence.
fn path_key(path: &ProjectPath) -> String {
    path.path.as_std_path().to_string_lossy().replace('\\', "/")
}
