use spacetimedb::{Identity, Timestamp, ReducerContext, Table};
use log;
use std::time::Duration;
use spacetimedb::spacetimedb_lib::ScheduleAt;
use std::cmp::min; // Import min for merging logic

// Import table traits AND concrete types
use crate::player as PlayerTableTrait;
// Consolidate item imports
use crate::items::{inventory_item as InventoryItemTableTrait, item_definition as ItemDefinitionTableTrait, InventoryItem, ItemDefinition, calculate_merge_result, split_stack_helper};
// Import helper functions
// REMOVED use crate::items::add_item_to_player_inventory; (Not directly used in campfire.rs)

// Import inventory management module and traits
use crate::inventory_management::{self, ItemContainer, ContainerItemClearer};
// Import Player struct
use crate::Player;

// Import necessary types, traits, and helpers from other modules
// REMOVED redundant imports for InventoryItem, ItemDefinition, calculate_merge_result, split_stack_helper
use crate::player_inventory::{move_item_to_inventory, move_item_to_hotbar}; // Import player inventory functions

// --- Constants ---
pub(crate) const CAMPFIRE_COLLISION_RADIUS: f32 = 18.0; // Smaller than player radius
pub(crate) const CAMPFIRE_COLLISION_Y_OFFSET: f32 = 10.0; // Y offset for collision checking (relative to fire's center)
pub(crate) const PLAYER_CAMPFIRE_COLLISION_DISTANCE_SQUARED: f32 = (super::PLAYER_RADIUS + CAMPFIRE_COLLISION_RADIUS) * (super::PLAYER_RADIUS + CAMPFIRE_COLLISION_RADIUS);
pub(crate) const CAMPFIRE_CAMPFIRE_COLLISION_DISTANCE_SQUARED: f32 = (CAMPFIRE_COLLISION_RADIUS * 2.0) * (CAMPFIRE_COLLISION_RADIUS * 2.0); // Prevent placing campfires too close

// Interaction Constants
pub(crate) const PLAYER_CAMPFIRE_INTERACTION_DISTANCE: f32 = 64.0;
pub(crate) const PLAYER_CAMPFIRE_INTERACTION_DISTANCE_SQUARED: f32 = PLAYER_CAMPFIRE_INTERACTION_DISTANCE * PLAYER_CAMPFIRE_INTERACTION_DISTANCE;

pub(crate) const WARMTH_RADIUS: f32 = 150.0; // How far the warmth effect reaches
pub(crate) const WARMTH_RADIUS_SQUARED: f32 = WARMTH_RADIUS * WARMTH_RADIUS;
pub(crate) const WARMTH_PER_SECOND: f32 = 5.0; // How much warmth is gained per second near a fire
pub(crate) const FUEL_CONSUME_INTERVAL_SECS: u64 = 5; // Consume 1 wood every 5 seconds
pub const NUM_FUEL_SLOTS: usize = 5; // Made public
const FUEL_CHECK_INTERVAL_SECS: u64 = 1; // Check every second

#[spacetimedb::table(name = campfire, public)]
#[derive(Clone)]
pub struct Campfire {
    #[primary_key]
    #[auto_inc]
    pub id: u32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub placed_by: Identity, // Track who placed it
    pub placed_at: Timestamp,
    pub is_burning: bool, // Is the fire currently lit?
    // Use individual fields instead of arrays
    pub fuel_instance_id_0: Option<u64>,
    pub fuel_def_id_0: Option<u64>,
    pub fuel_instance_id_1: Option<u64>,
    pub fuel_def_id_1: Option<u64>,
    pub fuel_instance_id_2: Option<u64>,
    pub fuel_def_id_2: Option<u64>,
    pub fuel_instance_id_3: Option<u64>,
    pub fuel_def_id_3: Option<u64>,
    pub fuel_instance_id_4: Option<u64>,
    pub fuel_def_id_4: Option<u64>,
    pub next_fuel_consume_at: Option<Timestamp>, // Timestamp for next fuel consumption check
}

// --- Schedule Table for Fuel Check --- 
#[spacetimedb::table(name = campfire_fuel_check_schedule, scheduled(check_campfire_fuel_consumption))]
#[derive(Clone)]
pub struct CampfireFuelCheckSchedule {
    #[primary_key]
    #[auto_inc]
    pub id: u64, // Must be u64
    pub scheduled_at: ScheduleAt,
}

// --- Reducers ---

/// Reducer called by the client when the player attempts to interact (e.g., press 'E')
/// Currently, this only validates proximity. The client will handle opening the UI.
#[spacetimedb::reducer]
pub fn interact_with_campfire(ctx: &ReducerContext, campfire_id: u32) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let campfires = ctx.db.campfire();

    // 1. Find Player
    let player = players.identity().find(sender_id)
        .ok_or_else(|| "Player not found".to_string())?;

    // 2. Find Campfire
    let campfire = campfires.id().find(campfire_id)
        .ok_or_else(|| format!("Campfire {} not found", campfire_id))?;

    // 3. Check Distance
    let dx = player.position_x - campfire.pos_x;
    let dy = player.position_y - campfire.pos_y;
    let dist_sq = dx * dx + dy * dy;

    if dist_sq > PLAYER_CAMPFIRE_INTERACTION_DISTANCE_SQUARED {
        return Err("Too far away to interact with the campfire".to_string());
    }

    log::debug!("Player {:?} interaction check OK for campfire {}", sender_id, campfire_id);
    // Interaction is valid, client can proceed to open UI
    Ok(())
}

/// Adds an item from the player's inventory as fuel to a specific campfire slot.
#[spacetimedb::reducer]
pub fn add_fuel_to_campfire(ctx: &ReducerContext, campfire_id: u32, target_slot_index: u8, item_instance_id: u64) -> Result<(), String> {
    // Validate campfire interaction and get campfire
    let (_player, mut campfire) = validate_campfire_interaction(ctx, campfire_id)?;
    
    // Validate that the item is valid fuel
    validate_fuel_item(ctx, item_instance_id)?;
    
    // Use the generic container handler to move the item to the campfire
    inventory_management::handle_move_to_container_slot(
        ctx,
        &mut campfire,
        target_slot_index,
        item_instance_id
    )?;
    
    // Make a copy of the campfire for later use
    let campfire_copy = campfire.clone();
    
    // Update campfire state
    ctx.db.campfire().id().update(campfire);
    
    // If adding fuel to a burning campfire, we might need to update fuel consumption schedule
    if campfire_copy.is_burning {
        // Ensure next fuel consumption is scheduled if not already
        if campfire_copy.next_fuel_consume_at.is_none() {
            // Schedule next fuel consumption in 30 seconds
            let next_consume_time = ctx.timestamp + Duration::from_secs(30).into();
            let mut updated_campfire = campfire_copy.clone();
            updated_campfire.next_fuel_consume_at = Some(next_consume_time);
            ctx.db.campfire().id().update(updated_campfire);
            log::info!("Scheduled next fuel consumption for campfire {} at {:?}", campfire_id, next_consume_time);
        }
    }

    Ok(())
}

/// Removes the fuel item from a specific campfire slot and returns it to the player,
/// attempting to merge with existing stacks first.
#[spacetimedb::reducer]
pub fn auto_remove_fuel_from_campfire(ctx: &ReducerContext, campfire_id: u32, source_slot_index: u8) -> Result<(), String> {
    let sender_id = ctx.sender;
    let mut campfires = ctx.db.campfire();

    log::info!(
        "[AutoRemoveFuel] Player {:?} removing fuel from campfire {} slot {}",
        sender_id, campfire_id, source_slot_index
    );

    // --- Basic Validations --- 
    let (_player, mut campfire) = validate_campfire_interaction(ctx, campfire_id)?;
    if source_slot_index >= NUM_FUEL_SLOTS as u8 {
        return Err(format!("Invalid source fuel slot index: {}", source_slot_index));
    }
    // Ensure the source slot is not empty before calling the handler
    if campfire.get_slot_instance_id(source_slot_index).is_none() {
        return Err(format!("Source slot {} is empty.", source_slot_index));
    }
    
    // --- Call Generic Handler --- 
    // This handles finding the item, adding/merging to player inv/hotbar, and clearing the campfire slot.
    inventory_management::handle_quick_move_from_container(
        ctx, 
        &mut campfire, 
        source_slot_index
    )?;

    // --- Update Campfire & Check Extinguish Status --- 
    // Need to check extinguish status *after* handler potentially cleared the slot.
    let still_has_fuel = check_if_campfire_has_fuel(ctx, &campfire);
    if !still_has_fuel && campfire.is_burning {
        campfire.is_burning = false;
        campfire.next_fuel_consume_at = None;
        log::info!(
            "Campfire {} extinguished as last valid fuel was removed.",
            campfire_id
        );
    }

    campfires.id().update(campfire); // Update the campfire state
    log::info!(
        "Quick moved fuel from campfire {} slot {}.",
        campfire_id, source_slot_index
    );

    Ok(())
}

// Helper function to check if any fuel slot contains valid fuel (Wood with quantity > 0)
// Change signature to take ReducerContext
pub(crate) fn check_if_campfire_has_fuel(ctx: &ReducerContext, campfire: &Campfire) -> bool {
    // Get table handles from context
    let inventory = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();

    let instance_ids = [
        campfire.fuel_instance_id_0,
        campfire.fuel_instance_id_1,
        campfire.fuel_instance_id_2,
        campfire.fuel_instance_id_3,
        campfire.fuel_instance_id_4,
    ];
    for instance_id_opt in instance_ids {
        if let Some(instance_id) = instance_id_opt {
            if let Some(item) = inventory.instance_id().find(instance_id) {
                if let Some(def) = item_defs.id().find(item.item_def_id) {
                    if def.name == "Wood" && item.quantity > 0 {
                        return true; // Found valid fuel
                    }
                }
            }
        }
    }
    false // No valid fuel found
}

/// Toggles the burning state of the campfire (lights or extinguishes it).
/// Relies on checking if *any* fuel slot has Wood with quantity > 0.
#[spacetimedb::reducer]
pub fn toggle_campfire_burning(ctx: &ReducerContext, campfire_id: u32) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let mut campfires = ctx.db.campfire();
    let inventory_items = ctx.db.inventory_item(); // Get table handle
    let item_defs = ctx.db.item_definition(); // Get table handle

    // 1. Find Player
    let player = players.identity().find(sender_id).ok_or("Player not found")?;

    // 2. Find Campfire
    let mut campfire = campfires.id().find(campfire_id).ok_or(format!("Campfire {} not found", campfire_id))?;

    // 3. Check Distance
    let dx = player.position_x - campfire.pos_x;
    let dy = player.position_y - campfire.pos_y;
    if (dx * dx + dy * dy) > PLAYER_CAMPFIRE_INTERACTION_DISTANCE_SQUARED { return Err("Too far away".to_string()); }

    // 4. Determine Action: Light or Extinguish?
        if campfire.is_burning {
        // --- Action: Extinguish ---
            campfire.is_burning = false;
        campfire.next_fuel_consume_at = None;
            campfires.id().update(campfire);
        log::info!("Campfire {} extinguished by player {:?}.", campfire_id, sender_id);
        Ok(())
        } else {
        // --- Action: Attempt to Light ---
        // Check if any slot has valid fuel (pass ctx)
        let has_valid_fuel = check_if_campfire_has_fuel(ctx, &campfire);
        if !has_valid_fuel {
            return Err("Cannot light campfire, requires Wood with quantity > 0 in at least one fuel slot".to_string());
        }

        // Checks passed, light the fire!
        campfire.is_burning = true;
        campfire.next_fuel_consume_at = Some(ctx.timestamp + Duration::from_secs(FUEL_CONSUME_INTERVAL_SECS).into());
        let next_check_time_for_log = campfire.next_fuel_consume_at;
        campfires.id().update(campfire);
        log::info!("Campfire {} lit by player {:?}. Next fuel check at {:?}.", campfire_id, sender_id, next_check_time_for_log);
        Ok(())
    }
}

// --- Fuel Consumption Check Reducer --- 

#[spacetimedb::reducer]
pub fn check_campfire_fuel_consumption(ctx: &ReducerContext, _schedule: CampfireFuelCheckSchedule) -> Result<(), String> {
    // --- Restore Original Logic --- 
    // Remove the simple trigger log 
    // log::info!("***** [Campfire Fuel Check] Scheduled reducer TRIGGERED at {:?} *****", ctx.timestamp);
    
    // Uncomment the original body
    let mut campfires = ctx.db.campfire(); 
    let mut inventory_items = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();
    let now = ctx.timestamp;
    let mut updates_made = false;

    let campfire_ids: Vec<u32> = campfires.iter().map(|c| c.id).collect();
    let mut campfires_to_update: Vec<Campfire> = Vec::new(); 

    log::trace!("[FuelCheck] Running scheduled check at {:?}", now);

    for campfire_id in campfire_ids {
        if let Some(campfire_ref) = campfires.id().find(campfire_id) {
            let mut campfire = campfire_ref.clone(); 
            let mut campfire_changed = false;
            if campfire.is_burning {
                if let Some(consume_time) = campfire.next_fuel_consume_at {
                    log::trace!("Campfire {}: Checking consumption. Now: {:?}, ConsumeAt: {:?}", campfire_id, now, consume_time);
                    if now >= consume_time {
                        log::info!("Campfire {}: Time to consume fuel.", campfire_id);
                        let mut remaining: u32 = 0; 
                        let mut slot_to_consume_from: Option<usize> = None;
                        let instance_ids = [
                            campfire.fuel_instance_id_0,
                            campfire.fuel_instance_id_1,
                            campfire.fuel_instance_id_2,
                            campfire.fuel_instance_id_3,
                            campfire.fuel_instance_id_4,
                        ];
                        for (slot_idx, instance_id_opt) in instance_ids.iter().enumerate() {
                             if let Some(instance_id) = instance_id_opt {
                                if let Some(item) = inventory_items.instance_id().find(*instance_id) {
                                    if let Some(def) = item_defs.id().find(item.item_def_id) {
                                        if def.name == "Wood" && item.quantity > 0 {
                                            slot_to_consume_from = Some(slot_idx);
                                            log::debug!("Campfire {}: Found valid fuel in slot {}", campfire_id, slot_idx);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        
                        if let Some(slot_idx) = slot_to_consume_from {
                            let instance_id = instance_ids[slot_idx].unwrap(); 
                            if let Some(mut fuel_item) = inventory_items.instance_id().find(instance_id) {
                                fuel_item.quantity -= 1;
                                remaining = fuel_item.quantity;
                                inventory_items.instance_id().update(fuel_item); 
                                log::info!("Campfire {}: Consumed 1 fuel from slot {}. Remaining: {}", campfire_id, slot_idx, remaining);
                                
                                campfire_changed = true;

                                if remaining == 0 {
                                    log::info!("Campfire {}: Fuel in slot {} ran out, deleting item {} and clearing slot.", campfire_id, slot_idx, instance_id);
                                    inventory_items.instance_id().delete(instance_id);
                                    match slot_idx {
                                        0 => { campfire.fuel_instance_id_0 = None; campfire.fuel_def_id_0 = None; },
                                        1 => { campfire.fuel_instance_id_1 = None; campfire.fuel_def_id_1 = None; },
                                        2 => { campfire.fuel_instance_id_2 = None; campfire.fuel_def_id_2 = None; },
                                        3 => { campfire.fuel_instance_id_3 = None; campfire.fuel_def_id_3 = None; },
                                        4 => { campfire.fuel_instance_id_4 = None; campfire.fuel_def_id_4 = None; },
                                        _ => {},
                                    }
                                    let still_has_fuel_after_empty = check_if_campfire_has_fuel(ctx, &campfire);
                                     log::info!("Campfire {}: Immediate extinguish check result: {}", campfire_id, still_has_fuel_after_empty);
                                    if !still_has_fuel_after_empty {
                                        campfire.is_burning = false;
                                        campfire.next_fuel_consume_at = None;
                                        log::info!("Campfire {}: Extinguishing immediately as last fuel in slot {} was consumed.", campfire_id, slot_idx);
                                    }
                                }
                            } else {
                                log::error!("Campfire {}: Could not find fuel item instance {}! Clearing slot.", campfire_id, instance_id);
                                 match slot_idx {
                                     0 => { campfire.fuel_instance_id_0 = None; campfire.fuel_def_id_0 = None; },
                                     1 => { campfire.fuel_instance_id_1 = None; campfire.fuel_def_id_1 = None; },
                                     2 => { campfire.fuel_instance_id_2 = None; campfire.fuel_def_id_2 = None; },
                                     3 => { campfire.fuel_instance_id_3 = None; campfire.fuel_def_id_3 = None; },
                                     4 => { campfire.fuel_instance_id_4 = None; campfire.fuel_def_id_4 = None; },
                                    _ => {},
                                }
                                campfire_changed = true;
                            }
                        } else {
                            log::warn!("Campfire {}: Was burning but no valid fuel found. Extinguishing.", campfire_id);
                            campfire.is_burning = false;
                            campfire.next_fuel_consume_at = None;
                            campfire_changed = true;
                        }

                        if campfire.is_burning {
                            log::debug!("Campfire {}: Still burning, checking if reschedule needed (remaining: {}).", campfire_id, remaining);
                            if remaining > 0 { 
                                let still_has_fuel = check_if_campfire_has_fuel(ctx, &campfire);
                                log::debug!("Campfire {}: check_if_campfire_has_fuel result: {}", campfire_id, still_has_fuel);
                                if still_has_fuel {
                                    let new_consume_time = now + Duration::from_secs(FUEL_CONSUME_INTERVAL_SECS).into();
                                    campfire.next_fuel_consume_at = Some(new_consume_time);
                                    log::info!("Campfire {}: Rescheduled fuel check to {:?}", campfire_id, new_consume_time);
                                    campfire_changed = true;
                                } else {
                                    campfire.is_burning = false;
                                    campfire.next_fuel_consume_at = None;
                                    log::warn!("Campfire {}: No remaining fuel after check. Extinguishing.", campfire_id);
                                    campfire_changed = true;
                                }
                            } else {
                                log::debug!("Campfire {}: Not rescheduling as remaining fuel is 0.", campfire_id);
                            }
                        } else {
                             log::debug!("Campfire {}: Not rescheduling as fire is not burning.", campfire_id);
                        }
                        
                    } else { // Added else block for logging when not time to consume
                        log::trace!("Campfire {}: Not time to consume fuel yet.", campfire_id);
                    }
                } else if campfire.is_burning { 
                     let still_has_fuel = check_if_campfire_has_fuel(ctx, &campfire);
                      log::debug!("Campfire {}: Burning but no consume time set. Has fuel? {}", campfire_id, still_has_fuel);
                     if still_has_fuel {
                         campfire.next_fuel_consume_at = Some(now + Duration::from_secs(FUEL_CONSUME_INTERVAL_SECS).into());
                         campfire_changed = true;
                         log::info!("Campfire {}: Scheduling initial fuel consumption check to {:?}.", campfire_id, campfire.next_fuel_consume_at);
                     } else {
                         campfire.is_burning = false;
                         campfire.next_fuel_consume_at = None;
                         campfire_changed = true;
                         log::warn!("Campfire {}: Extinguishing immediately, no valid fuel found upon check.", campfire_id);
                     }
                }
            }
            
            if campfire_changed {
                campfires_to_update.push(campfire);
                updates_made = true;
            }
        } 
    }

    // Batch update all modified campfires
    if updates_made {
        let update_count = campfires_to_update.len(); // Get length BEFORE move
        let mut campfire_table_update = ctx.db.campfire(); 
        for updated_campfire in campfires_to_update { // Move occurs here
            campfire_table_update.id().update(updated_campfire);
        }
        log::debug!("Finished checking campfire fuel consumption. {} updates.", update_count); // Use the stored count
    }
    
    Ok(())
}

// --- NEW: Split Stack Into Campfire Reducer ---

#[spacetimedb::reducer]
pub fn split_stack_into_campfire(
    ctx: &ReducerContext,
    source_item_instance_id: u64,
    quantity_to_split: u32,
    target_campfire_id: u32,
    target_slot_index: u8,
) -> Result<(), String> {
    // Validate item for fuel purposes
    let mut source_item = validate_fuel_item(ctx, source_item_instance_id)?;
    
    // Validate basic constraints for splitting
    if quantity_to_split == 0 || quantity_to_split >= source_item.quantity {
        return Err("Invalid split quantity".to_string());
    }
    
    // Get the item definition to check if it's stackable
    let item_def = ctx.db.item_definition().id().find(source_item.item_def_id)
        .ok_or("Item definition not found")?;
    if !item_def.is_stackable {
        return Err("Cannot split a non-stackable item".to_string());
    }
    
    // Validate campfire interaction and get campfire
    let (_player, mut campfire) = validate_campfire_interaction(ctx, target_campfire_id)?;
    
    // Use the generic container handler to split a stack into the campfire
    inventory_management::handle_split_into_container(
        ctx,
        &mut campfire,
        target_slot_index,
        &mut source_item,
        quantity_to_split
    )?;
    
    // Make a copy of the campfire for later use
    let campfire_copy = campfire.clone();
    
    // Update campfire state
    ctx.db.campfire().id().update(campfire);
    
    // If campfire is burning, ensure fuel consumption schedule exists
    if campfire_copy.is_burning && campfire_copy.next_fuel_consume_at.is_none() {
        // Schedule next fuel consumption
        let next_consume_time = ctx.timestamp + Duration::from_secs(30).into();
        let mut updated_campfire = campfire_copy.clone();
        updated_campfire.next_fuel_consume_at = Some(next_consume_time);
        ctx.db.campfire().id().update(updated_campfire);
        log::info!("Scheduled next fuel consumption for campfire {} at {:?}", 
                target_campfire_id, next_consume_time);
    }
    
    Ok(())
}

// --- NEW: Move/Merge/Swap Within Campfire Reducer ---

#[spacetimedb::reducer]
pub fn move_fuel_within_campfire(
    ctx: &ReducerContext,
    campfire_id: u32,
    source_slot_index: u8,
    target_slot_index: u8,
) -> Result<(), String> {
    // Validate campfire interaction and get campfire
    let (_player, mut campfire) = validate_campfire_interaction(ctx, campfire_id)?;
    
    // Use the generic container handler to move the item within the campfire
    inventory_management::handle_move_within_container(
        ctx,
        &mut campfire,
        source_slot_index,
        target_slot_index
    )?;
    
    // Update campfire state
    ctx.db.campfire().id().update(campfire);

    Ok(())
}

// --- NEW: Split Stack Within Campfire Reducer ---
#[spacetimedb::reducer]
pub fn split_stack_within_campfire(
    ctx: &ReducerContext,
    campfire_id: u32,
    source_slot_index: u8,
    quantity_to_split: u32,
    target_slot_index: u8,
) -> Result<(), String> {
    // Validate campfire interaction and get campfire
    let (_player, mut campfire) = validate_campfire_interaction(ctx, campfire_id)?;
    
    // Use the generic container handler to split a stack within the campfire
    inventory_management::handle_split_within_container(
        ctx,
        &mut campfire,
        source_slot_index,
        target_slot_index,
        quantity_to_split
    )?;
    
    // Update campfire state
    ctx.db.campfire().id().update(campfire);

    Ok(())
}

// --- NEW Reducer: Moves an item to the first available/mergeable campfire slot ---
#[spacetimedb::reducer]
pub fn quick_move_to_campfire(
    ctx: &ReducerContext,
    campfire_id: u32,
    item_instance_id: u64,
) -> Result<(), String> {
    // Validate the item being moved is valid fuel
    validate_fuel_item(ctx, item_instance_id)?;
    
    // Validate campfire interaction and get campfire
    let (_player, mut campfire) = validate_campfire_interaction(ctx, campfire_id)?;
    
    // Use the generic container handler for quick move to container
    inventory_management::handle_quick_move_to_container(
        ctx,
        &mut campfire,
                                        item_instance_id
    )?;
    
    // Make a copy of the campfire for later use
    let campfire_copy = campfire.clone();
    
    // Update campfire state
    ctx.db.campfire().id().update(campfire);
    
    // If campfire is burning, ensure fuel consumption schedule exists
    if campfire_copy.is_burning && campfire_copy.next_fuel_consume_at.is_none() {
        // Schedule next fuel consumption
        let next_consume_time = ctx.timestamp + Duration::from_secs(30).into();
        let mut updated_campfire = campfire_copy.clone();
        updated_campfire.next_fuel_consume_at = Some(next_consume_time);
        ctx.db.campfire().id().update(updated_campfire);
        log::info!("Scheduled next fuel consumption for campfire {} after quick move", campfire_id);
    }

    Ok(())
}

// --- Re-Add: Move Fuel Item to Player Slot Reducer --- 

#[spacetimedb::reducer]
pub fn move_fuel_item_to_player_slot(
    ctx: &ReducerContext,
    campfire_id: u32,
    source_slot_index: u8,
    target_slot_type: String,
    target_slot_index: u32, // u32 to match client flexibility
) -> Result<(), String> {
    // Validate campfire interaction and get campfire
    let (_player, mut campfire) = validate_campfire_interaction(ctx, campfire_id)?;
    
    // Use the generic container handler to move the item from campfire to player inventory
    inventory_management::handle_move_from_container_slot(
        ctx,
        &mut campfire,
        source_slot_index,
        target_slot_type,
        target_slot_index
    )?;
    
    // Make a copy of the campfire for later use
    let campfire_copy = campfire.clone();
    
    // Update campfire state
    ctx.db.campfire().id().update(campfire);
    
    // Check if this changes the campfire's burning status
    if campfire_copy.is_burning {
        // Recheck if campfire still has valid fuel
        let has_fuel = check_if_campfire_has_fuel(ctx, &campfire_copy);
        if !has_fuel {
            // Extinguish campfire if no fuel left
            let mut updated_campfire = campfire_copy.clone();
            updated_campfire.is_burning = false;
            updated_campfire.next_fuel_consume_at = None;
            ctx.db.campfire().id().update(updated_campfire);
            log::info!("Campfire {} extinguished due to removing all fuel", campfire_id);
        }
    }
    
    Ok(())
}

// --- Init Helper --- 
pub(crate) fn init_campfire_fuel_schedule(ctx: &ReducerContext) -> Result<(), String> {
    let schedule_table = ctx.db.campfire_fuel_check_schedule(); 
    // --- Force schedule insertion for debugging ---
    log::info!("Attempting to insert campfire fuel check schedule (every {}s).", FUEL_CHECK_INTERVAL_SECS);
    let interval = Duration::from_secs(FUEL_CHECK_INTERVAL_SECS);
    // Use try_insert and log potential errors
    match schedule_table.try_insert(CampfireFuelCheckSchedule {
        id: 0, // SpacetimeDB should handle auto-increment even if we provide 0
        scheduled_at: ScheduleAt::Interval(interval.into()),
    }) {
        Ok(_) => log::info!("Successfully inserted/ensured campfire schedule."),
        Err(e) => log::error!("Error trying to insert campfire schedule: {}", e),
    }
    /* --- Original check commented out ---
    if schedule_table.iter().count() == 0 {
        log::info!("Starting campfire fuel check schedule (every {}s).", FUEL_CHECK_INTERVAL_SECS);
        let interval = Duration::from_secs(FUEL_CHECK_INTERVAL_SECS);
        schedule_table.insert(CampfireFuelCheckSchedule {
            id: 0, // Auto-incremented
            scheduled_at: ScheduleAt::Interval(interval.into()),
        });
                } else {
        log::debug!("Campfire fuel check schedule already exists.");
    }
    */
    Ok(())
}

// --- Implement ItemContainer Trait for Campfire ---

impl ItemContainer for Campfire {
    fn num_slots(&self) -> usize {
        NUM_FUEL_SLOTS
    }

    fn get_slot_instance_id(&self, slot_index: u8) -> Option<u64> {
        if slot_index >= NUM_FUEL_SLOTS as u8 { return None; }
        match slot_index {
            0 => self.fuel_instance_id_0,
            1 => self.fuel_instance_id_1,
            2 => self.fuel_instance_id_2,
            3 => self.fuel_instance_id_3,
            4 => self.fuel_instance_id_4,
            _ => None, // Unreachable due to index check
        }
    }

    fn get_slot_def_id(&self, slot_index: u8) -> Option<u64> {
        if slot_index >= NUM_FUEL_SLOTS as u8 { return None; }
        match slot_index {
            0 => self.fuel_def_id_0,
            1 => self.fuel_def_id_1,
            2 => self.fuel_def_id_2,
            3 => self.fuel_def_id_3,
            4 => self.fuel_def_id_4,
            _ => None, // Unreachable due to index check
        }
    }

    fn set_slot(&mut self, slot_index: u8, instance_id: Option<u64>, def_id: Option<u64>) {
        if slot_index >= NUM_FUEL_SLOTS as u8 { return; }
        match slot_index {
            0 => { self.fuel_instance_id_0 = instance_id; self.fuel_def_id_0 = def_id; },
            1 => { self.fuel_instance_id_1 = instance_id; self.fuel_def_id_1 = def_id; },
            2 => { self.fuel_instance_id_2 = instance_id; self.fuel_def_id_2 = def_id; },
            3 => { self.fuel_instance_id_3 = instance_id; self.fuel_def_id_3 = def_id; },
            4 => { self.fuel_instance_id_4 = instance_id; self.fuel_def_id_4 = def_id; },
            _ => {}, // Unreachable due to index check
        }
    }
}

// --- Implement ContainerItemClearer Trait for Campfire ---

/// Helper struct to implement the ContainerItemClearer trait for Campfire
pub struct CampfireClearer;

// --- Helper to clear a specific item instance from any campfire fuel slot ---
pub(crate) fn clear_item_from_campfire_fuel_slots(ctx: &ReducerContext, item_instance_id_to_clear: u64) -> bool {
    let mut campfires = ctx.db.campfire();
    let mut found_and_cleared = false;
    
    // Iterate through campfires that *might* contain the item
    let potential_campfire_ids: Vec<u32> = campfires.iter()
                                            .filter(|c|
                                                // Check all individual slots
                                                c.fuel_instance_id_0 == Some(item_instance_id_to_clear) ||
                                                c.fuel_instance_id_1 == Some(item_instance_id_to_clear) ||
                                                c.fuel_instance_id_2 == Some(item_instance_id_to_clear) ||
                                                c.fuel_instance_id_3 == Some(item_instance_id_to_clear) ||
                                                c.fuel_instance_id_4 == Some(item_instance_id_to_clear)
                                            )
                                            .map(|c| c.id).collect();

    for campfire_id in potential_campfire_ids {
        // Use try_find to avoid panic if campfire disappears mid-iteration (less likely but safer)
        if let Some(mut campfire) = campfires.id().find(campfire_id) {
            let mut updated = false;
            // Check and clear each slot individually using NEW field names
            if campfire.fuel_instance_id_0 == Some(item_instance_id_to_clear) {
                campfire.fuel_instance_id_0 = None; campfire.fuel_def_id_0 = None; updated = true;
            }
            if campfire.fuel_instance_id_1 == Some(item_instance_id_to_clear) {
                campfire.fuel_instance_id_1 = None; campfire.fuel_def_id_1 = None; updated = true;
            }
            if campfire.fuel_instance_id_2 == Some(item_instance_id_to_clear) {
                campfire.fuel_instance_id_2 = None; campfire.fuel_def_id_2 = None; updated = true;
            }
            if campfire.fuel_instance_id_3 == Some(item_instance_id_to_clear) {
                campfire.fuel_instance_id_3 = None; campfire.fuel_def_id_3 = None; updated = true;
            }
            if campfire.fuel_instance_id_4 == Some(item_instance_id_to_clear) {
                campfire.fuel_instance_id_4 = None; campfire.fuel_def_id_4 = None; updated = true;
            }

            if updated {
                log::debug!("[ClearCampfireSlot] Cleared item {} from a fuel slot in campfire {}", item_instance_id_to_clear, campfire_id);
                // Check if fire should extinguish after clearing slot
                // Pass ctx instead of table handles
                let still_has_fuel = check_if_campfire_has_fuel(ctx, &campfire);
                 if !still_has_fuel && campfire.is_burning {
                    campfire.is_burning = false;
                    campfire.next_fuel_consume_at = None;
                    log::info!("Campfire {} extinguished as last valid fuel was removed.", campfire_id);
                }
    campfires.id().update(campfire);
                found_and_cleared = true; // Mark as found
            }
        }
    }
    
    found_and_cleared
}

impl ContainerItemClearer for CampfireClearer {
    fn clear_item(ctx: &ReducerContext, item_instance_id: u64) -> bool {
        clear_item_from_campfire_fuel_slots(ctx, item_instance_id)
    }
}

// --- Reducer: Split Stack From Campfire --- 
// (Moved from items.rs)
#[spacetimedb::reducer]
pub fn split_stack_from_campfire(
    ctx: &ReducerContext,
    source_campfire_id: u32,
    source_slot_index: u8,
    quantity_to_split: u32,
    target_slot_type: String,    // "inventory" or "hotbar"
    target_slot_index: u32,     // Numeric index for inventory/hotbar
) -> Result<(), String> {
    // Get mutable campfire table handle
    let mut campfires = ctx.db.campfire();

    // --- Basic Validations --- 
    let (_player, mut campfire) = validate_campfire_interaction(ctx, source_campfire_id)?;
    // Note: Further validations (item existence, stackability, quantity) are handled 
    //       within the generic handle_split_from_container function.

    log::info!(
        "[SplitFromCampfire] Player {:?} delegating split {} from campfire {} slot {} to {} slot {}",
        ctx.sender, quantity_to_split, source_campfire_id, source_slot_index, target_slot_type, target_slot_index
    );

    // --- Call GENERIC Handler --- 
    inventory_management::handle_split_from_container(
        ctx, 
        &mut campfire, 
        source_slot_index, 
        quantity_to_split,
        target_slot_type, 
        target_slot_index
    )?;

    // --- Commit Campfire Update --- 
    // The handler might have modified the source item quantity via split_stack_helper,
    // but the campfire state itself (slots) isn't directly changed by this handler.
    // However, to be safe and consistent with other reducers that fetch a mutable container,
    // we update it here. In the future, if the handler needed to modify the container state
    // (e.g., if the split failed and we needed to revert something), this update is necessary.
    campfires.id().update(campfire);

    Ok(())
}

// --- Reducer: Split From Campfire and Move/Merge --- 
// (Moved from items.rs)
#[spacetimedb::reducer]
pub fn split_and_move_from_campfire(
    ctx: &ReducerContext,
    source_campfire_id: u32,
    source_slot_index: u8,
    quantity_to_split: u32,
    target_slot_type: String,    // "inventory", "hotbar", or "campfire_fuel"
    target_slot_index: u32,     // Numeric index for inventory/hotbar/campfire
) -> Result<(), String> {
    let sender_id = ctx.sender; // Needed for potential move to inventory/hotbar
    let campfires = ctx.db.campfire();
    let mut inventory_items = ctx.db.inventory_item(); // Mutable for split helper and move reducers

    log::info!(
        "[SplitMoveFromCampfire] Player {:?} splitting {} from campfire {} slot {} to {} slot {}",
        sender_id, quantity_to_split, source_campfire_id, source_slot_index, target_slot_type, target_slot_index
    );

    // --- 1. Find Source Campfire & Item ID --- 
    let campfire = campfires.id().find(source_campfire_id)
        .ok_or(format!("Source campfire {} not found", source_campfire_id))?;
    
    if source_slot_index >= crate::campfire::NUM_FUEL_SLOTS as u8 {
        return Err(format!("Invalid source fuel slot index: {}", source_slot_index));
    }

    let source_instance_id = match source_slot_index {
        0 => campfire.fuel_instance_id_0,
        1 => campfire.fuel_instance_id_1,
        2 => campfire.fuel_instance_id_2,
        3 => campfire.fuel_instance_id_3,
        4 => campfire.fuel_instance_id_4,
        _ => None,
    }.ok_or(format!("No item found in source campfire slot {}", source_slot_index))?;

    // --- 2. Get Source Item & Validate Split --- 
    let mut source_item = inventory_items.instance_id().find(source_instance_id)
        .ok_or("Source item instance not found in inventory table")?;

    // Note: Ownership check isn't strictly needed here as item is in world container,
    // but we might add checks later if campfires become player-specific.

    // Get Item Definition for stackability check
    let item_def = ctx.db.item_definition().id().find(source_item.item_def_id)
        .ok_or_else(|| format!("Definition not found for item ID {}", source_item.item_def_id))?;
    
    if !item_def.is_stackable {
        return Err(format!("Item '{}' is not stackable.", item_def.name));
    }
    if quantity_to_split == 0 {
        return Err("Cannot split a quantity of 0.".to_string());
    }
    if quantity_to_split >= source_item.quantity {
        return Err(format!("Cannot split {} items, only {} available.", quantity_to_split, source_item.quantity));
    }

    // --- 3. Perform Split --- 
    // The helper updates the original source_item stack and returns the ID of the new stack.
    let new_item_instance_id = split_stack_helper(ctx, &mut source_item, quantity_to_split)?;

    // --- 4. Move/Merge the NEW Stack --- 
    log::debug!("[SplitMoveFromCampfire] Calling appropriate move/add reducer for new stack {}", new_item_instance_id);
    match target_slot_type.as_str() {
        "inventory" => {
            // Call move_item_to_inventory from player_inventory module
            move_item_to_inventory(ctx, new_item_instance_id, target_slot_index as u16)
        },
        "hotbar" => {
             // Call move_item_to_hotbar from player_inventory module
            move_item_to_hotbar(ctx, new_item_instance_id, target_slot_index as u8)
        },
        "campfire_fuel" => {
            // Call add_fuel_to_campfire, which handles merging onto existing stack or placing in empty slot.
            // We use the source_campfire_id because we are moving *within* the same fire if target is campfire.
            add_fuel_to_campfire(ctx, source_campfire_id, target_slot_index as u8, new_item_instance_id)
        },
        _ => {
            log::error!("[SplitMoveFromCampfire] Invalid target_slot_type: {}", target_slot_type);
            // Attempt to delete the orphaned split stack to prevent item loss
            ctx.db.inventory_item().instance_id().delete(new_item_instance_id);
            Err(format!("Invalid target slot type for split: {}", target_slot_type))
        }
    }
}

// --- Reducer: Add Wood to First Available Slot (with Merge) --- 
// (Moved from items.rs)
#[spacetimedb::reducer]
pub fn auto_add_wood_to_campfire(
    ctx: &ReducerContext,
    campfire_id: u32,
    item_instance_id: u64,
) -> Result<(), String> {
    // --- Initial Validations --- 
    // Validate that the item is valid fuel first (checks ownership, location, type)
    validate_fuel_item(ctx, item_instance_id)?;
    // Validate campfire interaction and get campfire
    let (_player, mut campfire) = validate_campfire_interaction(ctx, campfire_id)?;

            log::info!(
        "[AutoAddWood] Player {:?} trying to quick move item {} to campfire {}",
        ctx.sender, item_instance_id, campfire_id
    );
    
    // --- Call Generic Handler --- 
    // This handles finding merge targets/empty slots in the container and moving/merging the item.
    inventory_management::handle_quick_move_to_container(
        ctx,
        &mut campfire,
        item_instance_id
    )?;

    // --- Update Campfire & Check Fuel Schedule --- 
    // Need to copy before update for the check
    let campfire_copy = campfire.clone();
    
    // Update campfire state (handler modified the mutable campfire struct)
    ctx.db.campfire().id().update(campfire);
    
    // If campfire is burning, ensure fuel consumption schedule exists
    if campfire_copy.is_burning && campfire_copy.next_fuel_consume_at.is_none() {
        // Schedule next fuel consumption
        let next_consume_time = ctx.timestamp + Duration::from_secs(30).into();
        let mut updated_campfire = campfire_copy.clone();
        updated_campfire.next_fuel_consume_at = Some(next_consume_time);
        ctx.db.campfire().id().update(updated_campfire);
        log::info!("Scheduled next fuel consumption for campfire {} after auto add wood", campfire_id);
    }

    Ok(())
}

// --- Helper Function (Validation) --- 

/// Validates if a player can interact with a specific campfire (checks existence and distance).
/// Returns Ok((Player struct instance, Campfire struct instance)) on success, or Err(String) on failure.
fn validate_campfire_interaction(
    ctx: &ReducerContext,
    campfire_id: u32,
) -> Result<(Player, Campfire), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let campfires = ctx.db.campfire();

    let player = players.identity().find(sender_id).ok_or_else(|| "Player not found".to_string())?;
    let campfire = campfires.id().find(campfire_id).ok_or_else(|| format!("Campfire {} not found", campfire_id))?;

    // Check distance between the interacting player and the campfire
    let dx = player.position_x - campfire.pos_x;
    let dy = player.position_y - campfire.pos_y;
    if (dx * dx + dy * dy) > PLAYER_CAMPFIRE_INTERACTION_DISTANCE_SQUARED {
        return Err("Too far away from campfire".to_string());
    }
    Ok((player, campfire))
}

/// Validates if an item can be used as fuel in a campfire.
/// Returns Ok(InventoryItem) if valid, Err(String) if not.
fn validate_fuel_item(
    ctx: &ReducerContext,
    item_instance_id: u64
) -> Result<InventoryItem, String> {
    let sender_id = ctx.sender;
    let inventory_items = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();

    // Find the item instance
    let item = inventory_items.instance_id().find(item_instance_id)
        .ok_or_else(|| format!("Item instance {} not found", item_instance_id))?;
    
    // Check ownership
    if item.player_identity != sender_id {
        return Err("Item does not belong to you".to_string());
    }

    // Check if it's in inventory or hotbar
    if item.inventory_slot.is_none() && item.hotbar_slot.is_none() {
        return Err("Item must be in inventory or hotbar".to_string());
    }

    // Get the definition and check if it's a valid fuel type
    let item_def = item_defs.id().find(item.item_def_id)
        .ok_or_else(|| "Item definition not found".to_string())?;
    
    // This is a campfire-specific check - only certain items can be used as fuel
    // Modify this list based on your game's fuel items
    let valid_fuel_items = ["Wood", "Stick", "Coal", "Tree Bark"];
    if !valid_fuel_items.contains(&item_def.name.as_str()) {
        return Err(format!("Item '{}' is not a valid fuel source", item_def.name));
    }

    Ok(item)
}