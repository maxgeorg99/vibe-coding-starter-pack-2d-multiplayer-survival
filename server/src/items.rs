use spacetimedb::{ReducerContext, SpacetimeType, Table};
use log;
// Import ActiveEquipment table definition
// use crate::active_equipment::{ActiveEquipment};
// ADD generated table trait import with alias
use crate::active_equipment::active_equipment as ActiveEquipmentTableTrait;

// --- Item Enums and Structs ---

// Define categories or types for items
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, SpacetimeType)]
pub enum ItemCategory {
    Tool,
    Material,
    Placeable,
    Armor,
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

    let source_hotbar_slot = item_to_move.hotbar_slot;
    let source_inventory_slot = item_to_move.inventory_slot;

    if source_inventory_slot == Some(target_inventory_slot) { return Ok(()); }

    // --- Clear from equipment BEFORE handling swaps/moves --- 
    clear_item_from_equipment_slots(ctx, ctx.sender, item_instance_id);

    // Check if the target slot is occupied
    if let Some(mut occupant) = find_item_in_inventory_slot(ctx, target_inventory_slot) {
        log::info!("Target inventory slot {} is occupied by item {}. Swapping.", target_inventory_slot, occupant.instance_id);
        // Swap: Move occupant to where item_to_move came from
        occupant.inventory_slot = source_inventory_slot;
        occupant.hotbar_slot = source_hotbar_slot;
        ctx.db.inventory_item().instance_id().update(occupant);
    } else {
         // If target inventory slot is empty, check if the item came from the hotbar
         // and if that original hotbar slot is now occupied by something else (edge case)
        if let Some(hotbar_slot) = source_hotbar_slot {
            if let Some(mut new_hotbar_occupant) = find_item_in_hotbar_slot(ctx, hotbar_slot) {
                 if new_hotbar_occupant.instance_id != item_instance_id {
                     log::warn!("Item {} moved from hotbar slot {} but another item {} now occupies it. Moving {} to first available slot.", item_instance_id, hotbar_slot, new_hotbar_occupant.instance_id, new_hotbar_occupant.instance_id);
                     // Ideally find the first empty inventory slot for new_hotbar_occupant
                     // For simplicity now, we might just clear its hotbar slot, or error
                     new_hotbar_occupant.hotbar_slot = None;
                     ctx.db.inventory_item().instance_id().update(new_hotbar_occupant);
                 }
            }
        }
    }

    // Move the item into the inventory slot
    item_to_move.inventory_slot = Some(target_inventory_slot);
    item_to_move.hotbar_slot = None;

    // Log the state RIGHT BEFORE update
    log::info!("[Reducer Debug] Updating item instance {} with data: {:?}",
             item_to_move.instance_id, // Use the ID from the struct itself
             item_to_move);

    ctx.db.inventory_item().instance_id().update(item_to_move);

    Ok(())
}

#[spacetimedb::reducer]
pub fn move_item_to_hotbar(ctx: &ReducerContext, item_instance_id: u64, target_hotbar_slot: u8) -> Result<(), String> {
     log::info!("Attempting to move item {} to hotbar slot {}", item_instance_id, target_hotbar_slot);
    let mut item_to_move = get_player_item(ctx, item_instance_id)?;

    let source_hotbar_slot = item_to_move.hotbar_slot;
    let source_inventory_slot = item_to_move.inventory_slot;

     if source_hotbar_slot == Some(target_hotbar_slot) { return Ok(()); }
     
    // --- Clear from equipment BEFORE handling swaps/moves --- 
    clear_item_from_equipment_slots(ctx, ctx.sender, item_instance_id);

    // Check if the target slot is occupied
    if let Some(mut occupant) = find_item_in_hotbar_slot(ctx, target_hotbar_slot) {
        log::info!("Target hotbar slot {} is occupied by item {}. Swapping.", target_hotbar_slot, occupant.instance_id);
        // Swap: Move occupant to where item_to_move came from
        occupant.inventory_slot = source_inventory_slot;
        occupant.hotbar_slot = source_hotbar_slot;
        ctx.db.inventory_item().instance_id().update(occupant);
    } else {
        // If target hotbar slot is empty, check if the item came from inventory
        // and if that original inventory slot is now occupied (edge case)
        if let Some(inv_slot) = source_inventory_slot {
            if let Some(mut new_inv_occupant) = find_item_in_inventory_slot(ctx, inv_slot) {
                 if new_inv_occupant.instance_id != item_instance_id {
                    log::warn!("Item {} moved from inventory slot {} but another item {} now occupies it. Moving {} to first available slot.", item_instance_id, inv_slot, new_inv_occupant.instance_id, new_inv_occupant.instance_id);
                    new_inv_occupant.inventory_slot = None;
                    ctx.db.inventory_item().instance_id().update(new_inv_occupant);
                 }
            }
        }
    }

    // Move the item into the hotbar slot
    item_to_move.hotbar_slot = Some(target_hotbar_slot);
    item_to_move.inventory_slot = None;

    // Log the state RIGHT BEFORE update
    log::info!("[Reducer Debug] Updating item instance {} with data: {:?}",
             item_to_move.instance_id, // Use the ID from the struct itself
             item_to_move);

    ctx.db.inventory_item().instance_id().update(item_to_move);

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

    // Validation: Must be equippable and NOT Armor (armor goes to equipment slots, handled separately later)
    if !item_def.is_equippable {
        return Err(format!("Item '{}' is not equippable.", item_def.name));
    }
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