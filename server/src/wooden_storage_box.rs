use spacetimedb::{Identity, ReducerContext, SpacetimeType, Table};
use log;

// --- Constants --- 
pub(crate) const BOX_COLLISION_RADIUS: f32 = 18.0; // Similar to campfire
pub(crate) const BOX_COLLISION_Y_OFFSET: f32 = 10.0; // Similar to campfire
pub(crate) const PLAYER_BOX_COLLISION_DISTANCE_SQUARED: f32 = (super::PLAYER_RADIUS + BOX_COLLISION_RADIUS) * (super::PLAYER_RADIUS + BOX_COLLISION_RADIUS);
// TODO: Consider box-box collision? For now, just player-box.

// Import InventoryItem and ItemDefinition tables/traits AND STRUCTS for item finding/checking
use crate::items::{InventoryItem, inventory_item as InventoryItemTableTrait, ItemDefinition, item_definition as ItemDefinitionTableTrait};
// Import Table Traits needed within the reducer
use crate::player as PlayerTableTrait;
// ADDED: Import the WoodenStorageBox table trait itself - REMOVED as it's defined here and accessed via ctx.db
// use crate::wooden_storage_box::wooden_storage_box as WoodenStorageBoxTableTrait;

#[spacetimedb::table(name = wooden_storage_box, public)]
#[derive(Clone)]
pub struct WoodenStorageBox {
    #[primary_key]
    #[auto_inc]
    pub id: u32, // Unique identifier for this storage box instance

    pub pos_x: f32,
    pub pos_y: f32,

    pub placed_by: Identity, // Who placed this storage box

    // Add fields for stored items later
    // For now, just position and placer
}

// Reducer is now uncommented
#[spacetimedb::reducer]
pub fn place_wooden_storage_box(ctx: &ReducerContext, world_x: f32, world_y: f32) -> Result<(), String> {
    let sender_id = ctx.sender;
    // Use table traits via ctx.db
    let inventory_items = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();
    let players = ctx.db.player();
    let wooden_storage_boxes = ctx.db.wooden_storage_box(); // Use trait alias

    log::info!(
        "[PlaceStorageBox] Player {:?} attempting placement at ({:.1}, {:.1})",
        sender_id, world_x, world_y
    );

    // --- 1. Find the 'Wooden Storage Box' Item Definition ID ---
    let box_def_id = item_defs.iter()
        .find(|def| def.name == "Wooden Storage Box")
        .map(|def| def.id)
        .ok_or_else(|| "Item definition 'Wooden Storage Box' not found.".to_string())?;

    // --- 2. Find an instance of the item in the player's inventory/hotbar ---
    // Prioritize finding it in the hotbar, then inventory
    let item_instance_opt = inventory_items.iter()
        .filter(|item| item.player_identity == sender_id && item.item_def_id == box_def_id)
        .min_by_key(|item| match item.hotbar_slot {
            Some(_) => 0, // Prefer hotbar (lower key)
            None => 1,    // Then inventory
        });

    // This check might need refinement if the item isn't deleted immediately or if quantity > 1 becomes possible
    let item_instance = item_instance_opt
        .ok_or_else(|| "Player does not have a Wooden Storage Box item.".to_string())?;

    let item_instance_id_to_delete = item_instance.instance_id; // Store ID before potential borrow issues

    // --- 3. Validate Placement (Simplified - basic distance check) ---
    if let Some(player) = players.identity().find(sender_id) {
        let dx = player.position_x - world_x;
        let dy = player.position_y - world_y;
        let dist_sq = dx * dx + dy * dy;
        // Use a reasonable placement distance squared (e.g., 96 pixels radius)
        let placement_range_sq = 96.0 * 96.0;
        if dist_sq > placement_range_sq {
            return Err("Placement location is too far away.".to_string());
        }
    } else {
        return Err("Could not find player data to validate placement distance.".to_string());
    }

    // TODO: Add collision checks? Ensure not placing inside another object?

    // --- 4. Consume the Item ---
    // Since storage boxes aren't stackable, we assume quantity is 1 and delete the item.
    log::info!(
        "[PlaceStorageBox] Consuming item instance {} (Def ID: {}) from player {:?}",
        item_instance_id_to_delete, box_def_id, sender_id
    );
    inventory_items.instance_id().delete(item_instance_id_to_delete);

    // --- 5. Create the WoodenStorageBox Entity ---
    let new_box = WoodenStorageBox {
        id: 0, // Auto-incremented
        pos_x: world_x,
        pos_y: world_y,
        placed_by: sender_id,
    };
    wooden_storage_boxes.insert(new_box);

    log::info!(
        "[PlaceStorageBox] Successfully placed Wooden Storage Box at ({:.1}, {:.1}) by {:?}",
        world_x, world_y, sender_id
    );

    Ok(())
}