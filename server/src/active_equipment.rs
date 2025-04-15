use spacetimedb::{ Identity, ReducerContext, SpacetimeType, Table, Timestamp };
use log;

use crate::items::InventoryItem; // Assuming InventoryItem is needed later
use crate::items::ItemDefinition; // Assuming ItemDefinition is needed later
// Import generated table traits
use crate::items::inventory_item as InventoryItemTableTrait;
use crate::items::item_definition as ItemDefinitionTableTrait;
use crate::active_equipment::active_equipment as ActiveEquipmentTableTrait;

#[spacetimedb::table(name = active_equipment, public)]
#[derive(Clone, Default, Debug)]
pub struct ActiveEquipment {
    #[primary_key]
    pub player_identity: Identity,
    pub equipped_item_def_id: Option<u32>, // ID from ItemDefinition table
    pub equipped_item_instance_id: Option<u64>, // Instance ID from InventoryItem
    pub swing_start_time_ms: u64, // Timestamp (ms) when the current swing started, 0 if not swinging
}

// Reducer to equip an item from the inventory
#[spacetimedb::reducer]
pub fn equip_item(ctx: &ReducerContext, item_instance_id: u64) -> Result<(), String> {
    let sender_id = ctx.sender;
    let inventory_items = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();
    let active_equipments = ctx.db.active_equipment();

    // Find the inventory item
    let item_to_equip = inventory_items.instance_id().find(item_instance_id)
        .ok_or_else(|| format!("Inventory item with instance ID {} not found.", item_instance_id))?;

    // Verify the item belongs to the sender
    if item_to_equip.player_identity != sender_id {
        return Err("Cannot equip an item that does not belong to you.".to_string());
    }

    // Find the item definition
    let item_def = item_defs.id().find(item_to_equip.item_def_id)
        .ok_or_else(|| format!("Item definition {} not found.", item_to_equip.item_def_id))?;

    // Check if item is actually equippable using the field from ItemDefinition
    if !item_def.is_equippable {
         // If not equippable, effectively unequip anything currently held
        if active_equipments.player_identity().find(sender_id).is_some() {
            active_equipments.player_identity().delete(sender_id);
            log::info!("Player {:?} unequipped item by selecting non-equippable item {}.", sender_id, item_def.name);
        }
        return Ok(()); // Not an error, just means nothing visible is equipped
    }

    // Update or insert the active equipment entry
    let new_equipment = ActiveEquipment {
        player_identity: sender_id,
        equipped_item_def_id: Some(item_def.id as u32),
        equipped_item_instance_id: Some(item_instance_id),
        swing_start_time_ms: 0, // Reset swing state when equipping
    };

    active_equipments.insert(new_equipment);
    log::info!("Player {:?} equipped item: {} (Instance ID: {})", sender_id, item_def.name, item_instance_id);

    Ok(())
}

// Reducer to explicitly unequip whatever item is active
#[spacetimedb::reducer]
pub fn unequip_item(ctx: &ReducerContext) -> Result<(), String> {
    let sender_id = ctx.sender;
    let active_equipments = ctx.db.active_equipment();

    if active_equipments.player_identity().find(sender_id).is_some() {
        active_equipments.player_identity().delete(sender_id);
        log::info!("Player {:?} explicitly unequipped item.", sender_id);
    }
    // Not an error if nothing was equipped
    Ok(())
}

// Reducer to trigger the 'use' action (swing) of the equipped item
#[spacetimedb::reducer]
pub fn use_equipped_item(ctx: &ReducerContext) -> Result<(), String> {
    let sender_id = ctx.sender;
    let active_equipments = ctx.db.active_equipment();

    let mut current_equipment = active_equipments.player_identity().find(sender_id)
        .ok_or_else(|| "No item equipped to use.".to_string())?;

    // TODO: Add cooldown check? Prevent spamming swings?

    let now_micros = ctx.timestamp.to_micros_since_unix_epoch();
    let now_ms = (now_micros / 1000) as u64;
    let equipped_instance_id = current_equipment.equipped_item_instance_id;

    current_equipment.swing_start_time_ms = now_ms;
    active_equipments.player_identity().update(current_equipment);

    log::info!("Player {:?} started using equipped item (Instance ID: {:?}) at {}ms",
             sender_id, equipped_instance_id, now_ms);

    // TODO: Trigger actual game logic based on the item used (e.g., check for nearby tree/stone)
    // This might involve querying nearby entities and calling other reducers or functions.

    Ok(())
}
