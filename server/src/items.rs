use spacetimedb::{ReducerContext, SpacetimeType, Table};
use log;
// Import ActiveEquipment table definition
// use crate::active_equipment::{ActiveEquipment};
// ADD generated table trait import with alias
use crate::active_equipment::active_equipment as ActiveEquipmentTableTrait;
// Import Campfire table trait
use crate::campfire::campfire as CampfireTableTrait;
// REMOVE unused concrete table type imports
// use crate::items::{InventoryItemTable, ItemDefinitionTable};
use std::cmp::min;
use spacetimedb::Identity; // ADDED for add_item_to_player_inventory

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

    let initial_items = vec![
        ItemDefinition {
            id: 0,
            name: "Wood".to_string(),
            description: "A sturdy piece of wood.".to_string(),
            category: ItemCategory::Material,
            icon_asset_name: "wood.png".to_string(),
            damage: None,
            is_stackable: true,
            stack_size: 1000,
            is_equippable: false,
            equipment_slot: None,
        },
        ItemDefinition {
            id: 0,
            name: "Stone".to_string(),
            description: "A chunk of rock.".to_string(),
            category: ItemCategory::Material,
            icon_asset_name: "stone.png".to_string(),
            damage: None,
            is_stackable: true,
            stack_size: 1000,
            is_equippable: false,
            equipment_slot: None,
        },
        ItemDefinition {
            id: 0,
            name: "Stone Hatchet".to_string(),
            description: "A simple hatchet for chopping wood.".to_string(),
            category: ItemCategory::Tool,
            icon_asset_name: "wood_hatchet.png".to_string(),
            damage: Some(5),
            is_stackable: false,
            stack_size: 1,
            is_equippable: true,
            equipment_slot: None,
        },
        ItemDefinition {
            id: 0,
            name: "Stone Pickaxe".to_string(),
            description: "A simple pickaxe for breaking rocks.".to_string(),
            category: ItemCategory::Tool,
            icon_asset_name: "pick_axe.png".to_string(),
            damage: Some(5),
            is_stackable: false,
            stack_size: 1,
            is_equippable: true,
            equipment_slot: None,
        },
        ItemDefinition {
            id: 0,
            name: "Rock".to_string(),
            description: "A basic tool for gathering.".to_string(),
            category: ItemCategory::Tool,
            icon_asset_name: "rock_item.png".to_string(),
            damage: Some(1),
            is_stackable: false,
            stack_size: 1,
            is_equippable: true,
            equipment_slot: None,
        },
        ItemDefinition {
            id: 0,
            name: "Camp Fire".to_string(),
            description: "Provides warmth and light. Requires fuel.".to_string(),
            category: ItemCategory::Placeable,
            icon_asset_name: "campfire.png".to_string(),
            damage: None,
            is_stackable: false,
            stack_size: 1,
            is_equippable: false,
            equipment_slot: None,
        },
        ItemDefinition {
            id: 0,
            name: "Cloth Shirt".to_string(),
            description: "Simple protection for the torso.".to_string(),
            category: ItemCategory::Armor,
            icon_asset_name: "cloth_shirt.png".to_string(),
            damage: None,
            is_stackable: false,
            stack_size: 1,
            is_equippable: true,
            equipment_slot: Some(EquipmentSlot::Chest),
        },
        ItemDefinition {
            id: 0,
            name: "Cloth Pants".to_string(),
            description: "Simple protection for the legs.".to_string(),
            category: ItemCategory::Armor,
            icon_asset_name: "cloth_pants.png".to_string(),
            damage: None,
            is_stackable: false,
            stack_size: 1,
            is_equippable: true,
            equipment_slot: Some(EquipmentSlot::Legs),
        },
        ItemDefinition {
            id: 0,
            name: "Cloth Hood".to_string(),
            description: "Basic head covering.".to_string(),
            category: ItemCategory::Armor,
            icon_asset_name: "cloth_hood.png".to_string(),
            damage: None,
            is_stackable: false,
            stack_size: 1,
            is_equippable: true,
            equipment_slot: Some(EquipmentSlot::Head),
        },
        ItemDefinition {
            id: 0,
            name: "Cloth Boots".to_string(),
            description: "Simple footwear.".to_string(),
            category: ItemCategory::Armor,
            icon_asset_name: "cloth_boots.png".to_string(),
            damage: None,
            is_stackable: false,
            stack_size: 1,
            is_equippable: true,
            equipment_slot: Some(EquipmentSlot::Feet),
        },
        ItemDefinition {
            id: 0,
            name: "Cloth Gloves".to_string(),
            description: "Basic hand coverings.".to_string(),
            category: ItemCategory::Armor,
            icon_asset_name: "cloth_gloves.png".to_string(),
            damage: None,
            is_stackable: false,
            stack_size: 1,
            is_equippable: true,
            equipment_slot: Some(EquipmentSlot::Hands),
        },
        ItemDefinition {
            id: 0,
            name: "Burlap Backpack".to_string(),
            description: "A rough sack for carrying things.".to_string(),
            category: ItemCategory::Armor,
            icon_asset_name: "burlap_backpack.png".to_string(),
            damage: None,
            is_stackable: false,
            stack_size: 1,
            is_equippable: true,
            equipment_slot: Some(EquipmentSlot::Back),
        },
        ItemDefinition {
            id: 0,
            name: "Mushroom".to_string(),
            description: "A common edible fungus.".to_string(),
            category: ItemCategory::Consumable,
            icon_asset_name: "mushroom.png".to_string(),
            damage: None,
            is_stackable: true,
            stack_size: 50,
            is_equippable: false,
            equipment_slot: None,
        },
    ];

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

// Function to find the first available inventory slot (0-23)
// Needs to be pub(crate) to be callable from other modules like campfire.rs
pub(crate) fn find_first_empty_inventory_slot(ctx: &ReducerContext, player_id: Identity) -> Option<u16> {
    let occupied_slots: std::collections::HashSet<u16> = ctx.db
        .inventory_item().iter()
        .filter(|i| i.player_identity == player_id && i.inventory_slot.is_some())
        .map(|i| i.inventory_slot.unwrap())
        .collect();

    // Assuming 24 inventory slots (0-23)
    (0..24).find(|slot| !occupied_slots.contains(slot))
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
fn clear_specific_item_from_equipment_slots(ctx: &ReducerContext, player_id: spacetimedb::Identity, item_instance_id_to_clear: u64) {
    let active_equip_table = ctx.db.active_equipment();
    if let Some(mut equip) = active_equip_table.player_identity().find(player_id) {
        let mut updated = false;

        // Check main hand (less likely for armor, but good practice)
        if equip.equipped_item_instance_id == Some(item_instance_id_to_clear) {
             equip.equipped_item_instance_id = None;
             equip.equipped_item_def_id = None;
             equip.swing_start_time_ms = 0;
             updated = true;
             log::debug!("[ClearSpecificEquip] Removed item {} from main hand slot for player {:?}", item_instance_id_to_clear, player_id);
        }
        // Check armor slots
        if equip.head_item_instance_id == Some(item_instance_id_to_clear) {
            equip.head_item_instance_id = None;
            updated = true;
            log::debug!("[ClearSpecificEquip] Removed item {} from Head slot for player {:?}", item_instance_id_to_clear, player_id);
        }
        if equip.chest_item_instance_id == Some(item_instance_id_to_clear) {
            equip.chest_item_instance_id = None;
            updated = true;
            log::debug!("[ClearSpecificEquip] Removed item {} from Chest slot for player {:?}", item_instance_id_to_clear, player_id);
        }
        if equip.legs_item_instance_id == Some(item_instance_id_to_clear) {
            equip.legs_item_instance_id = None;
            updated = true;
            log::debug!("[ClearSpecificEquip] Removed item {} from Legs slot for player {:?}", item_instance_id_to_clear, player_id);
        }
        if equip.feet_item_instance_id == Some(item_instance_id_to_clear) {
            equip.feet_item_instance_id = None;
            updated = true;
            log::debug!("[ClearSpecificEquip] Removed item {} from Feet slot for player {:?}", item_instance_id_to_clear, player_id);
        }
        if equip.hands_item_instance_id == Some(item_instance_id_to_clear) {
            equip.hands_item_instance_id = None;
            updated = true;
            log::debug!("[ClearSpecificEquip] Removed item {} from Hands slot for player {:?}", item_instance_id_to_clear, player_id);
        }
        if equip.back_item_instance_id == Some(item_instance_id_to_clear) {
            equip.back_item_instance_id = None;
            updated = true;
            log::debug!("[ClearSpecificEquip] Removed item {} from Back slot for player {:?}", item_instance_id_to_clear, player_id);
        }

        if updated {
            active_equip_table.player_identity().update(equip);
        }
    } else {
        log::warn!("[ClearSpecificEquip] Could not find ActiveEquipment for player {:?} when trying to clear item {}.", player_id, item_instance_id_to_clear);
    }
}

// NEW Helper: Clear a specific item instance from any campfire fuel slot
fn clear_item_from_campfire_fuel_slots(ctx: &ReducerContext, item_instance_id_to_clear: u64) {
    let mut campfires = ctx.db.campfire();
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
                let still_has_fuel = crate::campfire::check_if_campfire_has_fuel(ctx, &campfire);
                 if !still_has_fuel && campfire.is_burning {
                    campfire.is_burning = false;
                    campfire.next_fuel_consume_at = None;
                    log::info!("Campfire {} extinguished as last valid fuel was removed during item move.", campfire_id);
                }
                campfires.id().update(campfire);
            }
        }
    }
}

#[spacetimedb::reducer]
pub fn move_item_to_inventory(ctx: &ReducerContext, item_instance_id: u64, target_inventory_slot: u16) -> Result<(), String> {
    let sender_id = ctx.sender;
    log::info!("[MoveToInv] Player {:?} attempting move item {} to inv slot {}", sender_id, item_instance_id, target_inventory_slot);
    
    let mut item_to_move = ctx.db.inventory_item().instance_id().find(item_instance_id)
        .ok_or_else(|| format!("Item instance {} not found.", item_instance_id))?;

    let came_from_equip_or_fuel = item_to_move.inventory_slot.is_none() && item_to_move.hotbar_slot.is_none();
    if item_to_move.player_identity != sender_id && !came_from_equip_or_fuel {
        return Err(format!("Item instance {} not owned by caller or not movable.", item_instance_id));
    }

    // --- NEW: Check if item is currently fuel in a campfire --- 
    let mut maybe_campfire_to_update: Option<crate::campfire::Campfire> = None;
    if came_from_equip_or_fuel { // Only check campfires if item was in inv/hotbar
        let campfires = ctx.db.campfire(); // Get immutable borrow first
        for campfire in campfires.iter() {
            // Check ALL slots
            if campfire.fuel_instance_id_0 == Some(item_instance_id) ||
               campfire.fuel_instance_id_1 == Some(item_instance_id) ||
               campfire.fuel_instance_id_2 == Some(item_instance_id) ||
               campfire.fuel_instance_id_3 == Some(item_instance_id) ||
               campfire.fuel_instance_id_4 == Some(item_instance_id) 
            {
                log::debug!("[MoveToInv] Item {} is currently fuel for campfire {}. Will clear campfire state.", item_instance_id, campfire.id);
                maybe_campfire_to_update = Some(campfire); // Clone the campfire data
                break; // Assume item can only be fuel for one fire
            }
        }
    }

    // --- Clear the associated campfire state IF found --- 
    if let Some(mut campfire_to_update) = maybe_campfire_to_update {
        // Clear the specific slot where the item was found
        if campfire_to_update.fuel_instance_id_0 == Some(item_instance_id) { campfire_to_update.fuel_instance_id_0 = None; campfire_to_update.fuel_def_id_0 = None; }
        else if campfire_to_update.fuel_instance_id_1 == Some(item_instance_id) { campfire_to_update.fuel_instance_id_1 = None; campfire_to_update.fuel_def_id_1 = None; }
        else if campfire_to_update.fuel_instance_id_2 == Some(item_instance_id) { campfire_to_update.fuel_instance_id_2 = None; campfire_to_update.fuel_def_id_2 = None; }
        else if campfire_to_update.fuel_instance_id_3 == Some(item_instance_id) { campfire_to_update.fuel_instance_id_3 = None; campfire_to_update.fuel_def_id_3 = None; }
        else if campfire_to_update.fuel_instance_id_4 == Some(item_instance_id) { campfire_to_update.fuel_instance_id_4 = None; campfire_to_update.fuel_def_id_4 = None; }
        
        // Extinguish check
        let still_has_fuel = crate::campfire::check_if_campfire_has_fuel(ctx, &campfire_to_update);
        if !still_has_fuel && campfire_to_update.is_burning {
             campfire_to_update.is_burning = false;
             campfire_to_update.next_fuel_consume_at = None;
        }
        ctx.db.campfire().id().update(campfire_to_update); // Update the campfire table
    }

    // --- Pre-fetch definition for potential merge ---
    let item_def_to_move = ctx.db.item_definition().iter()
        .find(|def| def.id == item_to_move.item_def_id)
        .ok_or_else(|| format!("Definition missing for item {}", item_to_move.item_def_id))?;

    // Store original location *before* modification
    let source_hotbar_slot = item_to_move.hotbar_slot;
    let source_inventory_slot = item_to_move.inventory_slot;
    let was_originally_equipped = source_inventory_slot.is_none() && source_hotbar_slot.is_none();

    // Prevent dropping onto the exact same slot
    if source_inventory_slot == Some(target_inventory_slot) {
        log::info!("Item {} already in target slot {}. No action.", item_instance_id, target_inventory_slot);
        return Ok(());
    }

    let mut operation_complete = false; // Flag to track if merge/swap handled everything

    if let Some(mut occupant) = find_item_in_inventory_slot(ctx, target_inventory_slot) {
        // Target slot is occupied

        // Prevent merging/swapping with self (shouldn't happen with UI, but good check)
        if occupant.instance_id == item_to_move.instance_id {
             log::warn!("Attempted to merge/swap item {} with itself in slot {}. No action.", item_instance_id, target_inventory_slot);
            return Ok(());
        }

        // --- Attempt Stack Combine ---
        if item_def_to_move.is_stackable && item_to_move.item_def_id == occupant.item_def_id {
            let space_available = item_def_to_move.stack_size.saturating_sub(occupant.quantity);
            if space_available > 0 {
                let transfer_qty = min(item_to_move.quantity, space_available);
                log::info!("[StackCombine Inv] Merging {} item(s) (ID {}) onto stack {} (ID {}).", 
                         transfer_qty, item_to_move.instance_id, occupant.quantity, occupant.instance_id);
                         
                occupant.quantity += transfer_qty;
                item_to_move.quantity -= transfer_qty;

                // Update target stack, pass clone
                ctx.db.inventory_item().instance_id().update(occupant.clone()); 

                if item_to_move.quantity == 0 {
                    log::info!("[StackCombine Inv] Source stack (ID {}) depleted, deleting.", item_to_move.instance_id);
                    ctx.db.inventory_item().instance_id().delete(item_to_move.instance_id);
                } else {
                     log::info!("[StackCombine Inv] Source stack (ID {}) has {} remaining, updating.", item_to_move.instance_id, item_to_move.quantity);
                    // Update source stack, pass clone
                    ctx.db.inventory_item().instance_id().update(item_to_move.clone()); 
                }
                operation_complete = true; // Merge handled everything
            } else {
                log::info!("[StackCombine Inv] Target stack (ID {}) is full.", occupant.instance_id);
                // Fall through to swap logic
            }
        } else {
             log::info!("[StackCombine Inv] Items cannot be combined (Diff type/Not stackable). Falling back to swap.");
             // Fall through to swap logic
        }

        // --- Fallback to Swap Logic (only if merge didn't happen) ---
        if !operation_complete {
            log::info!("Performing swap: Target slot {} occupied by item {}. Moving occupant.", target_inventory_slot, occupant.instance_id);
            if source_inventory_slot.is_none() && source_hotbar_slot.is_none() {
                // Move occupant to first available inventory slot
                if let Some(empty_slot) = find_first_empty_inventory_slot(ctx, ctx.sender) {
                    log::info!("Moving occupant {} to first empty inventory slot: {}", occupant.instance_id, empty_slot);
                    occupant.inventory_slot = Some(empty_slot);
                    occupant.hotbar_slot = None;
                } else {
                    return Err("Inventory full, cannot swap item.".to_string());
                }
            } else {
                // Move occupant to where item_to_move came from
                log::info!("Moving occupant {} to source slot (Inv: {:?}, Hotbar: {:?}).", 
                         occupant.instance_id, source_inventory_slot, source_hotbar_slot);
                occupant.inventory_slot = source_inventory_slot;
                occupant.hotbar_slot = source_hotbar_slot;
            }
            ctx.db.inventory_item().instance_id().update(occupant); // Update the occupant first

            // Now, explicitly move item_to_move to the target slot
            let mut final_item_state = item_to_move; // Clone before potentially changing identity
            if came_from_equip_or_fuel {
                final_item_state.player_identity = sender_id;
                log::info!("[MoveToInv] Setting ownership of item {} to player {:?}", item_instance_id, sender_id);
            }
            final_item_state.inventory_slot = Some(target_inventory_slot);
            final_item_state.hotbar_slot = None;
            log::info!("[MoveToInv] Moving dragged item {} to target inv slot {}", final_item_state.instance_id, target_inventory_slot);
            ctx.db.inventory_item().instance_id().update(final_item_state);
            operation_complete = true; // Swap handled everything
        }

    } else { 
        // Target slot is empty - handle edge case where original slot gets filled
        if let Some(hotbar_slot) = source_hotbar_slot {
            if let Some(mut new_hotbar_occupant) = find_item_in_hotbar_slot(ctx, hotbar_slot) {
                 if new_hotbar_occupant.instance_id != item_instance_id {
                     log::warn!("Item {} moved from hotbar slot {} but another item {} now occupies it. Moving {} to first available slot.", item_instance_id, hotbar_slot, new_hotbar_occupant.instance_id, new_hotbar_occupant.instance_id);
                     if let Some(empty_slot) = find_first_empty_inventory_slot(ctx, ctx.sender) {
                          new_hotbar_occupant.inventory_slot = Some(empty_slot);
                          new_hotbar_occupant.hotbar_slot = None;
                     } else {
                          new_hotbar_occupant.hotbar_slot = None;
                          log::error!("Inventory full, cannot find slot for displaced hotbar item {}. Clearing its slot.", new_hotbar_occupant.instance_id);
                     }
                     ctx.db.inventory_item().instance_id().update(new_hotbar_occupant);
                 }
            }
        }
         // No merge or swap needed, just move the item to the empty target slot
         let mut final_item_state = item_to_move; // Clone before potentially changing identity
         if came_from_equip_or_fuel {
            final_item_state.player_identity = sender_id;
            log::info!("[MoveToInv] Setting ownership of item {} to player {:?}", item_instance_id, sender_id);
         }
        final_item_state.inventory_slot = Some(target_inventory_slot);
        final_item_state.hotbar_slot = None;
        log::info!("[MoveToInv] Moving item {} to empty inv slot {}", final_item_state.instance_id, target_inventory_slot);
        ctx.db.inventory_item().instance_id().update(final_item_state);
        operation_complete = true;
    }

    if !operation_complete {
        // This should ideally not be reached if logic is correct, but as a fallback
        log::error!("Item move to inventory slot {} failed to complete via merge, swap, or direct move.", target_inventory_slot);
        return Err("Failed to move item: Unknown state.".to_string());
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn move_item_to_hotbar(ctx: &ReducerContext, item_instance_id: u64, target_hotbar_slot: u8) -> Result<(), String> {
     let sender_id = ctx.sender;
     log::info!("[MoveToHotbar] Player {:?} attempting move item {} to hotbar slot {}", sender_id, item_instance_id, target_hotbar_slot);
    
    let mut item_to_move = ctx.db.inventory_item().instance_id().find(item_instance_id)
        .ok_or_else(|| format!("Item instance {} not found.", item_instance_id))?;

    let came_from_equip_or_fuel = item_to_move.inventory_slot.is_none() && item_to_move.hotbar_slot.is_none();
    if item_to_move.player_identity != sender_id && !came_from_equip_or_fuel {
        return Err(format!("Item instance {} not owned by caller or not movable.", item_instance_id));
    }

    // --- NEW: Check if item is currently fuel in a campfire --- 
    let mut maybe_campfire_to_update: Option<crate::campfire::Campfire> = None;
    if came_from_equip_or_fuel { 
        let campfires = ctx.db.campfire(); 
        for campfire in campfires.iter() {
             // Check ALL slots
            if campfire.fuel_instance_id_0 == Some(item_instance_id) ||
               campfire.fuel_instance_id_1 == Some(item_instance_id) ||
               campfire.fuel_instance_id_2 == Some(item_instance_id) ||
               campfire.fuel_instance_id_3 == Some(item_instance_id) ||
               campfire.fuel_instance_id_4 == Some(item_instance_id) 
            {
                log::debug!("[MoveToHotbar] Item {} is currently fuel for campfire {}. Will clear campfire state.", item_instance_id, campfire.id);
                maybe_campfire_to_update = Some(campfire); 
                break; 
            }
        }
    }

    // --- Clear the associated campfire state IF found --- 
    if let Some(mut campfire_to_update) = maybe_campfire_to_update {
        // Clear the specific slot where the item was found
        if campfire_to_update.fuel_instance_id_0 == Some(item_instance_id) { campfire_to_update.fuel_instance_id_0 = None; campfire_to_update.fuel_def_id_0 = None; }
        else if campfire_to_update.fuel_instance_id_1 == Some(item_instance_id) { campfire_to_update.fuel_instance_id_1 = None; campfire_to_update.fuel_def_id_1 = None; }
        else if campfire_to_update.fuel_instance_id_2 == Some(item_instance_id) { campfire_to_update.fuel_instance_id_2 = None; campfire_to_update.fuel_def_id_2 = None; }
        else if campfire_to_update.fuel_instance_id_3 == Some(item_instance_id) { campfire_to_update.fuel_instance_id_3 = None; campfire_to_update.fuel_def_id_3 = None; }
        else if campfire_to_update.fuel_instance_id_4 == Some(item_instance_id) { campfire_to_update.fuel_instance_id_4 = None; campfire_to_update.fuel_def_id_4 = None; }
        
        // Extinguish check
        let still_has_fuel = crate::campfire::check_if_campfire_has_fuel(ctx, &campfire_to_update);
        if !still_has_fuel && campfire_to_update.is_burning {
             campfire_to_update.is_burning = false;
             campfire_to_update.next_fuel_consume_at = None;
        }
        ctx.db.campfire().id().update(campfire_to_update); 
    }

    // --- Pre-fetch definition for potential merge ---
    let item_def_to_move = ctx.db.item_definition().iter()
        .find(|def| def.id == item_to_move.item_def_id)
        .ok_or_else(|| format!("Definition missing for item {}", item_to_move.item_def_id))?;

    // Store original location *before* modification
    let source_hotbar_slot = item_to_move.hotbar_slot;
    let source_inventory_slot = item_to_move.inventory_slot;
    let was_originally_equipped = source_inventory_slot.is_none() && source_hotbar_slot.is_none();

    // Prevent dropping onto the exact same slot
     if source_hotbar_slot == Some(target_hotbar_slot) {
        log::info!("Item {} already in target hotbar slot {}. No action.", item_instance_id, target_hotbar_slot);
        return Ok(());
     }

    let mut operation_complete = false; 
    if let Some(mut occupant) = find_item_in_hotbar_slot(ctx, target_hotbar_slot) {
        // Target slot is occupied

        // Prevent merging/swapping with self
        if occupant.instance_id == item_to_move.instance_id {
            log::warn!("Attempted to merge/swap item {} with itself in hotbar slot {}. No action.", item_instance_id, target_hotbar_slot);
            return Ok(());
        }

        // --- Attempt Stack Combine ---
        if item_def_to_move.is_stackable && item_to_move.item_def_id == occupant.item_def_id {
             let space_available = item_def_to_move.stack_size.saturating_sub(occupant.quantity);
             if space_available > 0 {
                let transfer_qty = min(item_to_move.quantity, space_available);
                log::info!("[StackCombine Hotbar] Merging {} item(s) (ID {}) onto stack {} (ID {}).", 
                         transfer_qty, item_to_move.instance_id, occupant.quantity, occupant.instance_id);
                
                occupant.quantity += transfer_qty;
                item_to_move.quantity -= transfer_qty;

                // Update target stack, pass clone
                ctx.db.inventory_item().instance_id().update(occupant.clone()); 

                if item_to_move.quantity == 0 {
                    log::info!("[StackCombine Hotbar] Source stack (ID {}) depleted, deleting.", item_to_move.instance_id);
                    ctx.db.inventory_item().instance_id().delete(item_to_move.instance_id);
                } else {
                    log::info!("[StackCombine Hotbar] Source stack (ID {}) has {} remaining, updating.", item_to_move.instance_id, item_to_move.quantity);
                    // Update source stack, pass clone
                    ctx.db.inventory_item().instance_id().update(item_to_move.clone()); 
                }
                operation_complete = true; // Merge handled everything
            } else {
                log::info!("[StackCombine Hotbar] Target stack (ID {}) is full.", occupant.instance_id);
                // Fall through to swap logic
            }
        } else {
             log::info!("[StackCombine Hotbar] Items cannot be combined (Diff type/Not stackable). Falling back to swap.");
             // Fall through to swap logic
        }
        
        // --- Fallback to Swap Logic (only if merge didn't happen) ---
        if !operation_complete {
            log::info!("Performing swap: Target hotbar slot {} occupied by item {}. Moving occupant.", target_hotbar_slot, occupant.instance_id);
            if source_inventory_slot.is_none() && source_hotbar_slot.is_none() {
                // Move occupant to first available inventory slot
                if let Some(empty_slot) = find_first_empty_inventory_slot(ctx, ctx.sender) {
                    log::info!("Moving occupant {} to first empty inventory slot: {}", occupant.instance_id, empty_slot);
                    occupant.inventory_slot = Some(empty_slot);
                    occupant.hotbar_slot = None;
                } else {
                    return Err("Inventory full, cannot swap item.".to_string());
                }
            } else {
                // Move occupant to where item_to_move came from
                log::info!("Moving occupant {} to source slot (Inv: {:?}, Hotbar: {:?}).", 
                         occupant.instance_id, source_inventory_slot, source_hotbar_slot);
                occupant.inventory_slot = source_inventory_slot;
                occupant.hotbar_slot = source_hotbar_slot;
            }
            ctx.db.inventory_item().instance_id().update(occupant); // Update the occupant first

            // Now, explicitly move item_to_move to the target slot
            let mut final_item_state = item_to_move; // Clone before potentially changing identity
            if came_from_equip_or_fuel {
                final_item_state.player_identity = sender_id;
                 log::info!("[MoveToHotbar] Setting ownership of item {} to player {:?}", item_instance_id, sender_id);
            }
            final_item_state.inventory_slot = None;
            final_item_state.hotbar_slot = Some(target_hotbar_slot);
            log::info!("[MoveToHotbar] Moving dragged item {} to target hotbar slot {}", final_item_state.instance_id, target_hotbar_slot);
            ctx.db.inventory_item().instance_id().update(final_item_state);
            operation_complete = true; // Swap handled everything
        }

    } else { // Target slot is empty
        // (Existing logic for handling edge case where original slot gets filled)
        if let Some(inv_slot) = source_inventory_slot {
            if let Some(mut new_inv_occupant) = find_item_in_inventory_slot(ctx, inv_slot) {
                 if new_inv_occupant.instance_id != item_instance_id {
                    log::warn!("Item {} moved from inventory slot {} but another item {} now occupies it. Moving {} to first available slot.", item_instance_id, inv_slot, new_inv_occupant.instance_id, new_inv_occupant.instance_id);
                     if let Some(empty_slot) = find_first_empty_inventory_slot(ctx, ctx.sender) {
                          new_inv_occupant.inventory_slot = Some(empty_slot);
                          new_inv_occupant.hotbar_slot = None;
                     } else {
                          new_inv_occupant.inventory_slot = None;
                          log::error!("Inventory full, cannot find slot for displaced inventory item {}. Clearing its slot.", new_inv_occupant.instance_id);
                     }
                    ctx.db.inventory_item().instance_id().update(new_inv_occupant);
                 }
            }
        }
        // No merge or swap needed, just move the item to the empty target slot
        let mut final_item_state = item_to_move; // Clone before potentially changing identity
        if came_from_equip_or_fuel {
            final_item_state.player_identity = sender_id;
            log::info!("[MoveToHotbar] Setting ownership of item {} to player {:?}", item_instance_id, sender_id);
        }
        final_item_state.hotbar_slot = Some(target_hotbar_slot);
        final_item_state.inventory_slot = None;
        log::info!("[MoveToHotbar] Moving item {} to empty hotbar slot {}", final_item_state.instance_id, target_hotbar_slot);
        ctx.db.inventory_item().instance_id().update(final_item_state);
        operation_complete = true;
    }

    if !operation_complete {
        // Fallback error
        log::error!("Item move to hotbar slot {} failed to complete via merge, swap, or direct move.", target_hotbar_slot);
        return Err("Failed to move item: Unknown state.".to_string());
    }

    Ok(())
}

// Reducer to equip armor from a drag-and-drop operation
#[spacetimedb::reducer]
pub fn equip_armor_from_drag(ctx: &ReducerContext, item_instance_id: u64, target_slot_name: String) -> Result<(), String> {
    log::info!("[EquipArmorDrag] Attempting to equip item {} to slot {}", item_instance_id, target_slot_name);
    let mut item_to_equip = get_player_item(ctx, item_instance_id)?;

    // Get item definition
    let item_def = ctx.db.item_definition().iter()
        .filter(|def| def.id == item_to_equip.item_def_id)
        .next()
        .ok_or_else(|| format!("Definition not found for item ID {}", item_to_equip.item_def_id))?;

    // --- Validations ---
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
    let mut equip = active_equip_table.player_identity().find(ctx.sender)
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
        match find_first_empty_inventory_slot(ctx, ctx.sender) {
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

    // Clear the inventory/hotbar slot of the equipped item
    item_to_equip.inventory_slot = None;
    item_to_equip.hotbar_slot = None;
    ctx.db.inventory_item().instance_id().update(item_to_equip);

    Ok(())
}

// Reducer to split a stack of items
#[spacetimedb::reducer]
pub fn split_stack(
    ctx: &ReducerContext,
    source_item_instance_id: u64,
    quantity_to_split: u32,        // How many to move to the NEW stack
    target_slot_type: String,    // "inventory" or "hotbar"
    target_slot_index: u32,    // Use u32 to accept both potential u8/u16 client values easily
) -> Result<(), String> {
    let sender_id = ctx.sender;
    log::info!(
        "[SplitStack] Player {:?} attempting to split {} from item {} to {} slot {}",
        sender_id, quantity_to_split, source_item_instance_id, target_slot_type, target_slot_index
    );

    // 1. Get the original item stack
    let mut source_item = get_player_item(ctx, source_item_instance_id)?;

    // 2. Get Item Definition
    let item_def = ctx.db.item_definition().iter()
        .find(|def| def.id == source_item.item_def_id)
        .ok_or_else(|| format!("Definition not found for item ID {}", source_item.item_def_id))?;

    // --- Validations ---
    // a. Must be stackable
    if !item_def.is_stackable {
        return Err(format!("Item '{}' is not stackable.", item_def.name));
    }
    // b. Cannot split zero or negative quantity
    if quantity_to_split == 0 {
        return Err("Cannot split a quantity of 0.".to_string());
    }
    // c. Cannot split more than available (leave at least 1 in original stack)
    if quantity_to_split >= source_item.quantity {
        return Err(format!("Cannot split {} items, only {} available.", quantity_to_split, source_item.quantity));
    }
    // d. Validate target slot type
    let target_is_inventory = match target_slot_type.as_str() {
        "inventory" => true,
        "hotbar" => false,
        _ => return Err(format!("Invalid target slot type: {}. Must be 'inventory' or 'hotbar'.", target_slot_type)),
    };
    // e. Basic range check for target index (adjust ranges if needed)
    if target_is_inventory && target_slot_index >= 24 { // Assuming 24 inventory slots (0-23)
        return Err(format!("Invalid target inventory slot index: {} (must be 0-23).", target_slot_index));
    }
    if !target_is_inventory && target_slot_index >= 6 { // Assuming 6 hotbar slots (0-5)
        return Err(format!("Invalid target hotbar slot index: {} (must be 0-5).", target_slot_index));
    }

    // --- Check if target slot is empty ---
    let target_inventory_slot_check = if target_is_inventory { Some(target_slot_index as u16) } else { None };
    let target_hotbar_slot_check = if !target_is_inventory { Some(target_slot_index as u8) } else { None };

    let target_occupied = ctx.db.inventory_item().iter().any(|i| {
        i.player_identity == sender_id &&
        ((target_is_inventory && i.inventory_slot == target_inventory_slot_check) ||
         (!target_is_inventory && i.hotbar_slot == target_hotbar_slot_check))
    });

    if target_occupied {
        return Err(format!("Target {} slot {} is already occupied.", target_slot_type, target_slot_index));
    }

    // --- Perform the split ---

    // a. Decrease quantity of the source item
    source_item.quantity -= quantity_to_split;
    ctx.db.inventory_item().instance_id().update(source_item.clone()); // Update original stack

    // b. Create the new item stack
    let new_item = InventoryItem {
        instance_id: 0, // Will be auto-generated
        player_identity: sender_id,
        item_def_id: source_item.item_def_id,
        quantity: quantity_to_split,
        hotbar_slot: if !target_is_inventory { Some(target_slot_index as u8) } else { None },
        inventory_slot: if target_is_inventory { Some(target_slot_index as u16) } else { None },
    };
    ctx.db.inventory_item().insert(new_item);

    log::info!(
        "[SplitStack] Successfully split {} of item {} (Def: {}) to {} slot {}. Original stack quantity now {}.",
        quantity_to_split, source_item_instance_id, source_item.item_def_id, 
        target_slot_type, target_slot_index, source_item.quantity
    );

    Ok(())
}

// NEW Reducer: Moves an item to the first available hotbar slot
#[spacetimedb::reducer]
pub fn move_to_first_available_hotbar_slot(ctx: &ReducerContext, item_instance_id: u64) -> Result<(), String> {
    let sender_id = ctx.sender;
    log::info!("[MoveToHotbar] Player {:?} trying to move item {} to first available hotbar slot.", sender_id, item_instance_id);

    // 1. Validate item exists and belongs to player (get_player_item does this)
    let _item_to_move = get_player_item(ctx, item_instance_id)?;

    // 2. Find the first empty hotbar slot (0-5)
    let occupied_slots: std::collections::HashSet<u8> = ctx.db.inventory_item().iter()
        .filter(|i| i.player_identity == sender_id && i.hotbar_slot.is_some())
        .map(|i| i.hotbar_slot.unwrap())
        .collect();

    match (0..6).find(|slot| !occupied_slots.contains(slot)) {
        Some(empty_slot) => {
            log::info!("[MoveToHotbar] Found empty slot: {}. Calling move_item_to_hotbar.", empty_slot);
            // 3. Call the existing move_item_to_hotbar reducer
            move_item_to_hotbar(ctx, item_instance_id, empty_slot)
        }
        None => {
            log::warn!("[MoveToHotbar] No empty hotbar slots available for player {:?}.", sender_id);
            Err("No empty hotbar slots available.".to_string())
        }
    }
} 