//! A* pathfinding on any graph that implements [`PathGraph`].
//!
//! # Quick start
//!
//! ## Tile map
//! ```no_run
//! use nene::ai::pathfinding::{find_path, TileMapGraph};
//! use nene::tilemap::TileMap;
//!
//! let mut map = TileMap::new(10, 10);
//! map.set_solid(3, 0, true);
//!
//! // 4-directional
//! let path = find_path(&TileMapGraph::new(&map, false), (0u32, 0u32), (5, 0));
//!
//! // 8-directional
//! let path = find_path(&TileMapGraph::new(&map, true), (0u32, 0u32), (5, 5));
//! ```
//!
//! ## Custom graph (any node type)
//! ```no_run
//! use nene::ai::pathfinding::{PathGraph, find_path};
//!
//! struct RoomGraph;
//! impl PathGraph for RoomGraph {
//!     type Node = &'static str;
//!     fn neighbors(&self, node: &Self::Node) -> Vec<Self::Node> {
//!         match *node {
//!             "hall"    => vec!["kitchen", "bedroom"],
//!             "kitchen" => vec!["hall"],
//!             "bedroom" => vec!["hall"],
//!             _         => vec![],
//!         }
//!     }
//!     fn heuristic(&self, _: &Self::Node, _: &Self::Node) -> u32 { 0 }
//! }
//!
//! let path = find_path(&RoomGraph, "hall", "bedroom");
//! ```

use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::hash::Hash;

use crate::tilemap::TileMap;

// ── PathGraph ─────────────────────────────────────────────────────────────────

/// A graph that A* can search.
///
/// `Node` can be anything hashable — `(u32, u32)` tile coords,
/// `(i32, i32, i32)` voxel coords, a waypoint index, a string label, etc.
pub trait PathGraph {
    type Node: Eq + Hash + Clone;

    /// All nodes reachable from `node` in one step.
    fn neighbors(&self, node: &Self::Node) -> Vec<Self::Node>;

    /// Admissible lower-bound cost estimate from `from` to `to`.
    /// Return `0` for Dijkstra-style behaviour.
    fn heuristic(&self, from: &Self::Node, to: &Self::Node) -> u32;
}

// ── Internal heap entry ───────────────────────────────────────────────────────

struct HeapNode<N> {
    f: u32,
    g: u32,
    pos: N,
}

impl<N: Eq> Eq for HeapNode<N> {}
impl<N: Eq> PartialEq for HeapNode<N> {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f && self.g == other.g
    }
}
impl<N: Eq> Ord for HeapNode<N> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.f.cmp(&self.f).then_with(|| other.g.cmp(&self.g))
    }
}
impl<N: Eq> PartialOrd for HeapNode<N> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ── A* ────────────────────────────────────────────────────────────────────────

/// Find the shortest path on any [`PathGraph`].
///
/// Returns `Vec<Node>` including both endpoints, or `None` if unreachable.
/// Returns `Some(vec![start])` when `start == goal`.
pub fn find_path<G: PathGraph>(graph: &G, start: G::Node, goal: G::Node) -> Option<Vec<G::Node>> {
    if start == goal {
        return Some(vec![start]);
    }

    let mut open: BinaryHeap<HeapNode<G::Node>> = BinaryHeap::new();
    let mut g_score: HashMap<G::Node, u32> = HashMap::new();
    let mut came_from: HashMap<G::Node, G::Node> = HashMap::new();

    g_score.insert(start.clone(), 0);
    open.push(HeapNode {
        f: graph.heuristic(&start, &goal),
        g: 0,
        pos: start,
    });

    while let Some(HeapNode { g, pos, .. }) = open.pop() {
        if pos == goal {
            return Some(reconstruct(came_from, pos));
        }
        if g > *g_score.get(&pos).unwrap_or(&u32::MAX) {
            continue;
        }
        for nb in graph.neighbors(&pos) {
            let tentative_g = g + 1;
            if tentative_g < *g_score.get(&nb).unwrap_or(&u32::MAX) {
                let h = graph.heuristic(&nb, &goal);
                came_from.insert(nb.clone(), pos.clone());
                g_score.insert(nb.clone(), tentative_g);
                open.push(HeapNode {
                    f: tentative_g + h,
                    g: tentative_g,
                    pos: nb,
                });
            }
        }
    }

    None
}

fn reconstruct<N: Eq + Hash + Clone>(came_from: HashMap<N, N>, mut current: N) -> Vec<N> {
    let mut path = vec![current.clone()];
    while let Some(prev) = came_from.get(&current) {
        path.push(prev.clone());
        current = prev.clone();
    }
    path.reverse();
    path
}

// ── TileMapGraph ──────────────────────────────────────────────────────────────

/// [`PathGraph`] adapter for [`TileMap`].
///
/// Wraps a `TileMap` reference and exposes it as a graph with `(u32, u32)` nodes.
/// Pass this to [`find_path`].
pub struct TileMapGraph<'a> {
    map: &'a TileMap,
    diagonal: bool,
}

impl<'a> TileMapGraph<'a> {
    /// Create a graph over `map`.
    ///
    /// - `diagonal` — `true` for 8-directional movement (corner-cutting blocked).
    pub fn new(map: &'a TileMap, diagonal: bool) -> Self {
        Self { map, diagonal }
    }
}

impl<'a> PathGraph for TileMapGraph<'a> {
    type Node = (u32, u32);

    fn neighbors(&self, &(col, row): &(u32, u32)) -> Vec<(u32, u32)> {
        if self.map.is_solid(col, row) {
            return vec![];
        }
        let offsets: &[(i32, i32)] = if self.diagonal {
            &[
                (-1, 0),
                (1, 0),
                (0, -1),
                (0, 1),
                (-1, -1),
                (1, -1),
                (-1, 1),
                (1, 1),
            ]
        } else {
            &[(-1, 0), (1, 0), (0, -1), (0, 1)]
        };
        offsets
            .iter()
            .filter_map(|&(dc, dr)| {
                let nc = col as i32 + dc;
                let nr = row as i32 + dr;
                if nc < 0 || nr < 0 || nc >= self.map.cols as i32 || nr >= self.map.rows as i32 {
                    return None;
                }
                let (nc, nr) = (nc as u32, nr as u32);
                if self.map.is_solid(nc, nr) {
                    return None;
                }
                if dc != 0
                    && dr != 0
                    && (self.map.is_solid((col as i32 + dc) as u32, row)
                        || self.map.is_solid(col, (row as i32 + dr) as u32))
                {
                    return None;
                }
                Some((nc, nr))
            })
            .collect()
    }

    fn heuristic(&self, &(ac, ar): &(u32, u32), &(bc, br): &(u32, u32)) -> u32 {
        let dx = ac.abs_diff(bc);
        let dy = ar.abs_diff(br);
        if self.diagonal { dx.max(dy) } else { dx + dy }
    }
}

// ── World-space helpers ────────────────────────────────────────────────────────

/// Convert a world-space position to a tile coordinate.
///
/// Uses the same Y convention as [`TileMap`]: row 0 is at world y = 0,
/// row increases downward (world y decreases).
pub fn world_to_tile(wx: f32, wy: f32, tile_size: f32, cols: u32, rows: u32) -> Option<(u32, u32)> {
    let col = (wx / tile_size).floor() as i32;
    let row = ((-wy) / tile_size).floor() as i32;
    if col < 0 || row < 0 || col >= cols as i32 || row >= rows as i32 {
        return None;
    }
    Some((col as u32, row as u32))
}

/// Convert a tile coordinate to its world-space center position.
pub fn tile_to_world(col: u32, row: u32, tile_size: f32) -> (f32, f32) {
    let wx = col as f32 * tile_size + tile_size * 0.5;
    let wy = -(row as f32 * tile_size + tile_size * 0.5);
    (wx, wy)
}
