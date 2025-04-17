use spacetimedb::{Identity, Timestamp, ReducerContext, Table};
use log;
use std::time::Duration;

// Declare the module
mod environment;
mod tree; // Add tree module
mod stone; // Add stone module
// Declare the items module
mod items;
// Declare the world_state module
mod world_state;
// Declare the campfire module
mod campfire;
// Declare the active_equipment module
mod active_equipment;
// Declare the mushroom module
mod mushroom;
// Declare the consumables module
mod consumables;
mod utils; // Declare utils module

// Import Table Traits needed in this module
use crate::tree::tree as TreeTableTrait; 
use crate::stone::stone as StoneTableTrait;
use crate::campfire::campfire as CampfireTableTrait; // Already present, but good to keep together
use crate::world_state::world_state as WorldStateTableTrait; // Already present
use crate::items::inventory_item as InventoryItemTableTrait; // Already present
use crate::items::item_definition as ItemDefinitionTableTrait; // Already present
use crate::player as PlayerTableTrait; // Needed for ctx.db.player()

// Use specific items needed globally (or use qualified paths)
// use crate::items::{inventory_item as InventoryItemTableTrait, item_definition as ItemDefinitionTableTrait}; 
use crate::world_state::{TimeOfDay, BASE_WARMTH_DRAIN_PER_SECOND, WARMTH_DRAIN_MULTIPLIER_DAWN_DUSK, WARMTH_DRAIN_MULTIPLIER_NIGHT, WARMTH_DRAIN_MULTIPLIER_MIDNIGHT};
use crate::campfire::{Campfire, WARMTH_RADIUS_SQUARED, WARMTH_PER_SECOND, CAMPFIRE_COLLISION_RADIUS, CAMPFIRE_CAMPFIRE_COLLISION_DISTANCE_SQUARED, CAMPFIRE_COLLISION_Y_OFFSET, PLAYER_CAMPFIRE_COLLISION_DISTANCE_SQUARED };

// Remove commented-out schedule imports
// use crate::campfire::{CampfireUpdateSchedule, CAMPFIRE_UPDATE_INTERVAL_SECS}; // Temporarily disabled
// use spacetimedb::ScheduleAt; // Temporarily disabled

// Import generated table traits with aliases to avoid name conflicts
// use crate::campfire::campfire_update_schedule as CampfireUpdateScheduleTableTrait; // Temporarily disabled

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
const JUMP_COOLDOWN_MS: u64 = 500; // Prevent jumping again for 500ms

// Status Effect Constants
const LOW_NEED_THRESHOLD: f32 = 20.0;         
const LOW_THIRST_SPEED_PENALTY: f32 = 0.75; 
const HEALTH_LOSS_PER_SEC_LOW_THIRST: f32 = 0.5; 
const HEALTH_LOSS_PER_SEC_LOW_HUNGER: f32 = 0.4; 
const HEALTH_LOSS_MULTIPLIER_AT_ZERO: f32 = 2.0; 
const HEALTH_RECOVERY_THRESHOLD: f32 = 80.0;    
const HEALTH_RECOVERY_PER_SEC: f32 = 1.0;      

// New Warmth Penalties
const HEALTH_LOSS_PER_SEC_LOW_WARMTH: f32 = 0.6; // Slightly higher than thirst/hunger
const LOW_WARMTH_SPEED_PENALTY: f32 = 0.8; // 20% speed reduction when cold

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
    pub is_dead: bool,
    pub respawn_at: Timestamp,
    pub last_hit_time: Option<Timestamp>,
}

// --- Lifecycle Reducers ---

// Called once when the module is published or updated
#[spacetimedb::reducer(init)]
pub fn init_module(ctx: &ReducerContext) -> Result<(), String> {
    log::info!("Initializing module...");

    // Remove commented-out schedule init logic
    /*
    let schedule_table = ctx.db.campfire_update_schedule();
    if schedule_table.iter().count() == 0 {
        log::info!("Starting campfire update schedule (every {}s).", CAMPFIRE_UPDATE_INTERVAL_SECS);
        let interval = Duration::from_secs(CAMPFIRE_UPDATE_INTERVAL_SECS);
        schedule_table.insert(CampfireUpdateSchedule {
            id: 0, // Auto-incremented
            schedule_info: ScheduleAt::Interval(interval.into()),
        })?;
    } else {
        log::debug!("Campfire update schedule already exists.");
    }
    */

    log::info!("Module initialization complete.");
    Ok(())
}

// When a client connects, we need to create a player for them
#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) -> Result<(), String> {
    // Call seeders using qualified paths
    crate::environment::seed_environment(ctx)?; // Call the updated seeder
    crate::items::seed_items(ctx)?; // Call the item seeder
    crate::world_state::seed_world_state(ctx)?; // Call the world state seeder
    // No seeder needed for Campfire yet, table will be empty initially
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
    let campfires = ctx.db.campfire(); // Get campfire table
    
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
                let dy = spawn_y - (tree.pos_y - crate::tree::TREE_COLLISION_Y_OFFSET); // Already qualified
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < crate::tree::PLAYER_TREE_COLLISION_DISTANCE_SQUARED { // Already qualified
                    collision = true;
                    break;
                }
            }
        }

        // 2.5 Check Player-Stone Collision (if no player/tree collision)
        if !collision {
            for stone in stones.iter() {
                let dx = spawn_x - stone.pos_x;
                let dy = spawn_y - (stone.pos_y - crate::stone::STONE_COLLISION_Y_OFFSET); // Already qualified
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < crate::stone::PLAYER_STONE_COLLISION_DISTANCE_SQUARED { // Already qualified
                    collision = true;
                    break;
                }
            }
        }

        // 2.7 Check Player-Campfire Collision
        if !collision {
            for fire in campfires.iter() {
                let dx = spawn_x - fire.pos_x;
                let dy = spawn_y - (fire.pos_y - CAMPFIRE_COLLISION_Y_OFFSET);
                let dist_sq = dx * dx + dy * dy;
                // Use specific player-campfire collision check distance
                if dist_sq < PLAYER_CAMPFIRE_COLLISION_DISTANCE_SQUARED {
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
        is_dead: false,
        respawn_at: ctx.timestamp,
        last_hit_time: None,
    };
    
    // Insert the new player
    match players.try_insert(player) {
        Ok(_) => {
            log::info!("Player registered: {}. Granting starting items...", username);

            // --- Grant Starting Items --- 
            let item_defs = ctx.db.item_definition();
            let inventory = ctx.db.inventory_item();

            // Define the full starting items array explicitly
            let starting_items = [
                // Tools/Resources on Hotbar
                ("Stone Hatchet", 1, Some(0u8), None), 
                ("Stone Pickaxe", 1, Some(1u8), None),
                // ("Wood", 500, Some(2u8), None), // REMOVED
                // ("Stone", 500, Some(3u8), None), // REMOVED
                ("Camp Fire", 1, Some(4u8), None),
                ("Camp Fire", 1, Some(5u8), None),
                ("Rock", 1, Some(6u8), None), 
                
                // Armor in Inventory 
                ("Cloth Shirt", 1, None, Some(0u16)), 
                ("Cloth Shirt", 1, None, Some(1u16)), 
                ("Cloth Pants", 1, None, Some(2u16)),
                ("Cloth Pants", 1, None, Some(3u16)),
                ("Cloth Hood", 1, None, Some(4u16)),
                ("Cloth Hood", 1, None, Some(5u16)),
                ("Cloth Boots", 1, None, Some(6u16)),
                ("Cloth Boots", 1, None, Some(7u16)),
                ("Cloth Gloves", 1, None, Some(8u16)),
                ("Cloth Gloves", 1, None, Some(9u16)),
                ("Burlap Backpack", 1, None, Some(10u16)),
                ("Burlap Backpack", 1, None, Some(11u16)),

                // NEW: Add starting materials to inventory
                // ("Wood", 600, None, Some(12u16)), // Add 600 Wood to inv slot 12
                // ("Wood", 500, None, Some(13u16)), // Add 500 Wood to inv slot 13
                // ("Stone", 500, None, Some(14u16)), // Add 500 Stone to inv slot 14
            ];

            log::info!("[Register Player] Defined {} starting item entries.", starting_items.len());

            for (item_name, quantity, hotbar_slot_opt, inventory_slot_opt) in starting_items.iter() {
                 log::debug!("[Register Player] Processing entry: {}", item_name);
                if let Some(item_def) = item_defs.iter().find(|def| def.name == *item_name) {
                    let item_to_insert = crate::items::InventoryItem { // Qualify struct path
                        instance_id: 0,
                        player_identity: sender_id,
                        item_def_id: item_def.id,
                        quantity: *quantity,
                        hotbar_slot: *hotbar_slot_opt,
                        inventory_slot: *inventory_slot_opt,
                    };
                    match inventory.try_insert(item_to_insert) {
                        Ok(_) => {
                             log::info!("[Register Player] Granted: {} (Qty: {}, H: {:?}, I: {:?})", 
                                         item_name, quantity, hotbar_slot_opt, inventory_slot_opt);
                        },
                        Err(e) => log::error!("[Register Player] FAILED insert for {}: {}", item_name, e),
                    }
                } else {
                    log::error!("[Register Player] Definition NOT FOUND for: {}", item_name);
                }
            }
            // --- End Grant Starting Items ---

            Ok(())
        },
        Err(e) => {
            Err(format!("Failed to register player: {}", e))
        }
    }
}

// Reducer to place a campfire
#[spacetimedb::reducer]
pub fn place_campfire(ctx: &ReducerContext, target_x: f32, target_y: f32) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let mut inventory_items = ctx.db.inventory_item(); // Mutable for insert/update
    let item_defs = ctx.db.item_definition();
    let mut campfires = ctx.db.campfire(); // Mutable for insert
    let trees = ctx.db.tree();
    let stones = ctx.db.stone();

    // --- 1. Check if player exists ---
    let player = players.identity().find(sender_id)
        .ok_or_else(|| "Player not found".to_string())?;

    // --- 2. Find the "Camp Fire" item definition ---
    let campfire_placeable_def = item_defs.iter()
        .find(|def| def.name == "Camp Fire")
        .ok_or_else(|| "Camp Fire item definition not found".to_string())?;

    // --- 3. Check if player has a Camp Fire item --- 
    let campfire_item_stack = inventory_items.iter()
        .find(|item| item.player_identity == sender_id && item.item_def_id == campfire_placeable_def.id && item.quantity > 0);

    if campfire_item_stack.is_none() {
        return Err("You do not have a Camp Fire item".to_string());
    }
    let mut campfire_item = campfire_item_stack.unwrap(); // Safe to unwrap here

    // --- 4. Validate Placement Position ---
    // World boundaries
    if target_x < CAMPFIRE_COLLISION_RADIUS || target_x > WORLD_WIDTH_PX - CAMPFIRE_COLLISION_RADIUS ||
       target_y < CAMPFIRE_COLLISION_RADIUS || target_y > WORLD_HEIGHT_PX - CAMPFIRE_COLLISION_RADIUS {
        return Err("Cannot place campfire outside world boundaries".to_string());
    }

    // Collision with other Campfires
    for other_fire in campfires.iter() {
        let dx = target_x - other_fire.pos_x;
        let dy = target_y - other_fire.pos_y; // No Y offset needed for fire-fire check
        if (dx * dx + dy * dy) < CAMPFIRE_CAMPFIRE_COLLISION_DISTANCE_SQUARED {
            return Err("Cannot place campfire too close to another campfire".to_string());
        }
    }

    // Collision with Trees
    for tree in trees.iter() {
        let dx = target_x - tree.pos_x;
        let dy = target_y - (tree.pos_y - crate::tree::TREE_COLLISION_Y_OFFSET); // Already qualified
        // Check if campfire placement overlaps tree collision area
        // Use a combined radius check (tree trunk + campfire radius)
        let combined_radius = crate::tree::TREE_TRUNK_RADIUS + CAMPFIRE_COLLISION_RADIUS; // Already qualified
        if (dx * dx + dy * dy) < (combined_radius * combined_radius) {
             return Err("Cannot place campfire too close to a tree".to_string());
        }
    }

    // Collision with Stones
    for stone in stones.iter() {
        let dx = target_x - stone.pos_x;
        let dy = target_y - (stone.pos_y - crate::stone::STONE_COLLISION_Y_OFFSET); // Already qualified
        let combined_radius = crate::stone::STONE_RADIUS + CAMPFIRE_COLLISION_RADIUS; // Already qualified
        if (dx * dx + dy * dy) < (combined_radius * combined_radius) {
            return Err("Cannot place campfire too close to a stone".to_string());
        }
    }

    // Collision with Players (check against all players, including self if needed, though less critical for placement)
    for other_player in players.iter() {
        let dx = target_x - other_player.position_x;
        let dy = target_y - other_player.position_y; // Use player's center
        let combined_radius = PLAYER_RADIUS + CAMPFIRE_COLLISION_RADIUS;
        if (dx * dx + dy * dy) < (combined_radius * combined_radius) {
             return Err("Cannot place campfire too close to a player".to_string());
        }
    }

    // --- 5. Consume Placable Item --- 
    campfire_item.quantity -= 1;
    let remaining_quantity = campfire_item.quantity; 
    if remaining_quantity == 0 {
        inventory_items.instance_id().delete(campfire_item.instance_id);
        log::info!("Player {} consumed last Camp Fire item (instance {}).", player.username, campfire_item.instance_id);
    } else {
        inventory_items.instance_id().update(campfire_item);
        log::info!("Player {} consumed one Camp Fire item. {} remaining.", player.username, remaining_quantity);
    }

    // --- 7. Create Campfire Entity --- 
    let current_time = ctx.timestamp;
    let new_campfire = Campfire {
        id: 0, // Auto-incremented
        pos_x: target_x,
        pos_y: target_y,
        placed_by: sender_id,
        placed_at: current_time,
        is_burning: false, // Start extinguished
        // Initialize individual fuel slots to None
        fuel_instance_id_0: None,
        fuel_def_id_0: None,
        fuel_instance_id_1: None,
        fuel_def_id_1: None,
        fuel_instance_id_2: None,
        fuel_def_id_2: None,
        fuel_instance_id_3: None,
        fuel_def_id_3: None,
        fuel_instance_id_4: None,
        fuel_def_id_4: None,
        next_fuel_consume_at: None,
    };

    campfires.try_insert(new_campfire)?;
    log::info!("Player {} placed a campfire at ({:.1}, {:.1}).", 
             player.username, target_x, target_y);

    Ok(())
}

// Called by the client to set the sprinting state
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
    move_dy: f32,  
    intended_direction: Option<String>
) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let trees = ctx.db.tree();
    let stones = ctx.db.stone();
    let campfires = ctx.db.campfire(); // Get campfire table
    let world_states = ctx.db.world_state();

    let current_player = players.identity()
        .find(sender_id)
        .ok_or_else(|| "Player not found".to_string())?;

    // --- Update Direction Immediately --- 
    let mut new_direction = current_player.direction.clone(); // Start with current direction
    if let Some(dir_str) = intended_direction {
        // Validate the direction string using direct comparison
        let dir_slice = dir_str.as_str();
        if dir_slice == "up" || dir_slice == "down" || dir_slice == "left" || dir_slice == "right" {
            if new_direction != dir_str { // Log only if direction actually changes
                log::trace!("Player {:?} intended direction set to: {}", sender_id, dir_str);
                new_direction = dir_str; // Assign the original String
            }
        } else {
            log::warn!("Player {:?} sent invalid direction: {}", sender_id, dir_str);
            // Keep the existing direction if the new one is invalid
        }
    } else if move_dx == 0.0 && move_dy == 0.0 {
        // If no direction is explicitly sent AND no movement is attempted,
        // keep the current direction. (This preserves facing direction when standing still).
    } else {
        // Fallback: Determine direction from movement delta if no explicit direction provided
        // This handles cases where the client might not send the direction yet, 
        // or if movement occurs without explicit direction keys (e.g., joystick diagonal)
        if move_dx.abs() > move_dy.abs() {
            new_direction = if move_dx > 0.0 { "right".to_string() } else { "left".to_string() };
        } else if move_dy != 0.0 {
            new_direction = if move_dy > 0.0 { "down".to_string() } else { "up".to_string() };
        }
        // If move_dx and move_dy are both 0, and intended_direction was None,
        // new_direction remains the original value from current_player.direction.clone()
        if current_player.direction != new_direction {
            log::trace!("Player {:?} direction inferred from movement: {}", sender_id, new_direction);
        }
    }
    // --- End Direction Update ---

    let world_state = world_states.iter().next()
        .ok_or_else(|| "WorldState not found".to_string())?;

    let now = ctx.timestamp;
    let last_update_time = current_player.last_update;
    let elapsed_micros = now.to_micros_since_unix_epoch().saturating_sub(last_update_time.to_micros_since_unix_epoch());
    let elapsed_seconds = (elapsed_micros as f64 / 1_000_000.0) as f32;
    let new_hunger = (current_player.hunger - (elapsed_seconds * HUNGER_DRAIN_PER_SECOND)).max(0.0);
    let new_thirst = (current_player.thirst - (elapsed_seconds * THIRST_DRAIN_PER_SECOND)).max(0.0);

    // --- Calculate new Warmth (Moved earlier) ---
    let mut warmth_change_per_sec: f32 = 0.0;
    // 1. Warmth Drain based on Time of Day
    let drain_multiplier = match world_state.time_of_day {
        TimeOfDay::Morning | TimeOfDay::Noon | TimeOfDay::Afternoon => 0.0, // No warmth drain during day
        TimeOfDay::Dawn | TimeOfDay::Dusk => WARMTH_DRAIN_MULTIPLIER_DAWN_DUSK, // Keep transition drain
        TimeOfDay::Night => WARMTH_DRAIN_MULTIPLIER_NIGHT * 1.25, // Increased night drain
        TimeOfDay::Midnight => WARMTH_DRAIN_MULTIPLIER_MIDNIGHT * 1.33, // Increased midnight drain
    };
    warmth_change_per_sec -= BASE_WARMTH_DRAIN_PER_SECOND * drain_multiplier;
    // 2. Warmth Gain from nearby Campfires
    for fire in campfires.iter() {
        let dx = current_player.position_x - fire.pos_x;
        let dy = current_player.position_y - fire.pos_y;
        if (dx * dx + dy * dy) < WARMTH_RADIUS_SQUARED {
            warmth_change_per_sec += WARMTH_PER_SECOND;
            log::trace!("Player {:?} gaining warmth from campfire {}", sender_id, fire.id);
        }
    }
    let new_warmth = (current_player.warmth + (warmth_change_per_sec * elapsed_seconds))
                     .max(0.0) // Clamp between 0 and 100
                     .min(100.0);
    let warmth_changed = (new_warmth - current_player.warmth).abs() > 0.01;
    if warmth_changed {
        log::debug!("Player {:?} warmth updated to {:.1}", sender_id, new_warmth);
    }
    // --- End Warmth Calculation ---

    // --- Stamina and Base Speed Calculation ---
    let mut new_stamina = current_player.stamina;
    let mut base_speed_multiplier = 1.0;
    let is_moving = move_dx != 0.0 || move_dy != 0.0;
    let mut current_sprinting_state = current_player.is_sprinting;
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
    if new_warmth < LOW_NEED_THRESHOLD {
        final_speed_multiplier *= LOW_WARMTH_SPEED_PENALTY;
        if is_moving {
            log::debug!("Player {:?} is cold. Applying speed penalty.", sender_id);
        }
    }

    // --- Health Update Calculation ---
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
    if new_warmth <= 0.0 {
        health_change_per_sec -= HEALTH_LOSS_PER_SEC_LOW_WARMTH * HEALTH_LOSS_MULTIPLIER_AT_ZERO;
        log::debug!("Player {:?} health decreasing rapidly due to freezing (zero warmth).", sender_id);
    } else if new_warmth < LOW_NEED_THRESHOLD {
        health_change_per_sec -= HEALTH_LOSS_PER_SEC_LOW_WARMTH;
        log::debug!("Player {:?} health decreasing due to low warmth.", sender_id);
    }
    if health_change_per_sec == 0.0 && 
       new_hunger >= HEALTH_RECOVERY_THRESHOLD && 
       new_thirst >= HEALTH_RECOVERY_THRESHOLD &&
       new_warmth >= LOW_NEED_THRESHOLD { // Must not be freezing to recover health
        health_change_per_sec += HEALTH_RECOVERY_PER_SEC;
        log::debug!("Player {:?} health recovering.", sender_id);
    }
    let new_health = (current_player.health + (health_change_per_sec * elapsed_seconds))
                     .max(0.0) // Allow health to reach zero
                     .min(100.0);
    let health_changed = (new_health - current_player.health).abs() > 0.01;

    // --- Death Check ---
    let mut player_died = false;
    let mut calculated_respawn_at = current_player.respawn_at; // Keep existing value by default
    if current_player.health > 0.0 && new_health <= 0.0 && !current_player.is_dead {
        player_died = true;
        calculated_respawn_at = ctx.timestamp + Duration::from_secs(5).into(); // Set respawn time
        log::warn!("Player {} ({:?}) has died! Will be respawnable at {:?}", 
                 current_player.username, sender_id, calculated_respawn_at);
        
        // Unequip item on death
        match active_equipment::unequip_item(ctx) {
            Ok(_) => log::info!("Unequipped item for dying player {:?}", sender_id),
            Err(e) => log::error!("Failed to unequip item for dying player {:?}: {}", sender_id, e),
        }
    }

    // --- Movement Calculation ---
    let proposed_x = current_player.position_x + move_dx * final_speed_multiplier;
    let proposed_y = current_player.position_y + move_dy * final_speed_multiplier;

    let clamped_x = proposed_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
    let clamped_y = proposed_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);

    let mut final_x = clamped_x;
    let mut final_y = clamped_y;
    let mut collision_handled = false;

    // --- Sliding Collision Checks ---
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
            if tree.health == 0 { continue; }

            let tree_collision_y = tree.pos_y - crate::tree::TREE_COLLISION_Y_OFFSET;
            let dx = clamped_x - tree.pos_x;
            let dy = clamped_y - tree_collision_y;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq < crate::tree::PLAYER_TREE_COLLISION_DISTANCE_SQUARED {
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
            if stone.health == 0 { continue; }

            let stone_collision_y = stone.pos_y - crate::stone::STONE_COLLISION_Y_OFFSET;
            let dx = clamped_x - stone.pos_x;
            let dy = clamped_y - stone_collision_y;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq < crate::stone::PLAYER_STONE_COLLISION_DISTANCE_SQUARED {
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
                collision_handled = true;
                // No need to set collision_handled=true here if it's the last check
                break; // Handle first stone collision
            }
        }
    }

    // Check Player-Campfire Collision
    if !collision_handled {
        for fire in campfires.iter() {
            let fire_collision_y = fire.pos_y - CAMPFIRE_COLLISION_Y_OFFSET;
            let dx = clamped_x - fire.pos_x;
            let dy = clamped_y - fire_collision_y;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq < PLAYER_CAMPFIRE_COLLISION_DISTANCE_SQUARED {
                log::debug!("Player-Campfire collision detected between {:?} and fire {}. Calculating slide.", sender_id, fire.id);

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
                break; // Handle first campfire collision
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
                let overlap = min_dist - distance;
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
            if tree.health == 0 { continue; }

            let tree_collision_y = tree.pos_y - crate::tree::TREE_COLLISION_Y_OFFSET;
            let dx = resolved_x - tree.pos_x;
            let dy = resolved_y - tree_collision_y;
            let dist_sq = dx * dx + dy * dy;
            let min_dist = PLAYER_RADIUS + crate::tree::TREE_TRUNK_RADIUS;
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
            if stone.health == 0 { continue; }

            let stone_collision_y = stone.pos_y - crate::stone::STONE_COLLISION_Y_OFFSET;
            let dx = resolved_x - stone.pos_x;
            let dy = resolved_y - stone_collision_y;
            let dist_sq = dx * dx + dy * dy;
            let min_dist = PLAYER_RADIUS + crate::stone::STONE_RADIUS;
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

        // Check Player-Campfire Overlap
        for fire in campfires.iter() {
            let fire_collision_y = fire.pos_y - CAMPFIRE_COLLISION_Y_OFFSET;
            let dx = resolved_x - fire.pos_x;
            let dy = resolved_y - fire_collision_y;
            let dist_sq = dx * dx + dy * dy;
            let min_dist = PLAYER_RADIUS + CAMPFIRE_COLLISION_RADIUS; // Use campfire radius
            let min_dist_sq = min_dist * min_dist;

            if dist_sq < min_dist_sq && dist_sq > 0.0 {
                overlap_found_in_iter = true;
                let distance = dist_sq.sqrt();
                let overlap = (min_dist - distance) + epsilon;
                let push_x = (dx / distance) * overlap;
                let push_y = (dy / distance) * overlap;
                resolved_x += push_x;
                resolved_y += push_y;
                log::trace!("Resolving player-campfire overlap iter {}. Push: ({}, {})", _iter, push_x, push_y);
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

    // --- Final Update ---
    // Determine final direction based on actual movement
    let actual_dx = resolved_x - current_player.position_x;
    let actual_dy = resolved_y - current_player.position_y;
    let position_changed = actual_dx != 0.0 || actual_dy != 0.0;
    // Update if position, health, or warmth changed, OR if player died, or if enough time passed
    let should_update = player_died || position_changed || health_changed || warmth_changed || elapsed_seconds > 0.1;

    if should_update {
        let player = Player {
            identity: sender_id,
            position_x: resolved_x,
            position_y: resolved_y,
            direction: new_direction,
            last_update: now,
            hunger: new_hunger,
            thirst: new_thirst,
            stamina: new_stamina,
            health: new_health,
            warmth: new_warmth,
            is_sprinting: current_sprinting_state,
            is_dead: player_died,
            respawn_at: calculated_respawn_at,
            last_hit_time: None,
            ..current_player
        };
        players.identity().update(player);
    }

    // --- Tick World State --- using qualified path
    // We pass the current context and its timestamp
    match crate::world_state::tick_world_state(ctx, ctx.timestamp) {
        Ok(_) => { /* Time ticked successfully (or no update needed) */ }
        Err(e) => log::error!("Error ticking world state: {}", e),
    }

    // --- Check Resource Respawns --- using qualified path
    match crate::environment::check_resource_respawns(ctx) {
        Ok(_) => { /* Resources checked successfully */ }
        Err(e) => log::error!("Error checking resource respawns: {}", e),
    }
    
    // --- Check Campfire Fuel Consumption --- 
    match crate::campfire::check_campfire_fuel_consumption(ctx) {
        Ok(_) => { /* Campfire fuel checked successfully */ }
        Err(e) => log::error!("Error checking campfire fuel: {}", e),
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

       // Check if the player is already jumping (within cooldown)
       if player.jump_start_time_ms > 0 && now_ms < player.jump_start_time_ms + JUMP_COOLDOWN_MS {
           return Err("Cannot jump again so soon.".to_string());
       }

       // Proceed with the jump
       player.jump_start_time_ms = now_ms;
       player.last_update = ctx.timestamp;
       players.identity().update(player);
       Ok(())
   } else {
       Err("Player not found".to_string())
   }
} 

// --- Client-Requested Respawn Reducer ---
#[spacetimedb::reducer]
pub fn request_respawn(ctx: &ReducerContext) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let item_defs = ctx.db.item_definition(); // Keep for potential future use (e.g., dropping items)
    let inventory = ctx.db.inventory_item();

    // Find the player requesting respawn
    let mut player = players.identity().find(&sender_id)
        .ok_or_else(|| "Player not found".to_string())?;

    // Check if the player is actually dead
    if !player.is_dead {
        log::warn!("Player {:?} requested respawn but is not dead.", sender_id);
        return Err("You are not dead.".to_string());
    }

    // Check if the respawn timer is up
    if ctx.timestamp < player.respawn_at {
        log::warn!("Player {:?} requested respawn too early.", sender_id);
        let remaining_micros = player.respawn_at.to_micros_since_unix_epoch().saturating_sub(ctx.timestamp.to_micros_since_unix_epoch());
        let remaining_secs = (remaining_micros as f64 / 1_000_000.0).ceil() as u64;
        return Err(format!("Respawn available in {} seconds.", remaining_secs));
    }

    log::info!("Respawning player {} ({:?}). Clearing inventory...", player.username, sender_id);

    // --- Clear Player Inventory ---
    let mut items_to_delete = Vec::new();
    for item in inventory.iter().filter(|item| item.player_identity == sender_id) {
        items_to_delete.push(item.instance_id);
    }
    let delete_count = items_to_delete.len();
    for item_instance_id in items_to_delete {
        inventory.instance_id().delete(item_instance_id);
    }
    log::info!("Cleared {} items from inventory for player {:?}.", delete_count, sender_id);
    // --- End Clear Inventory ---

    // --- Grant Starting Rock ---
    log::info!("Granting starting Rock to respawned player: {}", player.username);
    if let Some(rock_def) = item_defs.iter().find(|def| def.name == "Rock") {
        match inventory.try_insert(crate::items::InventoryItem { // Qualify struct path
            instance_id: 0, // Auto-incremented
            player_identity: sender_id,
            item_def_id: rock_def.id,
            quantity: 1,
            hotbar_slot: Some(0), // Put rock in first slot
            inventory_slot: None,
        }) {
            Ok(_) => log::info!("Granted 1 Rock (slot 0) to player {}", player.username),
            Err(e) => log::error!("Failed to grant starting Rock to player {}: {}", player.username, e),
        }
    } else {
        log::error!("Could not find item definition for starting Rock!");
    }
    // --- End Grant Starting Rock ---

    // --- Reset Stats and State ---
    player.health = 100.0;
    player.hunger = 100.0;
    player.thirst = 100.0;
    player.warmth = 100.0;
    player.stamina = 100.0;
    player.jump_start_time_ms = 0;
    player.is_sprinting = false;
    player.is_dead = false; // Mark as alive again
    player.last_hit_time = None; 

    // --- Reset Position ---
    let spawn_x = 640.0; // Simple initial spawn point
    let spawn_y = 480.0;
    player.position_x = spawn_x;
    player.position_y = spawn_y;
    player.direction = "down".to_string();

    // --- Update Timestamp ---
    player.last_update = ctx.timestamp;

    // --- Apply Player Changes ---
    players.identity().update(player);
    log::info!("Player {:?} respawned at ({:.1}, {:.1}).", sender_id, spawn_x, spawn_y);

    // Unequip item on respawn (ensure clean state)
    match active_equipment::unequip_item(ctx) {
        Ok(_) => log::info!("Ensured item is unequipped for respawned player {:?}", sender_id),
        Err(e) => log::error!("Failed to unequip item for respawned player {:?}: {}", sender_id, e),
    }

    Ok(())
} 