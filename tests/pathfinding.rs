use nene::pathfinding::{TileMapGraph, find_path, tile_to_world, world_to_tile};
use nene::tilemap::TileMap;

fn empty(cols: u32, rows: u32) -> TileMap {
    TileMap::new(cols, rows)
}

fn walled(cols: u32, rows: u32) -> TileMap {
    let mut map = TileMap::new(cols, rows);
    // Vertical wall at col 3, rows 0-4 (gap at row 5)
    for r in 0..5u32 {
        map.set_solid(3, r, true);
    }
    map
}

fn path4(map: &TileMap, start: (u32, u32), goal: (u32, u32)) -> Option<Vec<(u32, u32)>> {
    find_path(&TileMapGraph::new(map, false), start, goal)
}

fn path8(map: &TileMap, start: (u32, u32), goal: (u32, u32)) -> Option<Vec<(u32, u32)>> {
    find_path(&TileMapGraph::new(map, true), start, goal)
}

// ── Basic ─────────────────────────────────────────────────────────────────────

#[test]
fn same_start_goal() {
    let map = empty(10, 10);
    let path = path4(&map, (2, 2), (2, 2)).unwrap();
    assert_eq!(path, vec![(2, 2)]);
}

#[test]
fn adjacent_step() {
    let map = empty(10, 10);
    let path = path4(&map, (0, 0), (1, 0)).unwrap();
    assert_eq!(path.len(), 2);
    assert_eq!(path[0], (0, 0));
    assert_eq!(path[path.len() - 1], (1, 0));
}

#[test]
fn straight_line_4dir() {
    let map = empty(10, 10);
    let path = path4(&map, (0, 0), (5, 0)).unwrap();
    assert_eq!(path.len(), 6);
    assert_eq!(path[0], (0, 0));
    assert_eq!(path[5], (5, 0));
}

#[test]
fn no_path_blocked() {
    let mut map = TileMap::new(6, 1);
    for r in 0..1u32 {
        map.set_solid(3, r, true);
    }
    assert!(path4(&map, (0, 0), (5, 0)).is_none());
}

#[test]
fn blocked_start_returns_none() {
    let mut map = empty(10, 10);
    map.set_solid(0, 0, true);
    assert!(path4(&map, (0, 0), (5, 5)).is_none());
}

#[test]
fn blocked_goal_returns_none() {
    let mut map = empty(10, 10);
    map.set_solid(5, 5, true);
    assert!(path4(&map, (0, 0), (5, 5)).is_none());
}

// ── Wall avoidance ────────────────────────────────────────────────────────────

#[test]
fn goes_around_wall_4dir() {
    let map = walled(10, 10);
    let path = path4(&map, (0, 0), (5, 0)).unwrap();
    assert_eq!(path[0], (0, 0));
    assert_eq!(path[path.len() - 1], (5, 0));
    assert!(path.iter().any(|&(_, r)| r >= 5));
    for &(c, r) in &path {
        assert!(!map.is_solid(c, r), "solid tile in path: ({c},{r})");
    }
}

#[test]
fn path_avoids_solid_tiles() {
    let map = walled(10, 10);
    let path = path4(&map, (1, 2), (6, 2)).unwrap();
    for &(c, r) in &path {
        assert!(!map.is_solid(c, r));
    }
}

// ── Diagonal ──────────────────────────────────────────────────────────────────

#[test]
fn diagonal_shorter_than_4dir() {
    let map = empty(10, 10);
    let p4 = path4(&map, (0, 0), (4, 4)).unwrap();
    let p8 = path8(&map, (0, 0), (4, 4)).unwrap();
    assert!(p8.len() < p4.len());
}

#[test]
fn diagonal_no_corner_cutting() {
    let mut map = TileMap::new(5, 5);
    map.set_solid(1, 0, true);
    map.set_solid(0, 1, true);
    let result = path8(&map, (0, 0), (1, 1));
    if let Some(p) = result {
        assert!(p.len() > 2);
    }
}

#[test]
fn diagonal_path_valid_tiles() {
    let map = empty(10, 10);
    let path = path8(&map, (0, 0), (5, 5)).unwrap();
    for &(c, r) in &path {
        assert!(!map.is_solid(c, r));
    }
}

// ── Out of bounds ─────────────────────────────────────────────────────────────

#[test]
fn out_of_bounds_start_returns_none() {
    let map = empty(5, 5);
    assert!(path4(&map, (10, 10), (0, 0)).is_none());
}

// ── Custom graph ──────────────────────────────────────────────────────────────

use nene::pathfinding::PathGraph;

struct Chain(u32); // nodes 0..n, each connects to next
impl PathGraph for Chain {
    type Node = u32;
    fn neighbors(&self, &n: &u32) -> Vec<u32> {
        if n + 1 < self.0 { vec![n + 1] } else { vec![] }
    }
    fn heuristic(&self, &a: &u32, &b: &u32) -> u32 {
        b.saturating_sub(a)
    }
}

#[test]
fn custom_graph_finds_path() {
    let path = find_path(&Chain(10), 0u32, 5).unwrap();
    assert_eq!(path, vec![0, 1, 2, 3, 4, 5]);
}

#[test]
fn custom_graph_no_path() {
    assert!(find_path(&Chain(10), 5u32, 0).is_none()); // one-directional chain
}

// ── Coordinate conversion ─────────────────────────────────────────────────────

#[test]
fn tile_to_world_center() {
    let (wx, wy) = tile_to_world(0, 0, 1.0);
    assert!((wx - 0.5).abs() < 1e-6);
    assert!((wy - (-0.5)).abs() < 1e-6);
}

#[test]
fn world_to_tile_basic() {
    let map = empty(10, 10);
    let t = world_to_tile(1.5, -1.5, 1.0, map.cols, map.rows).unwrap();
    assert_eq!(t, (1, 1));
}

#[test]
fn world_to_tile_out_of_bounds() {
    let map = empty(5, 5);
    assert!(world_to_tile(-1.0, 0.0, 1.0, map.cols, map.rows).is_none());
    assert!(world_to_tile(0.0, 1.0, 1.0, map.cols, map.rows).is_none());
}

#[test]
fn roundtrip_tile_world() {
    let map = empty(10, 10);
    let tile_size = 2.0;
    for col in 0..5u32 {
        for row in 0..5u32 {
            let (wx, wy) = tile_to_world(col, row, tile_size);
            let back = world_to_tile(wx, wy, tile_size, map.cols, map.rows).unwrap();
            assert_eq!(back, (col, row));
        }
    }
}
