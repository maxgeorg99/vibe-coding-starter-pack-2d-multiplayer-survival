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

// Helper: Clear a specific item instance from any campfire fuel slot
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
                let still_has_fuel = crate::campfire::check_if_campfire_has_fuel(ctx, &campfire);
                 if !still_has_fuel && campfire.is_burning {
                    campfire.is_burning = false;
                    campfire.next_fuel_consume_at = None;
                    log::info!("Campfire {} extinguished as last valid fuel was removed.", campfire_id);
                }
                campfires.id().update(campfire);
            }
        }
    }
}

// NEW Refactored Helper: Clears an item from equipment OR campfire slots based on its state
// This should be called *before* modifying or deleting the InventoryItem itself.
fn clear_item_from_source_location(ctx: &ReducerContext, item_instance_id: u64) -> Result<(), String> {
    let sender_id = ctx.sender; // Assume the operation is initiated by the sender

    // Check if item exists (implicitly checks ownership if called after get_player_item)
    // Or, if the item is potentially unowned (e.g., in a campfire), we need to find it first.
    let item_opt = ctx.db.inventory_item().instance_id().find(item_instance_id);

    if item_opt.is_none() {
        // Item might have already been deleted (e.g., during a merge). This is ok.
        log::debug!("[ClearSource] Item {} already gone. No clearing needed.", item_instance_id);
        return Ok(());
    }
    let item = item_opt.unwrap(); // Safe to unwrap now

    // Determine if it was equipped or in a campfire
    // Note: An item can't be *both* equipped and fuel.
    // It also can't be equipped/fuel AND in inventory/hotbar.
    let was_equipped_or_fuel = item.inventory_slot.is_none() && item.hotbar_slot.is_none();

    if was_equipped_or_fuel {
        // Try clearing from equipment first (most common case for non-inv/hotbar)
        clear_specific_item_from_equipment_slots(ctx, sender_id, item_instance_id);

        // Then clear from campfires (in case it was fuel)
        // This is safe even if it wasn't fuel, the inner function handles lookup.
        clear_item_from_campfire_fuel_slots(ctx, item_instance_id);

        log::debug!("[ClearSource] Attempted clearing item {} from equipment/campfire slots for player {:?}", item_instance_id, sender_id);
    } else {
        log::debug!("[ClearSource] Item {} was in inventory/hotbar. No equipment/campfire clearing needed.", item_instance_id);
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn move_item_to_inventory(ctx: &ReducerContext, item_instance_id: u64, target_inventory_slot: u16) -> Result<(), String> {
    let sender_id = ctx.sender;
    log::info!("[MoveToInv] Player {:?} attempting move item {} to inv slot {}", sender_id, item_instance_id, target_inventory_slot);

    // --- Find Item First ---
    // Use try_find to handle cases where the item might not exist (e.g., race condition)
    let mut item_to_move = ctx.db.inventory_item().instance_id().find(item_instance_id)
        .ok_or_else(|| format!("Item instance {} not found.", item_instance_id))?;

    let was_originally_equipped_or_fuel = item_to_move.inventory_slot.is_none() && item_to_move.hotbar_slot.is_none();

    // --- Validate Ownership (only if NOT coming from equip/fuel) ---
    // If it *was* equipped/fuel, we assume the player interacting (sender_id) implicitly gains ownership.
    if !was_originally_equipped_or_fuel && item_to_move.player_identity != sender_id {
        return Err(format!("Item instance {} not owned by caller.", item_instance_id));
    }

    // --- Clear From Original Location (Equip/Campfire) *BEFORE* modifying item ---
    clear_item_from_source_location(ctx, item_instance_id)?;

    // --- Pre-fetch definition for potential merge ---
    let item_def_to_move = ctx.db.item_definition().id().find(item_to_move.item_def_id)
        .ok_or_else(|| format!("Definition missing for item {}", item_to_move.item_def_id))?;

    // Store original inventory/hotbar location *before* modification
    let source_hotbar_slot = item_to_move.hotbar_slot;
    let source_inventory_slot = item_to_move.inventory_slot;

    // Prevent dropping onto the exact same slot
    if source_inventory_slot == Some(target_inventory_slot) {
        log::info!("Item {} already in target slot {}. No action.", item_instance_id, target_inventory_slot);
        return Ok(());
    }

    let mut operation_complete = false; // Flag to track if merge/swap handled everything

    if let Some(mut occupant) = find_item_in_inventory_slot(ctx, target_inventory_slot) {
        // Target slot is occupied

        // Prevent merging/swapping with self
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

                // Update target stack
                ctx.db.inventory_item().instance_id().update(occupant.clone());

                if item_to_move.quantity == 0 {
                    log::info!("[StackCombine Inv] Source stack (ID {}) depleted, deleting.", item_to_move.instance_id);
                    ctx.db.inventory_item().instance_id().delete(item_to_move.instance_id);
                } else {
                     log::info!("[StackCombine Inv] Source stack (ID {}) has {} remaining, updating.", item_to_move.instance_id, item_to_move.quantity);
                    // Explicitly set ownership if needed *before* updating source
                     if was_originally_equipped_or_fuel {
                        item_to_move.player_identity = sender_id;
                        log::debug!("[StackCombine Inv] Setting ownership of remaining source item {} to player {:?}", item_instance_id, sender_id);
                     }
                    // Update source stack
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
            // Determine where the occupant should go
            if was_originally_equipped_or_fuel {
                 // Move occupant to first available inventory slot if the source was equip/fuel
                if let Some(empty_slot) = find_first_empty_inventory_slot(ctx, sender_id) {
                    log::info!("Moving occupant {} to first empty inventory slot: {}", occupant.instance_id, empty_slot);
                    occupant.inventory_slot = Some(empty_slot);
                    occupant.hotbar_slot = None;
                } else {
                    return Err("Inventory full, cannot swap item.".to_string());
                }
            } else {
                 // Move occupant to where item_to_move came from (inv/hotbar)
                log::info!("Moving occupant {} to source slot (Inv: {:?}, Hotbar: {:?}).",
                         occupant.instance_id, source_inventory_slot, source_hotbar_slot);
                occupant.inventory_slot = source_inventory_slot;
                occupant.hotbar_slot = source_hotbar_slot;
            }
            ctx.db.inventory_item().instance_id().update(occupant); // Update the occupant first

            // Now, explicitly move item_to_move to the target slot
             // Ensure ownership is set if coming from equip/fuel
            if was_originally_equipped_or_fuel {
                item_to_move.player_identity = sender_id;
                 log::info!("[MoveToInv Swap] Setting ownership of item {} to player {:?}", item_instance_id, sender_id);
            }
            item_to_move.inventory_slot = Some(target_inventory_slot);
            item_to_move.hotbar_slot = None;
            log::info!("[MoveToInv Swap] Moving dragged item {} to target inv slot {}", item_to_move.instance_id, target_inventory_slot);
            ctx.db.inventory_item().instance_id().update(item_to_move);
            operation_complete = true; // Swap handled everything
        }

    } else {
        // Target slot is empty
        // Ensure ownership is set if coming from equip/fuel
         if was_originally_equipped_or_fuel {
            item_to_move.player_identity = sender_id;
            log::info!("[MoveToInv Empty] Setting ownership of item {} to player {:?}", item_instance_id, sender_id);
         }
        item_to_move.inventory_slot = Some(target_inventory_slot);
        item_to_move.hotbar_slot = None;
        log::info!("[MoveToInv Empty] Moving item {} to empty inv slot {}", item_to_move.instance_id, target_inventory_slot);
        ctx.db.inventory_item().instance_id().update(item_to_move);
        operation_complete = true;
    }

    if !operation_complete {
        // Fallback error
        log::error!("Item move to inventory slot {} failed to complete. State might be inconsistent.", target_inventory_slot);
        return Err("Failed to move item: Unknown state.".to_string());
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn move_item_to_hotbar(ctx: &ReducerContext, item_instance_id: u64, target_hotbar_slot: u8) -> Result<(), String> {
     let sender_id = ctx.sender;
     log::info!("[MoveToHotbar] Player {:?} attempting move item {} to hotbar slot {}", sender_id, item_instance_id, target_hotbar_slot);

    // --- Find Item First ---
    let mut item_to_move = ctx.db.inventory_item().instance_id().find(item_instance_id)
        .ok_or_else(|| format!("Item instance {} not found.", item_instance_id))?;

    let was_originally_equipped_or_fuel = item_to_move.inventory_slot.is_none() && item_to_move.hotbar_slot.is_none();

    // --- Validate Ownership (only if NOT coming from equip/fuel) ---
    if !was_originally_equipped_or_fuel && item_to_move.player_identity != sender_id {
        return Err(format!("Item instance {} not owned by caller.", item_instance_id));
    }

    // --- Clear From Original Location (Equip/Campfire) *BEFORE* modifying item ---
    clear_item_from_source_location(ctx, item_instance_id)?;

    // --- Pre-fetch definition for potential merge ---
    let item_def_to_move = ctx.db.item_definition().id().find(item_to_move.item_def_id)
        .ok_or_else(|| format!("Definition missing for item {}", item_to_move.item_def_id))?;

    // Store original inventory/hotbar location *before* modification
    let source_hotbar_slot = item_to_move.hotbar_slot;
    let source_inventory_slot = item_to_move.inventory_slot;

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

                // Update target stack
                ctx.db.inventory_item().instance_id().update(occupant.clone());

                if item_to_move.quantity == 0 {
                    log::info!("[StackCombine Hotbar] Source stack (ID {}) depleted, deleting.", item_to_move.instance_id);
                    ctx.db.inventory_item().instance_id().delete(item_to_move.instance_id);
                } else {
                    log::info!("[StackCombine Hotbar] Source stack (ID {}) has {} remaining, updating.", item_to_move.instance_id, item_to_move.quantity);
                    // Explicitly set ownership if needed *before* updating source
                     if was_originally_equipped_or_fuel {
                        item_to_move.player_identity = sender_id;
                        log::debug!("[StackCombine Hotbar] Setting ownership of remaining source item {} to player {:?}", item_instance_id, sender_id);
                     }
                    // Update source stack
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
            // Determine where the occupant should go
            if was_originally_equipped_or_fuel {
                 // Move occupant to first available inventory slot if the source was equip/fuel
                if let Some(empty_slot) = find_first_empty_inventory_slot(ctx, sender_id) {
                    log::info!("Moving occupant {} to first empty inventory slot: {}", occupant.instance_id, empty_slot);
                    occupant.inventory_slot = Some(empty_slot);
                    occupant.hotbar_slot = None;
                } else {
                    return Err("Inventory full, cannot swap item.".to_string());
                }
            } else {
                 // Move occupant to where item_to_move came from (inv/hotbar)
                log::info!("Moving occupant {} to source slot (Inv: {:?}, Hotbar: {:?}).",
                         occupant.instance_id, source_inventory_slot, source_hotbar_slot);
                occupant.inventory_slot = source_inventory_slot;
                occupant.hotbar_slot = source_hotbar_slot;
            }
            ctx.db.inventory_item().instance_id().update(occupant); // Update the occupant first

            // Now, explicitly move item_to_move to the target slot
             // Ensure ownership is set if coming from equip/fuel
            if was_originally_equipped_or_fuel {
                item_to_move.player_identity = sender_id;
                 log::info!("[MoveToHotbar Swap] Setting ownership of item {} to player {:?}", item_instance_id, sender_id);
            }
            item_to_move.inventory_slot = None;
            item_to_move.hotbar_slot = Some(target_hotbar_slot);
            log::info!("[MoveToHotbar Swap] Moving dragged item {} to target hotbar slot {}", item_to_move.instance_id, target_hotbar_slot);
            ctx.db.inventory_item().instance_id().update(item_to_move);
            operation_complete = true; // Swap handled everything
        }

    } else { // Target slot is empty
         // Ensure ownership is set if coming from equip/fuel
        if was_originally_equipped_or_fuel {
            item_to_move.player_identity = sender_id;
            log::info!("[MoveToHotbar Empty] Setting ownership of item {} to player {:?}", item_instance_id, sender_id);
        }
        item_to_move.hotbar_slot = Some(target_hotbar_slot);
        item_to_move.inventory_slot = None;
        log::info!("[MoveToHotbar Empty] Moving item {} to empty hotbar slot {}", item_to_move.instance_id, target_hotbar_slot);
        ctx.db.inventory_item().instance_id().update(item_to_move);
        operation_complete = true;
    }

    if !operation_complete {
        // Fallback error
        log::error!("Item move to hotbar slot {} failed to complete. State might be inconsistent.", target_hotbar_slot);
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

// --- UNCOMMENTED Original Split Stack Reducer ---

#[spacetimedb::reducer]
pub fn split_stack(
    ctx: &ReducerContext,
    source_item_instance_id: u64,
    quantity_to_split: u32,        // How many to move to the NEW stack
    target_slot_type: String,    // "inventory" or "hotbar"
    target_slot_index: u32,    // Use u32 to accept both potential u8/u16 client values easily
) -> Result<(), String> {
    // Logic of the original reducer restored
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

// --- NEW: Split Stack From Campfire Reducer ---

#[spacetimedb::reducer]
pub fn split_stack_from_campfire(
    ctx: &ReducerContext,
    source_campfire_id: u32,
    source_slot_index: u8,
    quantity_to_split: u32,
    target_slot_type: String,    // "inventory" or "hotbar"
    target_slot_index: u32,     // Numeric index for inventory/hotbar
) -> Result<(), String> {
    let sender_id = ctx.sender;
    let mut inventory_items = ctx.db.inventory_item();
    let campfires = ctx.db.campfire();

    log::info!("[SplitFromCampfire] Player {:?} splitting {} from campfire {} slot {} to {} slot {}",
             sender_id, quantity_to_split, source_campfire_id, source_slot_index, target_slot_type, target_slot_index);

    // 1. Validate source slot index
    if source_slot_index >= crate::campfire::NUM_FUEL_SLOTS as u8 {
        return Err(format!("Invalid source fuel slot index: {}", source_slot_index));
    }

    // 2. Find source campfire
    let campfire = campfires.id().find(source_campfire_id)
        .ok_or(format!("Source campfire {} not found", source_campfire_id))?;

    // 3. Find the item instance ID in the source campfire slot
    let source_instance_id = match source_slot_index {
        0 => campfire.fuel_instance_id_0,
        1 => campfire.fuel_instance_id_1,
        2 => campfire.fuel_instance_id_2,
        3 => campfire.fuel_instance_id_3,
        4 => campfire.fuel_instance_id_4,
        _ => None,
    }.ok_or(format!("No item found in source campfire slot {}", source_slot_index))?;

    // 4. Get the source item (mutable needed for split_stack helper)
    let mut source_item = inventory_items.instance_id().find(source_instance_id)
        .ok_or("Source item instance not found in inventory table")?;

    // 5. Validate split quantity (using info from mutable source_item)
    if quantity_to_split == 0 || quantity_to_split >= source_item.quantity {
        return Err(format!("Invalid split quantity {} (must be > 0 and < {})", quantity_to_split, source_item.quantity));
    }
    
    // --- Validate Target --- (Similar to move reducers, simplified here)
    let target_is_inventory = match target_slot_type.as_str() {
        "inventory" => true,
        "hotbar" => false,
        _ => return Err(format!("Invalid target slot type: {}", target_slot_type)),
    };
    if target_is_inventory && target_slot_index >= 24 { return Err("Invalid target inventory slot".to_string()); }
    if !target_is_inventory && target_slot_index >= 6 { return Err("Invalid target hotbar slot".to_string()); }

    // --- Check Target Occupancy (Simplified - No Merge/Swap for split target yet) ---
    let target_inv_slot_check = if target_is_inventory { Some(target_slot_index as u16) } else { None };
    let target_hotbar_slot_check = if !target_is_inventory { Some(target_slot_index as u8) } else { None };
    let target_occupied = inventory_items.iter().any(|i| {
        i.player_identity == sender_id &&
        ((target_is_inventory && i.inventory_slot == target_inv_slot_check) ||
         (!target_is_inventory && i.hotbar_slot == target_hotbar_slot_check))
    });
    if target_occupied {
        return Err(format!("Target {} slot {} is already occupied (merging split not implemented yet).", target_slot_type, target_slot_index));
    }

    // --- Perform Split --- 
    // Call the RENAMED helper (updates original source_item quantity and DB row)
    let new_item_instance_id = split_stack_helper(ctx, &mut source_item, quantity_to_split)?;

    // --- Place NEW Item --- 
    // Get the newly created item instance
    let mut new_item = inventory_items.instance_id().find(new_item_instance_id)
                     .ok_or("Newly split item instance not found!")?;
                     
    // Assign ownership (should already be correct, but good practice) and location
    new_item.player_identity = sender_id; 
    if target_is_inventory {
        new_item.inventory_slot = Some(target_slot_index as u16);
        new_item.hotbar_slot = None;
    } else {
        new_item.inventory_slot = None;
        new_item.hotbar_slot = Some(target_slot_index as u8);
    }
    inventory_items.instance_id().update(new_item);

    log::info!("[SplitFromCampfire] Split successful. New item {} (qty {}) placed in {} slot {}.", 
             new_item_instance_id, quantity_to_split, target_slot_type, target_slot_index);

    Ok(())
}

// --- Re-Add missing Reducer ---
// NEW Reducer: Moves an item to the first available hotbar slot
#[spacetimedb::reducer]
pub fn move_to_first_available_hotbar_slot(ctx: &ReducerContext, item_instance_id: u64) -> Result<(), String> {
    let sender_id = ctx.sender;
    log::info!("[MoveToFirstAvailHotbar] Player {:?} trying to move item {} to first available hotbar slot.", sender_id, item_instance_id);

    // 1. Validate item exists and belongs to player (get_player_item does this)
    // Need to get the item to check its current location
    let item_to_move = get_player_item(ctx, item_instance_id)?;
    // Check if it's already in the hotbar
    if item_to_move.hotbar_slot.is_some() {
        return Err("Item is already in the hotbar.".to_string());
    }
    // Check if it's in the inventory (must be to move to hotbar like this)
    if item_to_move.inventory_slot.is_none() {
        return Err("Item must be in main inventory to move to hotbar this way.".to_string());
    }


    // 2. Find the first empty hotbar slot (0-5)
    let occupied_slots: std::collections::HashSet<u8> = ctx.db.inventory_item().iter()
        .filter(|i| i.player_identity == sender_id && i.hotbar_slot.is_some())
        .map(|i| i.hotbar_slot.unwrap())
        .collect();

    match (0..6).find(|slot| !occupied_slots.contains(slot)) {
        Some(empty_slot) => {
            log::info!("[MoveToFirstAvailHotbar] Found empty slot: {}. Calling move_item_to_hotbar.", empty_slot);
            // 3. Call the existing move_item_to_hotbar reducer
            move_item_to_hotbar(ctx, item_instance_id, empty_slot)
        }
        None => {
            log::warn!("[MoveToFirstAvailHotbar] No empty hotbar slots available for player {:?}.", sender_id);
            Err("No empty hotbar slots available.".to_string())
        }
    }
}

// --- NEW Reducer: Drop Item into the World ---
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

    // --- 4. Clear Item from Equip/Campfire Source (Important: Do this BEFORE modifying/deleting InventoryItem) ---
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

// --- NEW Reducer: Split and Move/Merge --- 

/// Splits a specified quantity from a source stack and attempts to move/merge 
/// the new stack onto a target slot.
#[spacetimedb::reducer]
pub fn split_and_move(
    ctx: &ReducerContext,
    source_item_instance_id: u64,
    quantity_to_split: u32,     
    target_slot_type: String,    // "inventory", "hotbar", or "campfire_fuel"
    target_slot_index: u32,     // Numeric index for inventory/hotbar/campfire
    target_campfire_id: Option<u32>, // Required only if target_slot_type is campfire_fuel
) -> Result<(), String> {
    let sender_id = ctx.sender;
    log::info!(
        "[SplitAndMove] Player {:?} splitting {} from item {} to {} slot {} (Campfire: {:?})",
        sender_id, quantity_to_split, source_item_instance_id, target_slot_type, target_slot_index, target_campfire_id
    );

    // --- 1. Get Source Item & Validate Split --- 
    // Use try_find as item might be gone
    let mut source_item = ctx.db.inventory_item().instance_id().find(source_item_instance_id)
        .ok_or_else(|| format!("Source item instance {} not found.", source_item_instance_id))?;

    // Basic ownership check (item must be owned if not coming from campfire)
    // Note: This reducer currently doesn't support splitting *from* campfire directly.
    // That would require a separate reducer like split_from_campfire_and_move.
    if source_item.player_identity != sender_id {
         return Err(format!("Source item {} not owned by caller.", source_item_instance_id));
    }
     if source_item.inventory_slot.is_none() && source_item.hotbar_slot.is_none() {
        return Err("Source item must be in inventory or hotbar to split this way".to_string());
    }

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

    // --- 2. Perform Split --- 
    // The helper updates the original stack and returns the ID of the new stack.
    let new_item_instance_id = split_stack_helper(ctx, &mut source_item, quantity_to_split)?;

    // --- 3. Move/Merge the NEW Stack --- 
    log::debug!("[SplitAndMove] Calling appropriate move/add reducer for new stack {}", new_item_instance_id);
    match target_slot_type.as_str() {
        "inventory" => {
            // Call move_item_to_inventory, which handles merging
            move_item_to_inventory(ctx, new_item_instance_id, target_slot_index as u16)
        },
        "hotbar" => {
             // Call move_item_to_hotbar, which handles merging
            move_item_to_hotbar(ctx, new_item_instance_id, target_slot_index as u8)
        },
        "campfire_fuel" => {
            // Call add_fuel_to_campfire, which handles merging
            let campfire_id = target_campfire_id.ok_or("target_campfire_id is required for campfire_fuel target".to_string())?;
            crate::campfire::add_fuel_to_campfire(ctx, campfire_id, target_slot_index as u8, new_item_instance_id)
        },
        _ => {
            log::error!("[SplitAndMove] Invalid target_slot_type: {}", target_slot_type);
            // Attempt to delete the orphaned split stack to prevent item loss
            ctx.db.inventory_item().instance_id().delete(new_item_instance_id);
            Err(format!("Invalid target slot type for split: {}", target_slot_type))
        }
    }
}

// --- NEW Reducer: Split From Campfire and Move/Merge ---

/// Splits a specified quantity from a source stack within a campfire and attempts 
/// to move/merge the new stack onto a target slot (inv, hotbar, or another campfire slot).
#[spacetimedb::reducer]
pub fn split_and_move_from_campfire(
    ctx: &ReducerContext,
    source_campfire_id: u32,
    source_slot_index: u8,
    quantity_to_split: u32,
    target_slot_type: String,    // "inventory", "hotbar", or "campfire_fuel"
    target_slot_index: u32,     // Numeric index for inventory/hotbar/campfire
    // target_campfire_id is only needed if target_slot_type is campfire_fuel, 
    // and it will be the SAME as source_campfire_id if moving within the same fire.
    // We already have source_campfire_id, so we don't need a separate target one.
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
        _ => None, // Should be caught by index check above
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
            // Call move_item_to_inventory, which handles merging
            move_item_to_inventory(ctx, new_item_instance_id, target_slot_index as u16)
        },
        "hotbar" => {
             // Call move_item_to_hotbar, which handles merging
            move_item_to_hotbar(ctx, new_item_instance_id, target_slot_index as u8)
        },
        "campfire_fuel" => {
            // Call add_fuel_to_campfire, which handles merging onto existing stack or placing in empty slot.
            // We use the source_campfire_id because we are moving *within* the same fire if target is campfire.
            crate::campfire::add_fuel_to_campfire(ctx, source_campfire_id, target_slot_index as u8, new_item_instance_id)
        },
        _ => {
            log::error!("[SplitMoveFromCampfire] Invalid target_slot_type: {}", target_slot_type);
            // Attempt to delete the orphaned split stack to prevent item loss
            ctx.db.inventory_item().instance_id().delete(new_item_instance_id);
            Err(format!("Invalid target slot type for split: {}", target_slot_type))
        }
    }
} 