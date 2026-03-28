use nene::tilemap::{TileLayer, TileMap};

// ── TileSet UV ────────────────────────────────────────────────────────────────

/// Replicate TileSet::uv() logic for CPU testing.
fn uv(id: u16, atlas_w: u32, atlas_h: u32, tile_w: u32, tile_h: u32) -> Option<[f32; 4]> {
    if id == 0 {
        return None;
    }
    let cols = atlas_w / tile_w;
    let rows = atlas_h / tile_h;
    let idx = (id - 1) as u32;
    let col = idx % cols;
    let row = idx / cols;
    if row >= rows {
        return None;
    }
    let aw = atlas_w as f32;
    let ah = atlas_h as f32;
    let u0 = col as f32 * tile_w as f32 / aw;
    let v0 = row as f32 * tile_h as f32 / ah;
    let uw = tile_w as f32 / aw;
    let vw = tile_h as f32 / ah;
    Some([u0, v0, uw, vw])
}

#[test]
fn uv_id_zero_is_none() {
    assert!(uv(0, 256, 256, 16, 16).is_none());
}

#[test]
fn uv_first_tile() {
    let [u0, v0, uw, vw] = uv(1, 256, 256, 16, 16).unwrap();
    assert!((u0 - 0.0).abs() < 1e-6);
    assert!((v0 - 0.0).abs() < 1e-6);
    assert!((uw - 16.0 / 256.0).abs() < 1e-6);
    assert!((vw - 16.0 / 256.0).abs() < 1e-6);
}

#[test]
fn uv_second_tile_in_row() {
    let [u0, v0, uw, _] = uv(2, 256, 256, 16, 16).unwrap();
    assert!((u0 - 16.0 / 256.0).abs() < 1e-6);
    assert!((v0 - 0.0).abs() < 1e-6);
    assert!((uw - 16.0 / 256.0).abs() < 1e-6);
}

#[test]
fn uv_wraps_to_next_row() {
    // atlas 32×32, tile 16×16 → 2 cols, 2 rows. Tile 3 = row 1, col 0.
    let [u0, v0, _, _] = uv(3, 32, 32, 16, 16).unwrap();
    assert!((u0 - 0.0).abs() < 1e-6);
    assert!((v0 - 0.5).abs() < 1e-6);
}

#[test]
fn uv_out_of_range_is_none() {
    // 2×2 tiles = 4 tiles max; id 5 is out of range
    assert!(uv(5, 32, 32, 16, 16).is_none());
}

// ── TileLayer ─────────────────────────────────────────────────────────────────

#[test]
fn layer_default_all_zero() {
    let layer = TileLayer::new(10, 8);
    for r in 0..8 {
        for c in 0..10 {
            assert_eq!(layer.get(c, r), 0);
        }
    }
}

#[test]
fn layer_set_get_roundtrip() {
    let mut layer = TileLayer::new(5, 5);
    layer.set(2, 3, 7);
    assert_eq!(layer.get(2, 3), 7);
}

#[test]
fn layer_out_of_bounds_get_is_zero() {
    let layer = TileLayer::new(4, 4);
    assert_eq!(layer.get(10, 10), 0);
}

#[test]
fn layer_out_of_bounds_set_is_noop() {
    let mut layer = TileLayer::new(4, 4);
    layer.set(99, 99, 1); // should not panic
}

#[test]
fn layer_fill_rect() {
    let mut layer = TileLayer::new(10, 10);
    layer.fill_rect(1, 1, 3, 2, 5);
    // Filled cells
    assert_eq!(layer.get(1, 1), 5);
    assert_eq!(layer.get(3, 2), 5);
    // Outside
    assert_eq!(layer.get(0, 0), 0);
    assert_eq!(layer.get(4, 1), 0);
}

#[test]
fn layer_fill_rect_clamps_to_bounds() {
    let mut layer = TileLayer::new(4, 4);
    layer.fill_rect(2, 2, 100, 100, 3); // should not panic
}

// ── TileMap ───────────────────────────────────────────────────────────────────

#[test]
fn map_default_one_layer() {
    let map = TileMap::new(10, 10);
    assert_eq!(map.layers.len(), 1);
}

#[test]
fn map_add_layer() {
    let mut map = TileMap::new(5, 5);
    let idx = map.add_layer();
    assert_eq!(idx, 1);
    assert_eq!(map.layers.len(), 2);
}

#[test]
fn map_set_get() {
    let mut map = TileMap::new(10, 10);
    map.set(3, 4, 0, 9);
    assert_eq!(map.get(3, 4, 0), 9);
}

#[test]
fn map_solid_default_false() {
    let map = TileMap::new(5, 5);
    assert!(!map.is_solid(2, 2));
}

#[test]
fn map_set_solid() {
    let mut map = TileMap::new(5, 5);
    map.set_solid(1, 1, true);
    assert!(map.is_solid(1, 1));
    assert!(!map.is_solid(2, 2));
}

#[test]
fn map_out_of_bounds_is_solid() {
    let map = TileMap::new(5, 5);
    assert!(map.is_solid(100, 100));
}

// ── AABB collision ────────────────────────────────────────────────────────────

#[test]
fn aabb_no_collision_empty_map() {
    let map = TileMap::new(10, 10);
    assert!(!map.aabb_solid(1.0, 1.0, 0.5, 0.5, 1.0));
}

#[test]
fn aabb_collision_with_solid_tile() {
    let mut map = TileMap::new(10, 10);
    map.set_solid(2, 3, true);
    // AABB exactly over tile (2,3) with tile_size=1.0
    assert!(map.aabb_solid(2.0, 3.0, 0.9, 0.9, 1.0));
}

#[test]
fn aabb_no_collision_adjacent_tile() {
    let mut map = TileMap::new(10, 10);
    map.set_solid(5, 5, true);
    // AABB one tile to the left
    assert!(!map.aabb_solid(3.0, 5.0, 0.9, 0.9, 1.0));
}

#[test]
fn aabb_collision_oob_left() {
    let map = TileMap::new(5, 5);
    assert!(map.aabb_solid(-1.0, 1.0, 0.5, 0.5, 1.0));
}
