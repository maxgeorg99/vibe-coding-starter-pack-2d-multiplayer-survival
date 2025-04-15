use spacetimedb::{Identity, Timestamp, ReducerContext, Table, SpacetimeType};
use log;
use rand::Rng; // Import for random tree type
use noise::{NoiseFn, Perlin, Seedable, Fbm}; // Import noise functions, Seedable, and Fbm
use std::collections::HashSet; // For tracking spawned locations

// World dimensions (in tiles)
const WORLD_WIDTH_TILES: u32 = 100;
const WORLD_HEIGHT_TILES: u32 = 100;
// Tile settings
const TILE_SIZE_PX: u32 = 48;

// Calculated world dimensions (in pixels)
const WORLD_WIDTH_PX: f32 = (WORLD_WIDTH_TILES * TILE_SIZE_PX) as f32;
const WORLD_HEIGHT_PX: f32 = (WORLD_HEIGHT_TILES * TILE_SIZE_PX) as f32;

const PLAYER_RADIUS: f32 = 24.0;
const PLAYER_DIAMETER_SQUARED: f32 = (PLAYER_RADIUS * 2.0) * (PLAYER_RADIUS * 2.0);

// Tree Collision settings
const TREE_TRUNK_RADIUS: f32 = 12.0; // Reduced radius for trunk base (was 20.0)
const TREE_COLLISION_Y_OFFSET: f32 = 20.0; // Offset the collision check upwards from the root
const PLAYER_TREE_COLLISION_DISTANCE_SQUARED: f32 = (PLAYER_RADIUS + TREE_TRUNK_RADIUS) * (PLAYER_RADIUS + TREE_TRUNK_RADIUS);

// Passive Stat Drain Rates
const HUNGER_DRAIN_PER_SECOND: f32 = 100.0 / (30.0 * 60.0); // 100 hunger over 30 mins
const THIRST_DRAIN_PER_SECOND: f32 = 100.0 / (20.0 * 60.0); // 100 thirst over 20 mins
const STAMINA_DRAIN_PER_SECOND: f32 = 20.0; // Stamina drains fast while sprinting
const STAMINA_RECOVERY_PER_SECOND: f32 = 5.0;  // Stamina recovers slower
const SPRINT_SPEED_MULTIPLIER: f32 = 1.5;     // 50% faster sprint

// Tree Spawning Parameters
const TREE_DENSITY_PERCENT: f32 = 0.01; // Target 1% of map tiles (was 0.05)
const TREE_SPAWN_NOISE_FREQUENCY: f64 = 8.0; // Keep noise frequency moderate for filtering
const TREE_SPAWN_NOISE_THRESHOLD: f64 = 0.7; // Increased threshold significantly (was 0.55)
const TREE_SPAWN_WORLD_MARGIN_TILES: u32 = 3; // Don't spawn in the outer 3 tiles (margin in tiles)
const MAX_TREE_SEEDING_ATTEMPTS_FACTOR: u32 = 5; // Try up to 5x the target number of trees
const MIN_TREE_DISTANCE_PX: f32 = 200.0; // Minimum distance between tree centers
const MIN_TREE_DISTANCE_SQ: f32 = MIN_TREE_DISTANCE_PX * MIN_TREE_DISTANCE_PX; // Squared for comparison

// Define the different types of trees
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, SpacetimeType)]
pub enum TreeType {
    Oak, // Represents tree.png
    // Pine, // Represents tree2.png - REMOVED
}

// Define the state of the tree
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, SpacetimeType)]
pub enum TreeState {
    Growing,
    Stump,
}

// Player table to store position and color
#[spacetimedb::table(name = player, public)]
#[derive(Clone)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
    pub username: String,
    pub position_x: f32,
    pub position_y: f32,
    pub color: String, // Hex color code (e.g., "#FF0000" for red)
    pub direction: String, // "up", "down", "left", "right"
    pub last_update: Timestamp,
    pub jump_start_time_ms: u64, // Timestamp when the jump started (0 if not jumping)
    // New status fields
    pub health: f32,
    pub stamina: f32,
    pub thirst: f32,
    pub hunger: f32,
    pub warmth: f32,
    pub is_sprinting: bool, // Is the player currently trying to sprint?
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
    pub state: TreeState,
    // pub respawn_at: u64, // We can add this later for respawning logic
}

// When a client connects, we need to create a player for them
#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) -> Result<(), String> {
    // Ensure trees are seeded when the first player connects (or on server start implicitly)
    // Note: This might run multiple times if players connect before the first check completes,
    // but the `seed_trees` function itself prevents reseeding.
    seed_trees(ctx)?;

    // Player creation is handled elsewhere

    Ok(())
}

// When a client disconnects, we need to clean up
#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    log::info!("identity_disconnected triggered for identity: {:?}", ctx.sender);
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    
    if let Some(player) = players.identity().find(sender_id) {
        let username = player.username.clone();
        players.identity().delete(sender_id);
        log::info!("Attempted delete for disconnected player: {} ({:?})", username, sender_id);
    }
}

// Register a new player
#[spacetimedb::reducer]
pub fn register_player(ctx: &ReducerContext, username: String) -> Result<(), String> {
    log::info!("register_player called by {:?} with username: {}", ctx.sender, username);
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let trees = ctx.db.tree(); // Get tree table instance
    
    // Check if username is already taken by *any* player
    let username_taken = players.iter().any(|p| p.username == username);
    if username_taken {
        log::warn!("Username '{}' already taken. Registration failed for {:?}.", username, sender_id);
        return Err(format!("Username '{}' is already taken.", username));
    }
    
    // Check if this identity is already registered (shouldn't happen if disconnect works, but good safety check)
    if players.identity().find(sender_id).is_some() {
        log::warn!("Identity {:?} already registered. Registration failed.", sender_id);
        return Err("Player identity already registered".to_string());
    }
    
    // --- Find a valid spawn position --- 
    let initial_x = 640.0; 
    let initial_y = 480.0;
    let mut spawn_x = initial_x;
    let mut spawn_y = initial_y;
    let max_attempts = 10; // Prevent infinite loops
    let offset_step = PLAYER_RADIUS * 2.5; // How far to step when looking for space
    let mut attempt = 0;

    loop {
        let mut collision = false;

        // 1. Check Player-Player Collision
        for other_player in players.iter() {
            let dx = spawn_x - other_player.position_x;
            let dy = spawn_y - other_player.position_y;
            if (dx * dx + dy * dy) < PLAYER_DIAMETER_SQUARED {
                collision = true;
                break;
            }
        }

        // 2. Check Player-Tree Collision (if no player collision)
        if !collision {
            for tree in trees.iter() {
                let dx = spawn_x - tree.pos_x;
                let dy = spawn_y - (tree.pos_y - TREE_COLLISION_Y_OFFSET); // Use offset
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < PLAYER_TREE_COLLISION_DISTANCE_SQUARED {
                    collision = true;
                    break;
                }
            }
        }

        // 3. Decide if position is valid or max attempts reached
        if !collision || attempt >= max_attempts {
            if attempt >= max_attempts && collision { // Check if we hit max attempts AND were colliding
                 log::warn!("Could not find clear spawn point for {}, spawning at default (may collide).", username);
                 spawn_x = initial_x;
                 spawn_y = initial_y;
            }
            break; // Found a spot or gave up
        }

        // Simple offset pattern: move right, down, left, up, then spiral out slightly?
        // This is basic, could be improved (random, spiral search)
        match attempt % 4 {
            0 => spawn_x += offset_step, // Try right
            1 => spawn_y += offset_step, // Try down
            2 => spawn_x -= offset_step * 2.0, // Try left (further)
            3 => spawn_y -= offset_step * 2.0, // Try up (further)
            _ => {}, // Should not happen
        }
        // Reset to center if offset gets too wild after a few attempts (basic safeguard)
        if attempt == 5 { 
             spawn_x = initial_x;
             spawn_y = initial_y;
             spawn_x += offset_step * 1.5; // Try a different diagonal
             spawn_y += offset_step * 1.5;
        }

        attempt += 1;
    }
    // --- End spawn position logic ---

    let color = random_color(&username);
    
    let player = Player {
        identity: sender_id,
        username: username.clone(), 
        position_x: spawn_x, // Use the found spawn position
        position_y: spawn_y, // Use the found spawn position
        color,
        direction: "down".to_string(), 
        last_update: ctx.timestamp,
        jump_start_time_ms: 0, // Initialize as not jumping
        // Initialize new status fields
        health: 100.0,
        stamina: 100.0,
        thirst: 100.0,
        hunger: 100.0,
        warmth: 100.0,
        is_sprinting: false, // Initialize sprint state
    };
    
    // Insert the new player
    match players.try_insert(player) {
        Ok(_) => {
            log::info!("Player registered: {}", username);
            Ok(())
        },
        Err(e) => {
            Err(format!("Failed to register player: {}", e))
        }
    }
}

// NEW Reducer: Called by the client to set the sprinting state
#[spacetimedb::reducer]
pub fn set_sprinting(ctx: &ReducerContext, sprinting: bool) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();

    if let Some(mut player) = players.identity().find(&sender_id) {
        // Only update if the state is actually changing
        if player.is_sprinting != sprinting {
            player.is_sprinting = sprinting;
            // Also update last_update time so stamina regen/drain calculation is accurate on next movement
            player.last_update = ctx.timestamp; 
            players.identity().update(player);
            log::debug!("Player {:?} set sprinting to {}", sender_id, sprinting);
        }
        Ok(())
    } else {
        Err("Player not found".to_string())
    }
}

// Update player movement, handle sprinting, stats, and collision
#[spacetimedb::reducer]
pub fn update_player_position(
    ctx: &ReducerContext, 
    move_dx: f32, // Delta X requested by client (based on base speed)
    move_dy: f32  // Delta Y requested by client (based on base speed)
) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let trees = ctx.db.tree(); // Get tree table

    let current_player = players.identity()
        .find(sender_id)
        .ok_or_else(|| "Player not found".to_string())?;

    let now = ctx.timestamp;
    let last_update_time = current_player.last_update;
    let elapsed_micros = now.to_micros_since_unix_epoch().saturating_sub(last_update_time.to_micros_since_unix_epoch());
    let elapsed_seconds = (elapsed_micros as f64 / 1_000_000.0) as f32;

    // --- Calculate Stat Changes --- 
    let new_hunger = (current_player.hunger - (elapsed_seconds * HUNGER_DRAIN_PER_SECOND)).max(0.0);
    let new_thirst = (current_player.thirst - (elapsed_seconds * THIRST_DRAIN_PER_SECOND)).max(0.0);
    
    let mut new_stamina = current_player.stamina;
    let mut actual_speed_multiplier = 1.0;
    let mut current_sprinting_state = current_player.is_sprinting;
    let is_moving = move_dx != 0.0 || move_dy != 0.0;

    if current_sprinting_state && is_moving && new_stamina > 0.0 {
        // Drain stamina if sprinting, moving, and has stamina
        new_stamina = (new_stamina - (elapsed_seconds * STAMINA_DRAIN_PER_SECOND)).max(0.0);
        if new_stamina > 0.0 { // Can still sprint
            actual_speed_multiplier = SPRINT_SPEED_MULTIPLIER;
        } else { // Ran out of stamina
            current_sprinting_state = false; // Force stop sprinting this update cycle
            log::debug!("Player {:?} ran out of stamina.", sender_id);
        }
    } else if !current_sprinting_state {
        // Recover stamina if not trying to sprint
        new_stamina = (new_stamina + (elapsed_seconds * STAMINA_RECOVERY_PER_SECOND)).min(100.0);
    }
    // Note: If player stops moving while holding sprint, stamina doesn't drain but also doesn't recover here.
    // It will recover on the next update where is_sprinting is false or is_moving is false.

    // --- Calculate Target Position --- 
    let proposed_x = current_player.position_x + move_dx * actual_speed_multiplier;
    let proposed_y = current_player.position_y + move_dy * actual_speed_multiplier;

    // --- Clamp Target Position to World Boundaries --- 
    let clamped_x = proposed_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
    let clamped_y = proposed_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);

    // --- Collision Detection (using clamped position) --- 
    let mut final_x = clamped_x;
    let mut final_y = clamped_y;
    let mut collided = false; // Flag to check if any collision occurred

    // 1. Player-Player Collision
    for other_player in players.iter() {
        if other_player.identity == sender_id {
            continue;
        }
        let dx = clamped_x - other_player.position_x;
        let dy = clamped_y - other_player.position_y;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq < PLAYER_DIAMETER_SQUARED {
            final_x = current_player.position_x;
            final_y = current_player.position_y;
            collided = true;
            log::debug!("Player-Player collision detected between {:?} and {:?}. Movement reverted.", sender_id, other_player.identity);
            break; // Exit player loop if collision found
        }
    }

    // 2. Player-Tree Collision (only if no player collision occurred)
    if !collided {
        for tree in trees.iter() {
            // Collide with both growing trees and stumps
            let dx = clamped_x - tree.pos_x;
            // Check against the offset Y position for collision
            let dy = clamped_y - (tree.pos_y - TREE_COLLISION_Y_OFFSET);
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < PLAYER_TREE_COLLISION_DISTANCE_SQUARED {
                final_x = current_player.position_x;
                final_y = current_player.position_y;
                collided = true; // Set flag
                log::debug!("Player-Tree collision detected between {:?} and tree {}. Movement reverted.", sender_id, tree.id);
                break; // Exit tree loop if collision found
            }
        }
    }

    // --- Determine Update & Final State --- 
    let actual_dx = final_x - current_player.position_x;
    let actual_dy = final_y - current_player.position_y;
    let position_changed = actual_dx != 0.0 || actual_dy != 0.0;
    // Update if position changed, or if enough time passed for passive drains/regen
    let should_update = position_changed || elapsed_seconds > 0.1;

    if should_update {
        let mut direction = current_player.direction.clone();
        if position_changed { // Only update direction if actual movement occurred
            if actual_dx.abs() > actual_dy.abs() { 
                if actual_dx > 0.0 { direction = "right".to_string(); }
                else if actual_dx < 0.0 { direction = "left".to_string(); }
            } else if actual_dy.abs() > actual_dx.abs() { 
                if actual_dy > 0.0 { direction = "down".to_string(); }
                else if actual_dy < 0.0 { direction = "up".to_string(); }
            } 
        }

        let updated_player = Player {
            position_x: final_x, // Use the final clamped/collision-adjusted position
            position_y: final_y, // Use the final clamped/collision-adjusted position
            direction,
            last_update: now,
            hunger: new_hunger,
            thirst: new_thirst,
            stamina: new_stamina,
            is_sprinting: current_sprinting_state,
            ..current_player
        };
        
        players.identity().update(updated_player);
    } 

    Ok(())
}

// Helper function to generate a deterministic color based on username
fn random_color(username: &str) -> String {
    let colors = [
        "#FF0000", // Red
        "#00FF00", // Green
        "#0000FF", // Blue
        "#FFFF00", // Yellow
        "#FF00FF", // Magenta
        "#00FFFF", // Cyan
        "#FF8000", // Orange
        "#8000FF", // Purple
    ];
    
    // Use the username bytes to select a color
    // Sum the bytes and take modulo of the number of colors
    let username_bytes = username.as_bytes();
    let sum_of_bytes: u64 = username_bytes.iter().map(|&byte| byte as u64).sum();
    let color_index = (sum_of_bytes % colors.len() as u64) as usize;
    
    colors[color_index].to_string()
}

// Reducer called by the client to initiate a jump.
#[spacetimedb::reducer]
pub fn jump(ctx: &ReducerContext) -> Result<(), String> {
   let identity = ctx.sender;
   let players = ctx.db.player();

   if let Some(mut player) = players.identity().find(&identity) {
       // Get microseconds since epoch and convert to milliseconds u64
       let now_micros = ctx.timestamp.to_micros_since_unix_epoch();
       let now_ms = (now_micros / 1000) as u64;

       player.jump_start_time_ms = now_ms;
       player.last_update = ctx.timestamp;

       players.identity().update(player);
       Ok(())
   } else {
       Err("Player not found".to_string())
   }
}

// Reducer to seed trees if none exist based on density and noise filter
#[spacetimedb::reducer]
pub fn seed_trees(ctx: &ReducerContext) -> Result<(), String> {
    let trees = ctx.db.tree();
    if trees.iter().count() > 0 {
        return Ok(());
    }

    log::info!("Seeding trees based on density, noise filter, and min distance...");

    let fbm = Fbm::<Perlin>::new(ctx.rng().gen());
    let mut rng = ctx.rng();

    // Calculate target number of trees
    let total_tiles = WORLD_WIDTH_TILES * WORLD_HEIGHT_TILES;
    let target_tree_count = (total_tiles as f32 * TREE_DENSITY_PERCENT) as u32;
    let max_attempts = target_tree_count * MAX_TREE_SEEDING_ATTEMPTS_FACTOR;

    log::info!(
        "Calculated tree spawning parameters: Total Tiles={}, Target Density={:.2}%, Target Count={}, Max Attempts={}",
        total_tiles, TREE_DENSITY_PERCENT * 100.0, target_tree_count, max_attempts
    );

    // Define spawn boundaries in tile coordinates
    let min_tile_x = TREE_SPAWN_WORLD_MARGIN_TILES;
    let max_tile_x = WORLD_WIDTH_TILES - TREE_SPAWN_WORLD_MARGIN_TILES;
    let min_tile_y = TREE_SPAWN_WORLD_MARGIN_TILES;
    let max_tile_y = WORLD_HEIGHT_TILES - TREE_SPAWN_WORLD_MARGIN_TILES;

    let mut spawned_tree_count = 0;
    let mut attempts = 0;
    let mut occupied_tiles = HashSet::<(u32, u32)>::new();
    let mut spawned_tree_positions = Vec::<(f32, f32)>::new(); // Store positions of spawned trees

    while spawned_tree_count < target_tree_count && attempts < max_attempts {
        attempts += 1;

        // 1. Select random tile & check if occupied
        let tile_x = rng.gen_range(min_tile_x..max_tile_x);
        let tile_y = rng.gen_range(min_tile_y..max_tile_y);
        if occupied_tiles.contains(&(tile_x, tile_y)) {
            continue;
        }

        // 2. Calculate world position
        let pos_x = (tile_x as f32 + 0.5) * TILE_SIZE_PX as f32;
        let pos_y = (tile_y as f32 + 0.5) * TILE_SIZE_PX as f32;

        // 3. Check noise threshold
        let noise_val = fbm.get([
            (pos_x as f64 / WORLD_WIDTH_PX as f64) * TREE_SPAWN_NOISE_FREQUENCY,
            (pos_y as f64 / WORLD_HEIGHT_PX as f64) * TREE_SPAWN_NOISE_FREQUENCY,
        ]);
        let normalized_noise = (noise_val + 1.0) / 2.0;

        if normalized_noise > TREE_SPAWN_NOISE_THRESHOLD {
            // 4. Check minimum distance from existing trees
            let mut too_close = false;
            for (existing_x, existing_y) in &spawned_tree_positions {
                let dx = pos_x - existing_x;
                let dy = pos_y - existing_y;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < MIN_TREE_DISTANCE_SQ {
                    too_close = true;
                    break;
                }
            }

            if too_close {
                continue; // Too close, try another spot
            }

            // 5. Spawn tree if noise is high enough AND not too close
            let tree_type = TreeType::Oak;
            match trees.try_insert(Tree {
                id: 0,
                pos_x,
                pos_y,
                health: 100,
                tree_type,
                state: TreeState::Growing,
            }) {
                Ok(_) => {
                    spawned_tree_count += 1;
                    occupied_tiles.insert((tile_x, tile_y));
                    spawned_tree_positions.push((pos_x, pos_y)); // Store position
                }
                Err(e) => {
                    log::error!("Failed to insert tree during density/distance seeding: {}", e);
                }
            }
        }
    }

    log::info!(
        "Finished seeding {} trees (target: {}, attempts: {}).",
        spawned_tree_count,
        target_tree_count,
        attempts
    );
    Ok(())
} 