use spacetimedb::{ReducerContext, SpacetimeType, Table};
use log;
// Import ActiveEquipment table definition
// use crate::active_equipment::{ActiveEquipment};
// ADD generated table trait import with alias
use crate::active_equipment::active_equipment as ActiveEquipmentTableTrait;
use std::cmp::min;

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

// NEW Helper: Find the first empty inventory slot (0-23 for now)
fn find_first_empty_inventory_slot(ctx: &ReducerContext) -> Option<u16> {
    let occupied_slots: std::collections::HashSet<u16> = ctx.db
        .inventory_item().iter()
        .filter(|i| i.player_identity == ctx.sender && i.inventory_slot.is_some())
        .map(|i| i.inventory_slot.unwrap())
        .collect();

    // Assuming 24 inventory slots (0-23)
    (0..24).find(|slot| !occupied_slots.contains(slot))
}

// Helper to clear an item from any equipment slot it might occupy
fn clear_item_from_equipment_slots(ctx: &ReducerContext, player_id: spacetimedb::Identity, item_instance_id: u64) {
    let active_equip_table = ctx.db.active_equipment();
    if let Some(mut equip) = active_equip_table.player_identity().find(player_id) {
        let mut updated = false;
        if equip.head_item_instance_id == Some(item_instance_id) { equip.head_item_instance_id = None; updated = true; }
        if equip.chest_item_instance_id == Some(item_instance_id) { equip.chest_item_instance_id = None; updated = true; }
        if equip.legs_item_instance_id == Some(item_instance_id) { equip.legs_item_instance_id = None; updated = true; }
        if equip.feet_item_instance_id == Some(item_instance_id) { equip.feet_item_instance_id = None; updated = true; }
        if equip.hands_item_instance_id == Some(item_instance_id) { equip.hands_item_instance_id = None; updated = true; }
        if equip.back_item_instance_id == Some(item_instance_id) { equip.back_item_instance_id = None; updated = true; }
        // Also check the main hand slot
        if equip.equipped_item_instance_id == Some(item_instance_id) { 
             equip.equipped_item_instance_id = None; 
             equip.equipped_item_def_id = None; // Clear def ID too
             equip.swing_start_time_ms = 0;
             updated = true; 
        }

        if updated {
            log::info!("[ClearEquip] Removing item {} from equipment slots for player {:?}", item_instance_id, player_id);
            active_equip_table.player_identity().update(equip);
        }
    }
}

#[spacetimedb::reducer]
pub fn move_item_to_inventory(ctx: &ReducerContext, item_instance_id: u64, target_inventory_slot: u16) -> Result<(), String> {
    log::info!("Attempting to move item {} to inventory slot {}", item_instance_id, target_inventory_slot);
    let mut item_to_move = get_player_item(ctx, item_instance_id)?;

    // --- Pre-fetch definition for potential merge ---
    let item_def_to_move = ctx.db.item_definition().iter()
        .find(|def| def.id == item_to_move.item_def_id)
        .ok_or_else(|| format!("Definition missing for item {}", item_to_move.item_def_id))?;

    let source_hotbar_slot = item_to_move.hotbar_slot;
    let source_inventory_slot = item_to_move.inventory_slot;
    let from_equipment = source_inventory_slot.is_none() && source_hotbar_slot.is_none();

    // Prevent dropping onto the exact same slot
    if source_inventory_slot == Some(target_inventory_slot) { 
        log::info!("Item {} already in target slot {}. No action.", item_instance_id, target_inventory_slot);
        return Ok(()); 
    }

    clear_item_from_equipment_slots(ctx, ctx.sender, item_instance_id);

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
            if from_equipment {
                // Move occupant to first available inventory slot
                if let Some(empty_slot) = find_first_empty_inventory_slot(ctx) {
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
            item_to_move.inventory_slot = Some(target_inventory_slot);
            item_to_move.hotbar_slot = None;
            log::info!("Moving dragged item {} to target slot {}", item_to_move.instance_id, target_inventory_slot);
            ctx.db.inventory_item().instance_id().update(item_to_move);
            operation_complete = true; // Swap handled everything
        }

    } else { 
        // Target slot is empty - handle edge case where original slot gets filled
        if let Some(hotbar_slot) = source_hotbar_slot {
            if let Some(mut new_hotbar_occupant) = find_item_in_hotbar_slot(ctx, hotbar_slot) {
                 if new_hotbar_occupant.instance_id != item_instance_id {
                     log::warn!("Item {} moved from hotbar slot {} but another item {} now occupies it. Moving {} to first available slot.", item_instance_id, hotbar_slot, new_hotbar_occupant.instance_id, new_hotbar_occupant.instance_id);
                     if let Some(empty_slot) = find_first_empty_inventory_slot(ctx) {
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
        item_to_move.inventory_slot = Some(target_inventory_slot);
        item_to_move.hotbar_slot = None;
        log::info!("Moving item {} to empty inventory slot {}", item_to_move.instance_id, target_inventory_slot);
        ctx.db.inventory_item().instance_id().update(item_to_move);
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
     log::info!("Attempting to move item {} to hotbar slot {}", item_instance_id, target_hotbar_slot);
    let mut item_to_move = get_player_item(ctx, item_instance_id)?;

    // --- Pre-fetch definition for potential merge ---
    let item_def_to_move = ctx.db.item_definition().iter()
        .find(|def| def.id == item_to_move.item_def_id)
        .ok_or_else(|| format!("Definition missing for item {}", item_to_move.item_def_id))?;

    let source_hotbar_slot = item_to_move.hotbar_slot;
    let source_inventory_slot = item_to_move.inventory_slot;
    let from_equipment = source_inventory_slot.is_none() && source_hotbar_slot.is_none();

    // Prevent dropping onto the exact same slot
     if source_hotbar_slot == Some(target_hotbar_slot) { 
        log::info!("Item {} already in target hotbar slot {}. No action.", item_instance_id, target_hotbar_slot);
        return Ok(()); 
     }

    clear_item_from_equipment_slots(ctx, ctx.sender, item_instance_id);

    let mut operation_complete = false; // Flag to track if merge/swap handled everything

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
            if from_equipment {
                // Move occupant to first available inventory slot
                if let Some(empty_slot) = find_first_empty_inventory_slot(ctx) {
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
            item_to_move.inventory_slot = None;
            item_to_move.hotbar_slot = Some(target_hotbar_slot);
            log::info!("Moving dragged item {} to target hotbar slot {}", item_to_move.instance_id, target_hotbar_slot);
            ctx.db.inventory_item().instance_id().update(item_to_move);
            operation_complete = true; // Swap handled everything
        }

    } else { // Target slot is empty
        // (Existing logic for handling edge case where original slot gets filled)
        if let Some(inv_slot) = source_inventory_slot {
            if let Some(mut new_inv_occupant) = find_item_in_inventory_slot(ctx, inv_slot) {
                 if new_inv_occupant.instance_id != item_instance_id {
                    log::warn!("Item {} moved from inventory slot {} but another item {} now occupies it. Moving {} to first available slot.", item_instance_id, inv_slot, new_inv_occupant.instance_id, new_inv_occupant.instance_id);
                     if let Some(empty_slot) = find_first_empty_inventory_slot(ctx) {
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
        item_to_move.hotbar_slot = Some(target_hotbar_slot);
        item_to_move.inventory_slot = None;
        log::info!("Moving item {} to empty hotbar slot {}", item_to_move.instance_id, target_hotbar_slot);
        ctx.db.inventory_item().instance_id().update(item_to_move);
        operation_complete = true;
    }

    if !operation_complete {
        // Fallback error
        log::error!("Item move to hotbar slot {} failed to complete via merge, swap, or direct move.", target_hotbar_slot);
        return Err("Failed to move item: Unknown state.".to_string());
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn equip_to_hotbar(ctx: &ReducerContext, item_instance_id: u64, target_hotbar_slot: Option<u8>) -> Result<(), String> {
    let item_to_equip = get_player_item(ctx, item_instance_id)?;

    // Get item definition
    let item_def = ctx.db.item_definition().iter()
        .filter(|def| def.id == item_to_equip.item_def_id)
        .next()
        .ok_or_else(|| format!("Definition not found for item ID {}", item_to_equip.item_def_id))?;

    // Validation: Removed is_equippable check. Still check for Armor.
    if item_def.category == ItemCategory::Armor {
         return Err(format!("Cannot equip armor '{}' to hotbar. Use equipment slots.", item_def.name));
    }

    let final_target_slot = match target_hotbar_slot {
        Some(slot) => {
            if slot >= 6 { return Err("Invalid hotbar slot index.".to_string()); }
            slot
        }
        None => { // Find first empty slot
            let occupied_slots: std::collections::HashSet<u8> = ctx.db.inventory_item().iter()
                .filter(|i| i.player_identity == ctx.sender && i.hotbar_slot.is_some())
                .map(|i| i.hotbar_slot.unwrap())
                .collect();

            (0..6).find(|slot| !occupied_slots.contains(slot))
                  .ok_or_else(|| "No empty hotbar slots available.".to_string())?
        }
    };

    // Call the move reducer
    move_item_to_hotbar(ctx, item_instance_id, final_target_slot)
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
        match find_first_empty_inventory_slot(ctx) {
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