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
mod dropped_item; // Declare dropped_item module
mod wooden_storage_box; // Add the new module
mod items_database; // <<< ADDED module declaration
mod starting_items; // <<< ADDED module declaration
mod inventory_management; // <<< ADDED new module
mod spatial_grid; // ADD: Spatial grid module for optimized collision detection
mod crafting; // ADD: Crafting recipe definitions
mod crafting_queue; // ADD: Crafting queue logic
mod player_stats; // ADD: Player stat scheduling logic
mod global_tick; // ADD: Global tick scheduling logic

// Import Table Traits needed in this module
use crate::tree::tree as TreeTableTrait;
use crate::stone::stone as StoneTableTrait;
use crate::campfire::campfire as CampfireTableTrait;
use crate::world_state::world_state as WorldStateTableTrait;
use crate::items::inventory_item as InventoryItemTableTrait;
use crate::items::item_definition as ItemDefinitionTableTrait;
use crate::active_equipment::active_equipment as ActiveEquipmentTableTrait;
use crate::dropped_item::dropped_item_despawn_schedule as DroppedItemDespawnScheduleTableTrait;
use crate::campfire::campfire_fuel_check_schedule as CampfireFuelCheckScheduleTableTrait;
use crate::wooden_storage_box::wooden_storage_box as WoodenStorageBoxTableTrait;

// Use struct names directly for trait aliases
use crate::crafting::Recipe as RecipeTableTrait;
use crate::crafting_queue::CraftingQueueItem as CraftingQueueItemTableTrait;
use crate::crafting_queue::CraftingFinishSchedule as CraftingFinishScheduleTableTrait;
use crate::global_tick::GlobalTickSchedule as GlobalTickScheduleTableTrait;

// Import constants needed from player_stats
use crate::player_stats::{
    SPRINT_SPEED_MULTIPLIER,
    JUMP_COOLDOWN_MS,
    LOW_NEED_THRESHOLD,
    LOW_THIRST_SPEED_PENALTY,
    LOW_WARMTH_SPEED_PENALTY
};

// Use specific items needed globally (or use qualified paths)
use crate::world_state::TimeOfDay; // Keep TimeOfDay if needed elsewhere, otherwise remove
use crate::campfire::{Campfire, WARMTH_RADIUS_SQUARED, WARMTH_PER_SECOND, CAMPFIRE_COLLISION_RADIUS, CAMPFIRE_CAMPFIRE_COLLISION_DISTANCE_SQUARED, CAMPFIRE_COLLISION_Y_OFFSET, PLAYER_CAMPFIRE_COLLISION_DISTANCE_SQUARED, PLAYER_CAMPFIRE_INTERACTION_DISTANCE_SQUARED };

// --- Global Constants ---
pub const TILE_SIZE_PX: u32 = 48;
pub const PLAYER_RADIUS: f32 = 32.0; // Player collision radius
pub const PLAYER_SPEED: f32 = 300.0; // Speed in pixels per second
pub const PLAYER_SPRINT_MULTIPLIER: f32 = 1.6;

// World Dimensions (example)
pub const WORLD_WIDTH_TILES: u32 = 100;
pub const WORLD_HEIGHT_TILES: u32 = 100;
// Change back to f32 as they are used in float calculations
pub const WORLD_WIDTH_PX: f32 = (WORLD_WIDTH_TILES * TILE_SIZE_PX) as f32;
pub const WORLD_HEIGHT_PX: f32 = (WORLD_HEIGHT_TILES * TILE_SIZE_PX) as f32;

// Campfire Placement Constants (Restored)
pub const CAMPFIRE_PLACEMENT_MAX_DISTANCE: f32 = 96.0;
pub const CAMPFIRE_PLACEMENT_MAX_DISTANCE_SQUARED: f32 = CAMPFIRE_PLACEMENT_MAX_DISTANCE * CAMPFIRE_PLACEMENT_MAX_DISTANCE;

// Respawn Collision Check Constants
pub const RESPAWN_CHECK_RADIUS: f32 = TILE_SIZE_PX as f32 * 0.8; // Check slightly less than a tile radius
pub const RESPAWN_CHECK_RADIUS_SQ: f32 = RESPAWN_CHECK_RADIUS * RESPAWN_CHECK_RADIUS;
pub const MAX_RESPAWN_OFFSET_ATTEMPTS: u32 = 8; // Max times to try offsetting
pub const RESPAWN_OFFSET_DISTANCE: f32 = TILE_SIZE_PX as f32 * 0.5; // How far to offset each attempt

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
    pub last_update: Timestamp, // Timestamp of the last update (movement or stats)
    pub last_stat_update: Timestamp, // Timestamp of the last stat processing tick
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

    // Initialize the dropped item despawn schedule
    crate::dropped_item::init_dropped_item_schedule(ctx)?;
    // Initialize the campfire fuel check schedule
    crate::campfire::init_campfire_fuel_schedule(ctx)?;
    // Initialize the crafting finish check schedule
    crate::crafting_queue::init_crafting_schedule(ctx)?;
    // ADD: Initialize the player stat update schedule
    crate::player_stats::init_player_stat_schedule(ctx)?;
    // ADD: Initialize the global tick schedule
    crate::global_tick::init_global_tick_schedule(ctx)?;

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
    crate::crafting::seed_recipes(ctx)?; // Seed the crafting recipes
    // No seeder needed for Campfire yet, table will be empty initially

    // Note: Initial scheduling for player stats happens in register_player
    // Note: Initial scheduling for global ticks happens in init_module
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
        // 1. Delete the Player entity
        players.identity().delete(sender_id);
        log::info!("Deleted Player entity for disconnected player: {} ({:?})", username, sender_id);

        // 2. Delete player's inventory items (ONLY those in main inventory or hotbar)
        let inventory = ctx.db.inventory_item();
        let mut items_to_delete = Vec::new();
        for item in inventory.iter().filter(|i| i.player_identity == sender_id) {
            // Only delete if actually in inventory/hotbar
            if item.inventory_slot.is_some() || item.hotbar_slot.is_some() {
                items_to_delete.push(item.instance_id);
            }
        }
        let delete_count = items_to_delete.len();
        for item_instance_id in items_to_delete {
            inventory.instance_id().delete(item_instance_id);
        }
        log::info!("Deleted {} inventory items for player {:?}", delete_count, sender_id);

        // 3. Delete player's active equipment entry
        let equipment_table = ctx.db.active_equipment();
        if equipment_table.player_identity().find(&sender_id).is_some() {
            equipment_table.player_identity().delete(sender_id);
            log::info!("Deleted active equipment for player {:?}", sender_id);
        }

        // 4. Clear player's crafting queue and refund resources
        crate::crafting_queue::clear_player_crafting_queue(ctx, sender_id);

    } else {
        log::warn!("Disconnected identity {:?} did not have a registered player entity. No cleanup needed.", sender_id);
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
    let wooden_storage_boxes = ctx.db.wooden_storage_box(); // <<< ADDED: Get box table

    // Check if username is already taken by *any* player
    let username_taken = players.iter().any(|p| p.username == username);
    if username_taken {
        log::warn!("Username '{}' already taken. Registration failed for {:?}.", username, sender_id);
        return Err(format!("Username '{}' is already taken.", username));
    }

    // Check if this identity is already registered (shouldn't happen if disconnect works, but good safety check)
    if players.identity().find(&sender_id).is_some() {
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
             // Don't collide with dead players during spawn
            if other_player.is_dead { continue; }
            let dx = spawn_x - other_player.position_x;
            let dy = spawn_y - other_player.position_y;
            if (dx * dx + dy * dy) < PLAYER_RADIUS * PLAYER_RADIUS {
                collision = true;
                break;
            }
        }

        // 2. Check Player-Tree Collision (if no player collision)
        if !collision {
            for tree in trees.iter() {
                 // Don't collide with felled trees
                if tree.health == 0 { continue; }
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
                // Don't collide with depleted stones
                if stone.health == 0 { continue; }
                let dx = spawn_x - stone.pos_x;
                let dy = spawn_y - (stone.pos_y - crate::stone::STONE_COLLISION_Y_OFFSET); // Already qualified
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < crate::stone::PLAYER_STONE_COLLISION_DISTANCE_SQUARED { // Already qualified
                    collision = true;
                    break;
                }
            }
        }

        // 2.7 Check Player-Campfire Collision (Allow spawning on campfires)
        // if !collision {
        //     for fire in campfires.iter() {
        //         let dx = spawn_x - fire.pos_x;
        //         let dy = spawn_y - (fire.pos_y - CAMPFIRE_COLLISION_Y_OFFSET);
        //         let dist_sq = dx * dx + dy * dy;
        //         // Use specific player-campfire collision check distance
        //         if dist_sq < PLAYER_CAMPFIRE_COLLISION_DISTANCE_SQUARED {
        //             collision = true;
        //             break;
        //         }
        //     }
        // }

        // 2.8 Check Player-WoodenStorageBox Collision <<< ADDED Check
        if !collision {
            for box_instance in wooden_storage_boxes.iter() {
                // Use constants from wooden_storage_box module
                let dx = spawn_x - box_instance.pos_x;
                let dy = spawn_y - (box_instance.pos_y - crate::wooden_storage_box::BOX_COLLISION_Y_OFFSET);
                let dist_sq = dx * dx + dy * dy;
                // Use specific player-box collision check distance
                if dist_sq < crate::wooden_storage_box::PLAYER_BOX_COLLISION_DISTANCE_SQUARED {
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
        last_update: ctx.timestamp, // Set initial timestamp
        last_stat_update: ctx.timestamp, // Initialize stat timestamp
        jump_start_time_ms: 0,
        health: 100.0,
        stamina: 100.0,
        thirst: 100.0,
        hunger: 100.0,
        warmth: 100.0,
        is_sprinting: false,
        is_dead: false,
        respawn_at: ctx.timestamp, // Set initial respawn time (not dead yet)
        last_hit_time: None,
    };

    // Insert the new player
    match players.try_insert(player) {
        Ok(_) => {
            log::info!("Player registered: {}. Granting starting items...", username);

            // --- Grant Starting Items ---
            // Call the dedicated function from the starting_items module
            match crate::starting_items::grant_starting_items(ctx, sender_id, &username) {
                Ok(_) => { /* Items granted (or individual errors logged) */ },
                Err(e) => {
                    // This function currently always returns Ok, but handle error just in case
                    log::error!("Unexpected error during grant_starting_items for player {}: {}", username, e);
                    // Potentially return the error from register_player if item grant failure is critical
                    // return Err(format!("Failed to grant starting items: {}", e));
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
pub fn place_campfire(ctx: &ReducerContext, item_instance_id: u64, world_x: f32, world_y: f32) -> Result<(), String> {
    let sender_id = ctx.sender;
    let inventory_items = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();
    let players = ctx.db.player();
    let campfires = ctx.db.campfire();

    log::info!(
        "[PlaceCampfire] Player {:?} attempting placement of item {} at ({:.1}, {:.1})",
        sender_id, item_instance_id, world_x, world_y
    );

    // --- 1. Validate Player and Placement Rules ---
    let player = players.identity().find(sender_id)
        .ok_or_else(|| "Player not found".to_string())?;

    // Check distance from player
    let dx_place = world_x - player.position_x;
    let dy_place = world_y - player.position_y;
    let dist_sq_place = dx_place * dx_place + dy_place * dy_place;
    if dist_sq_place > CAMPFIRE_PLACEMENT_MAX_DISTANCE_SQUARED {
        return Err(format!("Cannot place campfire too far away ({} > {}).",
                dist_sq_place.sqrt(), CAMPFIRE_PLACEMENT_MAX_DISTANCE));
    }

    // Check collision with other campfires
    for other_fire in campfires.iter() {
        let dx_fire = world_x - other_fire.pos_x;
        let dy_fire = world_y - other_fire.pos_y;
        let dist_sq_fire = dx_fire * dx_fire + dy_fire * dy_fire;
        if dist_sq_fire < CAMPFIRE_CAMPFIRE_COLLISION_DISTANCE_SQUARED {
            return Err("Cannot place campfire too close to another campfire.".to_string());
        }
    }
    // Add more collision checks here if needed (e.g., vs trees, stones)

    // --- 2. Find the "Camp Fire" item definition ---
    let campfire_def_id = item_defs.iter()
        .find(|def| def.name == "Camp Fire")
        .map(|def| def.id)
        .ok_or_else(|| "Item definition 'Camp Fire' not found.".to_string())?;

    // --- 3. Find the specific item instance and validate ---
    let item_to_consume = inventory_items.instance_id().find(item_instance_id)
        .ok_or_else(|| format!("Item instance {} not found.", item_instance_id))?;

    // Validate ownership
    if item_to_consume.player_identity != sender_id {
        return Err(format!("Item instance {} not owned by player {:?}.", item_instance_id, sender_id));
    }
    // Validate item type
    if item_to_consume.item_def_id != campfire_def_id {
        return Err(format!("Item instance {} is not a Camp Fire (expected def {}, got {}).",
                        item_instance_id, campfire_def_id, item_to_consume.item_def_id));
    }
    // Validate location (must be in inv or hotbar)
    if item_to_consume.inventory_slot.is_none() && item_to_consume.hotbar_slot.is_none() {
        return Err(format!("Item instance {} must be in inventory or hotbar to be placed.", item_instance_id));
    }

    // Use the validated item_instance_id directly
    let item_instance_id_to_delete = item_instance_id;

    // --- 4. Consume the Item ---
    log::info!(
        "[PlaceCampfire] Consuming item instance {} (Def ID: {}) from player {:?}",
        item_instance_id_to_delete, campfire_def_id, sender_id
    );
    inventory_items.instance_id().delete(item_instance_id_to_delete);

    // --- 5. Create Campfire Entity ---
    // --- 5a. Create Initial Fuel Item (Wood) ---
    let wood_def = item_defs.iter()
        .find(|def| def.name == "Wood")
        .ok_or_else(|| "Wood item definition not found for initial fuel".to_string())?;

    let initial_fuel_item = crate::items::InventoryItem {
        instance_id: 0, // Auto-inc
        player_identity: sender_id, // Belongs to the placer initially (needed? maybe not)
        item_def_id: wood_def.id,
        quantity: 50, // Start with 50 wood
        hotbar_slot: None, // Not in hotbar
        inventory_slot: None, // Not in inventory (it's "in" the campfire slot 0)
    };
    // Insert the fuel item and get its generated instance ID
    let inserted_fuel_item = inventory_items.try_insert(initial_fuel_item)
        .map_err(|e| format!("Failed to insert initial fuel item: {}", e))?;
    let fuel_instance_id = inserted_fuel_item.instance_id;
    log::info!("[PlaceCampfire] Created initial fuel item (Wood, instance {}) for campfire.", fuel_instance_id);

    // --- 5b. Initialize Campfire with Fuel and Burning ---
    let current_time = ctx.timestamp;
    // Use constant from campfire module
    let first_consumption_time = current_time + Duration::from_secs(crate::campfire::FUEL_CONSUME_INTERVAL_SECS).into();

    // Initialize all fields explicitly
    let new_campfire = crate::campfire::Campfire {
        id: 0, // Auto-incremented
        pos_x: world_x,
        pos_y: world_y,
        placed_by: sender_id,
        placed_at: ctx.timestamp,
        is_burning: true, // Start burning
        // Initialize all fuel slots to None
        fuel_instance_id_0: Some(fuel_instance_id), // Add the wood
        fuel_def_id_0: Some(wood_def.id),
        fuel_instance_id_1: None,
        fuel_def_id_1: None,
        fuel_instance_id_2: None,
        fuel_def_id_2: None,
        fuel_instance_id_3: None,
        fuel_def_id_3: None,
        fuel_instance_id_4: None,
        fuel_def_id_4: None,
        next_fuel_consume_at: Some(first_consumption_time), // Schedule consumption
    };

    campfires.try_insert(new_campfire)
        .map_err(|e| format!("Failed to insert campfire: {}", e))?;
    log::info!("Player {} placed a campfire at ({:.1}, {:.1}) with initial fuel (Item {} in slot 0).",
             player.username, world_x, world_y, fuel_instance_id);

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
            player.last_update = ctx.timestamp; // Update timestamp when sprint state changes
            players.identity().update(player);
            log::debug!("Player {:?} set sprinting to {}", sender_id, sprinting);
        }
        Ok(())
    } else {
        Err("Player not found".to_string())
    }
}

// Update player movement, handle sprinting, and collision
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
    let wooden_storage_boxes = ctx.db.wooden_storage_box(); // <<< ADDED

    let current_player = players.identity()
        .find(sender_id)
        .ok_or_else(|| "Player not found".to_string())?;

    // --- If player is dead, prevent movement ---
    if current_player.is_dead {
        log::trace!("Ignoring movement input for dead player {:?}", sender_id);
        return Ok(()); // Do nothing if dead
    }

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
        if move_dx.abs() > move_dy.abs() {
            new_direction = if move_dx > 0.0 { "right".to_string() } else { "left".to_string() };
        } else if move_dy != 0.0 {
            new_direction = if move_dy > 0.0 { "down".to_string() } else { "up".to_string() };
        }
        if current_player.direction != new_direction {
            log::trace!("Player {:?} direction inferred from movement: {}", sender_id, new_direction);
        }
    }
    // --- End Direction Update ---

    let now = ctx.timestamp;
    // REMOVED: Elapsed time calculation for stats (moved to player_stats.rs)
    // REMOVED: Hunger/Thirst drain (moved to player_stats.rs)
    // REMOVED: Warmth calculation (moved to player_stats.rs)

    // --- Stamina Drain & Base Speed Calculation ---
    let mut new_stamina = current_player.stamina;
    let mut base_speed_multiplier = 1.0;
    let is_moving = move_dx != 0.0 || move_dy != 0.0;
    let mut current_sprinting_state = current_player.is_sprinting;

    // Determine speed multiplier based on current sprint state and stamina
    if current_sprinting_state && new_stamina > 0.0 { // Check current stamina > 0
        base_speed_multiplier = SPRINT_SPEED_MULTIPLIER;
    } else if current_sprinting_state && new_stamina <= 0.0 {
        // If trying to sprint but no stamina, force sprint state off for this tick's movement calc
        current_sprinting_state = false;
        base_speed_multiplier = 1.0; // Use base speed
        // The actual player.is_sprinting state will be forced off in player_stats.rs
    }

    // --- Calculate Final Speed Multiplier based on Current Stats ---
    let mut final_speed_multiplier = base_speed_multiplier;
    // Use current player stats read at the beginning of the reducer
    if current_player.thirst < LOW_NEED_THRESHOLD {
        final_speed_multiplier *= LOW_THIRST_SPEED_PENALTY;
        if is_moving {
            log::debug!("Player {:?} has low thirst. Applying speed penalty.", sender_id);
        }
    }
    if current_player.warmth < LOW_NEED_THRESHOLD {
        final_speed_multiplier *= LOW_WARMTH_SPEED_PENALTY;
        if is_moving {
            log::debug!("Player {:?} is cold. Applying speed penalty.", sender_id);
        }
    }

    // REMOVED: Health Update Calculation (moved to player_stats.rs)
    // REMOVED: Death Check (moved to player_stats.rs)

    // --- Movement Calculation ---
    let proposed_x = current_player.position_x + move_dx * final_speed_multiplier;
    let proposed_y = current_player.position_y + move_dy * final_speed_multiplier;

    let clamped_x = proposed_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
    let clamped_y = proposed_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);

    let mut final_x = clamped_x;
    let mut final_y = clamped_y;
    let mut collision_handled = false;

    // --- Collision Detection (using spatial grid) ---
    let mut grid = spatial_grid::SpatialGrid::new();
    grid.populate_from_world(&ctx.db);
    let nearby_entities = grid.get_entities_in_range(clamped_x, clamped_y);

    // Check collisions with nearby entities (Slide calculation)
    for entity in &nearby_entities {
        match entity {
            spatial_grid::EntityType::Player(other_identity) => {
                if *other_identity == sender_id { continue; } // Skip self
                 // Find the player in the database
                if let Some(other_player) = players.identity().find(other_identity) {
                    // Don't collide with dead players
                    if other_player.is_dead { continue; }

                    let dx = clamped_x - other_player.position_x;
                    let dy = clamped_y - other_player.position_y;
                    let dist_sq = dx * dx + dy * dy;

                    if dist_sq < PLAYER_RADIUS * PLAYER_RADIUS {
                        log::debug!("Player-Player collision detected between {:?} and {:?}. Calculating slide.", sender_id, other_player.identity);
                        // Slide calculation (same as before)
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
                        break;
                    }
                }
            },
            spatial_grid::EntityType::Tree(tree_id) => {
                 if collision_handled { continue; }
                 if let Some(tree) = trees.id().find(tree_id) {
                    if tree.health == 0 { continue; }
                    let tree_collision_y = tree.pos_y - crate::tree::TREE_COLLISION_Y_OFFSET;
                    let dx = clamped_x - tree.pos_x;
                    let dy = clamped_y - tree_collision_y;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq < crate::tree::PLAYER_TREE_COLLISION_DISTANCE_SQUARED {
                         log::debug!("Player-Tree collision detected between {:?} and tree {}. Calculating slide.", sender_id, tree.id);
                         // Slide calculation (same as before)
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
                    }
                }
            },
            spatial_grid::EntityType::Stone(stone_id) => {
                 if collision_handled { continue; }
                 if let Some(stone) = stones.id().find(stone_id) {
                     if stone.health == 0 { continue; }
                     let stone_collision_y = stone.pos_y - crate::stone::STONE_COLLISION_Y_OFFSET;
                     let dx = clamped_x - stone.pos_x;
                     let dy = clamped_y - stone_collision_y;
                     let dist_sq = dx * dx + dy * dy;
                     if dist_sq < crate::stone::PLAYER_STONE_COLLISION_DISTANCE_SQUARED {
                         log::debug!("Player-Stone collision detected between {:?} and stone {}. Calculating slide.", sender_id, stone.id);
                         // Slide calculation (same as before)
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
                     }
                 }
            },
            spatial_grid::EntityType::WoodenStorageBox(box_id) => {
                if collision_handled { continue; }
                if let Some(box_instance) = wooden_storage_boxes.id().find(box_id) {
                    let box_collision_y = box_instance.pos_y - crate::wooden_storage_box::BOX_COLLISION_Y_OFFSET;
                    let dx = clamped_x - box_instance.pos_x;
                    let dy = clamped_y - box_collision_y;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq < crate::wooden_storage_box::PLAYER_BOX_COLLISION_DISTANCE_SQUARED {
                         log::debug!("Player-Box collision detected between {:?} and box {}. Calculating slide.", sender_id, box_instance.id);
                         // Slide calculation (same as before)
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
                    }
                }
            },
             spatial_grid::EntityType::Campfire(_) => {
                // No collision with campfires
             },
            _ => {} // Ignore other types for collision
        }
    }

    // --- Iterative Collision Resolution (Push-out) ---
    let mut resolved_x = final_x;
    let mut resolved_y = final_y;
    let resolution_iterations = 5;
    let epsilon = 0.01;

    for _iter in 0..resolution_iterations {
        let mut overlap_found_in_iter = false;
        let nearby_entities_resolve = grid.get_entities_in_range(resolved_x, resolved_y); // Re-query near resolved position

        for entity in &nearby_entities_resolve {
             match entity {
                 spatial_grid::EntityType::Player(other_identity) => {
                    if *other_identity == sender_id { continue; }
                    if let Some(other_player) = players.identity().find(other_identity) {
                         if other_player.is_dead { continue; } // Don't resolve against dead players
                         let dx = resolved_x - other_player.position_x;
                         let dy = resolved_y - other_player.position_y;
                         let dist_sq = dx * dx + dy * dy;
                         let min_dist = PLAYER_RADIUS * 2.0;
                         let min_dist_sq = min_dist * min_dist;
                         if dist_sq < min_dist_sq && dist_sq > 0.0 {
                             overlap_found_in_iter = true;
                             let distance = dist_sq.sqrt();
                             let overlap = min_dist - distance;
                             let push_amount = (overlap / 2.0) + epsilon;
                             let push_x = (dx / distance) * push_amount;
                             let push_y = (dy / distance) * push_amount;
                             resolved_x += push_x;
                             resolved_y += push_y;
                         }
                    }
                },
                 spatial_grid::EntityType::Tree(tree_id) => {
                     if let Some(tree) = trees.id().find(tree_id) {
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
                         }
                     }
                },
                 spatial_grid::EntityType::Stone(stone_id) => {
                    if let Some(stone) = stones.id().find(stone_id) {
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
                        }
                    }
                },
                 spatial_grid::EntityType::WoodenStorageBox(box_id) => {
                     if let Some(box_instance) = wooden_storage_boxes.id().find(box_id) {
                         let box_collision_y = box_instance.pos_y - crate::wooden_storage_box::BOX_COLLISION_Y_OFFSET;
                         let dx = resolved_x - box_instance.pos_x;
                         let dy = resolved_y - box_collision_y;
                         let dist_sq = dx * dx + dy * dy;
                         let min_dist = PLAYER_RADIUS + crate::wooden_storage_box::BOX_COLLISION_RADIUS;
                         let min_dist_sq = min_dist * min_dist;
                         if dist_sq < min_dist_sq && dist_sq > 0.0 {
                             overlap_found_in_iter = true;
                             let distance = dist_sq.sqrt();
                             let overlap = (min_dist - distance) + epsilon;
                             let push_x = (dx / distance) * overlap;
                             let push_y = (dy / distance) * overlap;
                             resolved_x += push_x;
                             resolved_y += push_y;
                         }
                     }
                },
                 spatial_grid::EntityType::Campfire(_) => {
                     // No overlap resolution with campfires
                 },
                _ => {}
             }
        }

        resolved_x = resolved_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
        resolved_y = resolved_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);

        if !overlap_found_in_iter {
            log::trace!("Overlap resolution complete after {} iterations.", _iter + 1);
            break;
        }
        if _iter == resolution_iterations - 1 {
            log::warn!("Overlap resolution reached max iterations ({}) for player {:?}. Position might still overlap slightly.", resolution_iterations, sender_id);
        }
    }
    // --- End Collision ---

    // --- Final Update ---
    let mut player_to_update = current_player; // Get a mutable copy from the initial read

    // Check if position, direction, stamina, or sprint status actually changed
    let position_changed = (resolved_x - player_to_update.position_x).abs() > 0.01 ||
                           (resolved_y - player_to_update.position_y).abs() > 0.01;
    let direction_changed = player_to_update.direction != new_direction;
    let stamina_changed = (player_to_update.stamina - new_stamina).abs() > 0.01;
    let sprint_status_changed = player_to_update.is_sprinting != current_sprinting_state;

    let should_update = position_changed || direction_changed || stamina_changed || sprint_status_changed;

    // --- Determine if timestamp needs updating --- 
    // Update timestamp if state changed OR if there was movement input
    let needs_timestamp_update = should_update || (move_dx != 0.0 || move_dy != 0.0);

    if should_update {
        log::trace!("Updating player {:?} - PosChange: {}, DirChange: {}, StamChange: {}, SprintChange: {}",
            sender_id, position_changed, direction_changed, stamina_changed, sprint_status_changed);

        player_to_update.position_x = resolved_x;
        player_to_update.position_y = resolved_y;
        player_to_update.direction = new_direction;
        player_to_update.last_update = now; // Update timestamp for movement processing
        // player_to_update.stamina = new_stamina; // DO NOT update stamina here
        // player_to_update.is_sprinting = current_sprinting_state; // DO NOT update sprint state here (handled by player_stats)
        // last_hit_time is managed elsewhere (e.g., when taking damage)

        players.identity().update(player_to_update); // Update the modified player struct
    } else if needs_timestamp_update { // If no state changed, but movement input occurred
         log::trace!("No movement-related state changes detected for player {:?}, but updating timestamp due to movement input.", sender_id);
         // Update only the timestamp
         // Fetch the player again to avoid overwriting potential stat changes from other reducers? Or just update timestamp on the copy?
         // Let's update the timestamp on the copy we already have.
         player_to_update.last_update = now;
         players.identity().update(player_to_update);
    } else {
         log::trace!("No state changes or movement input detected for player {:?}, skipping update.", sender_id);
    }

    // REMOVED: World State Tick (moved to global_tick.rs)
    // REMOVED: Resource Respawn Check (moved to global_tick.rs)

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
       // Don't allow jumping if dead
       if player.is_dead {
           return Err("Cannot jump while dead.".to_string());
       }

       let now_micros = ctx.timestamp.to_micros_since_unix_epoch();
       let now_ms = (now_micros / 1000) as u64;

       // Check if the player is already jumping (within cooldown)
       if player.jump_start_time_ms > 0 && now_ms < player.jump_start_time_ms + JUMP_COOLDOWN_MS {
           return Err("Cannot jump again so soon.".to_string());
       }

       // Proceed with the jump
       player.jump_start_time_ms = now_ms;
       player.last_update = ctx.timestamp; // Update timestamp on jump
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
    let item_defs = ctx.db.item_definition();
    let inventory = ctx.db.inventory_item();

    // Find the player requesting respawn
    let mut player = players.identity().find(&sender_id)
        .ok_or_else(|| "Player not found".to_string())?;

    // Check if the player is actually dead
    if !player.is_dead {
        log::warn!("Player {:?} requested respawn but is not dead.", sender_id);
        return Err("You are not dead.".to_string());
    }

    // Check if the respawn timer is up (uses respawn_at set by player_stats reducer)
    if ctx.timestamp < player.respawn_at {
        log::warn!("Player {:?} requested respawn too early.", sender_id);
        let remaining_micros = player.respawn_at.to_micros_since_unix_epoch().saturating_sub(ctx.timestamp.to_micros_since_unix_epoch());
        let remaining_secs = (remaining_micros as f64 / 1_000_000.0).ceil() as u64;
        return Err(format!("Respawn available in {} seconds.", remaining_secs));
    }

    log::info!("Respawning player {} ({:?}). Clearing inventory and crafting queue...", player.username, sender_id);

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

    // --- Clear Crafting Queue & Refund ---
    crate::crafting_queue::clear_player_crafting_queue(ctx, sender_id);
    // --- END Clear Crafting Queue ---

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

    // --- Reset Position (Consider finding a safe spawn instead of fixed coords) ---
    // TODO: Implement safe spawn finding logic here, similar to register_player
    let spawn_x = 640.0; // Simple initial spawn point for now
    let spawn_y = 480.0;
    player.position_x = spawn_x;
    player.position_y = spawn_y;
    player.direction = "down".to_string();

    // --- Update Timestamp ---
    player.last_update = ctx.timestamp;
    player.last_stat_update = ctx.timestamp; // Reset stat timestamp on respawn

    // --- Apply Player Changes ---
    players.identity().update(player);
    log::info!("Player {:?} respawned at ({:.1}, {:.1}).", sender_id, spawn_x, spawn_y);

    // Ensure item is unequipped on respawn
    match active_equipment::unequip_item(ctx, sender_id) {
        Ok(_) => log::info!("Ensured item is unequipped for respawned player {:?}", sender_id),
        Err(e) => log::error!("Failed to unequip item for respawned player {:?}: {}", sender_id, e),
    }

    Ok(())
} 