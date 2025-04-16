// server/src/environment.rs
use spacetimedb::{ReducerContext, SpacetimeType, Table, Timestamp};
use crate::{WORLD_WIDTH_PX, WORLD_HEIGHT_PX, TILE_SIZE_PX, PLAYER_RADIUS}; // Removed WORLD_WIDTH_TILES, WORLD_HEIGHT_TILES
use crate::mushroom::{
    MUSHROOM_DENSITY_PERCENT,
    MIN_MUSHROOM_DISTANCE_SQ,
    MIN_MUSHROOM_TREE_DISTANCE_SQ,
    MIN_MUSHROOM_STONE_DISTANCE_SQ
};
use crate::mushroom::mushroom as MushroomTableTrait;
use noise::{NoiseFn, Perlin, Fbm};
use rand::Rng;
use std::collections::HashSet;
use log;

// --- Tree-Specific Constants ---

// Tree Collision settings
pub(crate) const TREE_TRUNK_RADIUS: f32 = 30.0; // Reduced radius for trunk base (was 20.0)
pub(crate) const TREE_COLLISION_Y_OFFSET: f32 = 20.0; // Offset the collision check upwards from the root
pub(crate) const PLAYER_TREE_COLLISION_DISTANCE_SQUARED: f32 = (PLAYER_RADIUS + TREE_TRUNK_RADIUS) * (PLAYER_RADIUS + TREE_TRUNK_RADIUS);

// Tree Spawning Parameters
const TREE_DENSITY_PERCENT: f32 = 0.01; // Target 1% of map tiles (was 0.05)
const TREE_SPAWN_NOISE_FREQUENCY: f64 = 8.0; // Keep noise frequency moderate for filtering
const TREE_SPAWN_NOISE_THRESHOLD: f64 = 0.7; // Increased threshold significantly (was 0.55)
const TREE_SPAWN_WORLD_MARGIN_TILES: u32 = 3; // Don't spawn in the outer 3 tiles (margin in tiles)
const MAX_TREE_SEEDING_ATTEMPTS_FACTOR: u32 = 5; // Try up to 5x the target number of trees
const MIN_TREE_DISTANCE_PX: f32 = 200.0; // Minimum distance between tree centers
const MIN_TREE_DISTANCE_SQ: f32 = MIN_TREE_DISTANCE_PX * MIN_TREE_DISTANCE_PX; // Squared for comparison

// --- Stone-Specific Constants ---
pub(crate) const STONE_RADIUS: f32 = 40.0; // Collision radius for stone nodes
pub(crate) const PLAYER_STONE_COLLISION_DISTANCE_SQUARED: f32 = (PLAYER_RADIUS + STONE_RADIUS) * (PLAYER_RADIUS + STONE_RADIUS);
pub(crate) const STONE_COLLISION_Y_OFFSET: f32 = 50.0; // Offset the collision check upwards from the root (reduced)
const STONE_DENSITY_PERCENT: f32 = TREE_DENSITY_PERCENT / 5.0; // Make stones 2.5x less populous than original (1/5th of trees)
const MIN_STONE_DISTANCE_PX: f32 = 150.0; // Minimum distance between stone centers
const MIN_STONE_DISTANCE_SQ: f32 = MIN_STONE_DISTANCE_PX * MIN_STONE_DISTANCE_PX;
const MIN_STONE_TREE_DISTANCE_PX: f32 = 100.0; // Min distance between a stone and a tree
const MIN_STONE_TREE_DISTANCE_SQ: f32 = MIN_STONE_TREE_DISTANCE_PX * MIN_STONE_TREE_DISTANCE_PX;

// --- Tree Enums and Structs ---

// Define the different types of trees
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, SpacetimeType)]
pub enum TreeType {
    Oak, // Represents tree.png
}

#[spacetimedb::table(name = tree, public)]
#[derive(Clone)]
pub struct Tree {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub pos_x: f32,
    pub pos_y: f32,
    pub health: u32,
    pub tree_type: TreeType,
    pub last_hit_time: Option<Timestamp>,
}

// --- Stone Struct and Table ---
#[spacetimedb::table(name = stone, public)]
#[derive(Clone)]
pub struct Stone {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub pos_x: f32,
    pub pos_y: f32,
    pub health: u32, // Stones just disappear when health is 0
    pub last_hit_time: Option<Timestamp>, // Added for shake effect
    pub respawn_at: Option<Timestamp>, // Added for respawn timer
}

// --- Environment Seeding ---

// Reducer to seed trees, stones, AND MUSHROOMS if none exist
#[spacetimedb::reducer]
pub fn seed_environment(ctx: &ReducerContext) -> Result<(), String> {
    let trees = ctx.db.tree();
    let stones = ctx.db.stone();
    let mushrooms = ctx.db.mushroom(); // Get mushroom table

    // Check if ALL tables are empty before seeding
    if trees.iter().count() > 0 || stones.iter().count() > 0 || mushrooms.iter().count() > 0 {
        log::info!(
            "Environment already seeded (Trees: {}, Stones: {}, Mushrooms: {}). Skipping.",
            trees.iter().count(),
            stones.iter().count(),
            mushrooms.iter().count()
        );
        return Ok(());
    }

    log::info!("Seeding environment (trees, stones, mushrooms)..." );

    let fbm = Fbm::<Perlin>::new(ctx.rng().gen());
    let mut rng = ctx.rng();

    let total_tiles = crate::WORLD_WIDTH_TILES * crate::WORLD_HEIGHT_TILES;

    // Calculate targets and limits for trees
    let target_tree_count = (total_tiles as f32 * TREE_DENSITY_PERCENT) as u32;
    let max_tree_attempts = target_tree_count * MAX_TREE_SEEDING_ATTEMPTS_FACTOR;

    // Calculate targets and limits for stones
    let target_stone_count = (total_tiles as f32 * STONE_DENSITY_PERCENT) as u32;
    let max_stone_attempts = target_stone_count * MAX_TREE_SEEDING_ATTEMPTS_FACTOR;

    // Calculate targets and limits for mushrooms
    let target_mushroom_count = (total_tiles as f32 * MUSHROOM_DENSITY_PERCENT) as u32;
    let max_mushroom_attempts = target_mushroom_count * MAX_TREE_SEEDING_ATTEMPTS_FACTOR; // Reuse factor

    log::info!(
        "Target Trees: {}, Max Attempts: {}",
        target_tree_count, max_tree_attempts
    );
     log::info!(
        "Target Stones: {}, Max Attempts: {}",
        target_stone_count, max_stone_attempts
    );
    log::info!(
        "Target Mushrooms: {}, Max Attempts: {}",
        target_mushroom_count, max_mushroom_attempts
    );

    let min_tile_x = TREE_SPAWN_WORLD_MARGIN_TILES; // Use same margin for all
    let max_tile_x = crate::WORLD_WIDTH_TILES - TREE_SPAWN_WORLD_MARGIN_TILES;
    let min_tile_y = TREE_SPAWN_WORLD_MARGIN_TILES;
    let max_tile_y = crate::WORLD_HEIGHT_TILES - TREE_SPAWN_WORLD_MARGIN_TILES;

    let mut spawned_tree_count = 0;
    let mut spawned_stone_count = 0;
    let mut spawned_mushroom_count = 0;
    let mut tree_attempts = 0;
    let mut stone_attempts = 0;
    let mut mushroom_attempts = 0;
    let mut occupied_tiles = HashSet::<(u32, u32)>::new();
    let mut spawned_tree_positions = Vec::<(f32, f32)>::new();
    let mut spawned_stone_positions = Vec::<(f32, f32)>::new();
    let mut spawned_mushroom_positions = Vec::<(f32, f32)>::new(); // Track mushroom positions

    // --- Seed Trees ---
    log::info!("Seeding Trees...");
    while spawned_tree_count < target_tree_count && tree_attempts < max_tree_attempts {
        tree_attempts += 1;
        let tile_x = rng.gen_range(min_tile_x..max_tile_x);
        let tile_y = rng.gen_range(min_tile_y..max_tile_y);
        if occupied_tiles.contains(&(tile_x, tile_y)) {
            continue;
        }

        let pos_x = (tile_x as f32 + 0.5) * TILE_SIZE_PX as f32;
        let pos_y = (tile_y as f32 + 0.5) * TILE_SIZE_PX as f32;

        // Noise check
        let noise_val = fbm.get([
            (pos_x as f64 / WORLD_WIDTH_PX as f64) * TREE_SPAWN_NOISE_FREQUENCY,
            (pos_y as f64 / WORLD_HEIGHT_PX as f64) * TREE_SPAWN_NOISE_FREQUENCY,
        ]);
        let normalized_noise = (noise_val + 1.0) / 2.0;

        if normalized_noise > TREE_SPAWN_NOISE_THRESHOLD { // Use tree threshold for trees
            // Distance check against other trees
            let mut too_close_tree = false;
            for (existing_x, existing_y) in &spawned_tree_positions {
                let dx = pos_x - existing_x;
                let dy = pos_y - existing_y;
                if (dx * dx + dy * dy) < MIN_TREE_DISTANCE_SQ {
                    too_close_tree = true;
                    break;
                }
            }
            if too_close_tree { continue; }

            // Spawn the tree
            let tree_type = TreeType::Oak;
            match trees.try_insert(Tree {
                id: 0, pos_x, pos_y, health: 100, tree_type,
                last_hit_time: None,
            }) {
                Ok(_) => {
                    spawned_tree_count += 1;
                    occupied_tiles.insert((tile_x, tile_y));
                    spawned_tree_positions.push((pos_x, pos_y));
                }
                Err(e) => log::error!("Failed to insert tree during seeding: {}", e),
            }
        }
    }
     log::info!(
        "Finished seeding {} trees (target: {}, attempts: {}).",
        spawned_tree_count, target_tree_count, tree_attempts
    );

    // --- Seed Stones ---
     log::info!("Seeding Stones...");
    while spawned_stone_count < target_stone_count && stone_attempts < max_stone_attempts {
        stone_attempts += 1;

        let tile_x = rng.gen_range(min_tile_x..max_tile_x);
        let tile_y = rng.gen_range(min_tile_y..max_tile_y);
        if occupied_tiles.contains(&(tile_x, tile_y)) {
            continue;
        }

        let pos_x = (tile_x as f32 + 0.5) * TILE_SIZE_PX as f32;
        let pos_y = (tile_y as f32 + 0.5) * TILE_SIZE_PX as f32;

        // Noise check (using the same noise parameters for simplicity)
        let noise_val = fbm.get([
            (pos_x as f64 / WORLD_WIDTH_PX as f64) * TREE_SPAWN_NOISE_FREQUENCY,
            (pos_y as f64 / WORLD_HEIGHT_PX as f64) * TREE_SPAWN_NOISE_FREQUENCY,
        ]);
        let normalized_noise = (noise_val + 1.0) / 2.0;

        if normalized_noise > TREE_SPAWN_NOISE_THRESHOLD { // Use tree threshold for stones too
            // Distance check against other stones
            let mut too_close_stone = false;
            for (existing_x, existing_y) in &spawned_stone_positions {
                let dx = pos_x - existing_x;
                let dy = pos_y - existing_y;
                if (dx * dx + dy * dy) < MIN_STONE_DISTANCE_SQ {
                    too_close_stone = true;
                    break;
                }
            }
            if too_close_stone { continue; }

            // Distance check against existing trees
            let mut too_close_tree = false;
            for (existing_x, existing_y) in &spawned_tree_positions {
                let dx = pos_x - existing_x;
                let dy = pos_y - existing_y;
                if (dx * dx + dy * dy) < MIN_STONE_TREE_DISTANCE_SQ {
                    too_close_tree = true;
                    break;
                }
            }
             if too_close_tree { continue; }

            // Spawn the stone
            match stones.try_insert(Stone {
                id: 0, pos_x, pos_y, health: 100,
                last_hit_time: None,
                respawn_at: None,
            }) {
                Ok(_) => {
                    spawned_stone_count += 1;
                    occupied_tiles.insert((tile_x, tile_y));
                    spawned_stone_positions.push((pos_x, pos_y));
                }
                Err(e) => log::error!("Failed to insert stone during seeding: {}", e),
            }
        }
    }

    log::info!(
        "Finished seeding {} stones (target: {}, attempts: {}).",
        spawned_stone_count, target_stone_count, stone_attempts
    );

    // --- Seed Mushrooms --- 
    log::info!("Seeding Mushrooms...");
    while spawned_mushroom_count < target_mushroom_count && mushroom_attempts < max_mushroom_attempts {
        mushroom_attempts += 1;

        let tile_x = rng.gen_range(min_tile_x..max_tile_x);
        let tile_y = rng.gen_range(min_tile_y..max_tile_y);
        if occupied_tiles.contains(&(tile_x, tile_y)) {
            continue;
        }

        let pos_x = (tile_x as f32 + 0.5) * TILE_SIZE_PX as f32;
        let pos_y = (tile_y as f32 + 0.5) * TILE_SIZE_PX as f32;

        // Noise check (using the same noise parameters for simplicity)
        let noise_val = fbm.get([
            (pos_x as f64 / WORLD_WIDTH_PX as f64) * TREE_SPAWN_NOISE_FREQUENCY,
            (pos_y as f64 / WORLD_HEIGHT_PX as f64) * TREE_SPAWN_NOISE_FREQUENCY,
        ]);
        let normalized_noise = (noise_val + 1.0) / 2.0;

        // Use a slightly *lower* threshold for mushrooms? Let's try 0.65
        if normalized_noise > 0.65 { 
            // Distance check against other mushrooms
            let mut too_close_mushroom = false;
            for (existing_x, existing_y) in &spawned_mushroom_positions {
                let dx = pos_x - existing_x;
                let dy = pos_y - existing_y;
                if (dx * dx + dy * dy) < MIN_MUSHROOM_DISTANCE_SQ {
                    too_close_mushroom = true;
                    break;
                }
            }
            if too_close_mushroom { continue; }

            // Distance check against existing trees
            let mut too_close_tree = false;
            for (existing_x, existing_y) in &spawned_tree_positions {
                let dx = pos_x - existing_x;
                let dy = pos_y - existing_y;
                if (dx * dx + dy * dy) < MIN_MUSHROOM_TREE_DISTANCE_SQ {
                    too_close_tree = true;
                    break;
                }
            }
             if too_close_tree { continue; }
            
            // Distance check against existing stones
            let mut too_close_stone = false;
            for (existing_x, existing_y) in &spawned_stone_positions {
                let dx = pos_x - existing_x;
                let dy = pos_y - existing_y;
                if (dx * dx + dy * dy) < MIN_MUSHROOM_STONE_DISTANCE_SQ {
                    too_close_stone = true;
                    break;
                }
            }
            if too_close_stone { continue; }

            // Spawn the mushroom
            match mushrooms.try_insert(crate::mushroom::Mushroom {
                id: 0, // Auto-inc
                pos_x,
                pos_y,
            }) {
                Ok(_) => {
                    spawned_mushroom_count += 1;
                    occupied_tiles.insert((tile_x, tile_y)); // Mark tile as occupied
                    spawned_mushroom_positions.push((pos_x, pos_y));
                }
                Err(e) => log::error!("Failed to insert mushroom during seeding: {}", e),
            }
        }
    }
    log::info!(
        "Finished seeding {} mushrooms (target: {}, attempts: {}).",
        spawned_mushroom_count, target_mushroom_count, mushroom_attempts
    );

    log::info!("Environment seeding complete.");
    Ok(())
}

// --- Resource Respawn Reducer ---
const TREE_INITIAL_HEALTH: u32 = 100;
const STONE_INITIAL_HEALTH: u32 = 100;

#[spacetimedb::reducer]
pub fn check_resource_respawns(ctx: &ReducerContext) -> Result<(), String> {
    let now_ts = ctx.timestamp;
    let mut stones_to_respawn = Vec::new();

    // Identify respawnable stones
    for stone in ctx.db.stone().iter().filter(|s| s.health == 0 && s.respawn_at.is_some()) {
        if let Some(respawn_time) = stone.respawn_at {
            if now_ts >= respawn_time {
                stones_to_respawn.push(stone.id);
            }
        }
    }

    // Respawn stones
    for stone_id in stones_to_respawn {
        if let Some(mut stone) = ctx.db.stone().id().find(stone_id) {
            log::info!("Respawning Stone {}", stone_id);
            stone.health = STONE_INITIAL_HEALTH;
            stone.respawn_at = None; // Clear respawn timer
            stone.last_hit_time = None; // Clear last hit time
            ctx.db.stone().id().update(stone);
        } else {
            log::warn!("Could not find Stone {} to respawn.", stone_id);
        }
    }

    Ok(())
}
