use nene::pathfinding::{find_path, tile_to_world, world_to_tile};
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

// ── Basic ─────────────────────────────────────────────────────────────────────

#[test]
fn same_start_goal() {
    let map = empty(10, 10);
    let path = find_path(&map, (2, 2), (2, 2), false).unwrap();
    assert_eq!(path, vec![(2, 2)]);
}

#[test]
fn adjacent_step() {
    let map = empty(10, 10);
    let path = find_path(&map, (0, 0), (1, 0), false).unwrap();
    assert_eq!(path.len(), 2);
    assert_eq!(path[0], (0, 0));
    assert_eq!(path[path.len() - 1], (1, 0));
}

#[test]
fn straight_line_4dir() {
    let map = empty(10, 10);
    let path = find_path(&map, (0, 0), (5, 0), false).unwrap();
    assert_eq!(path.len(), 6); // 0..5 inclusive
    assert_eq!(path[0], (0, 0));
    assert_eq!(path[5], (5, 0));
}

#[test]
fn no_path_blocked() {
    let mut map = TileMap::new(6, 1);
    // Full vertical barrier
    for r in 0..1u32 {
        map.set_solid(3, r, true);
    }
    assert!(find_path(&map, (0, 0), (5, 0), false).is_none());
}

#[test]
fn blocked_start_returns_none() {
    let mut map = empty(10, 10);
    map.set_solid(0, 0, true);
    assert!(find_path(&map, (0, 0), (5, 5), false).is_none());
}

#[test]
fn blocked_goal_returns_none() {
    let mut map = empty(10, 10);
    map.set_solid(5, 5, true);
    assert!(find_path(&map, (0, 0), (5, 5), false).is_none());
}

// ── Wall avoidance ────────────────────────────────────────────────────────────

#[test]
fn goes_around_wall_4dir() {
    let map = walled(10, 10);
    let path = find_path(&map, (0, 0), (5, 0), false).unwrap();
    assert_eq!(path[0], (0, 0));
    assert_eq!(path[path.len() - 1], (5, 0));
    // Path must go through the gap at row 5
    assert!(path.iter().any(|&(_, r)| r >= 5));
    // No step should be on a solid tile
    for &(c, r) in &path {
        assert!(!map.is_solid(c, r), "solid tile in path: ({c},{r})");
    }
}

#[test]
fn path_avoids_solid_tiles() {
    let map = walled(10, 10);
    let path = find_path(&map, (1, 2), (6, 2), false).unwrap();
    for &(c, r) in &path {
        assert!(!map.is_solid(c, r));
    }
}

// ── Diagonal ──────────────────────────────────────────────────────────────────

#[test]
fn diagonal_shorter_than_4dir() {
    let map = empty(10, 10);
    let p4 = find_path(&map, (0, 0), (4, 4), false).unwrap();
    let p8 = find_path(&map, (0, 0), (4, 4), true).unwrap();
    // 4-dir needs 8 steps; 8-dir needs 5 (diagonal straight line)
    assert!(p8.len() < p4.len());
}

#[test]
fn diagonal_no_corner_cutting() {
    let mut map = TileMap::new(5, 5);
    // Block (1,0) and (0,1) to force corner cutting check from (0,0) to (1,1)
    map.set_solid(1, 0, true);
    map.set_solid(0, 1, true);
    // (0,0) -> (1,1) diagonal should be blocked since both cardinal neighbors are solid
    let result = find_path(&map, (0, 0), (1, 1), true);
    // Either no path, or path goes around (longer than 2 steps)
    if let Some(p) = result {
        assert!(p.len() > 2);
    }
}

#[test]
fn diagonal_path_valid_tiles() {
    let map = empty(10, 10);
    let path = find_path(&map, (0, 0), (5, 5), true).unwrap();
    for &(c, r) in &path {
        assert!(!map.is_solid(c, r));
    }
}

// ── Out of bounds ─────────────────────────────────────────────────────────────

#[test]
fn out_of_bounds_start_returns_none() {
    let map = empty(5, 5);
    // (10, 10) is out of bounds — is_solid returns true for OOB
    assert!(find_path(&map, (10, 10), (0, 0), false).is_none());
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
    let t = world_to_tile(1.5, -1.5, 1.0, &map).unwrap();
    assert_eq!(t, (1, 1));
}

#[test]
fn world_to_tile_out_of_bounds() {
    let map = empty(5, 5);
    assert!(world_to_tile(-1.0, 0.0, 1.0, &map).is_none());
    assert!(world_to_tile(0.0, 1.0, 1.0, &map).is_none()); // positive y = above row 0
}

#[test]
fn roundtrip_tile_world() {
    let map = empty(10, 10);
    let tile_size = 2.0;
    for col in 0..5u32 {
        for row in 0..5u32 {
            let (wx, wy) = tile_to_world(col, row, tile_size);
            let back = world_to_tile(wx, wy, tile_size, &map).unwrap();
            assert_eq!(back, (col, row));
        }
    }
}
