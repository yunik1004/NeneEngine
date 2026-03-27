use nene::particle::{EmitterConfig, ParticlePool};

// ── ParticlePool ──────────────────────────────────────────────────────────────

#[test]
fn pool_starts_empty() {
    let pool = ParticlePool::new(EmitterConfig::fire());
    assert_eq!(pool.active_count(), 0);
}

#[test]
fn burst_adds_particles() {
    let mut pool = ParticlePool::new(EmitterConfig::fire());
    pool.burst(10, [0.0, 0.0, 0.0]);
    assert_eq!(pool.active_count(), 10);
}

#[test]
fn burst_respects_max_particles() {
    let cfg = EmitterConfig {
        max_particles: 5,
        ..EmitterConfig::sparks()
    };
    let mut pool = ParticlePool::new(cfg);
    pool.burst(20, [0.0; 3]);
    assert!(pool.active_count() <= 5);
}

#[test]
fn particles_die_over_time() {
    let cfg = EmitterConfig {
        lifetime: 0.1,
        lifetime_variance: 0.0,
        emit_rate: 0.0,
        ..EmitterConfig::fire()
    };
    let mut pool = ParticlePool::new(cfg);
    pool.burst(8, [0.0; 3]);
    assert_eq!(pool.active_count(), 8);
    pool.update(0.2, [0.0; 3]); // longer than lifetime
    assert_eq!(pool.active_count(), 0);
}

#[test]
fn clear_removes_all() {
    let mut pool = ParticlePool::new(EmitterConfig::fire());
    pool.burst(20, [0.0; 3]);
    pool.clear();
    assert_eq!(pool.active_count(), 0);
}

#[test]
fn instances_count_matches_active() {
    let mut pool = ParticlePool::new(EmitterConfig::fire());
    pool.burst(5, [0.0; 3]);
    assert_eq!(pool.instances().len(), pool.active_count());
}

#[test]
fn continuous_emission_spawns_particles() {
    let cfg = EmitterConfig {
        emit_rate: 100.0,
        max_particles: 256,
        ..EmitterConfig::fire()
    };
    let mut pool = ParticlePool::new(cfg);
    pool.update(0.1, [0.0; 3]); // should spawn ~10 particles
    assert!(pool.active_count() > 0);
}

#[test]
fn particles_move_after_update() {
    let cfg = EmitterConfig {
        speed: 10.0,
        speed_variance: 0.0,
        gravity: 0.0,
        lifetime: 5.0,
        lifetime_variance: 0.0,
        emit_rate: 0.0,
        spread: 0.0,
        direction: [1.0, 0.0, 0.0],
        ..EmitterConfig::fire()
    };
    let mut pool = ParticlePool::new(cfg);
    pool.burst(1, [0.0; 3]);

    let before = pool.instances()[0].pos_size;
    pool.update(0.1, [0.0; 3]);
    let after = pool.instances()[0].pos_size;

    // Particle must have moved along X
    assert!((after[0] - before[0]).abs() > 0.01);
}

#[test]
fn instance_color_lerps_over_lifetime() {
    let cfg = EmitterConfig {
        lifetime: 1.0,
        lifetime_variance: 0.0,
        emit_rate: 0.0,
        color_start: [1.0, 0.0, 0.0, 1.0],
        color_end: [0.0, 1.0, 0.0, 0.0],
        ..EmitterConfig::fire()
    };
    let mut pool = ParticlePool::new(cfg);
    pool.burst(1, [0.0; 3]);

    // At birth: near color_start
    let inst0 = pool.instances()[0];
    assert!(inst0.color[0] > 0.9); // red channel high at birth

    // Advance 90% of lifetime
    pool.update(0.9, [0.0; 3]);
    let inst1 = pool.instances()[0];
    // red should be much lower, green higher
    assert!(inst1.color[0] < inst0.color[0]);
    assert!(inst1.color[1] > inst0.color[1]);
}

#[test]
fn instance_size_shrinks_over_lifetime() {
    let cfg = EmitterConfig {
        lifetime: 1.0,
        lifetime_variance: 0.0,
        emit_rate: 0.0,
        size_start: 1.0,
        size_end: 0.0,
        gravity: 0.0,
        ..EmitterConfig::fire()
    };
    let mut pool = ParticlePool::new(cfg);
    pool.burst(1, [0.0; 3]);

    let size0 = pool.instances()[0].pos_size[3];
    pool.update(0.5, [0.0; 3]);
    let size1 = pool.instances()[0].pos_size[3];

    assert!(size1 < size0, "size should shrink: {size0} → {size1}");
}

#[test]
fn gravity_pulls_particles_down() {
    let cfg = EmitterConfig {
        gravity: -10.0,
        speed: 0.0,
        speed_variance: 0.0,
        lifetime: 5.0,
        lifetime_variance: 0.0,
        emit_rate: 0.0,
        ..EmitterConfig::fire()
    };
    let mut pool = ParticlePool::new(cfg);
    pool.burst(1, [0.0, 0.0, 0.0]);

    pool.update(0.5, [0.0; 3]);
    let inst = pool.instances()[0];
    assert!(inst.pos_size[1] < 0.0, "gravity should pull Y down");
}

#[test]
fn fire_preset_has_positive_emit_rate() {
    assert!(EmitterConfig::fire().emit_rate > 0.0);
}

#[test]
fn sparks_preset_has_zero_emit_rate() {
    assert_eq!(EmitterConfig::sparks().emit_rate, 0.0);
}
