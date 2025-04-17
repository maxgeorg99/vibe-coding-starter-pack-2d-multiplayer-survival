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
    let slot_idx = target_slot_index as usize; // Use usize for indexing later

    // 2. Find Player & Campfire
    let player = players.identity().find(sender_id).ok_or("Player not found")?;
    let mut campfire = campfires.id().find(campfire_id).ok_or(format!("Campfire {} not found", campfire_id))?;

    // 3. Check Distance
    let dx = player.position_x - campfire.pos_x;
    let dy = player.position_y - campfire.pos_y;
    if (dx * dx + dy * dy) > PLAYER_CAMPFIRE_INTERACTION_DISTANCE_SQUARED { return Err("Too far away".to_string()); }

    // 4. Find the dragged item (item_to_add) & its definition
    let mut item_to_add = inventory_items.instance_id().find(item_instance_id).ok_or("Item instance not found")?;
    if item_to_add.player_identity != sender_id { return Err("Item does not belong to player".to_string()); }
    let definition_to_add = item_defs.id().find(item_to_add.item_def_id).ok_or("Item definition not found")?;

    // 5. Check the target campfire fuel slot
    let target_instance_id_opt = match target_slot_index {
        0 => campfire.fuel_instance_id_0,
        1 => campfire.fuel_instance_id_1,
        2 => campfire.fuel_instance_id_2,
        3 => campfire.fuel_instance_id_3,
        4 => campfire.fuel_instance_id_4,
        _ => None,
    };

    if let Some(target_instance_id) = target_instance_id_opt {
        // --- Target Slot is Occupied: Try to Merge --- 
        let mut target_item = inventory_items.instance_id().find(target_instance_id)
                                .ok_or_else(|| format!("Target item instance {} in slot {} not found!", target_instance_id, target_slot_index))?;

        // Call the merge helper
        match crate::items::calculate_merge_result(&item_to_add, &target_item, &definition_to_add) {
            Ok((qty_transfer, source_new_qty, target_new_qty, delete_source)) => {
                log::info!("[AddFuel Merge] Merging {} from item {} onto item {}. New Qty: {}, Target Qty: {}", 
                         qty_transfer, item_instance_id, target_instance_id, source_new_qty, target_new_qty);

                // Update target item quantity
                target_item.quantity = target_new_qty;
                inventory_items.instance_id().update(target_item);

                // Update or delete source item
                if delete_source {
                    inventory_items.instance_id().delete(item_instance_id);
                    log::info!("[AddFuel Merge] Source item {} deleted.", item_instance_id);
                } else {
                    item_to_add.quantity = source_new_qty;
                    inventory_items.instance_id().update(item_to_add);
                }
                // Campfire state doesn't change in a merge (item ref remains the same)
            },
            Err(e) => {
                // Merge failed (e.g., different types, target full) - reject the drop for now
                log::warn!("[AddFuel Merge] Cannot merge item {} onto slot {}: {}", item_instance_id, target_slot_index, e);
                return Err(format!("Cannot merge into slot {}: {}", target_slot_index, e));
                // FUTURE: Could implement swapping logic here as a fallback
            }
        }

    } else {
        // --- Target Slot is Empty: Place Item --- 
        // 6. Update item (remove from inv/hotbar)
        item_to_add.inventory_slot = None;
        item_to_add.hotbar_slot = None;
        inventory_items.instance_id().update(item_to_add.clone()); // Clone needed as item_to_add borrowed for log
        log::info!("Moved item {} ({}) from player {:?} inv/hotbar to campfire {}", item_instance_id, definition_to_add.name, sender_id, campfire_id);

        // 7. Update campfire state in the specific slot (use match)
        match target_slot_index {
            0 => {
                log::info!("[AddFuel Debug] Matching slot 0");
                campfire.fuel_instance_id_0 = Some(item_instance_id); campfire.fuel_def_id_0 = Some(definition_to_add.id); 
            },
            1 => { campfire.fuel_instance_id_1 = Some(item_instance_id); campfire.fuel_def_id_1 = Some(definition_to_add.id); },
            2 => { campfire.fuel_instance_id_2 = Some(item_instance_id); campfire.fuel_def_id_2 = Some(definition_to_add.id); },
            3 => { campfire.fuel_instance_id_3 = Some(item_instance_id); campfire.fuel_def_id_3 = Some(definition_to_add.id); },
            4 => { campfire.fuel_instance_id_4 = Some(item_instance_id); campfire.fuel_def_id_4 = Some(definition_to_add.id); },
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
        log::info!("Added item instance {} (Def {}) as fuel to campfire {} slot {}.", item_instance_id, definition_to_add.id, campfire_id, target_slot_index);
    }

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
        0 => { 
            log::info!("[RemoveFuel Debug] Matching slot 0");
            campfire.fuel_instance_id_0 = None; campfire.fuel_def_id_0 = None; 
        },
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

// --- NEW: Split Stack Into Campfire Reducer ---

#[spacetimedb::reducer]
pub fn split_stack_into_campfire(
    ctx: &ReducerContext,
    source_item_instance_id: u64,
    quantity_to_split: u32,
    target_campfire_id: u32,
    target_slot_index: u8,
) -> Result<(), String> {
    let sender_id = ctx.sender;
    let mut inventory_items = ctx.db.inventory_item();
    let mut campfires = ctx.db.campfire();
    let item_defs = ctx.db.item_definition();

    log::info!("[SplitIntoCampfire] Player {:?} splitting {} from item {} into campfire {} slot {}",
             sender_id, quantity_to_split, source_item_instance_id, target_campfire_id, target_slot_index);

    // 1. Validate target slot index
    if target_slot_index >= NUM_FUEL_SLOTS as u8 {
        return Err(format!("Invalid target fuel slot index: {}", target_slot_index));
    }

    // 2. Find source item and check ownership/location
    let mut source_item = inventory_items.instance_id().find(source_item_instance_id)
        .ok_or("Source item instance not found")?;
    if source_item.player_identity != sender_id { return Err("Source item not owned by caller".to_string()); }
    if source_item.inventory_slot.is_none() && source_item.hotbar_slot.is_none() {
        return Err("Source item must be in inventory or hotbar to split into campfire".to_string());
    }

    // 3. Find target campfire
    let mut campfire = campfires.id().find(target_campfire_id)
        .ok_or(format!("Target campfire {} not found", target_campfire_id))?;

    // 4. Check if target slot is empty
    let is_slot_occupied = match target_slot_index {
        0 => campfire.fuel_instance_id_0.is_some(),
        1 => campfire.fuel_instance_id_1.is_some(),
        2 => campfire.fuel_instance_id_2.is_some(),
        3 => campfire.fuel_instance_id_3.is_some(),
        4 => campfire.fuel_instance_id_4.is_some(),
        _ => return Err("Invalid slot index logic".to_string()),
    };
    if is_slot_occupied {
        return Err(format!("Target campfire fuel slot {} is already occupied.", target_slot_index));
    }

    // 5. Call the core split logic helper (passes mutable borrow of source_item)
    let new_item_instance_id = crate::items::split_stack_helper(ctx, &mut source_item, quantity_to_split)?;

    // 6. Get definition ID for the new item (same as source)
    let definition_id = source_item.item_def_id;

    // 7. Update the target campfire slot with the NEW item instance ID
    match target_slot_index {
        0 => { campfire.fuel_instance_id_0 = Some(new_item_instance_id); campfire.fuel_def_id_0 = Some(definition_id); },
        1 => { campfire.fuel_instance_id_1 = Some(new_item_instance_id); campfire.fuel_def_id_1 = Some(definition_id); },
        2 => { campfire.fuel_instance_id_2 = Some(new_item_instance_id); campfire.fuel_def_id_2 = Some(definition_id); },
        3 => { campfire.fuel_instance_id_3 = Some(new_item_instance_id); campfire.fuel_def_id_3 = Some(definition_id); },
        4 => { campfire.fuel_instance_id_4 = Some(new_item_instance_id); campfire.fuel_def_id_4 = Some(definition_id); },
        _ => {}, // Should not happen
    }

    // Re-check fuel state and update campfire
    let can_light_now = check_if_campfire_has_fuel(ctx, &campfire);
    if !can_light_now && campfire.is_burning {
        campfire.is_burning = false;
        campfire.next_fuel_consume_at = None;
        log::warn!("Campfire {} extinguished as newly added fuel is not valid wood.", target_campfire_id);
    }
    campfires.id().update(campfire);

    log::info!("[SplitIntoCampfire] Split successful. New item {} placed in campfire {} slot {}.", 
             new_item_instance_id, target_campfire_id, target_slot_index);

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
    let sender_id = ctx.sender;
    let mut campfires = ctx.db.campfire();
    let mut inventory_items = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();

    log::info!("[MoveWithinCampfire] Player {:?} moving fuel from slot {} to slot {} in campfire {}",
             sender_id, source_slot_index, target_slot_index, campfire_id);

    // 1. Validate slot indices
    if source_slot_index >= NUM_FUEL_SLOTS as u8 || target_slot_index >= NUM_FUEL_SLOTS as u8 || source_slot_index == target_slot_index {
        return Err("Invalid source or target slot index".to_string());
    }
    let source_idx = source_slot_index as usize;
    let target_idx = target_slot_index as usize;

    // 2. Find Campfire
    let mut campfire = campfires.id().find(campfire_id)
        .ok_or(format!("Target campfire {} not found", campfire_id))?;

    // 3. Get source item ID and definition ID
    let (source_instance_id, source_def_id) = match source_slot_index {
        0 => (campfire.fuel_instance_id_0, campfire.fuel_def_id_0),
        1 => (campfire.fuel_instance_id_1, campfire.fuel_def_id_1),
        2 => (campfire.fuel_instance_id_2, campfire.fuel_def_id_2),
        3 => (campfire.fuel_instance_id_3, campfire.fuel_def_id_3),
        4 => (campfire.fuel_instance_id_4, campfire.fuel_def_id_4),
        _ => (None, None),
    };
    let source_instance_id = source_instance_id.ok_or(format!("Source slot {} is empty", source_slot_index))?;
    let source_def_id = source_def_id.ok_or("Source definition ID missing")?;

    // 4. Get target item ID and definition ID (if occupied)
     let (target_instance_id_opt, target_def_id_opt) = match target_slot_index {
        0 => (campfire.fuel_instance_id_0, campfire.fuel_def_id_0),
        1 => (campfire.fuel_instance_id_1, campfire.fuel_def_id_1),
        2 => (campfire.fuel_instance_id_2, campfire.fuel_def_id_2),
        3 => (campfire.fuel_instance_id_3, campfire.fuel_def_id_3),
        4 => (campfire.fuel_instance_id_4, campfire.fuel_def_id_4),
        _ => (None, None),
    };

    // --- Logic Branching --- 
    if let Some(target_instance_id) = target_instance_id_opt {
        // == Target is Occupied: Attempt Merge then Swap ==
        let mut source_item = inventory_items.instance_id().find(source_instance_id).ok_or("Source item not found")?;
        let mut target_item = inventory_items.instance_id().find(target_instance_id).ok_or("Target item not found")?;
        let item_def = item_defs.id().find(source_def_id).ok_or("Item definition not found")?; // Assume source/target defs match if merge possible

        match crate::items::calculate_merge_result(&source_item, &target_item, &item_def) {
            Ok((_, source_new_qty, target_new_qty, delete_source)) => {
                // -- Merge Possible --
                log::info!("[MoveWithinCampfire] Merging slot {} onto slot {}", source_slot_index, target_slot_index);
                target_item.quantity = target_new_qty;
                inventory_items.instance_id().update(target_item);
                if delete_source {
                    inventory_items.instance_id().delete(source_instance_id);
                } else {
                    source_item.quantity = source_new_qty;
                    inventory_items.instance_id().update(source_item);
                }
                // Clear source slot in campfire
                 match source_slot_index {
                    0 => { campfire.fuel_instance_id_0 = None; campfire.fuel_def_id_0 = None; },
                    1 => { campfire.fuel_instance_id_1 = None; campfire.fuel_def_id_1 = None; },
                    2 => { campfire.fuel_instance_id_2 = None; campfire.fuel_def_id_2 = None; },
                    3 => { campfire.fuel_instance_id_3 = None; campfire.fuel_def_id_3 = None; },
                    4 => { campfire.fuel_instance_id_4 = None; campfire.fuel_def_id_4 = None; },
                    _ => {}
                }
            },
            Err(_) => {
                // -- Merge Failed: Perform Swap --
                log::info!("[MoveWithinCampfire] Cannot merge, swapping slot {} and {}", source_slot_index, target_slot_index);
                // Just swap the references in the campfire struct
                 match (source_slot_index, target_slot_index) {
                    // This requires temporary variables to avoid overwriting
                    // A bit verbose, but necessary without direct array indexing
                    _ => { // Generic swap logic (handle all pairs)
                        let temp_instance_id = target_instance_id_opt;
                        let temp_def_id = target_def_id_opt;
                        
                        // Set target slot from source
                         match target_slot_index {
                            0 => { campfire.fuel_instance_id_0 = Some(source_instance_id); campfire.fuel_def_id_0 = Some(source_def_id); },
                            1 => { campfire.fuel_instance_id_1 = Some(source_instance_id); campfire.fuel_def_id_1 = Some(source_def_id); },
                            2 => { campfire.fuel_instance_id_2 = Some(source_instance_id); campfire.fuel_def_id_2 = Some(source_def_id); },
                            3 => { campfire.fuel_instance_id_3 = Some(source_instance_id); campfire.fuel_def_id_3 = Some(source_def_id); },
                            4 => { campfire.fuel_instance_id_4 = Some(source_instance_id); campfire.fuel_def_id_4 = Some(source_def_id); },
                            _ => {}
                        }
                        // Set source slot from target (using temps)
                         match source_slot_index {
                            0 => { campfire.fuel_instance_id_0 = temp_instance_id; campfire.fuel_def_id_0 = temp_def_id; },
                            1 => { campfire.fuel_instance_id_1 = temp_instance_id; campfire.fuel_def_id_1 = temp_def_id; },
                            2 => { campfire.fuel_instance_id_2 = temp_instance_id; campfire.fuel_def_id_2 = temp_def_id; },
                            3 => { campfire.fuel_instance_id_3 = temp_instance_id; campfire.fuel_def_id_3 = temp_def_id; },
                            4 => { campfire.fuel_instance_id_4 = temp_instance_id; campfire.fuel_def_id_4 = temp_def_id; },
                            _ => {}
                        }
                    }
                }
            }
        }
    } else {
        // == Target is Empty: Move Item ==
        log::info!("[MoveWithinCampfire] Moving from slot {} to empty slot {}", source_slot_index, target_slot_index);
        // Clear source slot
        match source_slot_index {
            0 => { campfire.fuel_instance_id_0 = None; campfire.fuel_def_id_0 = None; },
            1 => { campfire.fuel_instance_id_1 = None; campfire.fuel_def_id_1 = None; },
            2 => { campfire.fuel_instance_id_2 = None; campfire.fuel_def_id_2 = None; },
            3 => { campfire.fuel_instance_id_3 = None; campfire.fuel_def_id_3 = None; },
            4 => { campfire.fuel_instance_id_4 = None; campfire.fuel_def_id_4 = None; },
            _ => {}
        }
        // Set target slot
        match target_slot_index {
            0 => { campfire.fuel_instance_id_0 = Some(source_instance_id); campfire.fuel_def_id_0 = Some(source_def_id); },
            1 => { campfire.fuel_instance_id_1 = Some(source_instance_id); campfire.fuel_def_id_1 = Some(source_def_id); },
            2 => { campfire.fuel_instance_id_2 = Some(source_instance_id); campfire.fuel_def_id_2 = Some(source_def_id); },
            3 => { campfire.fuel_instance_id_3 = Some(source_instance_id); campfire.fuel_def_id_3 = Some(source_def_id); },
            4 => { campfire.fuel_instance_id_4 = Some(source_instance_id); campfire.fuel_def_id_4 = Some(source_def_id); },
            _ => {}
        }
    }

    // Update the campfire state
    campfires.id().update(campfire);

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
    let sender_id = ctx.sender;
    let mut inventory_items = ctx.db.inventory_item();
    let mut campfires = ctx.db.campfire();

    log::info!("[SplitWithinCampfire] Player {:?} splitting {} from slot {} to slot {} in campfire {}",
             sender_id, quantity_to_split, source_slot_index, target_slot_index, campfire_id);

    // 1. Validate slot indices
    if source_slot_index >= NUM_FUEL_SLOTS as u8 || target_slot_index >= NUM_FUEL_SLOTS as u8 || source_slot_index == target_slot_index {
        return Err("Invalid source or target slot index for split".to_string());
    }

    // 2. Find campfire
    let mut campfire = campfires.id().find(campfire_id)
        .ok_or(format!("Target campfire {} not found", campfire_id))?;

    // 3. Get source item ID
    let source_instance_id = match source_slot_index {
        0 => campfire.fuel_instance_id_0,
        1 => campfire.fuel_instance_id_1,
        2 => campfire.fuel_instance_id_2,
        3 => campfire.fuel_instance_id_3,
        4 => campfire.fuel_instance_id_4,
        _ => None,
    }.ok_or(format!("No item found in source campfire slot {}", source_slot_index))?;

    // 4. Get source item (mutable)
    let mut source_item = inventory_items.instance_id().find(source_instance_id)
        .ok_or("Source item instance not found in inventory table")?;

    // 5. Check if target slot is empty
    let is_target_occupied = match target_slot_index {
        0 => campfire.fuel_instance_id_0.is_some(),
        1 => campfire.fuel_instance_id_1.is_some(),
        2 => campfire.fuel_instance_id_2.is_some(),
        3 => campfire.fuel_instance_id_3.is_some(),
        4 => campfire.fuel_instance_id_4.is_some(),
        _ => true, // Treat invalid index as occupied
    };
    if is_target_occupied {
        return Err(format!("Target campfire fuel slot {} is already occupied.", target_slot_index));
    }
    
    // 6. Validate split quantity (using info from mutable source_item)
    let item_def = ctx.db.item_definition().id().find(source_item.item_def_id).ok_or("Item def not found")?;
     if !item_def.is_stackable { return Err("Source item is not stackable".to_string()); }
     if quantity_to_split == 0 || quantity_to_split >= source_item.quantity {
        return Err(format!("Invalid split quantity {} (must be > 0 and < {})", quantity_to_split, source_item.quantity));
    }

    // 7. Perform Split using helper
    let new_item_instance_id = crate::items::split_stack_helper(ctx, &mut source_item, quantity_to_split)?;
    let new_item_def_id = source_item.item_def_id;

    // 8. Update target campfire slot
    match target_slot_index {
        0 => { campfire.fuel_instance_id_0 = Some(new_item_instance_id); campfire.fuel_def_id_0 = Some(new_item_def_id); },
        1 => { campfire.fuel_instance_id_1 = Some(new_item_instance_id); campfire.fuel_def_id_1 = Some(new_item_def_id); },
        2 => { campfire.fuel_instance_id_2 = Some(new_item_instance_id); campfire.fuel_def_id_2 = Some(new_item_def_id); },
        3 => { campfire.fuel_instance_id_3 = Some(new_item_instance_id); campfire.fuel_def_id_3 = Some(new_item_def_id); },
        4 => { campfire.fuel_instance_id_4 = Some(new_item_instance_id); campfire.fuel_def_id_4 = Some(new_item_def_id); },
        _ => {}, // Should not happen
    }
    campfires.id().update(campfire);

     log::info!("[SplitWithinCampfire] Split successful. New item {} placed in slot {}.", 
             new_item_instance_id, target_slot_index);

    Ok(())
}

// --- NEW Reducer: Add Wood to First Available Slot ---
#[spacetimedb::reducer]
pub fn add_wood_to_first_available_campfire_slot(
    ctx: &ReducerContext,
    campfire_id: u32,
    item_instance_id: u64,
) -> Result<(), String> {
    let sender_id = ctx.sender;
    let mut inventory_items = ctx.db.inventory_item();
    let mut campfires = ctx.db.campfire();
    let item_defs = ctx.db.item_definition();

    log::info!("[AddWoodToCampfire] Player {:?} trying to add item {} to first available slot in campfire {}",
             sender_id, item_instance_id, campfire_id);

    // 1. Find Campfire & Item
    let mut campfire = campfires.id().find(campfire_id)
        .ok_or(format!("Target campfire {} not found", campfire_id))?;
    let mut item_to_add = inventory_items.instance_id().find(item_instance_id)
        .ok_or("Source item instance not found")?;

    // 2. Validations
    if item_to_add.player_identity != sender_id { return Err("Item not owned by caller".to_string()); }
    if item_to_add.inventory_slot.is_none() && item_to_add.hotbar_slot.is_none() {
        return Err("Item must be in inventory or hotbar".to_string());
    }
    let definition = item_defs.id().find(item_to_add.item_def_id)
        .ok_or("Item definition not found")?;
    if definition.name != "Wood" {
        return Err("Item is not Wood".to_string());
    }

    // 3. Find first empty slot
    let mut found_slot: Option<u8> = None;
    if campfire.fuel_instance_id_0.is_none() { found_slot = Some(0); }
    else if campfire.fuel_instance_id_1.is_none() { found_slot = Some(1); }
    else if campfire.fuel_instance_id_2.is_none() { found_slot = Some(2); }
    else if campfire.fuel_instance_id_3.is_none() { found_slot = Some(3); }
    else if campfire.fuel_instance_id_4.is_none() { found_slot = Some(4); }

    if let Some(slot_index) = found_slot {
        // 4. Update item (remove from inv/hotbar)
        item_to_add.inventory_slot = None;
        item_to_add.hotbar_slot = None;
        inventory_items.instance_id().update(item_to_add);

        // 5. Update campfire slot
        match slot_index {
            0 => { campfire.fuel_instance_id_0 = Some(item_instance_id); campfire.fuel_def_id_0 = Some(definition.id); },
            1 => { campfire.fuel_instance_id_1 = Some(item_instance_id); campfire.fuel_def_id_1 = Some(definition.id); },
            2 => { campfire.fuel_instance_id_2 = Some(item_instance_id); campfire.fuel_def_id_2 = Some(definition.id); },
            3 => { campfire.fuel_instance_id_3 = Some(item_instance_id); campfire.fuel_def_id_3 = Some(definition.id); },
            4 => { campfire.fuel_instance_id_4 = Some(item_instance_id); campfire.fuel_def_id_4 = Some(definition.id); },
            _ => {}, // Should not happen
        }
        campfires.id().update(campfire);
        log::info!("[AddWoodToCampfire] Added item {} to campfire {} slot {}", item_instance_id, campfire_id, slot_index);
        Ok(())
    } else {
        Err("Campfire fuel slots are full".to_string())
    }
}