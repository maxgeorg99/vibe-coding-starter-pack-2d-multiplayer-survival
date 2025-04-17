use spacetimedb::{Identity, Timestamp, ReducerContext, Table};
use log;
use std::time::Duration;

// Import table traits AND concrete types
use crate::player as PlayerTableTrait;
use crate::items::{inventory_item as InventoryItemTableTrait, item_definition as ItemDefinitionTableTrait, InventoryItem, ItemDefinition};
// Import helper functions
use crate::items::add_item_to_player_inventory;

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
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let mut campfires = ctx.db.campfire();
    let mut inventory_items = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();

    // 1. Validate slot index
    if target_slot_index >= NUM_FUEL_SLOTS as u8 {
        return Err(format!("Invalid target fuel slot index: {}", target_slot_index));
    }

    // 2. Find Player & Campfire
    let player = players.identity().find(sender_id).ok_or("Player not found")?;
    let mut campfire = campfires.id().find(campfire_id).ok_or(format!("Campfire {} not found", campfire_id))?;

    // 3. Check Distance
    let dx = player.position_x - campfire.pos_x;
    let dy = player.position_y - campfire.pos_y;
    if (dx * dx + dy * dy) > PLAYER_CAMPFIRE_INTERACTION_DISTANCE_SQUARED { return Err("Too far away".to_string()); }

    // 4. Check if target campfire fuel slot is empty (use match for field access)
    let is_slot_occupied = match target_slot_index {
        0 => campfire.fuel_instance_id_0.is_some(),
        1 => campfire.fuel_instance_id_1.is_some(),
        2 => campfire.fuel_instance_id_2.is_some(),
        3 => campfire.fuel_instance_id_3.is_some(),
        4 => campfire.fuel_instance_id_4.is_some(),
        _ => return Err("Invalid slot index logic".to_string()), // Should be caught by earlier check
    };
    if is_slot_occupied {
        return Err(format!("Campfire fuel slot {} is already occupied", target_slot_index));
    }

    // 5. Find Inventory Item & Definition
    let mut item_to_add = inventory_items.instance_id().find(item_instance_id).ok_or("Item instance not found")?;
    if item_to_add.player_identity != sender_id { return Err("Item does not belong to player".to_string()); }
    let definition = item_defs.id().find(item_to_add.item_def_id).ok_or("Item definition not found")?;

    // 6. Update item (remove from inv/hotbar)
    item_to_add.inventory_slot = None;
    item_to_add.hotbar_slot = None;
    inventory_items.instance_id().update(item_to_add);
    log::info!("Moved item {} ({}) from player {:?} inv/hotbar to campfire {}", item_instance_id, definition.name, sender_id, campfire_id);

    // 7. Update campfire state in the specific slot (use match)
    match target_slot_index {
        0 => { campfire.fuel_instance_id_0 = Some(item_instance_id); campfire.fuel_def_id_0 = Some(definition.id); },
        1 => { campfire.fuel_instance_id_1 = Some(item_instance_id); campfire.fuel_def_id_1 = Some(definition.id); },
        2 => { campfire.fuel_instance_id_2 = Some(item_instance_id); campfire.fuel_def_id_2 = Some(definition.id); },
        3 => { campfire.fuel_instance_id_3 = Some(item_instance_id); campfire.fuel_def_id_3 = Some(definition.id); },
        4 => { campfire.fuel_instance_id_4 = Some(item_instance_id); campfire.fuel_def_id_4 = Some(definition.id); },
        _ => {}, // Should not happen
    }
    
    // Re-check if fire should extinguish if it was burning without valid fuel
    let can_light_now = check_if_campfire_has_fuel(ctx, &campfire);
    if !can_light_now && campfire.is_burning {
        campfire.is_burning = false;
        campfire.next_fuel_consume_at = None;
        log::warn!("Campfire {} extinguished as newly added fuel is not valid wood.", campfire_id);
    }

    campfires.id().update(campfire); // Update the campfire
    log::info!("Added item instance {} (Def {}) as fuel to campfire {} slot {}.", item_instance_id, definition.id, campfire_id, target_slot_index);

    Ok(())
}

/// Removes the fuel item from a specific campfire slot and returns it to the player.
#[spacetimedb::reducer]
pub fn remove_fuel_from_campfire(ctx: &ReducerContext, campfire_id: u32, source_slot_index: u8) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let mut campfires = ctx.db.campfire();
    let mut inventory_items = ctx.db.inventory_item();

    // 1. Validate slot index
    if source_slot_index >= NUM_FUEL_SLOTS as u8 {
        return Err(format!("Invalid source fuel slot index: {}", source_slot_index));
    }

    // 2. Find Player & Campfire
    let player = players.identity().find(sender_id).ok_or("Player not found")?;
    let mut campfire = campfires.id().find(campfire_id).ok_or(format!("Campfire {} not found", campfire_id))?;

    // 3. Check Distance
    let dx = player.position_x - campfire.pos_x;
    let dy = player.position_y - campfire.pos_y;
    if (dx * dx + dy * dy) > PLAYER_CAMPFIRE_INTERACTION_DISTANCE_SQUARED { return Err("Too far away".to_string()); }

    // 4. Check if there is a fuel item in the specified slot (use match)
    let fuel_instance_id = match source_slot_index {
        0 => campfire.fuel_instance_id_0,
        1 => campfire.fuel_instance_id_1,
        2 => campfire.fuel_instance_id_2,
        3 => campfire.fuel_instance_id_3,
        4 => campfire.fuel_instance_id_4,
        _ => None,
    }.ok_or_else(|| format!("No fuel item in campfire slot {} to remove", source_slot_index))?;

    // 5. Find Inventory Item
    let mut item_to_return = inventory_items.instance_id().find(fuel_instance_id)
        .ok_or_else(|| format!("Could not find fuel item instance {}", fuel_instance_id))?;

    // 6. Find empty inventory slot for the sender
    let first_empty_slot = crate::items::find_first_empty_inventory_slot(ctx, sender_id)
        .ok_or_else(|| "No empty inventory slot found".to_string())?;
    
    // 7. Update item ownership and place in sender's inventory
    item_to_return.player_identity = sender_id;
    item_to_return.inventory_slot = Some(first_empty_slot);
    item_to_return.hotbar_slot = None;
    inventory_items.instance_id().update(item_to_return);
    log::info!("Returned fuel item {} from campfire {} slot {} to player {:?} inv slot {}", 
             fuel_instance_id, campfire_id, source_slot_index, sender_id, first_empty_slot);

    // 8. Update campfire state: clear the specific slot (use match)
    match source_slot_index {
        0 => { campfire.fuel_instance_id_0 = None; campfire.fuel_def_id_0 = None; },
        1 => { campfire.fuel_instance_id_1 = None; campfire.fuel_def_id_1 = None; },
        2 => { campfire.fuel_instance_id_2 = None; campfire.fuel_def_id_2 = None; },
        3 => { campfire.fuel_instance_id_3 = None; campfire.fuel_def_id_3 = None; },
        4 => { campfire.fuel_instance_id_4 = None; campfire.fuel_def_id_4 = None; },
        _ => {}, // Should not happen
    }
    
    // Check if fire should extinguish
    let still_has_fuel = check_if_campfire_has_fuel(ctx, &campfire);
    if !still_has_fuel && campfire.is_burning {
        campfire.is_burning = false;
        campfire.next_fuel_consume_at = None;
        log::info!("Campfire {} extinguished as last valid fuel was removed.", campfire_id);
    }
    
    campfires.id().update(campfire); // Update the campfire
    log::info!("Removed fuel from campfire {} slot {}.", campfire_id, source_slot_index);

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

// --- Fuel Consumption Check Reducer (Called periodically, e.g., from lib.rs) ---

#[spacetimedb::reducer]
pub fn check_campfire_fuel_consumption(ctx: &ReducerContext) -> Result<(), String> {
    let mut campfires = ctx.db.campfire();
    let mut inventory_items = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();
    let now = ctx.timestamp;
    let mut updates_made = false;

    let campfire_ids: Vec<u32> = campfires.iter().map(|c| c.id).collect();

    for campfire_id in campfire_ids {
        if let Some(mut campfire) = campfires.id().find(campfire_id) {
            if campfire.is_burning {
                if let Some(consume_time) = campfire.next_fuel_consume_at {
                    if now >= consume_time {
                        let mut consumed_this_cycle = false;
                        let mut slot_to_consume_from: Option<usize> = None;

                        // Find the first slot with valid fuel
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
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Consume fuel from the found slot (if any)
                        if let Some(slot_idx) = slot_to_consume_from {
                            let instance_id = instance_ids[slot_idx].unwrap(); // Safe to unwrap
                            if let Some(mut fuel_item) = inventory_items.instance_id().find(instance_id) {
                                fuel_item.quantity -= 1;
                                let remaining = fuel_item.quantity;
                                inventory_items.instance_id().update(fuel_item);
                                log::info!("Campfire {}: Consumed 1 fuel from slot {}. Remaining: {}", campfire_id, slot_idx, remaining);
                                consumed_this_cycle = true;
                                updates_made = true;

                                if remaining == 0 {
                                    log::info!("Campfire {}: Fuel in slot {} ran out, deleting item {} and clearing slot.", campfire_id, slot_idx, instance_id);
                                    inventory_items.instance_id().delete(instance_id);
                                    // Clear the specific slot using match
                                    match slot_idx {
                                        0 => { campfire.fuel_instance_id_0 = None; campfire.fuel_def_id_0 = None; },
                                        1 => { campfire.fuel_instance_id_1 = None; campfire.fuel_def_id_1 = None; },
                                        2 => { campfire.fuel_instance_id_2 = None; campfire.fuel_def_id_2 = None; },
                                        3 => { campfire.fuel_instance_id_3 = None; campfire.fuel_def_id_3 = None; },
                                        4 => { campfire.fuel_instance_id_4 = None; campfire.fuel_def_id_4 = None; },
                                        _ => {}, // Should not happen
                                    }
                                }
                            } else {
                                log::error!("Campfire {}: Could not find fuel item instance {} in slot {}! Clearing slot.", campfire_id, instance_id, slot_idx);
                                match slot_idx {
                                     0 => { campfire.fuel_instance_id_0 = None; campfire.fuel_def_id_0 = None; },
                                     1 => { campfire.fuel_instance_id_1 = None; campfire.fuel_def_id_1 = None; },
                                     2 => { campfire.fuel_instance_id_2 = None; campfire.fuel_def_id_2 = None; },
                                     3 => { campfire.fuel_instance_id_3 = None; campfire.fuel_def_id_3 = None; },
                                     4 => { campfire.fuel_instance_id_4 = None; campfire.fuel_def_id_4 = None; },
                                    _ => {}, // Should not happen
                                }
                                updates_made = true;
                            }
                        } else {
                             log::warn!("Campfire {}: Was burning but no valid fuel found. Extinguishing.", campfire_id);
                             campfire.is_burning = false;
                             campfire.next_fuel_consume_at = None;
                             updates_made = true;
                        }

                        // Reschedule or extinguish based on remaining fuel
                        if campfire.is_burning {
                            // Pass ctx to the helper function
                            let still_has_fuel = check_if_campfire_has_fuel(ctx, &campfire);
                            if still_has_fuel {
                                campfire.next_fuel_consume_at = Some(now + Duration::from_secs(FUEL_CONSUME_INTERVAL_SECS).into());
                                log::debug!("Campfire {}: Rescheduled fuel check to {:?}", campfire_id, campfire.next_fuel_consume_at);
                            } else {
                                campfire.is_burning = false;
                                campfire.next_fuel_consume_at = None;
                                log::info!("Campfire {}: No remaining fuel. Extinguishing.", campfire_id);
                                updates_made = true;
                            }
                        }
                        
                        // Update the campfire state if changes occurred
                        if updates_made {
                             // Need to get a new handle for update after potential borrows in check_if_campfire_has_fuel
                             ctx.db.campfire().id().update(campfire);
                        }

                    } // else: Not time to consume yet
                } // else: Not scheduled for consumption
            } // else: Not burning
        } // else: Campfire not found
    }

    if updates_made {
        log::debug!("Finished checking campfire fuel consumption.");
    }

    Ok(())
}