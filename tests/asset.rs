use nene::asset::Assets;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn write_obj(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    std::fs::write(
        &path,
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nvn 0 0 1\nf 1//1 2//1 3//1\n",
    )
    .unwrap();
    path
}

// ── Cache bookkeeping ─────────────────────────────────────────────────────────

#[test]
fn starts_empty() {
    let assets = Assets::new();
    assert!(assets.is_empty());
    assert_eq!(assets.len(), 0);
}

#[test]
fn model_increments_len() {
    let path = write_obj("nene_asset_len.obj");
    let mut assets = Assets::new();
    assets.model(&path);
    assert_eq!(assets.len(), 1);
}

#[test]
fn loading_same_model_twice_counts_once() {
    let path = write_obj("nene_asset_dedup.obj");
    let mut assets = Assets::new();
    assets.model(&path);
    assets.model(&path);
    assert_eq!(assets.len(), 1);
}

#[test]
fn model_cached_after_first_load() {
    let path = write_obj("nene_asset_cache.obj");
    let mut assets = Assets::new();
    let h1 = assets.model(&path);
    let h2 = assets.model(&path);
    assert!(
        h1 == h2,
        "second load should return the same Arc allocation"
    );
}

#[test]
fn different_paths_are_different_handles() {
    let a = write_obj("nene_asset_diff_a.obj");
    let b = write_obj("nene_asset_diff_b.obj");
    let mut assets = Assets::new();
    let ha = assets.model(&a);
    let hb = assets.model(&b);
    assert!(ha != hb);
}

#[test]
fn clear_empties_cache() {
    let path = write_obj("nene_asset_clear.obj");
    let mut assets = Assets::new();
    assets.model(&path);
    assert_eq!(assets.len(), 1);
    assets.clear();
    assert_eq!(assets.len(), 0);
    assert!(assets.is_empty());
}

#[test]
fn evict_removes_only_target() {
    let a = write_obj("nene_asset_evict_a.obj");
    let b = write_obj("nene_asset_evict_b.obj");
    let mut assets = Assets::new();
    assets.model(&a);
    assets.model(&b);
    assert_eq!(assets.len(), 2);

    assets.evict_model(&a);
    assert_eq!(assets.len(), 1);
}

#[test]
fn evict_then_reload_gives_new_handle() {
    let path = write_obj("nene_asset_evict_reload.obj");
    let mut assets = Assets::new();

    let h1 = assets.model(&path);
    assets.evict_model(&path);
    let h2 = assets.model(&path);

    assert!(
        h1 != h2,
        "eviction + reload should produce a new allocation"
    );
}

// ── Handle survival ───────────────────────────────────────────────────────────

#[test]
fn handle_survives_cache_clear() {
    let path = write_obj("nene_asset_survive.obj");
    let mut assets = Assets::new();

    let handle = assets.model(&path);
    assets.clear();

    // The Arc inside is still alive — deref must not panic.
    let _ = handle.meshes.len();
}

#[test]
fn handle_clone_equals_original() {
    let path = write_obj("nene_asset_clone.obj");
    let mut assets = Assets::new();
    let h = assets.model(&path);
    let h2 = h.clone();
    assert!(h == h2);
}

#[test]
fn handle_deref_accesses_inner() {
    let path = write_obj("nene_asset_deref.obj");
    let mut assets = Assets::new();
    let handle = assets.model(&path);
    // Deref to Model — should have at least 1 mesh
    assert!(!handle.meshes.is_empty());
}
