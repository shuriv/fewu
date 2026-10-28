//! The Grid-Box canvas renderer.
//!
//! It draws clusters, file boxes and edges directly with `Window::paint_quad`
//! through GPUI's leaf `canvas` element, so it creates no Taffy layout nodes.
//! Nodes and edges are frustum-culled to the viewport, and node labels are
//! only shaped above a zoom threshold (level of detail), cached across frames.

use std::collections::HashMap;

use gpui::{
    App, Bounds, Entity, Font, Hsla, IntoElement, Pixels, Point, ShapedLine, SharedString,
    TextAlign, TextRun, Window, canvas, fill, point, px, size,
};
use theme::ActiveTheme;
use util::ResultExt as _;

use crate::grid_box_view::{GridBoxView, NodeId};
use crate::layout::{NODE_SIZE, Vec2};

/// Below this zoom, file labels are not shaped at all.
const LOD_TEXT_ZOOM: f32 = 0.35;
const LABEL_FONT_SIZE: f32 = 11.0;
const CLUSTER_TITLE_FONT_SIZE: f32 = 12.0;
const LABEL_PADDING: f32 = 8.0;
const EDGE_THICKNESS: f32 = 1.5;
const NODE_RADIUS: f32 = 4.0;
const CLUSTER_RADIUS: f32 = 6.0;

/// Shaped node labels, kept across frames and shaped on a cache miss.
#[derive(Default)]
pub struct LabelCache {
    lines: HashMap<NodeId, ShapedLine>,
}

struct CanvasState {
    clusters: Vec<(Bounds<Pixels>, SharedString)>,
    nodes: Vec<(Bounds<Pixels>, SharedString, NodeId)>,
    edges: Vec<(Point<Pixels>, Point<Pixels>)>,
    node_labels: Vec<(Bounds<Pixels>, ShapedLine)>,
    cluster_titles: Vec<(Point<Pixels>, ShapedLine)>,
}

/// Renders the Grid-Box graph for `view`, using `labels` to cache shaped text.
pub fn render_canvas(view: Entity<GridBoxView>, labels: Entity<LabelCache>) -> impl IntoElement {
    canvas(
        move |bounds, window, cx| prepaint(bounds, &view, &labels, window, cx),
        paint,
    )
}

fn prepaint(
    bounds: Bounds<Pixels>,
    view: &Entity<GridBoxView>,
    labels: &Entity<LabelCache>,
    window: &mut Window,
    cx: &mut App,
) -> CanvasState {
    let (nodes, edges, positions, clusters, camera) = {
        let view = view.read(cx);
        (
            view.nodes().to_vec(),
            view.edges().to_vec(),
            view.node_positions().clone(),
            view.cluster_bounds().to_vec(),
            view.camera(),
        )
    };

    let zoom = camera.zoom;
    let pan = Vec2::new(f32::from(camera.pan.x), f32::from(camera.pan.y));
    let origin = Vec2::new(f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let to_screen = |world: Vec2| {
        point(
            px(origin.x + (world.x - pan.x) * zoom),
            px(origin.y + (world.y - pan.y) * zoom),
        )
    };

    let mut visible_nodes = Vec::new();
    for node in &nodes {
        let Some(position) = positions.get(&node.id).copied() else {
            continue;
        };
        let node_bounds = Bounds::new(
            to_screen(position),
            size(px(NODE_SIZE.x * zoom), px(NODE_SIZE.y * zoom)),
        );
        if node_bounds.intersects(&bounds) {
            visible_nodes.push((node_bounds, node.label.clone(), node.id));
        }
    }

    let mut visible_edges = Vec::new();
    for edge in &edges {
        let (Some(from), Some(to)) = (positions.get(&edge.source), positions.get(&edge.target))
        else {
            continue;
        };
        let from = to_screen(*from);
        let to = to_screen(*to);
        if segment_bounds(from, to).intersects(&bounds) {
            visible_edges.push((from, to));
        }
    }

    let mut visible_clusters = Vec::new();
    for cluster in &clusters {
        let cluster_bounds = Bounds::new(
            to_screen(cluster.rect.origin),
            size(px(cluster.rect.size.x * zoom), px(cluster.rect.size.y * zoom)),
        );
        if cluster_bounds.intersects(&bounds) {
            visible_clusters.push((cluster_bounds, SharedString::from(cluster.label.clone())));
        }
    }

    let font = Font::default();
    let text_color = cx.theme().colors().text;
    let muted_color = cx.theme().colors().text_muted;

    let mut node_labels = Vec::new();
    if zoom >= LOD_TEXT_ZOOM {
        labels.update(cx, |cache, _cx| {
            for (node_bounds, label, id) in &visible_nodes {
                let line = cache
                    .lines
                    .entry(*id)
                    .or_insert_with(|| {
                        shape_label(window, label, LABEL_FONT_SIZE, font.clone(), text_color)
                    })
                    .clone();
                node_labels.push((*node_bounds, line));
            }
        });
    }

    let mut cluster_titles = Vec::new();
    for (cluster_bounds, label) in &visible_clusters {
        let line = shape_label(
            window,
            label,
            CLUSTER_TITLE_FONT_SIZE,
            font.clone(),
            muted_color,
        );
        cluster_titles.push((
            point(
                cluster_bounds.origin.x + px(LABEL_PADDING),
                cluster_bounds.origin.y + px(LABEL_PADDING / 2.0),
            ),
            line,
        ));
    }

    CanvasState {
        clusters: visible_clusters,
        nodes: visible_nodes,
        edges: visible_edges,
        node_labels,
        cluster_titles,
    }
}

fn paint(bounds: Bounds<Pixels>, state: CanvasState, window: &mut Window, cx: &mut App) {
    let colors = cx.theme().colors();
    let cluster_fill = colors.surface_background;
    let cluster_border = colors.border_variant;
    let node_fill = colors.elevated_surface_background;
    let node_border = colors.border;
    let edge_color = colors.border_variant;

    window.paint_layer(bounds, |window| {
        for (cluster_bounds, _) in &state.clusters {
            window.paint_quad(
                fill(*cluster_bounds, cluster_fill)
                    .corner_radii(px(CLUSTER_RADIUS))
                    .border_widths(px(1.0))
                    .border_color(cluster_border),
            );
        }

        for (from, to) in &state.edges {
            paint_edge(window, *from, *to, edge_color);
        }

        for (node_bounds, _, _) in &state.nodes {
            window.paint_quad(
                fill(*node_bounds, node_fill)
                    .corner_radii(px(NODE_RADIUS))
                    .border_widths(px(1.5))
                    .border_color(node_border),
            );
        }

        for (node_bounds, line) in &state.node_labels {
            line.paint(
                point(
                    node_bounds.origin.x + px(LABEL_PADDING),
                    node_bounds.origin.y + px(LABEL_PADDING),
                ),
                px(LABEL_FONT_SIZE + 4.0),
                TextAlign::Left,
                None,
                window,
                cx,
            )
            .log_err();
        }

        for (origin, line) in &state.cluster_titles {
            line.paint(
                *origin,
                px(CLUSTER_TITLE_FONT_SIZE + 4.0),
                TextAlign::Left,
                None,
                window,
                cx,
            )
            .log_err();
        }
    });
}

fn shape_label(
    window: &Window,
    text: &SharedString,
    font_size: f32,
    font: Font,
    color: Hsla,
) -> ShapedLine {
    let run = TextRun {
        len: text.len(),
        font,
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(text.clone(), px(font_size), &[run], None)
}

/// Draws one edge as a three-segment orthogonal (Manhattan) path.
fn paint_edge(window: &mut Window, from: Point<Pixels>, to: Point<Pixels>, color: Hsla) {
    let (x1, y1) = (f32::from(from.x), f32::from(from.y));
    let (x2, y2) = (f32::from(to.x), f32::from(to.y));
    let mid_y = (y1 + y2) / 2.0;
    paint_segment(window, x1, y1, x1, mid_y, color);
    paint_segment(window, x1, mid_y, x2, mid_y, color);
    paint_segment(window, x2, mid_y, x2, y2, color);
}

fn paint_segment(window: &mut Window, x1: f32, y1: f32, x2: f32, y2: f32, color: Hsla) {
    let half = EDGE_THICKNESS / 2.0;
    let (x, width) = if (x2 - x1).abs() < 0.01 {
        (x1 - half, EDGE_THICKNESS)
    } else {
        (x1.min(x2), (x2 - x1).abs())
    };
    let (y, height) = if (y2 - y1).abs() < 0.01 {
        (y1 - half, EDGE_THICKNESS)
    } else {
        (y1.min(y2), (y2 - y1).abs())
    };
    window.paint_quad(fill(
        Bounds::new(point(px(x), px(y)), size(px(width), px(height))),
        color,
    ));
}

fn segment_bounds(from: Point<Pixels>, to: Point<Pixels>) -> Bounds<Pixels> {
    let (from_x, from_y) = (f32::from(from.x), f32::from(from.y));
    let (to_x, to_y) = (f32::from(to.x), f32::from(to.y));
    Bounds::new(
        point(px(from_x.min(to_x)), px(from_y.min(to_y))),
        size(px((to_x - from_x).abs()), px((to_y - from_y).abs())),
    )
}
