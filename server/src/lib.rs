use spacetimedb::{Identity, Timestamp, ReducerContext, Table};
use log;
use std::time::Duration;
use spacetimedb::rand::rngs::StdRng;
use spacetimedb::rand::SeedableRng;
use crate::environment::calculate_chunk_index; // Make sure this helper is available

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
// Declare the player_inventory module
mod player_inventory;
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
mod chat;
mod character;
mod enemy;
mod buff;
mod player_pin; // ADD: Player pin module for minimap

// Re-export chat types and reducers for use in other modules
pub use chat::Message;

// Import Table Traits needed in this module
use crate::tree::tree as TreeTableTrait;
use crate::stone::stone as StoneTableTrait;
use crate::campfire::campfire as CampfireTableTrait;
use crate::world_state::world_state as WorldStateTableTrait;
use crate::items::inventory_item as InventoryItemTableTrait;
use crate::items::item_definition as ItemDefinitionTableTrait;
use crate::active_equipment::active_equipment as ActiveEquipmentTableTrait;
use crate::buff::{buff as BuffTableTrait, Buff};
use crate::dropped_item::dropped_item_despawn_schedule as DroppedItemDespawnScheduleTableTrait;
use crate::campfire::campfire_fuel_check_schedule as CampfireFuelCheckScheduleTableTrait;
use crate::wooden_storage_box::wooden_storage_box as WoodenStorageBoxTableTrait;
use crate::chat::message as MessageTableTrait; // Import the trait for Message table

// Use struct names directly for trait aliases
use crate::crafting::Recipe as RecipeTableTrait;
use crate::crafting_queue::CraftingQueueItem as CraftingQueueItemTableTrait;
use crate::crafting_queue::CraftingFinishSchedule as CraftingFinishScheduleTableTrait;
use crate::global_tick::GlobalTickSchedule as GlobalTickScheduleTableTrait;

// Import constants needed from player_stats
use crate::player_stats::{SPRINT_SPEED_MULTIPLIER, JUMP_COOLDOWN_MS, LOW_NEED_THRESHOLD, LOW_THIRST_SPEED_PENALTY, LOW_WARMTH_SPEED_PENALTY, BASE_EXP_TO_LEVEL, EXP_TO_LEVEL_MULTIPLIER, player_stats as PlayerStatsTableTrait};

// Use specific items needed globally (or use qualified paths)
use crate::world_state::TimeOfDay; // Keep TimeOfDay if needed elsewhere, otherwise remove
use crate::campfire::{Campfire, WARMTH_RADIUS_SQUARED, WARMTH_PER_SECOND, CAMPFIRE_COLLISION_RADIUS, CAMPFIRE_CAMPFIRE_COLLISION_DISTANCE_SQUARED, CAMPFIRE_COLLISION_Y_OFFSET, PLAYER_CAMPFIRE_COLLISION_DISTANCE_SQUARED, PLAYER_CAMPFIRE_INTERACTION_DISTANCE_SQUARED };

// --- Global Constants ---
pub const TILE_SIZE_PX: u32 = 48;
pub const PLAYER_RADIUS: f32 = 32.0; // Player collision radius
pub const PLAYER_SPEED: f32 = 600.0; // Speed in pixels per second
pub const PLAYER_SPRINT_MULTIPLIER: f32 = 1.6;

// World Dimensions (example)
pub const WORLD_WIDTH_TILES: u32 = 500;
pub const WORLD_HEIGHT_TILES: u32 = 500;
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
#[spacetimedb::table(
    name = player,
    public,
    // Add spatial index
    index(name = idx_player_pos, btree(columns = [position_x, position_y]))
)]
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
    pub is_sprinting: bool,
    pub is_dead: bool,
    pub respawn_at: Timestamp,
    pub last_hit_time: Option<Timestamp>,
}

// --- NEW: Define ClientViewport Table ---
#[spacetimedb::table(name = client_viewport)]
#[derive(Clone, Debug)]
pub struct ClientViewport {
    #[primary_key]
    client_identity: Identity,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    last_update: Timestamp,
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

        // --- NEW: Delete ClientViewport entry ---
        let viewports = ctx.db.client_viewport();
        if viewports.client_identity().find(&sender_id).is_some() {
            viewports.client_identity().delete(sender_id);
            log::info!("Deleted client viewport for player {:?}", sender_id);
        }
        // --- End NEW ---

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
        is_sprinting: false,
        is_dead: false,
        respawn_at: ctx.timestamp, // Set initial respawn time (not dead yet)
        last_hit_time: None,
    };

    // Insert the new player
    match players.try_insert(player) {
        Ok(_) => {
            log::info!("Player registered: {}. Initializing systems...", username);

            // Initialize player stats
            match crate::player_stats::initialize_player_stats(ctx, sender_id) {
                Ok(_) => log::info!("Player stats initialized for {}", username),
                Err(e) => log::error!("Failed to initialize player stats: {}", e),
            }

            // Grant starting items
            match crate::starting_items::grant_starting_items(ctx, sender_id, &username) {
                Ok(_) => log::info!("Starting items granted to {}", username),
                Err(e) => log::error!("Failed to grant starting items: {}", e),
            }

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
    let first_consumption_time = current_time + Duration::from_secs(crate::campfire::FUEL_CONSUME_INTERVAL_SECS);

    // --- ADD: Calculate chunk index ---
    let chunk_idx = calculate_chunk_index(world_x, world_y);
    // --- END ADD ---

    // Initialize all fields explicitly
    let new_campfire = crate::campfire::Campfire {
        id: 0, // Auto-incremented
        pos_x: world_x,
        pos_y: world_y,
        chunk_index: chunk_idx, // <<< SET chunk_index HERE
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
    // Renamed parameters to represent normalized direction vector from client
    move_x: f32,
    move_y: f32,
) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let players_stats = ctx.db.player_stats();
    let trees = ctx.db.tree();
    let stones = ctx.db.stone();
    let campfires = ctx.db.campfire(); // Get campfire table
    let wooden_storage_boxes = ctx.db.wooden_storage_box(); // <<< ADDED

    let current_player = players.identity()
        .find(sender_id)
        .ok_or_else(|| "Player not found".to_string())?;

    let current_player_stats = players_stats.player_id().find(sender_id).ok_or_else(|| "Player stats not found")?;

    // --- If player is dead, prevent movement ---
    if current_player.is_dead {
        log::trace!("Ignoring movement input for dead player {:?}", sender_id);
        return Ok(()); // Do nothing if dead
    }

    // --- Determine Animation Direction from Input Vector ---
    let mut final_anim_direction = current_player.direction.clone();
    // Basic check: If there's significant movement
    if move_x.abs() > 0.01 || move_y.abs() > 0.01 {
        // Prioritize horizontal or vertical based on magnitude
        if move_x.abs() > move_y.abs() {
            final_anim_direction = if move_x > 0.0 { "right".to_string() } else { "left".to_string() };
        } else {
            final_anim_direction = if move_y > 0.0 { "down".to_string() } else { "up".to_string() };
        }
    }
    // If input is (0,0), keep the previous direction

    if final_anim_direction != current_player.direction {
        log::trace!("Player {:?} animation direction set to: {}", sender_id, final_anim_direction);
    }
    // --- End Animation Direction ---

    let now = ctx.timestamp;

    // --- Calculate Delta Time ---
    let elapsed_micros = now.to_micros_since_unix_epoch().saturating_sub(current_player.last_update.to_micros_since_unix_epoch());
    // Clamp max delta time to avoid huge jumps on first update or after lag spikes (e.g., 100ms)
    let delta_time_secs = (elapsed_micros as f32 / 1_000_000.0).min(0.1); // Clamp max delta time

    // --- Stamina Drain & Base Speed Calculation ---
    let mut new_stamina = current_player_stats.stamina; // Base this on current_player for speed calc
    let mut base_speed_multiplier = 1.0;
    // Movement now depends only on having a direction input from the client
    let is_moving = move_x.abs() > 0.01 || move_y.abs() > 0.01;
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

    // --- Calculate Target Velocity & Server Displacement ---
    let target_speed = PLAYER_SPEED * base_speed_multiplier;
    // Velocity is the normalized direction vector scaled by target speed
    let velocity_x = move_x * target_speed;
    let velocity_y = move_y * target_speed;

    let server_dx = velocity_x * delta_time_secs;
    let server_dy = velocity_y * delta_time_secs;


    // --- Movement Calculation ---
    // Use server-calculated displacement
    let proposed_x = current_player.position_x + server_dx;
    let proposed_y = current_player.position_y + server_dy;

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
                    let min_dist = PLAYER_RADIUS * 2.0; // Player-Player collision distance
                    let min_dist_sq = min_dist * min_dist;

                    if dist_sq < min_dist_sq {
                        log::debug!("Player-Player collision detected between {:?} and {:?}. Calculating slide.", sender_id, other_player.identity);
                        // Slide calculation
                        let collision_normal_x = dx;
                        let collision_normal_y = dy;
                        let normal_mag_sq = dist_sq;

                        if normal_mag_sq > 0.0 {
                            let normal_mag = normal_mag_sq.sqrt();
                            let norm_x = collision_normal_x / normal_mag;
                            let norm_y = collision_normal_y / normal_mag;
                            // Use server_dx/dy for slide calculation
                            let dot_product = server_dx * norm_x + server_dy * norm_y;
                            let projection_x = dot_product * norm_x;
                            let projection_y = dot_product * norm_y;
                            let slide_dx = server_dx - projection_x;
                            let slide_dy = server_dy - projection_y;
                            final_x = current_player.position_x + slide_dx;
                            final_y = current_player.position_y + slide_dy;
                            // Clamp after slide application
                            final_x = final_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
                            final_y = final_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);
                        } else {
                            // If directly overlapping (dist_sq == 0), just stay put relative to this collision
                            final_x = current_player.position_x;
                            final_y = current_player.position_y;
                        }
                        collision_handled = true;
                        // break; // Handle one collision at a time for simplicity? Or continue checking? Continuing check for now.
                    }
                }
            },
            spatial_grid::EntityType::Tree(tree_id) => {
                 // if collision_handled { continue; } // Allow checking multiple collisions?
                 if let Some(tree) = trees.id().find(tree_id) {
                    if tree.health == 0 { continue; }
                    let tree_collision_y = tree.pos_y - crate::tree::TREE_COLLISION_Y_OFFSET;
                    let dx = clamped_x - tree.pos_x;
                    let dy = clamped_y - tree_collision_y;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq < crate::tree::PLAYER_TREE_COLLISION_DISTANCE_SQUARED {
                         log::debug!("Player-Tree collision detected between {:?} and tree {}. Calculating slide.", sender_id, tree.id);
                         // Slide calculation
                         let collision_normal_x = dx;
                         let collision_normal_y = dy;
                         let normal_mag_sq = dist_sq;
                         if normal_mag_sq > 0.0 {
                            let normal_mag = normal_mag_sq.sqrt();
                            let norm_x = collision_normal_x / normal_mag;
                            let norm_y = collision_normal_y / normal_mag;
                            // Use server_dx/dy for slide calculation
                            let dot_product = server_dx * norm_x + server_dy * norm_y;
                            let projection_x = dot_product * norm_x;
                            let projection_y = dot_product * norm_y;
                            let slide_dx = server_dx - projection_x;
                            let slide_dy = server_dy - projection_y;
                            final_x = current_player.position_x + slide_dx;
                            final_y = current_player.position_y + slide_dy;
                             // Clamp after slide application
                            final_x = final_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
                            final_y = final_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);
                        } else {
                            final_x = current_player.position_x;
                            final_y = current_player.position_y;
                        }
                        collision_handled = true; // Mark collision handled for this type
                    }
                }
            },
            spatial_grid::EntityType::Stone(stone_id) => {
                 // if collision_handled { continue; }
                 if let Some(stone) = stones.id().find(stone_id) {
                     if stone.health == 0 { continue; }
                     let stone_collision_y = stone.pos_y - crate::stone::STONE_COLLISION_Y_OFFSET;
                     let dx = clamped_x - stone.pos_x;
                     let dy = clamped_y - stone_collision_y;
                     let dist_sq = dx * dx + dy * dy;
                     if dist_sq < crate::stone::PLAYER_STONE_COLLISION_DISTANCE_SQUARED {
                         log::debug!("Player-Stone collision detected between {:?} and stone {}. Calculating slide.", sender_id, stone.id);
                         // Slide calculation
                         let collision_normal_x = dx;
                         let collision_normal_y = dy;
                         let normal_mag_sq = dist_sq;
                         if normal_mag_sq > 0.0 {
                             let normal_mag = normal_mag_sq.sqrt();
                             let norm_x = collision_normal_x / normal_mag;
                             let norm_y = collision_normal_y / normal_mag;
                             // Use server_dx/dy for slide calculation
                             let dot_product = server_dx * norm_x + server_dy * norm_y;
                             let projection_x = dot_product * norm_x;
                             let projection_y = dot_product * norm_y;
                             let slide_dx = server_dx - projection_x;
                             let slide_dy = server_dy - projection_y;
                             final_x = current_player.position_x + slide_dx;
                             final_y = current_player.position_y + slide_dy;
                             // Clamp after slide application
                             final_x = final_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
                             final_y = final_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);
                         } else {
                             final_x = current_player.position_x;
                             final_y = current_player.position_y;
                         }
                         collision_handled = true; // Mark collision handled
                     }
                 }
            },
            spatial_grid::EntityType::WoodenStorageBox(box_id) => {
                // if collision_handled { continue; }
                if let Some(box_instance) = wooden_storage_boxes.id().find(box_id) {
                    let box_collision_y = box_instance.pos_y - crate::wooden_storage_box::BOX_COLLISION_Y_OFFSET;
                    let dx = clamped_x - box_instance.pos_x;
                    let dy = clamped_y - box_collision_y;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq < crate::wooden_storage_box::PLAYER_BOX_COLLISION_DISTANCE_SQUARED {
                         log::debug!("Player-Box collision detected between {:?} and box {}. Calculating slide.", sender_id, box_instance.id);
                         // Slide calculation
                         let collision_normal_x = dx;
                         let collision_normal_y = dy;
                         let normal_mag_sq = dist_sq;
                         if normal_mag_sq > 0.0 {
                             let normal_mag = normal_mag_sq.sqrt();
                             let norm_x = collision_normal_x / normal_mag;
                             let norm_y = collision_normal_y / normal_mag;
                             // Use server_dx/dy for slide calculation
                             let dot_product = server_dx * norm_x + server_dy * norm_y;
                             let projection_x = dot_product * norm_x;
                             let projection_y = dot_product * norm_y;
                             let slide_dx = server_dx - projection_x;
                             let slide_dy = server_dy - projection_y;
                             final_x = current_player.position_x + slide_dx;
                             final_y = current_player.position_y + slide_dy;
                             // Clamp after slide application
                             final_x = final_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
                             final_y = final_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);
                         } else {
                             final_x = current_player.position_x;
                             final_y = current_player.position_y;
                         }
                         collision_handled = true; // Mark collision handled
                    }
                }
            },
             spatial_grid::EntityType::Campfire(_) => {
                // No collision with campfires
             },
            _ => {} // Ignore other types for collision
        }
        // If a slide occurred, the 'clamped_x/y' used for subsequent checks in this loop iteration
        // won't reflect the slide. This might lead to missed secondary collisions after sliding.
        // For simplicity, we keep it this way for now. A more robust solution would re-check
        // collisions after each slide within the loop, or use the push-out method below.
    }
    // --- End Initial Collision Check ---


    // --- Iterative Collision Resolution (Push-out) ---
    // Apply push-out based on the potentially slid final_x/final_y
    let mut resolved_x = final_x;
    let mut resolved_y = final_y;
    let resolution_iterations = 5;
    let epsilon = 0.01;

    for _iter in 0..resolution_iterations {
        let mut overlap_found_in_iter = false;
        // Re-query near the currently resolved position for this iteration
        let nearby_entities_resolve = grid.get_entities_in_range(resolved_x, resolved_y);

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
                             let push_amount = (overlap / 2.0) + epsilon; // Push each player half the overlap
                             let push_x = (dx / distance) * push_amount;
                             let push_y = (dy / distance) * push_amount;
                             resolved_x += push_x;
                             resolved_y += push_y;
                             // Note: This only pushes the current player. Ideally, both would be pushed.
                             // Full resolution is complex. This provides basic separation.
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
                             let overlap = (min_dist - distance) + epsilon; // Calculate overlap
                             let push_x = (dx / distance) * overlap; // Push player away by full overlap
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

        // Clamp position after each iteration's adjustments
        resolved_x = resolved_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
        resolved_y = resolved_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);

        if !overlap_found_in_iter {
            // log::trace!("Overlap resolution complete after {} iterations.", _iter + 1);
            break;
        }
        if _iter == resolution_iterations - 1 {
            log::warn!("Overlap resolution reached max iterations ({}) for player {:?}. Position might still overlap slightly.", resolution_iterations, sender_id);
        }
    }
    // --- End Collision Resolution ---


    // --- Final Update ---
    let mut player_to_update = current_player; // Get a mutable copy from the initial read

    // Check if position or direction actually changed
    let position_changed = (resolved_x - player_to_update.position_x).abs() > 0.01 ||
                           (resolved_y - player_to_update.position_y).abs() > 0.01;
    // Check against the animation direction determined earlier
    let direction_changed = player_to_update.direction != final_anim_direction;
    // Don't check stamina/sprint changes here, they are handled by player_stats
    let should_update_state = position_changed || direction_changed;

    // Always update timestamp if delta_time > 0 to prevent accumulation on next tick
    // This ensures last_update reflects the time this reducer processed movement,
    // even if the final position/direction didn't change due to collision or no input.
    let needs_timestamp_update = delta_time_secs > 0.0;

    if should_update_state {
        log::trace!("Updating player {:?} - PosChange: {}, DirChange: {}",
            sender_id, position_changed, direction_changed);

        player_to_update.position_x = resolved_x;
        player_to_update.position_y = resolved_y;
        player_to_update.direction = final_anim_direction; // Update animation direction
        player_to_update.last_update = now; // Update timestamp because state changed

        players.identity().update(player_to_update); // Update the modified player struct
    } else if needs_timestamp_update { // If no state changed, but time passed
         log::trace!("No movement state changes detected for player {:?}, but updating timestamp due to elapsed time.", sender_id);
         // Update only the timestamp on the existing player data
         player_to_update.last_update = now;
         players.identity().update(player_to_update);
    } else {
         // This case should be rare (delta_time <= 0.0)
         log::trace!("No state changes and no time elapsed for player {:?}, skipping update.", sender_id);
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

// --- NEW: Reducer to Update Viewport ---
#[spacetimedb::reducer]
pub fn update_viewport(ctx: &ReducerContext, min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Result<(), String> {
    let client_id = ctx.sender;
    let viewports = ctx.db.client_viewport();
    log::trace!("Reducer update_viewport called by {:?} with bounds: ({}, {}), ({}, {})",
             client_id, min_x, min_y, max_x, max_y);

    let viewport_data = ClientViewport {
        client_identity: client_id,
        min_x,
        min_y,
        max_x,
        max_y,
        last_update: ctx.timestamp,
    };

    // Use insert_or_update logic
    if viewports.client_identity().find(&client_id).is_some() {
        viewports.client_identity().update(viewport_data);
        log::trace!("Updated viewport for client {:?}", client_id);
    } else {
        match viewports.try_insert(viewport_data) {
            Ok(_) => {
                log::trace!("Inserted new viewport for client {:?}", client_id);
            },
            Err(e) => {
                 log::error!("Failed to insert viewport for client {:?}: {}", client_id, e);
                 return Err(format!("Failed to insert viewport: {}", e));
            }
        }
    }
    Ok(())
}

// --- Reducers ---
#[spacetimedb::reducer]
pub fn add_experience(ctx: &ReducerContext, amount: f32) -> Result<(), String> {
    let sender_id = ctx.sender;
    let player_stats = ctx.db.player_stats();
    let buffs = ctx.db.buff();

    let mut stats = player_stats.player_id().find(sender_id)
        .ok_or_else(|| "Player stats not found".to_string())?;

    stats.experience += amount;

    // Check for level up
    while stats.experience >= stats.experience_to_next_level {
        stats.level += 1;
        stats.experience -= stats.experience_to_next_level;
        stats.experience_to_next_level = BASE_EXP_TO_LEVEL * (EXP_TO_LEVEL_MULTIPLIER.powi(stats.level as i32 - 1));

        // Generate random buffs for level up
        let buff_count = 3; // Number of buffs to choose from
        let mut available_buffs = Vec::new();

        for _ in 0..buff_count {
            let mut rng = StdRng::from_rng(ctx.rng()).map_err(|e| format!("Failed to seed RNG: {}", e))?;

            let rarity = crate::buff::get_random_rarity(&mut rng);
            let buff_type = crate::buff::get_random_buff(&mut rng,rarity.clone());

            let buff = Buff {
                id: 0, // Auto-incremented
                player_id: sender_id,
                buff_type: buff_type.clone(),
                rarity: rarity.clone(),
            };

            available_buffs.push(buff);
        }

        // Store available buffs (they will be selected by the client)
        for buff in available_buffs {
            buffs.insert(buff);
        }

        log::info!("Player {:?} reached level {}!", sender_id, stats.level);
    }

    player_stats.player_id().update(stats);
    Ok(())
}