use spacetimedb::{ReducerContext, SpacetimeType, Table};
use log;
// Import ActiveEquipment table definition
// use crate::active_equipment::{ActiveEquipment};
// ADD generated table trait import with alias
use crate::active_equipment::active_equipment as ActiveEquipmentTableTrait;
// Import Campfire table trait
use crate::campfire::campfire as CampfireTableTrait;
// Import Player table trait
use crate::player as PlayerTableTrait;
// Import DroppedItem helpers
use crate::dropped_item::{calculate_drop_position, create_dropped_item_entity};
// REMOVE unused concrete table type imports
// use crate::items::{InventoryItemTable, ItemDefinitionTable};
use crate::items_database; // ADD import for new module
use std::cmp::min;
use spacetimedb::Identity; // ADDED for add_item_to_player_inventory
// Import the ContainerItemClearer trait
use crate::inventory_management::ContainerItemClearer;
// Import the function that was moved
use crate::player_inventory::move_item_to_hotbar;
use crate::player_inventory::move_item_to_inventory;
// Import helper used locally
use crate::player_inventory::find_first_empty_inventory_slot; 

// --- Item Enums and Structs ---

// Define categories or types for items
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, SpacetimeType)]
pub enum ItemCategory {
    Tool,
    Material,
    Placeable,
    Armor,
    Consumable,
    // Add other categories as needed (Consumable, Wearable, etc.)
}

// Define specific slots for equippable armor/items
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, SpacetimeType)]
pub enum EquipmentSlot {
    Head,
    Chest,
    Legs,
    Feet,
    Hands,
    Back,
    // Maybe add Trinket1, Trinket2 etc. later
}

#[spacetimedb::table(name = item_definition, public)]
#[derive(Clone)]
pub struct ItemDefinition {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub name: String,          // Unique name used as an identifier too?
    pub description: String,   // Optional flavor text
    pub category: ItemCategory,
    pub icon_asset_name: String, // e.g., "stone_hatchet.png", used by client
    pub damage: Option<u32>,   // Damage dealt (e.g., by tools)
    pub is_stackable: bool,    // Can multiple instances exist in one inventory slot?
    pub stack_size: u32,       // Max number per stack (if stackable)
    pub is_equippable: bool,   // Can this item be equipped (in hand OR on body)?
    pub equipment_slot: Option<EquipmentSlot>, // If equippable, does it go in a specific body slot?
}

// --- Inventory Table ---

// Represents an instance of an item in a player's inventory
#[spacetimedb::table(name = inventory_item, public)]
#[derive(Clone, Debug)]
pub struct InventoryItem {
    #[primary_key]
    #[auto_inc]
    pub instance_id: u64,      // Unique ID for this specific item instance
    pub player_identity: spacetimedb::Identity, // Who owns this item
    pub item_def_id: u64,      // Links to ItemDefinition table (FK)
    pub quantity: u32,         // How many of this item
    pub hotbar_slot: Option<u8>, // Which hotbar slot (0-5), if any
    pub inventory_slot: Option<u16>, // Which main inventory slot (e.g., 0-23), if any
    // Add other instance-specific data later (e.g., current_durability)
}

// --- Item Reducers ---

// Reducer to seed initial item definitions if the table is empty
#[spacetimedb::reducer]
pub fn seed_items(ctx: &ReducerContext) -> Result<(), String> {
    let items = ctx.db.item_definition();
    if items.iter().count() > 0 {
        log::info!("Item definitions already seeded ({}). Skipping.", items.iter().count());
        return Ok(());
    }

    log::info!("Seeding initial item definitions...");

    let initial_items = items_database::get_initial_item_definitions(); // REPLACE vector literal with function call

    let mut seeded_count = 0;
    for item_def in initial_items {
        match items.try_insert(item_def) {
            Ok(_) => seeded_count += 1,
            Err(e) => log::error!("Failed to insert item definition during seeding: {}", e),
        }
    }

    log::info!("Finished seeding {} item definitions.", seeded_count);
    Ok(())
}

// --- Inventory Management Reducers ---

// Helper to find an item instance owned by the caller
fn get_player_item(ctx: &ReducerContext, instance_id: u64) -> Result<InventoryItem, String> {
    ctx.db
        .inventory_item().iter()
        .filter(|i| i.instance_id == instance_id && i.player_identity == ctx.sender)
        .next()
        .ok_or_else(|| format!("Item instance {} not found or not owned by caller.", instance_id))
}

// Helper to find an item occupying a specific inventory slot for the caller
fn find_item_in_inventory_slot(ctx: &ReducerContext, slot: u16) -> Option<InventoryItem> {
    ctx.db
        .inventory_item().iter()
        .filter(|i| i.player_identity == ctx.sender && i.inventory_slot == Some(slot))
        .next()
}

// Helper to find an item occupying a specific hotbar slot for the caller
fn find_item_in_hotbar_slot(ctx: &ReducerContext, slot: u8) -> Option<InventoryItem> {
    ctx.db
        .inventory_item().iter()
        .filter(|i| i.player_identity == ctx.sender && i.hotbar_slot == Some(slot))
        .next()
}

// Helper to add an item to inventory, prioritizing hotbar for stacking and new slots.
// Called when items are gathered/added directly (e.g., picking mushrooms, gathering resources).
pub(crate) fn add_item_to_player_inventory(ctx: &ReducerContext, player_id: Identity, item_def_id: u64, quantity: u32) -> Result<(), String> {
    let inventory = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();
    let mut remaining_quantity = quantity; // Use remaining_quantity throughout

    let item_def = item_defs.id().find(item_def_id)
        .ok_or_else(|| format!("Item definition {} not found", item_def_id))?;

    // 1. Try to stack onto existing items - PRIORITIZE HOTBAR
    if item_def.is_stackable && remaining_quantity > 0 {
        let mut items_to_update: Vec<crate::items::InventoryItem> = Vec::new();

        // --- Stack on Hotbar First ---
        for mut item in inventory.iter().filter(|i| i.player_identity == player_id && i.item_def_id == item_def_id && i.hotbar_slot.is_some()) {
            let space_available = item_def.stack_size.saturating_sub(item.quantity);
            if space_available > 0 {
                let transfer_qty = std::cmp::min(remaining_quantity, space_available);
                item.quantity += transfer_qty;
                remaining_quantity -= transfer_qty;
                items_to_update.push(item); // Add item to update list
                if remaining_quantity == 0 { break; } // Done stacking
            }
        }

        // --- Then Stack on Inventory ---
        if remaining_quantity > 0 {
            for mut item in inventory.iter().filter(|i| i.player_identity == player_id && i.item_def_id == item_def_id && i.inventory_slot.is_some()) {
                let space_available = item_def.stack_size.saturating_sub(item.quantity);
                if space_available > 0 {
                    let transfer_qty = std::cmp::min(remaining_quantity, space_available);
                    item.quantity += transfer_qty;
                    remaining_quantity -= transfer_qty;
                    items_to_update.push(item); // Add item to update list
                    if remaining_quantity == 0 { break; } // Done stacking
                }
            }
        }

        // Apply updates if any stacking occurred
        for item in items_to_update {
             inventory.instance_id().update(item);
        }

        // If quantity fully stacked, return early
        if remaining_quantity == 0 {
            log::info!("[AddItem] Fully stacked {} of item def {} for player {:?}.", quantity, item_def_id, player_id);
            return Ok(());
        }
    } // End of stacking logic

    // If quantity still remains (or item not stackable), find an empty slot
    if remaining_quantity > 0 {
        let final_quantity_to_add = if item_def.is_stackable { remaining_quantity } else { 1 }; // Non-stackable always adds 1

        // 2. Find first empty HOTBAR slot
        let occupied_hotbar_slots: std::collections::HashSet<u8> = inventory.iter()
            .filter(|i| i.player_identity == player_id && i.hotbar_slot.is_some())
            .map(|i| i.hotbar_slot.unwrap())
            .collect();

        if let Some(empty_hotbar_slot) = (0..6).find(|slot| !occupied_hotbar_slots.contains(slot)) {
            // Found empty hotbar slot
            let new_item = crate::items::InventoryItem {
                instance_id: 0, // Auto-inc
                player_identity: player_id,
                item_def_id,
                quantity: final_quantity_to_add,
                hotbar_slot: Some(empty_hotbar_slot),
                inventory_slot: None,
            };
            inventory.insert(new_item);
            log::info!("[AddItem] Added {} of item def {} to hotbar slot {} for player {:?}.",
                     final_quantity_to_add, item_def_id, empty_hotbar_slot, player_id);
            return Ok(()); // Item added successfully
        } else {
             // 3. Hotbar full, find first empty INVENTORY slot
            let occupied_inventory_slots: std::collections::HashSet<u16> = inventory.iter()
                .filter(|i| i.player_identity == player_id && i.inventory_slot.is_some())
                .map(|i| i.inventory_slot.unwrap())
                .collect();

            if let Some(empty_inventory_slot) = (0..24).find(|slot| !occupied_inventory_slots.contains(slot)) {
                // Found empty inventory slot
                let new_item = crate::items::InventoryItem {
                    instance_id: 0, // Auto-inc
                    player_identity: player_id,
                    item_def_id,
                    quantity: final_quantity_to_add,
                    hotbar_slot: None,
                    inventory_slot: Some(empty_inventory_slot),
                };
                inventory.insert(new_item);
                log::info!("[AddItem] Added {} of item def {} to inventory slot {} for player {:?}. (Hotbar was full)",
                         final_quantity_to_add, item_def_id, empty_inventory_slot, player_id);
                return Ok(()); // Item added successfully
            } else {
                // 4. Both hotbar and inventory are full
                log::error!("[AddItem] No empty hotbar or inventory slots for player {:?} to add item def {}.", player_id, item_def_id);
                return Err("Inventory is full".to_string());
            }
        }
    } else {
         // This case should only be reached if stacking happened perfectly and remaining_quantity became 0
         // No further action needed, the stacking return above handles this.
         log::debug!("[AddItem] Stacking completed successfully for item def {} for player {:?}. No new slot needed.", item_def_id, player_id);
         Ok(())
    }
}

// Helper to clear a specific item instance from any equipment slot it might occupy
pub(crate) fn clear_specific_item_from_equipment_slots(ctx: &ReducerContext, player_id: spacetimedb::Identity, item_instance_id_to_clear: u64) {
    let active_equip_table = ctx.db.active_equipment();
    // Use try_find to avoid panic if player has no equipment entry yet
    if let Some(mut equip) = active_equip_table.player_identity().find(player_id) {
        let mut updated = false;

        // Check main hand
        if equip.equipped_item_instance_id == Some(item_instance_id_to_clear) {
             equip.equipped_item_instance_id = None;
             equip.equipped_item_def_id = None;
             equip.swing_start_time_ms = 0;
             updated = true;
             log::debug!("[ClearEquip] Removed item {} from main hand slot for player {:?}", item_instance_id_to_clear, player_id);
        }
        // Check armor slots
        if equip.head_item_instance_id == Some(item_instance_id_to_clear) {
            equip.head_item_instance_id = None;
            updated = true;
            log::debug!("[ClearEquip] Removed item {} from Head slot for player {:?}", item_instance_id_to_clear, player_id);
        }
        if equip.chest_item_instance_id == Some(item_instance_id_to_clear) {
            equip.chest_item_instance_id = None;
            updated = true;
            log::debug!("[ClearEquip] Removed item {} from Chest slot for player {:?}", item_instance_id_to_clear, player_id);
        }
        if equip.legs_item_instance_id == Some(item_instance_id_to_clear) {
            equip.legs_item_instance_id = None;
            updated = true;
            log::debug!("[ClearEquip] Removed item {} from Legs slot for player {:?}", item_instance_id_to_clear, player_id);
        }
        if equip.feet_item_instance_id == Some(item_instance_id_to_clear) {
            equip.feet_item_instance_id = None;
            updated = true;
            log::debug!("[ClearEquip] Removed item {} from Feet slot for player {:?}", item_instance_id_to_clear, player_id);
        }
        if equip.hands_item_instance_id == Some(item_instance_id_to_clear) {
            equip.hands_item_instance_id = None;
            updated = true;
            log::debug!("[ClearEquip] Removed item {} from Hands slot for player {:?}", item_instance_id_to_clear, player_id);
        }
        if equip.back_item_instance_id == Some(item_instance_id_to_clear) {
            equip.back_item_instance_id = None;
            updated = true;
            log::debug!("[ClearEquip] Removed item {} from Back slot for player {:?}", item_instance_id_to_clear, player_id);
        }

        if updated {
            active_equip_table.player_identity().update(equip);
        }
    } else {
        // This is not necessarily an error, player might not have equipment entry yet
        log::debug!("[ClearEquip] No ActiveEquipment found for player {:?} when trying to clear item {}.", player_id, item_instance_id_to_clear);
    }
}

// Checks all registered container types and removes the specified item instance if found.
// This function delegates to specific container modules via the ContainerItemClearer trait.
pub(crate) fn clear_item_from_any_container(ctx: &ReducerContext, item_instance_id: u64) {
    // Delegate to container-specific clearing functions
    // Each returns a boolean indicating if the item was found and cleared
    
    // Check wooden storage boxes
    let found_in_boxes = crate::wooden_storage_box::WoodenStorageBoxClearer::clear_item(ctx, item_instance_id);
    
    // If not found in boxes, check campfires
    if !found_in_boxes {
        // Call the function from campfire module
        let _found_in_campfire = crate::campfire::clear_item_from_campfire_fuel_slots(ctx, item_instance_id);
    }
    
    // Additional container types can be added here in the future:
    // if !found_in_boxes && !found_in_campfire {
    //     crate::some_container::SomeContainerClearer::clear_item(ctx, item_instance_id);
    // }
}

// Clears an item from equipment OR container slots based on its state
// This should be called *before* modifying or deleting the InventoryItem itself.
fn clear_item_from_source_location(ctx: &ReducerContext, item_instance_id: u64) -> Result<(), String> {
    let sender_id = ctx.sender; // Assume the operation is initiated by the sender

    // Check if item exists
    let item_opt = ctx.db.inventory_item().instance_id().find(item_instance_id);

    if item_opt.is_none() {
        log::debug!("[ClearSource] Item {} already gone. No clearing needed.", item_instance_id);
        return Ok(());
    }
    let item = item_opt.unwrap(); // Safe to unwrap now

    // Determine if it was equipped or in a container (not in player inv/hotbar)
    let was_equipped_or_in_container = item.inventory_slot.is_none() && item.hotbar_slot.is_none();

    if was_equipped_or_in_container {
        // Try clearing from equipment first
        clear_specific_item_from_equipment_slots(ctx, sender_id, item_instance_id);

        // Then try clearing from ANY container 
        // This will delegate to campfire, box, etc.
        clear_item_from_any_container(ctx, item_instance_id);

        log::debug!("[ClearSource] Attempted clearing item {} from equipment/container slots for player {:?}", item_instance_id, sender_id);
    } else {
        log::debug!("[ClearSource] Item {} was in inventory/hotbar. No equipment/container clearing needed.", item_instance_id);
    }

    Ok(())
}

// Reducer to equip armor from a drag-and-drop operation
#[spacetimedb::reducer]
pub fn equip_armor_from_drag(ctx: &ReducerContext, item_instance_id: u64, target_slot_name: String) -> Result<(), String> {
    log::info!("[EquipArmorDrag] Attempting to equip item {} to slot {}", item_instance_id, target_slot_name);
    let sender_id = ctx.sender; // Get sender early
    let inventory_items = ctx.db.inventory_item(); // Need table access

    // 1. Get Item and Definition (Fetch directly, don't assume player ownership yet)
    let mut item_to_equip = inventory_items.instance_id().find(item_instance_id)
        .ok_or_else(|| format!("Item instance {} not found.", item_instance_id))?;
    let item_def = ctx.db.item_definition().id().find(item_to_equip.item_def_id)
        .ok_or_else(|| format!("Definition not found for item ID {}", item_to_equip.item_def_id))?;

    // --- Store original location --- 
    let original_inv_slot = item_to_equip.inventory_slot;
    let original_hotbar_slot = item_to_equip.hotbar_slot;
    let came_from_player_inv = original_inv_slot.is_some() || original_hotbar_slot.is_some();

    // --- Validations --- 
    // Basic ownership check: Player must own it if it came from inv/hotbar
    if came_from_player_inv && item_to_equip.player_identity != sender_id {
        return Err(format!("Item {} in inventory/hotbar not owned by caller.", item_instance_id));
    }
    // 1. Must be Armor category
    if item_def.category != ItemCategory::Armor {
        return Err(format!("Item '{}' is not armor.", item_def.name));
    }
    // 2. Must have a defined equipment slot
    let required_slot_enum = item_def.equipment_slot.ok_or_else(|| format!("Armor '{}' has no defined equipment slot in its definition.", item_def.name))?;
    // 3. Target slot name must match the item's defined equipment slot
    let target_slot_enum = match target_slot_name.as_str() {
        "Head" => EquipmentSlot::Head,
        "Chest" => EquipmentSlot::Chest,
        "Legs" => EquipmentSlot::Legs,
        "Feet" => EquipmentSlot::Feet,
        "Hands" => EquipmentSlot::Hands,
        "Back" => EquipmentSlot::Back,
        _ => return Err(format!("Invalid target equipment slot name: {}", target_slot_name)),
    };
    if required_slot_enum != target_slot_enum {
        return Err(format!("Cannot equip '{}' ({:?}) into {} slot ({:?}).", item_def.name, required_slot_enum, target_slot_name, target_slot_enum));
    }

    // --- Logic ---
    let active_equip_table = ctx.db.active_equipment();
    let mut equip = active_equip_table.player_identity().find(sender_id)
                     .ok_or_else(|| "ActiveEquipment entry not found for player.".to_string())?;

    // Check if something is already in the target slot and unequip it
    let current_item_in_slot: Option<u64> = match target_slot_enum {
        EquipmentSlot::Head => equip.head_item_instance_id,
        EquipmentSlot::Chest => equip.chest_item_instance_id,
        EquipmentSlot::Legs => equip.legs_item_instance_id,
        EquipmentSlot::Feet => equip.feet_item_instance_id,
        EquipmentSlot::Hands => equip.hands_item_instance_id,
        EquipmentSlot::Back => equip.back_item_instance_id,
    };

    if let Some(currently_equipped_id) = current_item_in_slot {
        if currently_equipped_id == item_instance_id { return Ok(()); } // Already equipped

        log::info!("[EquipArmorDrag] Unequipping item {} from slot {:?}", currently_equipped_id, target_slot_enum);
        // Try to move the currently equipped item to the first available inventory slot
        match find_first_empty_inventory_slot(ctx, sender_id) {
            Some(empty_slot) => {
                if let Ok(mut currently_equipped_item) = get_player_item(ctx, currently_equipped_id) {
                    currently_equipped_item.inventory_slot = Some(empty_slot);
                    currently_equipped_item.hotbar_slot = None;
                    ctx.db.inventory_item().instance_id().update(currently_equipped_item);
                    log::info!("[EquipArmorDrag] Moved previously equipped item {} to inventory slot {}", currently_equipped_id, empty_slot);
                } else {
                    log::error!("[EquipArmorDrag] Failed to find InventoryItem for previously equipped item {}!", currently_equipped_id);
                    // Continue anyway, clearing the slot, but log the error
                }
            }
            None => {
                log::error!("[EquipArmorDrag] Inventory full! Cannot unequip item {} from slot {:?}. Aborting equip.", currently_equipped_id, target_slot_enum);
                return Err("Inventory full, cannot unequip existing item.".to_string());
            }
        }
    }

    // Equip the new item
    log::info!("[EquipArmorDrag] Equipping item {} to slot {:?}", item_instance_id, target_slot_enum);
    match target_slot_enum {
        EquipmentSlot::Head => equip.head_item_instance_id = Some(item_instance_id),
        EquipmentSlot::Chest => equip.chest_item_instance_id = Some(item_instance_id),
        EquipmentSlot::Legs => equip.legs_item_instance_id = Some(item_instance_id),
        EquipmentSlot::Feet => equip.feet_item_instance_id = Some(item_instance_id),
        EquipmentSlot::Hands => equip.hands_item_instance_id = Some(item_instance_id),
        EquipmentSlot::Back => equip.back_item_instance_id = Some(item_instance_id),
    };

    // Update ActiveEquipment table
    active_equip_table.player_identity().update(equip);

    // Clear the original slot of the equipped item
    if came_from_player_inv {
        log::debug!("[EquipArmorDrag] Clearing original inv/hotbar slot for item {}.", item_instance_id);
        item_to_equip.inventory_slot = None;
        item_to_equip.hotbar_slot = None;
        inventory_items.instance_id().update(item_to_equip); // Update the item itself
    } else {
        log::debug!("[EquipArmorDrag] Item {} potentially came from container. Clearing containers.", item_instance_id);
        // Item didn't come from player inv/hotbar, try clearing containers
        clear_item_from_any_container(ctx, item_instance_id);
        // Also update the item instance itself to remove slot info just in case (should be None already)
        // and assign ownership to the equipping player if it wasn't already.
        if item_to_equip.player_identity != sender_id {
             item_to_equip.player_identity = sender_id;
        }
        item_to_equip.inventory_slot = None; 
        item_to_equip.hotbar_slot = None;
        inventory_items.instance_id().update(item_to_equip);
    }

    Ok(())
}

// Calculates the result of merging source onto target
// Returns: (qty_to_transfer, source_new_qty, target_new_qty, delete_source)
pub(crate) fn calculate_merge_result(
    source_item: &InventoryItem,
    target_item: &InventoryItem, 
    item_def: &ItemDefinition
) -> Result<(u32, u32, u32, bool), String> {
    if !item_def.is_stackable || source_item.item_def_id != target_item.item_def_id {
        return Err("Items cannot be merged".to_string());
    }

    let space_available = item_def.stack_size.saturating_sub(target_item.quantity);
    if space_available == 0 {
        return Err("Target stack is full".to_string()); // Or handle as a swap later
    }

    let qty_to_transfer = std::cmp::min(source_item.quantity, space_available);
    let source_new_qty = source_item.quantity - qty_to_transfer;
    let target_new_qty = target_item.quantity + qty_to_transfer;
    let delete_source = source_new_qty == 0;

    Ok((qty_to_transfer, source_new_qty, target_new_qty, delete_source))
}

// Renamed helper function
pub(crate) fn split_stack_helper(
    ctx: &ReducerContext,
    source_item: &mut InventoryItem, // Takes mutable reference to modify quantity
    quantity_to_split: u32
) -> Result<u64, String> {
    // Validations already done in reducers calling this, but sanity check:
    if quantity_to_split == 0 || quantity_to_split >= source_item.quantity {
        return Err("Invalid split quantity".to_string());
    }

    // Decrease quantity of the source item
    source_item.quantity -= quantity_to_split;
    // Update source item in DB *before* creating new one to avoid potential unique constraint issues if moved immediately
    ctx.db.inventory_item().instance_id().update(source_item.clone()); 

    // Create the new item stack with the split quantity
    let new_item = InventoryItem {
        instance_id: 0, // Will be auto-generated
        player_identity: source_item.player_identity, // Initially belongs to the same player
        item_def_id: source_item.item_def_id,
        quantity: quantity_to_split,
        hotbar_slot: None, // New item has no location yet
        inventory_slot: None,
    };
    let inserted_item = ctx.db.inventory_item().insert(new_item);
    let new_instance_id = inserted_item.instance_id;

    log::info!(
        "[SplitStack Helper] Split {} from item {}. New stack ID: {}. Original stack qty: {}.",
        quantity_to_split, source_item.instance_id, new_instance_id, source_item.quantity
    );

    Ok(new_instance_id)
}

// --- NEW: Drop Item into the World ---
#[spacetimedb::reducer]
pub fn drop_item(
    ctx: &ReducerContext,
    item_instance_id: u64,
    quantity_to_drop: u32, // How many to drop (can be less than total stack)
) -> Result<(), String> {
    let sender_id = ctx.sender;
    log::info!("[DropItem] Player {:?} attempting to drop {} of item instance {}", sender_id, quantity_to_drop, item_instance_id);

    // --- 1. Find Player ---
    let player = ctx.db.player().identity().find(sender_id)
        .ok_or_else(|| "Player not found.".to_string())?;

    // --- 2. Find Item & Validate ---
    let mut item_to_drop = ctx.db.inventory_item().instance_id().find(item_instance_id)
        .ok_or_else(|| format!("Item instance {} not found.", item_instance_id))?;

    let was_originally_equipped_or_fuel = item_to_drop.inventory_slot.is_none() && item_to_drop.hotbar_slot.is_none();

    // Validate ownership if it wasn't equipped/fuel
    if !was_originally_equipped_or_fuel && item_to_drop.player_identity != sender_id {
        return Err(format!("Item instance {} not owned by caller.", item_instance_id));
    }
    // Validate quantity
    if quantity_to_drop == 0 {
        return Err("Cannot drop a quantity of 0.".to_string());
    }
    if quantity_to_drop > item_to_drop.quantity {
        return Err(format!("Cannot drop {} items, only {} available in stack.", quantity_to_drop, item_to_drop.quantity));
    }

    // --- 3. Get Item Definition ---
    let item_def = ctx.db.item_definition().id().find(item_to_drop.item_def_id)
        .ok_or_else(|| format!("Definition missing for item {}", item_to_drop.item_def_id))?;

    // Temporarily comment out the problematic call
    // clear_item_from_source_location(ctx, item_instance_id)?;
    // Restore the call now that the helper is fixed
    clear_item_from_source_location(ctx, item_instance_id)?;

    // --- 4.5 NEW: Check if dropped item was the EQUIPPED item and unequip --- 
    let active_equip_table = ctx.db.active_equipment();
    if let Some(mut equip) = active_equip_table.player_identity().find(sender_id) {
         // Check only the main hand slot (equipped_item_instance_id)
         if equip.equipped_item_instance_id == Some(item_instance_id) {
            log::info!("[DropItem] Dropped item {} was equipped. Unequipping.", item_instance_id);
            equip.equipped_item_instance_id = None;
            equip.equipped_item_def_id = None;
            equip.swing_start_time_ms = 0;
            active_equip_table.player_identity().update(equip); // Update the equipment table
         }
    }
    // No need to check armor slots here, as dropping is usually from hotbar/inventory
    // Armor unequipping happens via equip_armor_from_drag or potentially a context menu action.

    // --- 5. Calculate Drop Position ---
    let (drop_x, drop_y) = calculate_drop_position(&player);
    log::debug!("[DropItem] Calculated drop position: ({:.1}, {:.1}) for player {:?}", drop_x, drop_y, sender_id);
    // TODO: Add collision check for drop position? Ensure it's not inside a wall/tree? For now, just place it.

    // --- 6. Handle Item Quantity (Split or Delete Original) ---
    if quantity_to_drop == item_to_drop.quantity {
        // Dropping the entire stack
        log::info!("[DropItem] Dropping entire stack (ID: {}, Qty: {}). Deleting original InventoryItem.", item_instance_id, quantity_to_drop);
        ctx.db.inventory_item().instance_id().delete(item_instance_id);
    } else {
        // Dropping part of the stack
        // Need to check if the item is actually stackable for splitting (though UI should prevent this)
        if !item_def.is_stackable {
             return Err(format!("Cannot drop partial quantity of non-stackable item '{}'.", item_def.name));
        }
        log::info!("[DropItem] Dropping partial stack (ID: {}, QtyDrop: {}). Reducing original quantity.", item_instance_id, quantity_to_drop);
        item_to_drop.quantity -= quantity_to_drop;
        // If the item was originally equip/fuel, assign ownership to the sender now
        if was_originally_equipped_or_fuel {
             item_to_drop.player_identity = sender_id;
             log::debug!("[DropItem] Assigning ownership of remaining stack {} to player {:?}", item_instance_id, sender_id);
        }
        ctx.db.inventory_item().instance_id().update(item_to_drop);
    }

    // --- 7. Create Dropped Item Entity in World ---
    create_dropped_item_entity(ctx, item_def.id, quantity_to_drop, drop_x, drop_y)?;

    log::info!("[DropItem] Successfully dropped {} of item def {} (Original ID: {}) at ({:.1}, {:.1}) for player {:?}.",
             quantity_to_drop, item_def.id, item_instance_id, drop_x, drop_y, sender_id);

    Ok(())
}

// --- NEW: Reducer to equip armor directly from inventory/hotbar ---
#[spacetimedb::reducer]
pub fn equip_armor_from_inventory(ctx: &ReducerContext, item_instance_id: u64) -> Result<(), String> {
    let sender_id = ctx.sender;
    log::info!("[EquipArmorInv] Player {:?} attempting to equip item {} from inventory/hotbar.", sender_id, item_instance_id);

    // 1. Get Item and Definition
    let mut item_to_equip = get_player_item(ctx, item_instance_id)?;
    let item_def = ctx.db.item_definition().id().find(item_to_equip.item_def_id)
        .ok_or_else(|| format!("Definition not found for item ID {}", item_to_equip.item_def_id))?;

    // 2. Validate Item Type and Location
    if item_def.category != ItemCategory::Armor {
        return Err(format!("Item '{}' is not armor.", item_def.name));
    }
    let target_slot_enum = item_def.equipment_slot
        .ok_or_else(|| format!("Armor '{}' has no defined equipment slot.", item_def.name))?;
    if item_to_equip.inventory_slot.is_none() && item_to_equip.hotbar_slot.is_none() {
        return Err("Item must be in inventory or hotbar to be equipped this way.".to_string());
    }

    // 3. Get ActiveEquipment and Handle Unequipping Existing Item
    let active_equip_table = ctx.db.active_equipment();
    let mut equip = active_equip_table.player_identity().find(sender_id)
                     .ok_or_else(|| "ActiveEquipment entry not found for player.".to_string())?;

    let current_item_in_slot_id: Option<u64> = match target_slot_enum {
        EquipmentSlot::Head => equip.head_item_instance_id,
        EquipmentSlot::Chest => equip.chest_item_instance_id,
        EquipmentSlot::Legs => equip.legs_item_instance_id,
        EquipmentSlot::Feet => equip.feet_item_instance_id,
        EquipmentSlot::Hands => equip.hands_item_instance_id,
        EquipmentSlot::Back => equip.back_item_instance_id,
    };

    if let Some(currently_equipped_id) = current_item_in_slot_id {
        if currently_equipped_id == item_instance_id { return Ok(()); } // Already equipped in the correct slot

        log::info!("[EquipArmorInv] Unequipping item {} from slot {:?}.", currently_equipped_id, target_slot_enum);
        match find_first_empty_inventory_slot(ctx, sender_id) {
            Some(empty_slot) => {
                if let Ok(mut currently_equipped_item) = get_player_item(ctx, currently_equipped_id) {
                    currently_equipped_item.inventory_slot = Some(empty_slot);
                    currently_equipped_item.hotbar_slot = None;
                    ctx.db.inventory_item().instance_id().update(currently_equipped_item);
                    log::info!("[EquipArmorInv] Moved previously equipped item {} to inventory slot {}.", currently_equipped_id, empty_slot);
                    // Clear the slot in ActiveEquipment *after* successfully moving the old item
                    match target_slot_enum {
                        EquipmentSlot::Head => equip.head_item_instance_id = None,
                        EquipmentSlot::Chest => equip.chest_item_instance_id = None,
                        EquipmentSlot::Legs => equip.legs_item_instance_id = None,
                        EquipmentSlot::Feet => equip.feet_item_instance_id = None,
                        EquipmentSlot::Hands => equip.hands_item_instance_id = None,
                        EquipmentSlot::Back => equip.back_item_instance_id = None,
                    };
                } else {
                    log::error!("[EquipArmorInv] Failed to find InventoryItem for previously equipped item {}! Aborting equip.", currently_equipped_id);
                    return Err("Failed to process currently equipped item.".to_string());
                }
            }
            None => {
                log::error!("[EquipArmorInv] Inventory full! Cannot unequip item {} from slot {:?}. Aborting equip.", currently_equipped_id, target_slot_enum);
                return Err("Inventory full, cannot unequip existing item.".to_string());
            }
        }
    } // End handling currently equipped item

    // 4. Equip the New Item
    log::info!("[EquipArmorInv] Equipping item {} to slot {:?}.", item_instance_id, target_slot_enum);
    match target_slot_enum {
        EquipmentSlot::Head => equip.head_item_instance_id = Some(item_instance_id),
        EquipmentSlot::Chest => equip.chest_item_instance_id = Some(item_instance_id),
        EquipmentSlot::Legs => equip.legs_item_instance_id = Some(item_instance_id),
        EquipmentSlot::Feet => equip.feet_item_instance_id = Some(item_instance_id),
        EquipmentSlot::Hands => equip.hands_item_instance_id = Some(item_instance_id),
        EquipmentSlot::Back => equip.back_item_instance_id = Some(item_instance_id),
    };
    active_equip_table.player_identity().update(equip);

    // 5. Clear the Inventory/Hotbar Slot of the Newly Equipped Item
    item_to_equip.inventory_slot = None;
    item_to_equip.hotbar_slot = None;
    ctx.db.inventory_item().instance_id().update(item_to_equip);

    Ok(())
} 