//! A* pathfinding on tile maps.
//!
//! # Quick start
//! ```no_run
//! use nene::pathfinding::find_path;
//! use nene::tilemap::TileMap;
//!
//! let mut map = TileMap::new(10, 10);
//! map.set_solid(3, 0, true);
//! map.set_solid(3, 1, true);
//! map.set_solid(3, 2, true);
//!
//! // 4-directional path from (0,0) to (5,0)
//! let path = find_path(&map, (0, 0), (5, 0), false);
//!
//! // 8-directional (diagonal movement allowed)
//! let path_diag = find_path(&map, (0, 0), (5, 5), true);
//! ```

use std::collections::BinaryHeap;
use std::collections::HashMap;

use crate::tilemap::TileMap;

// ── Node ──────────────────────────────────────────────────────────────────────

#[derive(Eq, PartialEq)]
struct Node {
    /// f = g + h
    f: u32,
    g: u32,
    pos: (u32, u32),
}

// Min-heap: smaller f = higher priority.
impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.f.cmp(&self.f).then_with(|| other.g.cmp(&self.g))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ── Heuristic ─────────────────────────────────────────────────────────────────

fn heuristic(a: (u32, u32), b: (u32, u32), diagonal: bool) -> u32 {
    let dx = a.0.abs_diff(b.0);
    let dy = a.1.abs_diff(b.1);
    if diagonal {
        // Chebyshev distance (each step costs 1, diagonal = 1 too).
        dx.max(dy)
    } else {
        // Manhattan distance.
        dx + dy
    }
}

// ── Neighbors ─────────────────────────────────────────────────────────────────

fn neighbors(pos: (u32, u32), map: &TileMap, diagonal: bool) -> impl Iterator<Item = (u32, u32)> {
    let (col, row) = pos;
    let cols = map.cols;
    let rows = map.rows;

    // Cardinal + optionally diagonal offsets.
    let offsets: &[(i32, i32)] = if diagonal {
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
        .filter_map(move |&(dc, dr)| {
            let nc = col as i32 + dc;
            let nr = row as i32 + dr;
            if nc < 0 || nr < 0 || nc >= cols as i32 || nr >= rows as i32 {
                return None;
            }
            let nc = nc as u32;
            let nr = nr as u32;
            if map.is_solid(nc, nr) {
                return None;
            }
            // For diagonals, block if both cardinal neighbors are solid (corner cutting).
            if dc != 0 && dr != 0 {
                let horiz_solid = map.is_solid((col as i32 + dc) as u32, row);
                let vert_solid = map.is_solid(col, (row as i32 + dr) as u32);
                if horiz_solid || vert_solid {
                    return None;
                }
            }
            Some((nc, nr))
        })
        .collect::<Vec<_>>()
        .into_iter()
}

// ── A* ────────────────────────────────────────────────────────────────────────

/// Find the shortest path between two tile coordinates using A*.
///
/// - `start` / `goal` — `(col, row)` tile coordinates.
/// - `diagonal` — when `true`, 8-directional movement is allowed;
///   when `false`, only 4-directional (cardinal) movement is used.
///
/// Returns the path as a `Vec<(col, row)>` **including both endpoints**, or
/// `None` if no path exists (blocked, out-of-bounds, or start == goal with no
/// solid tiles in the way — which returns `Some(vec![start])`).
///
/// Out-of-bounds cells and solid cells are treated as impassable.
pub fn find_path(
    map: &TileMap,
    start: (u32, u32),
    goal: (u32, u32),
    diagonal: bool,
) -> Option<Vec<(u32, u32)>> {
    if map.is_solid(start.0, start.1) || map.is_solid(goal.0, goal.1) {
        return None;
    }
    if start == goal {
        return Some(vec![start]);
    }

    let mut open = BinaryHeap::new();
    // g_score: best known cost to reach each node.
    let mut g_score: HashMap<(u32, u32), u32> = HashMap::new();
    // came_from: for path reconstruction.
    let mut came_from: HashMap<(u32, u32), (u32, u32)> = HashMap::new();

    g_score.insert(start, 0);
    open.push(Node {
        f: heuristic(start, goal, diagonal),
        g: 0,
        pos: start,
    });

    while let Some(Node { g, pos, .. }) = open.pop() {
        if pos == goal {
            return Some(reconstruct(came_from, pos));
        }

        // Skip if we've already found a better path.
        if g > *g_score.get(&pos).unwrap_or(&u32::MAX) {
            continue;
        }

        for nb in neighbors(pos, map, diagonal) {
            let tentative_g = g + 1;
            if tentative_g < *g_score.get(&nb).unwrap_or(&u32::MAX) {
                g_score.insert(nb, tentative_g);
                came_from.insert(nb, pos);
                open.push(Node {
                    f: tentative_g + heuristic(nb, goal, diagonal),
                    g: tentative_g,
                    pos: nb,
                });
            }
        }
    }

    None // no path found
}

fn reconstruct(
    came_from: HashMap<(u32, u32), (u32, u32)>,
    mut current: (u32, u32),
) -> Vec<(u32, u32)> {
    let mut path = vec![current];
    while let Some(&prev) = came_from.get(&current) {
        path.push(prev);
        current = prev;
    }
    path.reverse();
    path
}

// ── World-space helpers ────────────────────────────────────────────────────────

/// Convert a world-space position to a tile coordinate.
///
/// Uses the same Y convention as [`TileMap`]: tile row 0 is at world y = 0
/// and row increases downward (world y decreases).
///
/// Returns `None` if the position is outside the map.
pub fn world_to_tile(wx: f32, wy: f32, tile_size: f32, map: &TileMap) -> Option<(u32, u32)> {
    let col = (wx / tile_size).floor() as i32;
    let row = ((-wy) / tile_size).floor() as i32;
    if col < 0 || row < 0 || col >= map.cols as i32 || row >= map.rows as i32 {
        return None;
    }
    Some((col as u32, row as u32))
}

/// Convert a tile coordinate to its world-space center position.
///
/// Uses the same Y convention as [`TileMap`]: y decreases as row increases.
pub fn tile_to_world(col: u32, row: u32, tile_size: f32) -> (f32, f32) {
    let wx = col as f32 * tile_size + tile_size * 0.5;
    let wy = -(row as f32 * tile_size + tile_size * 0.5);
    (wx, wy)
}
