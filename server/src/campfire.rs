use spacetimedb::{Identity, Timestamp, ReducerContext, Table};
use log;
use std::time::Duration;
use spacetimedb::spacetimedb_lib::ScheduleAt;
use std::cmp::min; // Import min for merging logic

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

    // --- Determine Original Location --- 
    let original_location_was_equipment = item_to_add.inventory_slot.is_none() && item_to_add.hotbar_slot.is_none();
    if original_location_was_equipment {
        log::debug!("[AddFuel] Item {} potentially coming from equipment slot.", item_instance_id);
    }

    // 6. Check if target slot is occupied and handle merge/swap
    let target_instance_id = match target_slot_index {
        0 => campfire.fuel_instance_id_0,
        1 => campfire.fuel_instance_id_1,
        2 => campfire.fuel_instance_id_2,
        3 => campfire.fuel_instance_id_3,
        4 => campfire.fuel_instance_id_4,
        _ => None, // Should be caught by earlier validation
    };

    if let Some(existing_fuel_instance_id) = target_instance_id {
        // --- Target Slot Occupied: Attempt Merge --- 
        log::debug!("[AddFuel] Target slot {} occupied by item {}. Attempting merge with {}.", 
                 target_slot_index, existing_fuel_instance_id, item_instance_id);
        let mut existing_fuel_item = inventory_items.instance_id().find(existing_fuel_instance_id)
            .ok_or(format!("Existing fuel item {} not found in slot {}", existing_fuel_instance_id, target_slot_index))?;
        
        match crate::items::calculate_merge_result(&item_to_add, &existing_fuel_item, &definition_to_add) {
            Ok((qty_transfer, source_new_qty, target_new_qty, delete_source)) => {
                // Merge successful
                log::info!("[AddFuel Merge] Merging {} from item {} onto campfire item {}. Target new qty: {}.",
                         qty_transfer, item_instance_id, existing_fuel_instance_id, target_new_qty);
                existing_fuel_item.quantity = target_new_qty;
                inventory_items.instance_id().update(existing_fuel_item);

                if delete_source {
                    log::info!("[AddFuel Merge] Source item {} depleted. Deleting instance.", item_instance_id);
                    inventory_items.instance_id().delete(item_instance_id);
                    // Item is gone, no need to clear slots later
                } else {
                     log::info!("[AddFuel Merge] Source item {} reduced to {}. Updating instance.", item_instance_id, source_new_qty);
                     item_to_add.quantity = source_new_qty;
                     // Update the source item to reflect reduced quantity BUT KEEP its player slots for now
                     // It wasn't fully consumed.
                     inventory_items.instance_id().update(item_to_add);
                     // Return error because the *entire* source stack wasn't moved/merged
                     return Err("Could not merge entire stack into campfire slot.".to_string());
                }
                // No need to update campfire struct here, the target item quantity was updated.
            },
            Err(e) => {
                // Merge failed (different item type, target full, etc.)
                log::warn!("[AddFuel Merge Failed] Cannot merge item {} onto {}: {}. Aborting add.", 
                         item_instance_id, existing_fuel_instance_id, e);
                 return Err(format!("Cannot merge onto item in slot {}: {}", target_slot_index, e));
            }
        }
    } else {
        // --- Target Slot Empty: Place Item --- 
        log::info!("[AddFuel Place] Placing item {} (Def {}) into empty campfire slot {}.", 
                 item_instance_id, definition_to_add.id, target_slot_index);
        
        // UPDATE the item being moved: clear its player inventory/hotbar slots
        item_to_add.inventory_slot = None;
        item_to_add.hotbar_slot = None;
        // Don't change player_identity yet, it's now 'in' the campfire world object
        inventory_items.instance_id().update(item_to_add); 
        
        // Update the specific campfire slot field
        match target_slot_index {
            0 => { campfire.fuel_instance_id_0 = Some(item_instance_id); campfire.fuel_def_id_0 = Some(definition_to_add.id); },
            1 => { campfire.fuel_instance_id_1 = Some(item_instance_id); campfire.fuel_def_id_1 = Some(definition_to_add.id); },
            2 => { campfire.fuel_instance_id_2 = Some(item_instance_id); campfire.fuel_def_id_2 = Some(definition_to_add.id); },
            3 => { campfire.fuel_instance_id_3 = Some(item_instance_id); campfire.fuel_def_id_3 = Some(definition_to_add.id); },
            4 => { campfire.fuel_instance_id_4 = Some(item_instance_id); campfire.fuel_def_id_4 = Some(definition_to_add.id); },
            _ => {}, // Should not happen
        }
    }
        
    // --- Final Steps --- 
    // Re-check if fire should extinguish if it was burning without valid fuel
    // (Keep this logic as it might be relevant after merging/placing)
    let can_light_now = check_if_campfire_has_fuel(ctx, &campfire);
    if !can_light_now && campfire.is_burning {
        campfire.is_burning = false;
        campfire.next_fuel_consume_at = None;
        log::warn!("Campfire {} extinguished as newly added fuel is not valid wood.", campfire_id);
    }

    campfires.id().update(campfire); // Update the campfire state
    log::info!("Successfully added/merged item instance {} as fuel to campfire {} slot {}.", 
             item_instance_id, campfire_id, target_slot_index);

    // Clear equipment slot AFTER successful placement/merge
    // (This handles the edge case of equipping fuel directly)
    if original_location_was_equipment {
        crate::items::clear_specific_item_from_equipment_slots(ctx, sender_id, item_instance_id);
    }

    Ok(())
}

/// Removes the fuel item from a specific campfire slot and returns it to the player,
/// attempting to merge with existing stacks first.
#[spacetimedb::reducer]
pub fn auto_remove_fuel_from_campfire(ctx: &ReducerContext, campfire_id: u32, source_slot_index: u8) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let mut campfires = ctx.db.campfire();
    let mut inventory_items = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();

    log::info!(
        "[AutoRemoveFuel] Player {:?} removing fuel from campfire {} slot {}",
        sender_id, campfire_id, source_slot_index
    );

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
    if (dx * dx + dy * dy) > PLAYER_CAMPFIRE_INTERACTION_DISTANCE_SQUARED {
        return Err("Too far away".to_string());
    }

    // 4. Check if there is a fuel item in the specified slot (use match)
    let (fuel_instance_id, fuel_def_id) = match source_slot_index {
        0 => (campfire.fuel_instance_id_0, campfire.fuel_def_id_0),
        1 => (campfire.fuel_instance_id_1, campfire.fuel_def_id_1),
        2 => (campfire.fuel_instance_id_2, campfire.fuel_def_id_2),
        3 => (campfire.fuel_instance_id_3, campfire.fuel_def_id_3),
        4 => (campfire.fuel_instance_id_4, campfire.fuel_def_id_4),
        _ => (None, None),
    };
    let fuel_instance_id = fuel_instance_id
        .ok_or_else(|| format!("No fuel item in campfire slot {} to remove", source_slot_index))?;
    let fuel_def_id = fuel_def_id
        .ok_or_else(|| format!("Missing def ID for fuel item in slot {}", source_slot_index))?;

    // 5. Find Inventory Item (mutable) and Definition
    let mut item_to_return = inventory_items.instance_id().find(fuel_instance_id)
        .ok_or_else(|| format!("Could not find fuel item instance {}", fuel_instance_id))?;
    let definition = item_defs.id().find(fuel_def_id)
        .ok_or("Fuel item definition not found")?;

    // 6. Attempt to merge into player's inventory/hotbar
    let mut item_fully_merged = false;
    if definition.is_stackable {
        // Prioritize merging into hotbar
        let hotbar_items: Vec<InventoryItem> = inventory_items.iter()
            .filter(|i| i.player_identity == sender_id && i.item_def_id == fuel_def_id && i.hotbar_slot.is_some())
            .collect(); // Collect to avoid borrowing issues
        
        for mut target_item in hotbar_items {
             match crate::items::calculate_merge_result(&item_to_return, &target_item, &definition) {
                Ok((qty_transfer, source_new_qty, target_new_qty, delete_source)) => {
                    if qty_transfer > 0 {
                        log::info!(
                            "[AutoRemoveFuel Merge] Merging {} from campfire item {} onto hotbar item {}.",
                            qty_transfer, fuel_instance_id, target_item.instance_id
                        );
                        target_item.quantity = target_new_qty;
                        inventory_items.instance_id().update(target_item);
                        item_to_return.quantity = source_new_qty;
                        if delete_source {
                            item_fully_merged = true;
                            break;
                        }
                    }
                }
                 Err(_) => {} // Ignore errors (e.g., target full)
             }
        }

        // If not fully merged, try merging into main inventory
        if !item_fully_merged {
            let inventory_items_main: Vec<InventoryItem> = inventory_items.iter()
                .filter(|i| i.player_identity == sender_id && i.item_def_id == fuel_def_id && i.inventory_slot.is_some())
                .collect();

            for mut target_item in inventory_items_main {
                 match crate::items::calculate_merge_result(&item_to_return, &target_item, &definition) {
                    Ok((qty_transfer, source_new_qty, target_new_qty, delete_source)) => {
                        if qty_transfer > 0 {
                             log::info!(
                                "[AutoRemoveFuel Merge] Merging {} from campfire item {} onto inventory item {}.",
                                qty_transfer, fuel_instance_id, target_item.instance_id
                            );
                            target_item.quantity = target_new_qty;
                            inventory_items.instance_id().update(target_item);
                            item_to_return.quantity = source_new_qty;
                            if delete_source {
                                item_fully_merged = true;
                                break;
                            }
                        }
                    }
                    Err(_) => {} // Ignore errors
                 }
            }
        }
    }

    // 7. If item fully merged, delete the original and clear campfire slot
    if item_fully_merged {
        log::info!(
            "[AutoRemoveFuel] Item {} fully merged into player inventory. Deleting original.",
            fuel_instance_id
        );
        inventory_items.instance_id().delete(fuel_instance_id);
    } else {
        // 8. Item not fully merged, find empty slot (Hotbar first, then Inventory)
        log::info!(
            "[AutoRemoveFuel] Item {} (qty {}) not fully merged. Finding empty slot...",
            fuel_instance_id, item_to_return.quantity
        );
        let occupied_hotbar_slots: std::collections::HashSet<u8> = inventory_items.iter()
            .filter(|i| i.player_identity == sender_id && i.hotbar_slot.is_some())
            .map(|i| i.hotbar_slot.unwrap())
            .collect();
        let empty_hotbar_slot = (0..6).find(|slot| !occupied_hotbar_slots.contains(slot));

        if let Some(slot_index) = empty_hotbar_slot {
            // Place in empty hotbar slot
            log::info!(
                "[AutoRemoveFuel] Placing remaining item {} into hotbar slot {}",
                fuel_instance_id, slot_index
            );
            item_to_return.player_identity = sender_id; // Ensure ownership
            item_to_return.inventory_slot = None;
            item_to_return.hotbar_slot = Some(slot_index);
            inventory_items.instance_id().update(item_to_return);
        } else {
            // Hotbar full, try main inventory
            let occupied_inventory_slots: std::collections::HashSet<u16> = inventory_items.iter()
                .filter(|i| i.player_identity == sender_id && i.inventory_slot.is_some())
                .map(|i| i.inventory_slot.unwrap())
                .collect();
            let empty_inventory_slot = (0..24).find(|slot| !occupied_inventory_slots.contains(slot));

            if let Some(slot_index) = empty_inventory_slot {
                 // Place in empty inventory slot
                log::info!(
                    "[AutoRemoveFuel] Placing remaining item {} into inventory slot {}",
                    fuel_instance_id, slot_index
                );
                item_to_return.player_identity = sender_id; // Ensure ownership
                item_to_return.inventory_slot = Some(slot_index);
                item_to_return.hotbar_slot = None;
                inventory_items.instance_id().update(item_to_return);
            } else {
                // Inventory full, cannot place item
                 log::error!(
                    "[AutoRemoveFuel] Player {:?} inventory full, cannot return item {} from campfire.",
                    sender_id, fuel_instance_id
                );
                return Err("Inventory is full".to_string());
            }
        }
    }

    // 9. Update campfire state: clear the specific source slot (use match)
    match source_slot_index {
        0 => {
            campfire.fuel_instance_id_0 = None;
            campfire.fuel_def_id_0 = None;
        }
        1 => {
            campfire.fuel_instance_id_1 = None;
            campfire.fuel_def_id_1 = None;
        }
        2 => {
            campfire.fuel_instance_id_2 = None;
            campfire.fuel_def_id_2 = None;
        }
        3 => {
            campfire.fuel_instance_id_3 = None;
            campfire.fuel_def_id_3 = None;
        }
        4 => {
            campfire.fuel_instance_id_4 = None;
            campfire.fuel_def_id_4 = None;
        }
        _ => {} // Should not happen
    }

    // Check if fire should extinguish
    let still_has_fuel = check_if_campfire_has_fuel(ctx, &campfire);
    if !still_has_fuel && campfire.is_burning {
        campfire.is_burning = false;
        campfire.next_fuel_consume_at = None;
        log::info!(
            "Campfire {} extinguished as last valid fuel was removed.",
            campfire_id
        );
    }

    campfires.id().update(campfire); // Update the campfire
    log::info!(
        "Removed/merged fuel from campfire {} slot {}.",
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

// --- NEW Reducer: Moves an item to the first available/mergeable campfire slot ---
#[spacetimedb::reducer]
pub fn quick_move_to_campfire(
    ctx: &ReducerContext,
    campfire_id: u32,
    item_instance_id: u64,
) -> Result<(), String> {
    let sender_id = ctx.sender;
    let mut inventory_items = ctx.db.inventory_item();
    let mut campfires = ctx.db.campfire();
    let item_defs = ctx.db.item_definition();

    log::info!(
        "[QuickMoveToCampfire] Player {:?} trying to quick-move item {} to campfire {}",
        sender_id, item_instance_id, campfire_id
    );

    // 1. Find Campfire & Item
    let mut campfire = campfires
        .id()
        .find(campfire_id)
        .ok_or(format!("Target campfire {} not found", campfire_id))?;
    let mut item_to_add = inventory_items
        .instance_id()
        .find(item_instance_id)
        .ok_or("Source item instance not found")?;

    // 2. Validations
    if item_to_add.player_identity != sender_id {
        return Err("Item not owned by caller".to_string());
    }
    if item_to_add.inventory_slot.is_none() && item_to_add.hotbar_slot.is_none() {
        return Err("Item must be in inventory or hotbar".to_string());
    }
    let definition = item_defs
        .id()
        .find(item_to_add.item_def_id)
        .ok_or("Item definition not found")?;

    // Check stackability for merge logic
    let is_stackable = definition.is_stackable;
    let item_def_id_to_add = definition.id;

    // 3. Attempt to Merge onto existing matching stacks in campfire
    let fuel_instance_ids = [
        campfire.fuel_instance_id_0,
        campfire.fuel_instance_id_1,
        campfire.fuel_instance_id_2,
        campfire.fuel_instance_id_3,
        campfire.fuel_instance_id_4,
    ];

    let mut source_item_depleted = false;
    for target_instance_id_opt in fuel_instance_ids {
        if let Some(target_instance_id) = target_instance_id_opt {
            if let Some(mut target_item) = inventory_items.instance_id().find(target_instance_id) {
                // Check if the target item has the same definition ID AND item is stackable
                if target_item.item_def_id == item_def_id_to_add && is_stackable {
                    // Attempt merge
                    match crate::items::calculate_merge_result(&item_to_add, &target_item, &definition) {
                        Ok((qty_transfer, source_new_qty, target_new_qty, delete_source)) => {
                            if qty_transfer > 0 {
                                log::info!(
                                    "[QuickMoveToCampfire Merge] Merging {} from item {} onto campfire item {}.",
                                    qty_transfer, item_instance_id, target_instance_id
                                );
                                // Update target item quantity
                                target_item.quantity = target_new_qty;
                                inventory_items.instance_id().update(target_item);

                                // Update source item quantity
                                item_to_add.quantity = source_new_qty;

                                if delete_source {
                                    log::info!(
                                        "[QuickMoveToCampfire Merge] Source item {} depleted, deleting.",
                                        item_instance_id
                                    );
                                    inventory_items.instance_id().delete(item_instance_id);
                                    source_item_depleted = true;
                                    break; // Source is gone, stop trying to merge
                                } else {
                                    // Update source item in DB (still has quantity left)
                                    inventory_items.instance_id().update(item_to_add.clone());
                                }
                            }
                        }
                        Err(e) => {
                             log::debug!("[QuickMoveToCampfire Merge] Cannot merge onto {}: {}", target_instance_id, e);
                             // Continue to next slot if merge failed (e.g., target full)
                        }
                    }
                }
            }
        }
        // If source depleted, exit the loop
        if source_item_depleted {
            break;
        }
    }

    // 4. If source item still exists, find first empty slot and place it
    if !source_item_depleted {
        log::info!(
            "[QuickMoveToCampfire] Source item {} still has {} quantity after merge attempts. Finding empty slot...",
            item_instance_id, item_to_add.quantity
        );
        let mut empty_slot_found: Option<u8> = None;
        if campfire.fuel_instance_id_0.is_none() {
            empty_slot_found = Some(0);
        } else if campfire.fuel_instance_id_1.is_none() {
            empty_slot_found = Some(1);
        } else if campfire.fuel_instance_id_2.is_none() {
            empty_slot_found = Some(2);
        } else if campfire.fuel_instance_id_3.is_none() {
            empty_slot_found = Some(3);
        } else if campfire.fuel_instance_id_4.is_none() {
            empty_slot_found = Some(4);
        }

        if let Some(slot_index) = empty_slot_found {
            log::info!(
                "[QuickMoveToCampfire] Placing remaining item {} into empty slot {}",
                item_instance_id, slot_index
            );
            // Update item (remove from inv/hotbar)
            item_to_add.inventory_slot = None;
            item_to_add.hotbar_slot = None;
            inventory_items.instance_id().update(item_to_add);

            // Update campfire slot
            match slot_index {
                0 => {
                    campfire.fuel_instance_id_0 = Some(item_instance_id);
                    campfire.fuel_def_id_0 = Some(item_def_id_to_add); // Use correct def ID
                }
                1 => {
                    campfire.fuel_instance_id_1 = Some(item_instance_id);
                    campfire.fuel_def_id_1 = Some(item_def_id_to_add); // Use correct def ID
                }
                2 => {
                    campfire.fuel_instance_id_2 = Some(item_instance_id);
                    campfire.fuel_def_id_2 = Some(item_def_id_to_add); // Use correct def ID
                }
                3 => {
                    campfire.fuel_instance_id_3 = Some(item_instance_id);
                    campfire.fuel_def_id_3 = Some(item_def_id_to_add); // Use correct def ID
                }
                4 => {
                    campfire.fuel_instance_id_4 = Some(item_instance_id);
                    campfire.fuel_def_id_4 = Some(item_def_id_to_add); // Use correct def ID
                }
                _ => {} // Should not happen
            }
            campfires.id().update(campfire);
        } else {
            log::warn!(
                "[QuickMoveToCampfire] Campfire {} fuel slots full, cannot place remaining item {}.",
                campfire_id, item_instance_id
            );
            // If NO merge happened AND no empty slot, return error.
            // Check if quantity changed. If it did, some merge happened, so it's a partial success (Ok).
            // If quantity is unchanged AND no empty slot, then it failed.
            let original_quantity_opt = inventory_items.instance_id().find(item_instance_id).map(|i| i.quantity); // Re-fetch original quantity
            if let Some(original_quantity) = original_quantity_opt {
                 let current_quantity = inventory_items.instance_id().find(item_instance_id).map(|i| i.quantity).unwrap_or(0); // Get current quantity
                 if current_quantity == original_quantity { // No merge happened
                    return Err("Campfire fuel slots are full (no space to merge or place)".to_string());
                 } // else: merge happened, so Ok(()) is fine below
            } else {
                 // Item was somehow deleted mid-process? Should be rare.
                 log::error!("[QuickMoveToCampfire] Source item {} disappeared during processing!", item_instance_id);
                 return Err("Internal error during quick move".to_string());
            }
        }
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
    let sender_id = ctx.sender;
    let mut campfires = ctx.db.campfire();
    // Inventory items table needed for move functions
    let inventory_items = ctx.db.inventory_item(); 

    log::info!(
        "[MoveFuelToPlayer] Player {:?} moving fuel from campfire {} slot {} to {} slot {}",
        sender_id, campfire_id, source_slot_index, target_slot_type, target_slot_index
    );

    // 1. Validate source slot index
    if source_slot_index >= NUM_FUEL_SLOTS as u8 {
        return Err(format!("Invalid source fuel slot index: {}", source_slot_index));
    }

    // 2. Find Campfire
    let mut campfire = campfires.id().find(campfire_id)
        .ok_or(format!("Campfire {} not found", campfire_id))?;

    // 3. Get the instance ID from the source slot
    let fuel_instance_id = match source_slot_index {
        0 => campfire.fuel_instance_id_0,
        1 => campfire.fuel_instance_id_1,
        2 => campfire.fuel_instance_id_2,
        3 => campfire.fuel_instance_id_3,
        4 => campfire.fuel_instance_id_4,
        _ => None,
    }.ok_or(format!("No fuel item in campfire slot {} to move", source_slot_index))?;

    // 4. Call the appropriate move function from items.rs
    let move_result = match target_slot_type.as_str() {
        "inventory" => {
            if target_slot_index >= 24 { return Err("Invalid inventory target index".to_string()); }
            crate::items::move_item_to_inventory(ctx, fuel_instance_id, target_slot_index as u16)
        },
        "hotbar" => {
            if target_slot_index >= 6 { return Err("Invalid hotbar target index".to_string()); }
            crate::items::move_item_to_hotbar(ctx, fuel_instance_id, target_slot_index as u8)
        },
        _ => Err(format!("Invalid target slot type '{}'", target_slot_type)),
    };

    // 5. If move was successful, clear the source slot in the campfire
    if move_result.is_ok() {
        log::info!(
            "[MoveFuelToPlayer] Move successful. Clearing campfire {} slot {}.",
            campfire_id, source_slot_index
        );
        match source_slot_index {
            0 => { campfire.fuel_instance_id_0 = None; campfire.fuel_def_id_0 = None; },
            1 => { campfire.fuel_instance_id_1 = None; campfire.fuel_def_id_1 = None; },
            2 => { campfire.fuel_instance_id_2 = None; campfire.fuel_def_id_2 = None; },
            3 => { campfire.fuel_instance_id_3 = None; campfire.fuel_def_id_3 = None; },
            4 => { campfire.fuel_instance_id_4 = None; campfire.fuel_def_id_4 = None; },
            _ => {} // Should not happen
        }
        // Update campfire state AFTER clearing the slot
        campfires.id().update(campfire);
    } else {
        // Log error if move failed, but return the original error from move_result
        log::error!(
            "[MoveFuelToPlayer] Failed to move fuel item {} to player slot: {:?}. Campfire slot {} unchanged.",
            fuel_instance_id, move_result.as_ref().err(), source_slot_index
        );
    }

    move_result // Return the actual result of the move operation
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