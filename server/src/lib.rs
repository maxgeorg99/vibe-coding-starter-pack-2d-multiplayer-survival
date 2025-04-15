use spacetimedb::{Identity, Timestamp, ReducerContext, Table};
use log;

// Declare the module
mod environment;
// Use the public items from the module
use environment::*; 

// --- World/Player Constants --- 
pub(crate) const WORLD_WIDTH_TILES: u32 = 100;
pub(crate) const WORLD_HEIGHT_TILES: u32 = 100;
pub(crate) const TILE_SIZE_PX: u32 = 48;
pub(crate) const WORLD_WIDTH_PX: f32 = (WORLD_WIDTH_TILES * TILE_SIZE_PX) as f32;
pub(crate) const WORLD_HEIGHT_PX: f32 = (WORLD_HEIGHT_TILES * TILE_SIZE_PX) as f32;
pub(crate) const PLAYER_RADIUS: f32 = 24.0;
const PLAYER_DIAMETER_SQUARED: f32 = (PLAYER_RADIUS * 2.0) * (PLAYER_RADIUS * 2.0);

// Passive Stat Drain Rates
const HUNGER_DRAIN_PER_SECOND: f32 = 100.0 / (30.0 * 60.0); 
const THIRST_DRAIN_PER_SECOND: f32 = 100.0 / (20.0 * 60.0); 
const STAMINA_DRAIN_PER_SECOND: f32 = 20.0; 
const STAMINA_RECOVERY_PER_SECOND: f32 = 5.0;  
const SPRINT_SPEED_MULTIPLIER: f32 = 1.5;     

// Status Effect Constants
const LOW_NEED_THRESHOLD: f32 = 20.0;         
const LOW_THIRST_SPEED_PENALTY: f32 = 0.75; 
const HEALTH_LOSS_PER_SEC_LOW_THIRST: f32 = 0.5; 
const HEALTH_LOSS_PER_SEC_LOW_HUNGER: f32 = 0.4; 
const HEALTH_LOSS_MULTIPLIER_AT_ZERO: f32 = 2.0; 
const HEALTH_RECOVERY_THRESHOLD: f32 = 80.0;    
const HEALTH_RECOVERY_PER_SEC: f32 = 1.0;      
const MIN_HEALTH_FROM_NEEDS: f32 = 1.0;       

// Player table to store position and color
#[spacetimedb::table(name = player, public)]
#[derive(Clone)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
    pub username: String,
    pub position_x: f32,
    pub position_y: f32,
    pub color: String,
    pub direction: String,
    pub last_update: Timestamp,
    pub jump_start_time_ms: u64,
    pub health: f32,
    pub stamina: f32,
    pub thirst: f32,
    pub hunger: f32,
    pub warmth: f32,
    pub is_sprinting: bool,
}

// When a client connects, we need to create a player for them
#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(_ctx: &ReducerContext) -> Result<(), String> {
    // Call the seed_trees function from the environment module
    environment::seed_environment(_ctx)?;
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
    let trees = ctx.db.tree();
    let stones = ctx.db.stone();
    
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
    let max_attempts = 10;
    let offset_step = PLAYER_RADIUS * 2.5;
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
                let dy = spawn_y - (tree.pos_y - environment::TREE_COLLISION_Y_OFFSET);
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < environment::PLAYER_TREE_COLLISION_DISTANCE_SQUARED {
                    collision = true;
                    break;
                }
            }
        }

        // 2.5 Check Player-Stone Collision (if no player/tree collision)
        if !collision {
            for stone in stones.iter() {
                let dx = spawn_x - stone.pos_x;
                let dy = spawn_y - (stone.pos_y - environment::STONE_COLLISION_Y_OFFSET);
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < environment::PLAYER_STONE_COLLISION_DISTANCE_SQUARED {
                    collision = true;
                    break;
                }
            }
        }

        // 3. Decide if position is valid or max attempts reached
        if !collision || attempt >= max_attempts {
            if attempt >= max_attempts && collision { 
                 log::warn!("Could not find clear spawn point for {}, spawning at default (may collide).", username);
                 spawn_x = initial_x;
                 spawn_y = initial_y;
            }
            break;
        }

        // Simple offset pattern: move right, down, left, up, then spiral out slightly?
        // This is basic, could be improved (random, spiral search)
        match attempt % 4 {
            0 => spawn_x += offset_step, 
            1 => spawn_y += offset_step, 
            2 => spawn_x -= offset_step * 2.0, 
            3 => spawn_y -= offset_step * 2.0, 
            _ => {}, 
        }
        // Reset to center if offset gets too wild after a few attempts (basic safeguard)
        if attempt == 5 { 
             spawn_x = initial_x;
             spawn_y = initial_y;
             spawn_x += offset_step * 1.5; 
             spawn_y += offset_step * 1.5;
        }
        attempt += 1;
    }
    // --- End spawn position logic ---

    let color = random_color(&username);
    
    let player = Player {
        identity: sender_id,
        username: username.clone(), 
        position_x: spawn_x, 
        position_y: spawn_y, 
        color,
        direction: "down".to_string(), 
        last_update: ctx.timestamp,
        jump_start_time_ms: 0, 
        health: 100.0,
        stamina: 100.0,
        thirst: 100.0,
        hunger: 100.0,
        warmth: 100.0,
        is_sprinting: false,
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
    move_dx: f32, 
    move_dy: f32  
) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let trees = ctx.db.tree();
    let stones = ctx.db.stone();

    let current_player = players.identity()
        .find(sender_id)
        .ok_or_else(|| "Player not found".to_string())?;

    let now = ctx.timestamp;
    let last_update_time = current_player.last_update;
    let elapsed_micros = now.to_micros_since_unix_epoch().saturating_sub(last_update_time.to_micros_since_unix_epoch());
    let elapsed_seconds = (elapsed_micros as f64 / 1_000_000.0) as f32;
    let new_hunger = (current_player.hunger - (elapsed_seconds * HUNGER_DRAIN_PER_SECOND)).max(0.0);
    let new_thirst = (current_player.thirst - (elapsed_seconds * THIRST_DRAIN_PER_SECOND)).max(0.0);
    let mut new_stamina = current_player.stamina;
    let mut base_speed_multiplier = 1.0;
    let mut current_sprinting_state = current_player.is_sprinting;
    let is_moving = move_dx != 0.0 || move_dy != 0.0;
    if current_sprinting_state && is_moving && new_stamina > 0.0 {
        new_stamina = (new_stamina - (elapsed_seconds * STAMINA_DRAIN_PER_SECOND)).max(0.0);
        if new_stamina > 0.0 { 
            base_speed_multiplier = SPRINT_SPEED_MULTIPLIER;
        } else { 
            current_sprinting_state = false;
            log::debug!("Player {:?} ran out of stamina.", sender_id);
        }
    } else if !current_sprinting_state {
        new_stamina = (new_stamina + (elapsed_seconds * STAMINA_RECOVERY_PER_SECOND)).min(100.0);
    }
    let mut final_speed_multiplier = base_speed_multiplier;
    if new_thirst < LOW_NEED_THRESHOLD {
        final_speed_multiplier *= LOW_THIRST_SPEED_PENALTY;
        if is_moving { 
             log::debug!("Player {:?} has low thirst. Applying speed penalty.", sender_id);
        }
    }
    let mut health_change_per_sec: f32 = 0.0;
    if new_thirst <= 0.0 {
        health_change_per_sec -= HEALTH_LOSS_PER_SEC_LOW_THIRST * HEALTH_LOSS_MULTIPLIER_AT_ZERO;
        log::debug!("Player {:?} health decreasing rapidly due to zero thirst.", sender_id);
    } else if new_thirst < LOW_NEED_THRESHOLD {
        health_change_per_sec -= HEALTH_LOSS_PER_SEC_LOW_THIRST;
        log::debug!("Player {:?} health decreasing due to low thirst.", sender_id);
    }
    if new_hunger <= 0.0 {
        health_change_per_sec -= HEALTH_LOSS_PER_SEC_LOW_HUNGER * HEALTH_LOSS_MULTIPLIER_AT_ZERO;
        log::debug!("Player {:?} health decreasing rapidly due to zero hunger.", sender_id);
    } else if new_hunger < LOW_NEED_THRESHOLD {
        health_change_per_sec -= HEALTH_LOSS_PER_SEC_LOW_HUNGER;
        log::debug!("Player {:?} health decreasing due to low hunger.", sender_id);
    }
    if health_change_per_sec == 0.0 && 
       new_hunger >= HEALTH_RECOVERY_THRESHOLD && 
       new_thirst >= HEALTH_RECOVERY_THRESHOLD {
        health_change_per_sec += HEALTH_RECOVERY_PER_SEC;
        log::debug!("Player {:?} health recovering.", sender_id);
    }
    let new_health = (current_player.health + (health_change_per_sec * elapsed_seconds))
                     .max(MIN_HEALTH_FROM_NEEDS)
                     .min(100.0);
    let health_changed = (new_health - current_player.health).abs() > 0.01;

    let proposed_x = current_player.position_x + move_dx * final_speed_multiplier;
    let proposed_y = current_player.position_y + move_dy * final_speed_multiplier;

    let clamped_x = proposed_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
    let clamped_y = proposed_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);

    let mut final_x = clamped_x;
    let mut final_y = clamped_y;
    let mut collision_handled = false;

    // Check Player-Player Collision
    for other_player in players.iter() {
        if other_player.identity == sender_id {
            continue;
        }
        let dx = clamped_x - other_player.position_x;
        let dy = clamped_y - other_player.position_y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq < PLAYER_DIAMETER_SQUARED {
            log::debug!("Player-Player collision detected between {:?} and {:?}. Calculating slide.", sender_id, other_player.identity);

            // Calculate slide vector
            let intended_dx = clamped_x - current_player.position_x;
            let intended_dy = clamped_y - current_player.position_y;
            let collision_normal_x = dx;
            let collision_normal_y = dy;
            let normal_mag_sq = dist_sq;

            if normal_mag_sq > 0.0 {
                let normal_mag = normal_mag_sq.sqrt();
                let norm_x = collision_normal_x / normal_mag;
                let norm_y = collision_normal_y / normal_mag;

                let dot_product = intended_dx * norm_x + intended_dy * norm_y;

                // Project intended movement onto the normal
                let projection_x = dot_product * norm_x;
                let projection_y = dot_product * norm_y;

                // Subtract projection to get the slide vector (tangential movement)
                let slide_dx = intended_dx - projection_x;
                let slide_dy = intended_dy - projection_y;

                // Apply slide to the *original* position
                final_x = current_player.position_x + slide_dx;
                final_y = current_player.position_y + slide_dy;

                // Re-clamp to world boundaries after sliding
                final_x = final_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
                final_y = final_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);
            } else {
                // Fallback: If somehow distance is zero, just revert
                final_x = current_player.position_x;
                final_y = current_player.position_y;
            }
            collision_handled = true;
            break; // Handle first player collision
        }
    }

    // Only check trees if no player collision was handled
    if !collision_handled {
        for tree in trees.iter() {
            let tree_collision_y = tree.pos_y - environment::TREE_COLLISION_Y_OFFSET;
            let dx = clamped_x - tree.pos_x;
            let dy = clamped_y - tree_collision_y;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq < environment::PLAYER_TREE_COLLISION_DISTANCE_SQUARED {
                log::debug!("Player-Tree collision detected between {:?} and tree {}. Calculating slide.", sender_id, tree.id);

                let intended_dx = clamped_x - current_player.position_x;
                let intended_dy = clamped_y - current_player.position_y;
                let collision_normal_x = dx;
                let collision_normal_y = dy;
                let normal_mag_sq = dist_sq;

                if normal_mag_sq > 0.0 {
                    let normal_mag = normal_mag_sq.sqrt();
                    let norm_x = collision_normal_x / normal_mag;
                    let norm_y = collision_normal_y / normal_mag;
                    let dot_product = intended_dx * norm_x + intended_dy * norm_y;
                    let projection_x = dot_product * norm_x;
                    let projection_y = dot_product * norm_y;
                    let slide_dx = intended_dx - projection_x;
                    let slide_dy = intended_dy - projection_y;
                    final_x = current_player.position_x + slide_dx;
                    final_y = current_player.position_y + slide_dy;
                    final_x = final_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
                    final_y = final_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);
                } else {
                    final_x = current_player.position_x;
                    final_y = current_player.position_y;
                }
                collision_handled = true;
                break; // Handle first tree collision
            }
        }
    }

    // Only check stones if no player or tree collision was handled
    if !collision_handled {
        for stone in stones.iter() {
            let stone_collision_y = stone.pos_y - environment::STONE_COLLISION_Y_OFFSET;
            let dx = clamped_x - stone.pos_x;
            let dy = clamped_y - stone_collision_y;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq < environment::PLAYER_STONE_COLLISION_DISTANCE_SQUARED {
                log::debug!("Player-Stone collision detected between {:?} and stone {}. Calculating slide.", sender_id, stone.id);

                let intended_dx = clamped_x - current_player.position_x;
                let intended_dy = clamped_y - current_player.position_y;
                let collision_normal_x = dx;
                let collision_normal_y = dy;
                let normal_mag_sq = dist_sq;

                if normal_mag_sq > 0.0 {
                    let normal_mag = normal_mag_sq.sqrt();
                    let norm_x = collision_normal_x / normal_mag;
                    let norm_y = collision_normal_y / normal_mag;
                    let dot_product = intended_dx * norm_x + intended_dy * norm_y;
                    let projection_x = dot_product * norm_x;
                    let projection_y = dot_product * norm_y;
                    let slide_dx = intended_dx - projection_x;
                    let slide_dy = intended_dy - projection_y;
                    final_x = current_player.position_x + slide_dx;
                    final_y = current_player.position_y + slide_dy;
                    final_x = final_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
                    final_y = final_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);
                } else {
                    final_x = current_player.position_x;
                    final_y = current_player.position_y;
                }
                // No need to set collision_handled=true here if it's the last check
                break; // Handle first stone collision
            }
        }
    }

    // --- Iterative Collision Resolution (Push-out) ---
    let mut resolved_x = final_x;
    let mut resolved_y = final_y;
    let resolution_iterations = 5; // Max iterations to prevent infinite loops
    let epsilon = 0.01; // Tiny value to push slightly beyond contact

    for _iter in 0..resolution_iterations {
        let mut overlap_found_in_iter = false;

        // Check Player-Player Overlap
        for other_player in players.iter() {
            if other_player.identity == sender_id { continue; }
            let dx = resolved_x - other_player.position_x;
            let dy = resolved_y - other_player.position_y;
            let dist_sq = dx * dx + dy * dy;
            let min_dist = PLAYER_RADIUS * 2.0;
            let min_dist_sq = min_dist * min_dist;

            if dist_sq < min_dist_sq && dist_sq > 0.0 {
                overlap_found_in_iter = true;
                let distance = dist_sq.sqrt();
                let overlap = (min_dist - distance); 
                // Push each player half the overlap distance + epsilon
                let push_amount = (overlap / 2.0) + epsilon;
                let push_x = (dx / distance) * push_amount;
                let push_y = (dy / distance) * push_amount;
                resolved_x += push_x;
                resolved_y += push_y;
                // Note: Ideally, push other_player by -push_x, -push_y, but requires mutable access or separate update mechanism.
                // For now, only pushing the current player.
                log::trace!("Resolving player-player overlap iter {}. Push: ({}, {})", _iter, push_x, push_y);
            }
        }

        // Check Player-Tree Overlap
        for tree in trees.iter() {
            let tree_collision_y = tree.pos_y - environment::TREE_COLLISION_Y_OFFSET;
            let dx = resolved_x - tree.pos_x;
            let dy = resolved_y - tree_collision_y;
            let dist_sq = dx * dx + dy * dy;
            let min_dist = PLAYER_RADIUS + environment::TREE_TRUNK_RADIUS;
            let min_dist_sq = min_dist * min_dist;

            if dist_sq < min_dist_sq && dist_sq > 0.0 {
                 overlap_found_in_iter = true;
                 let distance = dist_sq.sqrt();
                 let overlap = (min_dist - distance) + epsilon;
                 let push_x = (dx / distance) * overlap;
                 let push_y = (dy / distance) * overlap;
                 resolved_x += push_x;
                 resolved_y += push_y;
                 log::trace!("Resolving player-tree overlap iter {}. Push: ({}, {})", _iter, push_x, push_y);
            }
        }

        // Check Player-Stone Overlap
        for stone in stones.iter() {
            let stone_collision_y = stone.pos_y - environment::STONE_COLLISION_Y_OFFSET;
            let dx = resolved_x - stone.pos_x;
            let dy = resolved_y - stone_collision_y;
            let dist_sq = dx * dx + dy * dy;
            let min_dist = PLAYER_RADIUS + environment::STONE_RADIUS;
            let min_dist_sq = min_dist * min_dist;

            if dist_sq < min_dist_sq && dist_sq > 0.0 {
                overlap_found_in_iter = true;
                let distance = dist_sq.sqrt();
                let overlap = (min_dist - distance) + epsilon;
                let push_x = (dx / distance) * overlap;
                let push_y = (dy / distance) * overlap;
                resolved_x += push_x;
                resolved_y += push_y;
                log::trace!("Resolving player-stone overlap iter {}. Push: ({}, {})", _iter, push_x, push_y);
            }
        }

        // Re-clamp final resolved position to world boundaries after each iteration
        resolved_x = resolved_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
        resolved_y = resolved_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);

        if !overlap_found_in_iter {
            log::trace!("Overlap resolution complete after {} iterations.", _iter + 1);
            break; // Exit iterations if no overlaps were found in this pass
        }
        if _iter == resolution_iterations - 1 {
             log::warn!("Overlap resolution reached max iterations ({}) for player {:?}. Position might still overlap slightly.", resolution_iterations, sender_id);
        }
    }

    // Determine final direction based on actual movement
    let actual_dx = resolved_x - current_player.position_x;
    let actual_dy = resolved_y - current_player.position_y;
    let position_changed = actual_dx != 0.0 || actual_dy != 0.0;
    let should_update = position_changed || health_changed || elapsed_seconds > 0.1;

    if should_update {
        let mut direction = current_player.direction.clone();
        if position_changed {
            if actual_dx.abs() > actual_dy.abs() { 
                if actual_dx > 0.0 { direction = "right".to_string(); }
                else if actual_dx < 0.0 { direction = "left".to_string(); }
            } else if actual_dy.abs() > actual_dx.abs() { 
                if actual_dy > 0.0 { direction = "down".to_string(); }
                else if actual_dy < 0.0 { direction = "up".to_string(); }
            } 
        }

        let updated_player = Player {
            position_x: resolved_x,
            position_y: resolved_y,
            direction,
            last_update: now,
            hunger: new_hunger,
            thirst: new_thirst,
            stamina: new_stamina,
            health: new_health,
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