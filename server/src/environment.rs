// server/src/environment.rs
use spacetimedb::{ReducerContext, Table, Timestamp};
use crate::{WORLD_WIDTH_PX, WORLD_HEIGHT_PX, TILE_SIZE_PX, WORLD_WIDTH_TILES, WORLD_HEIGHT_TILES};

// Import from specific resource modules using qualified paths mostly
use crate::tree;
use crate::stone;

// Import table traits specifically needed for ctx.db access
use crate::tree::tree as TreeTableTrait;
use crate::stone::stone as StoneTableTrait;
use crate::mushroom::mushroom as MushroomTableTrait;

use noise::{NoiseFn, Perlin, Fbm};
use rand::Rng;
use std::collections::HashSet;
use log;

// --- Tree-Specific Constants --- REMOVED - These should not be here ---

// --- Stone-Specific Constants --- REMOVED - These should not be here ---

// --- Tree Enums and Structs --- REMOVED - These should not be here ---

// --- Stone Struct and Table --- REMOVED - These should not be here ---


// --- Environment Seeding ---

// Reducer to seed trees, stones, AND MUSHROOMS if none exist
#[spacetimedb::reducer]
pub fn seed_environment(ctx: &ReducerContext) -> Result<(), String> {
    // Use table traits for clarity
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

    // Calculate targets and limits for trees (using constants from tree module)
    let target_tree_count = (total_tiles as f32 * crate::tree::TREE_DENSITY_PERCENT) as u32;
    let max_tree_attempts = target_tree_count * crate::tree::MAX_TREE_SEEDING_ATTEMPTS_FACTOR;

    // Calculate targets and limits for stones (using constants from stone module)
    let target_stone_count = (total_tiles as f32 * crate::stone::STONE_DENSITY_PERCENT) as u32;
    let max_stone_attempts = target_stone_count * crate::tree::MAX_TREE_SEEDING_ATTEMPTS_FACTOR; // Re-use tree factor intentionally?

    // Calculate targets and limits for mushrooms (using constants from mushroom module)
    let target_mushroom_count = (total_tiles as f32 * crate::mushroom::MUSHROOM_DENSITY_PERCENT) as u32;
    let max_mushroom_attempts = target_mushroom_count * crate::tree::MAX_TREE_SEEDING_ATTEMPTS_FACTOR; // Reuse factor

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

    let min_tile_x = crate::tree::TREE_SPAWN_WORLD_MARGIN_TILES; // Use tree margin for all
    let max_tile_x = crate::WORLD_WIDTH_TILES - crate::tree::TREE_SPAWN_WORLD_MARGIN_TILES;
    let min_tile_y = crate::tree::TREE_SPAWN_WORLD_MARGIN_TILES;
    let max_tile_y = crate::WORLD_HEIGHT_TILES - crate::tree::TREE_SPAWN_WORLD_MARGIN_TILES;

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

        // Generate random tile coordinates within the allowed margin
        let tile_x = rng.gen_range(min_tile_x..max_tile_x);
        let tile_y = rng.gen_range(min_tile_y..max_tile_y);

        if occupied_tiles.contains(&(tile_x, tile_y)) {
            continue; // Skip if tile already occupied
        }

        // Calculate world position (center of tile)
        let pos_x = (tile_x as f32 + 0.5) * TILE_SIZE_PX as f32;
        let pos_y = (tile_y as f32 + 0.5) * TILE_SIZE_PX as f32;

        // Noise check for density filtering
        let noise_val = fbm.get([
            (pos_x as f64 / WORLD_WIDTH_PX as f64) * crate::tree::TREE_SPAWN_NOISE_FREQUENCY,
            (pos_y as f64 / WORLD_HEIGHT_PX as f64) * crate::tree::TREE_SPAWN_NOISE_FREQUENCY,
        ]);
        let normalized_noise = (noise_val + 1.0) / 2.0;

        if normalized_noise > crate::tree::TREE_SPAWN_NOISE_THRESHOLD {
            // Distance check against other trees
            let mut too_close = false;
            for (existing_x, existing_y) in &spawned_tree_positions {
                let dx = pos_x - existing_x;
                let dy = pos_y - existing_y;
                if (dx * dx + dy * dy) < crate::tree::MIN_TREE_DISTANCE_SQ {
                    too_close = true;
                    break;
                }
            }
            if too_close { continue; }

            // Spawn the tree (using Tree struct from tree module)
            match trees.try_insert(crate::tree::Tree {
                id: 0, // Auto-incremented by SpacetimeDB
                pos_x,
                pos_y,
                health: crate::tree::TREE_INITIAL_HEALTH, // Use constant from tree module
                tree_type: crate::tree::TreeType::Oak, // Use enum from tree module
                last_hit_time: None,
                respawn_at: None, // Initialize respawn_at
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

        // Noise check (using the same noise parameters for simplicity - referencing tree module)
        let noise_val = fbm.get([
            (pos_x as f64 / WORLD_WIDTH_PX as f64) * crate::tree::TREE_SPAWN_NOISE_FREQUENCY,
            (pos_y as f64 / WORLD_HEIGHT_PX as f64) * crate::tree::TREE_SPAWN_NOISE_FREQUENCY,
        ]);
        let normalized_noise = (noise_val + 1.0) / 2.0;

        if normalized_noise > crate::tree::TREE_SPAWN_NOISE_THRESHOLD { // Use tree threshold for stones too
            // Distance check against other stones
            let mut too_close_stone = false;
            for (existing_x, existing_y) in &spawned_stone_positions {
                let dx = pos_x - existing_x;
                let dy = pos_y - existing_y;
                if (dx * dx + dy * dy) < crate::stone::MIN_STONE_DISTANCE_SQ { // Use constant from stone module
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
                if (dx * dx + dy * dy) < crate::stone::MIN_STONE_TREE_DISTANCE_SQ { // Use constant from stone module
                    too_close_tree = true;
                    break;
                }
            }
             if too_close_tree { continue; }

            // Spawn the stone (using Stone struct from stone module)
            match stones.try_insert(crate::stone::Stone {
                id: 0, pos_x, pos_y, health: crate::stone::STONE_INITIAL_HEALTH, // Use constant from stone module
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

        // Noise check (using the same noise parameters for simplicity - referencing tree module)
        let noise_val = fbm.get([
            (pos_x as f64 / WORLD_WIDTH_PX as f64) * crate::tree::TREE_SPAWN_NOISE_FREQUENCY,
            (pos_y as f64 / WORLD_HEIGHT_PX as f64) * crate::tree::TREE_SPAWN_NOISE_FREQUENCY,
        ]);
        let normalized_noise = (noise_val + 1.0) / 2.0;

        // Use a slightly *lower* threshold for mushrooms? Let's try 0.65
        if normalized_noise > 0.65 { 
            // Distance check against other mushrooms
            let mut too_close_mushroom = false;
            for (existing_x, existing_y) in &spawned_mushroom_positions {
                let dx = pos_x - existing_x;
                let dy = pos_y - existing_y;
                if (dx * dx + dy * dy) < crate::mushroom::MIN_MUSHROOM_DISTANCE_SQ { // Use constant from mushroom module
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
                if (dx * dx + dy * dy) < crate::mushroom::MIN_MUSHROOM_TREE_DISTANCE_SQ { // Use constant from mushroom module
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
                if (dx * dx + dy * dy) < crate::mushroom::MIN_MUSHROOM_STONE_DISTANCE_SQ { // Use constant from mushroom module
                    too_close_stone = true;
                    break;
                }
            }
            if too_close_stone { continue; }

            // Spawn the mushroom (using Mushroom struct from mushroom module)
            match mushrooms.try_insert(crate::mushroom::Mushroom {
                id: 0, // Auto-inc
                pos_x,
                pos_y,
                respawn_at: None, // Initialize respawn_at
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
// REMOVED TREE_INITIAL_HEALTH and STONE_INITIAL_HEALTH constants (moved to respective modules)

#[spacetimedb::reducer]
pub fn check_resource_respawns(ctx: &ReducerContext) -> Result<(), String> {
    let now_ts = ctx.timestamp;
    let mut stones_to_respawn = Vec::new();
    let mut trees_to_respawn = Vec::new();
    let mut mushrooms_to_respawn = Vec::new();

    // Identify respawnable stones
    for stone in ctx.db.stone().iter().filter(|s| s.health == 0 && s.respawn_at.is_some()) {
        if let Some(respawn_time) = stone.respawn_at {
            if now_ts >= respawn_time {
                stones_to_respawn.push(stone.id);
            }
        }
    }

    // Identify respawnable trees
    for tree in ctx.db.tree().iter().filter(|t| t.health == 0 && t.respawn_at.is_some()) {
        if let Some(respawn_time) = tree.respawn_at {
            if now_ts >= respawn_time {
                trees_to_respawn.push(tree.id);
            }
        }
    }

    // Identify respawnable mushrooms
    for mushroom in ctx.db.mushroom().iter().filter(|m| m.respawn_at.is_some()) {
        if let Some(respawn_time) = mushroom.respawn_at {
            if now_ts >= respawn_time {
                mushrooms_to_respawn.push(mushroom.id);
            }
        }
    }

    // Respawn stones
    for stone_id in stones_to_respawn {
        if let Some(mut stone) = ctx.db.stone().id().find(stone_id) { // Uses Stone from stone module implicitly via ctx.db
            log::info!("Respawning Stone {}", stone_id);
            stone.health = crate::stone::STONE_INITIAL_HEALTH; // Use constant from stone module
            stone.respawn_at = None; // Clear respawn timer
            stone.last_hit_time = None; // Clear last hit time
            ctx.db.stone().id().update(stone);
        } else {
            log::warn!("Could not find Stone {} to respawn.", stone_id);
        }
    }

    // Respawn trees
    for tree_id in trees_to_respawn {
        if let Some(mut tree) = ctx.db.tree().id().find(tree_id) { // Uses Tree from tree module implicitly via ctx.db
            log::info!("Respawning Tree {}", tree_id);
            tree.health = crate::tree::TREE_INITIAL_HEALTH; // Use constant from tree module
            tree.respawn_at = None; // Clear respawn timer
            tree.last_hit_time = None; // Clear last hit time
            ctx.db.tree().id().update(tree);
        } else {
            log::warn!("Could not find Tree {} to respawn.", tree_id);
        }
    }

    // Respawn mushrooms
    for mushroom_id in mushrooms_to_respawn {
        if let Some(mut mushroom) = ctx.db.mushroom().id().find(mushroom_id) { // Uses Mushroom from mushroom module implicitly via ctx.db
            log::info!("Respawning Mushroom {}", mushroom_id);
            mushroom.respawn_at = None; // Clear respawn timer
            ctx.db.mushroom().id().update(mushroom);
        } else {
            log::warn!("Could not find Mushroom {} to respawn.", mushroom_id);
        }
    }

    Ok(())
}
