//! The two Grid-Box layout solvers.
//!
//! `structured_grid` is deterministic and runs in `O(N)`, so it is the default
//! and costs nothing once computed. `force_directed` is a damped
//! Fruchterman-Reingold solver whose repulsion is approximated on a uniform
//! grid (a cheap stand-in for Barnes-Hut), so it also runs in roughly `O(N)`
//! per tick and is expected to be run on the background executor.

use std::collections::{BTreeMap, HashMap};
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

use crate::{GridEdge, GridNode, NodeId};

/// A point in world space, in device-independent pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, scale: f32) -> Vec2 {
        Vec2::new(self.x * scale, self.y * scale)
    }
}

impl Div<f32> for Vec2 {
    type Output = Vec2;
    fn div(self, scale: f32) -> Vec2 {
        Vec2::new(self.x / scale, self.y / scale)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, other: Vec2) {
        *self = *self + other;
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, other: Vec2) {
        *self = *self - other;
    }
}

/// An axis-aligned rectangle in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutRect {
    pub origin: Vec2,
    pub size: Vec2,
}

impl LayoutRect {
    pub const fn new(origin: Vec2, size: Vec2) -> Self {
        Self { origin, size }
    }
}

/// The bounds and label of a directory cluster in the structured grid.
#[derive(Clone, Debug, PartialEq)]
pub struct ClusterBounds {
    pub label: String,
    pub rect: LayoutRect,
}

/// The result of a layout pass: a position per node, plus any cluster bounds.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub positions: HashMap<NodeId, Vec2>,
    pub clusters: Vec<ClusterBounds>,
}

/// The nominal size of a file box.
pub const NODE_SIZE: Vec2 = Vec2::new(200.0, 56.0);
const NODE_GAP: f32 = 24.0;
const CLUSTER_PADDING: f32 = 16.0;
const CLUSTER_HEADER: f32 = 26.0;
const CLUSTER_GAP: f32 = 32.0;

/// Side length of the square world the force solver spreads nodes over.
const FORCE_AREA: f32 = 1200.0;
/// Upper bound on simulation ticks.
const MAX_TICKS: u32 = 120;
/// Temperature decay per tick.
const ALPHA_DECAY: f32 = 0.95;
/// The simulation stops once no node moves faster than this.
const CONVERGENCE_VELOCITY: f32 = 0.1;

/// Groups files into labelled directory clusters and grids them.
///
/// Runs in `O(N)` and is deterministic for a given graph.
pub fn structured_grid(nodes: &[GridNode], edges: &[GridEdge]) -> Layout {
    let in_degree = in_degrees(nodes, edges);
    let mut by_directory: BTreeMap<String, Vec<&GridNode>> = BTreeMap::new();
    for node in nodes {
        let directory = node
            .path
            .path
            .as_std_path()
            .parent()
            .map(|parent| parent.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        by_directory.entry(directory).or_default().push(node);
    }

    let mut positions = HashMap::with_capacity(nodes.len());
    let mut clusters = Vec::with_capacity(by_directory.len());
    let mut offset_y = 0.0;

    for (label, mut directory_nodes) in by_directory {
        directory_nodes.sort_by(|a, b| {
            in_degree
                .get(&b.id)
                .copied()
                .unwrap_or(0)
                .cmp(&in_degree.get(&a.id).copied().unwrap_or(0))
                .then_with(|| a.id.cmp(&b.id))
        });

        let columns = ceil_sqrt(directory_nodes.len());
        for (index, node) in directory_nodes.iter().enumerate() {
            let column = (index % columns) as f32;
            let row = (index / columns) as f32;
            positions.insert(
                node.id,
                Vec2::new(
                    CLUSTER_PADDING + column * (NODE_SIZE.x + NODE_GAP),
                    offset_y + CLUSTER_HEADER + CLUSTER_PADDING + row * (NODE_SIZE.y + NODE_GAP),
                ),
            );
        }

        let rows = directory_nodes.len().div_ceil(columns);
        let width = CLUSTER_PADDING * 2.0
            + columns as f32 * NODE_SIZE.x
            + columns.saturating_sub(1) as f32 * NODE_GAP;
        let height = CLUSTER_HEADER
            + CLUSTER_PADDING * 2.0
            + rows as f32 * NODE_SIZE.y
            + rows.saturating_sub(1) as f32 * NODE_GAP;
        clusters.push(ClusterBounds {
            label,
            rect: LayoutRect::new(Vec2::new(0.0, offset_y), Vec2::new(width, height)),
        });
        offset_y += height + CLUSTER_GAP;
    }

    Layout { positions, clusters }
}

/// Runs a damped Fruchterman-Reingold simulation and returns the settled
/// positions.
///
/// The solver performs at most [`MAX_TICKS`] ticks, cooling by a factor of
/// [`ALPHA_DECAY`] each tick, and stops early once it has converged, so it never
/// keeps running on an idle graph.
pub fn force_directed(nodes: &[GridNode], edges: &[GridEdge]) -> Layout {
    let count = nodes.len();
    if count == 0 {
        return Layout::default();
    }

    let ideal = (FORCE_AREA * FORCE_AREA / count as f32).sqrt();
    let mut positions: Vec<Vec2> = (0..count)
        .map(|index| {
            let angle = index as f32 / count as f32 * std::f32::consts::TAU;
            Vec2::new(angle.cos() * ideal, angle.sin() * ideal)
        })
        .collect();
    let index_of: HashMap<NodeId, usize> = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id, index))
        .collect();

    let mut displacement = vec![Vec2::default(); count];
    let mut alpha = 1.0f32;
    let cell_size = (ideal * 2.0).max(1.0);

    for _ in 0..MAX_TICKS {
        for value in displacement.iter_mut() {
            *value = Vec2::default();
        }

        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (index, position) in positions.iter().enumerate() {
            grid.entry(cell_of(*position, cell_size))
                .or_default()
                .push(index);
        }

        for (index, position) in positions.iter().enumerate() {
            let (cell_x, cell_y) = cell_of(*position, cell_size);
            for delta_x in -1..=1 {
                for delta_y in -1..=1 {
                    let Some(neighbours) = grid.get(&(cell_x + delta_x, cell_y + delta_y)) else {
                        continue;
                    };
                    for &other in neighbours {
                        if other == index {
                            continue;
                        }
                        let delta = *position - positions[other];
                        let distance = delta.length().max(0.01);
                        let force = ideal * ideal / distance;
                        displacement[index] += delta * (force / distance);
                    }
                }
            }
        }

        for edge in edges {
            let (Some(&source), Some(&target)) =
                (index_of.get(&edge.source), index_of.get(&edge.target))
            else {
                continue;
            };
            let delta = positions[source] - positions[target];
            let distance = delta.length().max(0.01);
            let force = distance * distance / ideal;
            displacement[source] -= delta * (force / distance);
            displacement[target] += delta * (force / distance);
        }

        let mut max_velocity = 0.0f32;
        for index in 0..count {
            let step = displacement[index];
            let length = step.length();
            positions[index] += if length > alpha {
                step * (alpha / length)
            } else {
                step
            };
            max_velocity = max_velocity.max(length.min(alpha));
        }

        alpha *= ALPHA_DECAY;
        if max_velocity < CONVERGENCE_VELOCITY {
            break;
        }
    }

    Layout {
        positions: nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id, positions[index]))
            .collect(),
        clusters: Vec::new(),
    }
}

/// Linearly interpolates between two layouts, used to animate a mode switch.
pub fn lerp(from: &Layout, to: &Layout, progress: f32) -> Layout {
    let progress = progress.clamp(0.0, 1.0);
    let mut positions = HashMap::with_capacity(to.positions.len());
    for (id, target) in &to.positions {
        let start = from.positions.get(id).copied().unwrap_or(*target);
        positions.insert(*id, start + (*target - start) * progress);
    }
    let clusters = if progress < 0.5 {
        from.clusters.clone()
    } else {
        to.clusters.clone()
    };
    Layout { positions, clusters }
}

fn in_degrees(nodes: &[GridNode], edges: &[GridEdge]) -> HashMap<NodeId, usize> {
    let mut in_degree: HashMap<NodeId, usize> = nodes.iter().map(|node| (node.id, 0)).collect();
    for edge in edges {
        *in_degree.entry(edge.target).or_default() += 1;
    }
    in_degree
}

fn ceil_sqrt(value: usize) -> usize {
    ((value as f32).sqrt().ceil() as usize).max(1)
}

fn cell_of(position: Vec2, cell_size: f32) -> (i32, i32) {
    (
        (position.x / cell_size).floor() as i32,
        (position.y / cell_size).floor() as i32,
    )
}
