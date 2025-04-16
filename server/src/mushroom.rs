use spacetimedb::{Table, ReducerContext, Identity};
// Add imports for required table traits
use crate::items::{inventory_item as InventoryItemTableTrait, item_definition as ItemDefinitionTableTrait};
use crate::player as PlayerTableTrait; // Assuming player table is defined in lib.rs
use log;

// --- Mushroom Constants ---
const MUSHROOM_RADIUS: f32 = 16.0; // Visual/interaction radius
const PLAYER_MUSHROOM_INTERACTION_DISTANCE: f32 = 64.0; // Max distance player can be to interact
const PLAYER_MUSHROOM_INTERACTION_DISTANCE_SQUARED: f32 = PLAYER_MUSHROOM_INTERACTION_DISTANCE * PLAYER_MUSHROOM_INTERACTION_DISTANCE;

// Constants for spawning (will be used in environment.rs)
pub(crate) const MUSHROOM_DENSITY_PERCENT: f32 = 0.005; // Target 0.5% of map tiles
pub(crate) const MIN_MUSHROOM_DISTANCE_PX: f32 = 60.0; // Min distance between mushrooms
pub(crate) const MIN_MUSHROOM_DISTANCE_SQ: f32 = MIN_MUSHROOM_DISTANCE_PX * MIN_MUSHROOM_DISTANCE_PX;
pub(crate) const MIN_MUSHROOM_TREE_DISTANCE_PX: f32 = 80.0; // Min distance from trees
pub(crate) const MIN_MUSHROOM_TREE_DISTANCE_SQ: f32 = MIN_MUSHROOM_TREE_DISTANCE_PX * MIN_MUSHROOM_TREE_DISTANCE_PX;
pub(crate) const MIN_MUSHROOM_STONE_DISTANCE_PX: f32 = 80.0; // Min distance from stones
pub(crate) const MIN_MUSHROOM_STONE_DISTANCE_SQ: f32 = MIN_MUSHROOM_STONE_DISTANCE_PX * MIN_MUSHROOM_STONE_DISTANCE_PX;

// --- Mushroom Table Definition ---
#[spacetimedb::table(name = mushroom, public)]
#[derive(Clone)]
pub struct Mushroom {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub pos_x: f32,
    pub pos_y: f32,
}

// --- Interaction Reducer ---

// Helper function to add an item to inventory (simplified version)
// TODO: Refactor inventory logic into its own module later
fn add_item_to_player_inventory(ctx: &ReducerContext, player_id: Identity, item_def_id: u64, quantity: u32) -> Result<(), String> {
    let inventory = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();

    let item_def = item_defs.id().find(item_def_id)
        .ok_or_else(|| format!("Item definition {} not found", item_def_id))?;

    // 1. Try to stack onto existing items (either inventory or hotbar)
    if item_def.is_stackable {
        let mut items_to_update: Vec<crate::items::InventoryItem> = Vec::new();
        let mut remaining_quantity = quantity;

        for mut item in inventory.iter().filter(|i| i.player_identity == player_id && i.item_def_id == item_def_id) {
            let space_available = item_def.stack_size.saturating_sub(item.quantity);
            if space_available > 0 {
                let transfer_qty = std::cmp::min(remaining_quantity, space_available);
                item.quantity += transfer_qty;
                remaining_quantity -= transfer_qty;
                items_to_update.push(item); // Add item to update list
                if remaining_quantity == 0 {
                    break; // Done stacking
                }
            }
        }
        // Apply updates
        for item in items_to_update {
             inventory.instance_id().update(item);
        }

        // If quantity remains, proceed to find empty slot
        if remaining_quantity == 0 {
            log::info!("[AddItem] Stacked {} of item def {} for player {:?}.", quantity, item_def_id, player_id);
            return Ok(());
        }
    } else {
        // Not stackable, must find empty slot immediately
    }

    // 2. Find first empty INVENTORY slot (more robust search needed later)
     let occupied_slots: std::collections::HashSet<u16> = inventory.iter()
        .filter(|i| i.player_identity == player_id && i.inventory_slot.is_some())
        .map(|i| i.inventory_slot.unwrap())
        .collect();

    // Assuming 24 inventory slots (0-23)
    if let Some(empty_slot) = (0..24).find(|slot| !occupied_slots.contains(slot)) {
        let new_item = crate::items::InventoryItem {
            instance_id: 0, // Auto-inc
            player_identity: player_id,
            item_def_id,
            quantity: if item_def.is_stackable { quantity } else { 1 }, // Use remaining if stackable
            hotbar_slot: None,
            inventory_slot: Some(empty_slot),
        };
        inventory.insert(new_item);
        log::info!("[AddItem] Added {} of item def {} to inventory slot {} for player {:?}.", 
                 if item_def.is_stackable { quantity } else { 1 }, item_def_id, empty_slot, player_id);
        Ok(())
    } else {
        log::error!("[AddItem] No empty inventory slots for player {:?} to add item def {}.", player_id, item_def_id);
        Err("Inventory is full".to_string())
    }
}

#[spacetimedb::reducer]
pub fn interact_with_mushroom(ctx: &ReducerContext, mushroom_id: u64) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let mushrooms = ctx.db.mushroom();
    let item_defs = ctx.db.item_definition();

    // 1. Find Player
    let player = players.identity().find(sender_id)
        .ok_or_else(|| "Player not found".to_string())?;

    // 2. Find Mushroom
    let mushroom = mushrooms.id().find(mushroom_id)
        .ok_or_else(|| format!("Mushroom {} not found", mushroom_id))?;

    // 3. Check Distance
    let dx = player.position_x - mushroom.pos_x;
    let dy = player.position_y - mushroom.pos_y;
    let dist_sq = dx * dx + dy * dy;

    if dist_sq > PLAYER_MUSHROOM_INTERACTION_DISTANCE_SQUARED {
        return Err("Too far away to interact with the mushroom".to_string());
    }

    // 4. Find Mushroom Item Definition
    let mushroom_def = item_defs.iter()
        .find(|def| def.name == "Mushroom")
        .ok_or_else(|| "Mushroom item definition not found".to_string())?;

    // 5. Add Mushroom to Inventory (using helper)
    add_item_to_player_inventory(ctx, sender_id, mushroom_def.id, 1)?;

    // 6. Delete the Mushroom Entity
    mushrooms.id().delete(mushroom_id);
    log::info!("Player {:?} picked up mushroom {}", sender_id, mushroom_id);

    Ok(())
} 