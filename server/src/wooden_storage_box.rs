use spacetimedb::{Identity, ReducerContext, SpacetimeType, Table};
use log;

// --- Constants --- 
pub(crate) const BOX_COLLISION_RADIUS: f32 = 18.0; // Similar to campfire
pub(crate) const BOX_COLLISION_Y_OFFSET: f32 = 10.0; // Similar to campfire
pub(crate) const PLAYER_BOX_COLLISION_DISTANCE_SQUARED: f32 = (super::PLAYER_RADIUS + BOX_COLLISION_RADIUS) * (super::PLAYER_RADIUS + BOX_COLLISION_RADIUS);
const BOX_INTERACTION_DISTANCE_SQUARED: f32 = 64.0 * 64.0; // Similar to campfire interaction
pub const NUM_BOX_SLOTS: usize = 18;
// TODO: Consider box-box collision? For now, just player-box.

// Import InventoryItem and ItemDefinition tables/traits AND STRUCTS for item finding/checking
use crate::items::{InventoryItem, inventory_item as InventoryItemTableTrait, ItemDefinition, item_definition as ItemDefinitionTableTrait};
// Import Table Traits needed within the reducer
use crate::player as PlayerTableTrait;
// ADDED: Import the WoodenStorageBox table trait itself - REMOVED as it's defined here and accessed via ctx.db
use crate::wooden_storage_box::wooden_storage_box as WoodenStorageBoxTableTrait;
// Import inventory management helpers
use crate::inventory_management; // Import the module
// Import add_item_to_player_inventory from items module
use crate::items::add_item_to_player_inventory;
// Import Player struct correctly
use crate::Player;
// Import the ItemContainer trait
use crate::inventory_management::ItemContainer;

#[spacetimedb::table(name = wooden_storage_box, public)]
#[derive(Clone)]
pub struct WoodenStorageBox {
    #[primary_key]
    #[auto_inc]
    pub id: u32, // Unique identifier for this storage box instance

    pub pos_x: f32,
    pub pos_y: f32,

    pub placed_by: Identity, // Who placed this storage box

    // --- Inventory Slots (0-17) --- 
    pub slot_instance_id_0: Option<u64>,
    pub slot_def_id_0: Option<u64>,
    pub slot_instance_id_1: Option<u64>,
    pub slot_def_id_1: Option<u64>,
    pub slot_instance_id_2: Option<u64>,
    pub slot_def_id_2: Option<u64>,
    pub slot_instance_id_3: Option<u64>,
    pub slot_def_id_3: Option<u64>,
    pub slot_instance_id_4: Option<u64>,
    pub slot_def_id_4: Option<u64>,
    pub slot_instance_id_5: Option<u64>,
    pub slot_def_id_5: Option<u64>,
    pub slot_instance_id_6: Option<u64>,
    pub slot_def_id_6: Option<u64>,
    pub slot_instance_id_7: Option<u64>,
    pub slot_def_id_7: Option<u64>,
    pub slot_instance_id_8: Option<u64>,
    pub slot_def_id_8: Option<u64>,
    pub slot_instance_id_9: Option<u64>,
    pub slot_def_id_9: Option<u64>,
    pub slot_instance_id_10: Option<u64>,
    pub slot_def_id_10: Option<u64>,
    pub slot_instance_id_11: Option<u64>,
    pub slot_def_id_11: Option<u64>,
    pub slot_instance_id_12: Option<u64>,
    pub slot_def_id_12: Option<u64>,
    pub slot_instance_id_13: Option<u64>,
    pub slot_def_id_13: Option<u64>,
    pub slot_instance_id_14: Option<u64>,
    pub slot_def_id_14: Option<u64>,
    pub slot_instance_id_15: Option<u64>,
    pub slot_def_id_15: Option<u64>,
    pub slot_instance_id_16: Option<u64>,
    pub slot_def_id_16: Option<u64>,
    pub slot_instance_id_17: Option<u64>,
    pub slot_def_id_17: Option<u64>,
}

// --- Trait Implementation --- 

impl ItemContainer for WoodenStorageBox {
    fn num_slots(&self) -> usize {
        NUM_BOX_SLOTS
    }

    fn get_slot_instance_id(&self, slot_index: u8) -> Option<u64> {
        crate::inventory_management::get_box_slot_instance_id(self, slot_index)
    }

    fn get_slot_def_id(&self, slot_index: u8) -> Option<u64> {
        crate::inventory_management::get_box_slot_def_id(self, slot_index)
    }

    fn set_slot(&mut self, slot_index: u8, instance_id: Option<u64>, def_id: Option<u64>) {
        crate::inventory_management::set_box_slot(self, slot_index, instance_id, def_id)
    }
}

// --- Helper Function (Validation) --- 

/// Validates if a player can interact with a specific box (checks existence and distance).
/// Returns Ok((Player struct instance, WoodenStorageBox struct instance)) on success, or Err(String) on failure.
/// Does NOT check ownership.
fn validate_box_interaction(
    ctx: &ReducerContext,
    box_id: u32,
) -> Result<(Player, WoodenStorageBox), String> { // Use corrected Player type
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    let boxes = ctx.db.wooden_storage_box();

    let player = players.identity().find(sender_id).ok_or_else(|| "Player not found".to_string())?;
    let storage_box = boxes.id().find(box_id).ok_or_else(|| format!("Storage Box {} not found", box_id))?;

    // Check distance between the interacting player and the box
    let dx = player.position_x - storage_box.pos_x;
    let dy = player.position_y - storage_box.pos_y;
    if (dx * dx + dy * dy) > BOX_INTERACTION_DISTANCE_SQUARED {
        return Err("Too far away".to_string());
    }
    Ok((player, storage_box))
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
        slot_instance_id_0: None,
        slot_def_id_0: None,
        slot_instance_id_1: None,
        slot_def_id_1: None,
        slot_instance_id_2: None,
        slot_def_id_2: None,
        slot_instance_id_3: None,
        slot_def_id_3: None,
        slot_instance_id_4: None,
        slot_def_id_4: None,
        slot_instance_id_5: None,
        slot_def_id_5: None,
        slot_instance_id_6: None,
        slot_def_id_6: None,
        slot_instance_id_7: None,
        slot_def_id_7: None,
        slot_instance_id_8: None,
        slot_def_id_8: None,
        slot_instance_id_9: None,
        slot_def_id_9: None,
        slot_instance_id_10: None,
        slot_def_id_10: None,
        slot_instance_id_11: None,
        slot_def_id_11: None,
        slot_instance_id_12: None,
        slot_def_id_12: None,
        slot_instance_id_13: None,
        slot_def_id_13: None,
        slot_instance_id_14: None,
        slot_def_id_14: None,
        slot_instance_id_15: None,
        slot_def_id_15: None,
        slot_instance_id_16: None,
        slot_def_id_16: None,
        slot_instance_id_17: None,
        slot_def_id_17: None,
    };
    wooden_storage_boxes.insert(new_box);

    log::info!(
        "[PlaceStorageBox] Successfully placed Wooden Storage Box at ({:.1}, {:.1}) by {:?}",
        world_x, world_y, sender_id
    );

    Ok(())
}

/// Reducer called by the client when the player attempts to interact (e.g., press 'E')
/// Validates proximity for opening the box UI.
#[spacetimedb::reducer]
pub fn interact_with_storage_box(ctx: &ReducerContext, box_id: u32) -> Result<(), String> {
    validate_box_interaction(ctx, box_id)?; // Use helper for validation
    log::debug!("Player {:?} interaction check OK for box {}", ctx.sender, box_id);
    Ok(())
}

/// Moves an item from the player's inventory/hotbar INTO a specified slot in the storage box.
#[spacetimedb::reducer]
pub fn move_item_to_box(
    ctx: &ReducerContext, 
    box_id: u32, 
    target_slot_index: u8, 
    item_instance_id: u64
) -> Result<(), String> {
    // Get tables
    let inventory_items = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();
    let mut boxes = ctx.db.wooden_storage_box(); 

    // --- Validations --- 
    let (_player, mut storage_box) = validate_box_interaction(ctx, box_id)?;
    let mut item_to_move = inventory_items.instance_id().find(item_instance_id).ok_or("Item instance not found")?;
    let item_def_to_move = item_defs.id().find(item_to_move.item_def_id).ok_or("Item definition not found")?;
    if target_slot_index >= NUM_BOX_SLOTS as u8 { return Err(format!("Invalid target box slot index: {}", target_slot_index)); }
    if item_to_move.player_identity != ctx.sender { return Err("Item does not belong to player".to_string()); }
    if item_to_move.inventory_slot.is_none() && item_to_move.hotbar_slot.is_none() { return Err("Item to move must be in player inventory or hotbar".to_string()); }
    let dx = _player.position_x - storage_box.pos_x;
    let dy = _player.position_y - storage_box.pos_y;
    if (dx * dx + dy * dy) > BOX_INTERACTION_DISTANCE_SQUARED { return Err("Too far away from storage box".to_string()); }

    // --- Call GENERIC Handler --- 
    inventory_management::handle_move_to_container_slot(
        ctx, 
        &mut storage_box, 
        target_slot_index, 
        &mut item_to_move, 
        &item_def_to_move
    )?;

    // --- Commit Box Update --- 
    boxes.id().update(storage_box);
    Ok(())
}

/// Moves an item FROM a storage box slot INTO the player's inventory.
#[spacetimedb::reducer]
pub fn move_item_from_box(
    ctx: &ReducerContext, 
    box_id: u32, 
    source_slot_index: u8
) -> Result<(), String> {
    // Get tables
    let mut boxes = ctx.db.wooden_storage_box();
    // inventory_items is accessed within handler via ctx

    // --- Validations --- 
    let (_player, mut storage_box) = validate_box_interaction(ctx, box_id)?;
    if source_slot_index >= NUM_BOX_SLOTS as u8 { return Err(format!("Invalid source box slot index: {}", source_slot_index)); }
    if inventory_management::get_box_slot_instance_id(&storage_box, source_slot_index).is_none() {
        return Err(format!("Source slot {} in box is empty", source_slot_index));
    }

    // --- Call GENERIC Handler --- 
    inventory_management::handle_move_from_container_slot(
        ctx, 
        &mut storage_box, 
        source_slot_index
    )?;

    // --- Commit Box Update --- 
    boxes.id().update(storage_box);
    Ok(())
}

/// Moves an item BETWEEN two slots within the same storage box.
#[spacetimedb::reducer]
pub fn move_item_within_box(
    ctx: &ReducerContext,
    box_id: u32,
    source_slot_index: u8,
    target_slot_index: u8,
) -> Result<(), String> {
    // Get tables
    let mut boxes = ctx.db.wooden_storage_box();
    // inventory_items and item_defs accessed in handler via ctx

    // --- Validations --- 
    let (_player, mut storage_box) = validate_box_interaction(ctx, box_id)?;
    if source_slot_index >= NUM_BOX_SLOTS as u8 || target_slot_index >= NUM_BOX_SLOTS as u8 || source_slot_index == target_slot_index {
        return Err("Invalid source or target slot index".to_string());
    }
    if inventory_management::get_box_slot_instance_id(&storage_box, source_slot_index).is_none() {
        return Err(format!("Source slot {} is empty", source_slot_index));
    }

    // --- Call GENERIC Handler --- 
    inventory_management::handle_move_within_container(
        ctx, 
        &mut storage_box, 
        source_slot_index, 
        target_slot_index
    )?;

    // --- Commit Box Update --- 
    boxes.id().update(storage_box);
    Ok(())
}

/// Splits a stack from player inventory/hotbar into an empty box slot.
#[spacetimedb::reducer]
pub fn split_stack_into_box(
    ctx: &ReducerContext,
    box_id: u32,
    target_slot_index: u8,
    source_item_instance_id: u64,
    quantity_to_split: u32,
) -> Result<(), String> {
    // Get tables
    let mut boxes = ctx.db.wooden_storage_box();
    let inventory_items = ctx.db.inventory_item(); // Need this to find source_item
    let item_defs = ctx.db.item_definition(); // Need this for validation

    // --- Validations --- 
    let (_player, mut storage_box) = validate_box_interaction(ctx, box_id)?;
    let mut source_item = inventory_items.instance_id().find(source_item_instance_id).ok_or("Source item not found")?;
    if quantity_to_split == 0 { return Err("Quantity to split must be greater than 0".to_string()); }
    if source_item.quantity < quantity_to_split { return Err(format!("Not enough items in source item instance: {} < {}", source_item.quantity, quantity_to_split)); }

    // --- Call GENERIC Handler --- 
    inventory_management::handle_split_into_container(
        ctx, 
        &mut storage_box, 
        target_slot_index, 
        &mut source_item, 
        quantity_to_split
    )?;

    // --- Commit Box Update --- 
    boxes.id().update(storage_box);
    Ok(())
}

/// Splits a stack from a box slot into the player's inventory/hotbar.
#[spacetimedb::reducer]
pub fn split_stack_from_box(
    ctx: &ReducerContext,
    box_id: u32,
    source_slot_index: u8,
    quantity_to_split: u32,
    target_slot_type: String, 
    target_slot_index: u32,   
) -> Result<(), String> {
    // Get tables
    let mut boxes = ctx.db.wooden_storage_box();
    let inventory_items = ctx.db.inventory_item(); // Need this to find source_item
    let item_defs = ctx.db.item_definition(); // Need this for validation

    // --- Validations --- 
    let (_player, mut storage_box) = validate_box_interaction(ctx, box_id)?;
    let source_instance_id = inventory_management::get_box_slot_instance_id(&storage_box, source_slot_index)
        .ok_or(format!("Source slot {} is empty", source_slot_index))?;
    let mut source_item = inventory_items.instance_id().find(source_instance_id).ok_or("Source item instance not found")?;
    if quantity_to_split == 0 { return Err("Quantity to split must be greater than 0".to_string()); }
    if source_item.quantity < quantity_to_split { return Err(format!("Not enough items in source item instance: {} < {}", source_item.quantity, quantity_to_split)); }

    // --- Call GENERIC Handler --- 
    inventory_management::handle_split_from_container(
        ctx, 
        &mut storage_box, 
        source_slot_index, 
        &mut source_item, 
        quantity_to_split,
        target_slot_type, 
        target_slot_index
    )?;

    // --- Commit Box Update --- 
    boxes.id().update(storage_box);
    Ok(())
}

/// Splits a stack from one box slot into another empty box slot.
#[spacetimedb::reducer]
pub fn split_stack_within_box(
    ctx: &ReducerContext,
    box_id: u32,
    source_slot_index: u8,
    target_slot_index: u8,
    quantity_to_split: u32,
) -> Result<(), String> {
    // Get tables
    let mut boxes = ctx.db.wooden_storage_box();
    let inventory_items = ctx.db.inventory_item(); // Need this to find source_item
    let item_defs = ctx.db.item_definition(); // Need this for validation

    // --- Validations --- 
    let (_player, mut storage_box) = validate_box_interaction(ctx, box_id)?;
    let source_instance_id = inventory_management::get_box_slot_instance_id(&storage_box, source_slot_index)
        .ok_or(format!("Source slot {} is empty", source_slot_index))?;
    let mut source_item = inventory_items.instance_id().find(source_instance_id).ok_or("Source item instance not found")?;
    if quantity_to_split == 0 { return Err("Quantity to split must be greater than 0".to_string()); }
    if source_item.quantity < quantity_to_split { return Err(format!("Not enough items in source item instance: {} < {}", source_item.quantity, quantity_to_split)); }

    // --- Call GENERIC Handler ---
    inventory_management::handle_split_within_container(
        ctx,
        &mut storage_box,
        source_slot_index,
        target_slot_index,
        &mut source_item, 
        quantity_to_split
    )?;

    // --- Commit Box Update --- 
    boxes.id().update(storage_box);
    Ok(())
}

/// Quickly moves an item from a box slot to the player inventory.
#[spacetimedb::reducer]
pub fn quick_move_from_box(
    ctx: &ReducerContext, 
    box_id: u32, 
    source_slot_index: u8
) -> Result<(), String> {
    // Get tables
    let mut boxes = ctx.db.wooden_storage_box();
    let inventory_items = ctx.db.inventory_item(); 

    // --- Validations --- 
    let (_player, mut storage_box) = validate_box_interaction(ctx, box_id)?;
    if source_slot_index >= NUM_BOX_SLOTS as u8 { return Err(format!("Invalid source box slot index: {}", source_slot_index)); }
    if inventory_management::get_box_slot_instance_id(&storage_box, source_slot_index).is_none() {
        return Err(format!("Source slot {} in box is empty", source_slot_index));
    }

    // --- Call Handler --- 
    inventory_management::handle_quick_move_from_container(
        ctx, 
        &mut storage_box, 
        source_slot_index
    )?;

    // --- Commit Box Update --- 
    boxes.id().update(storage_box);
    Ok(())
}

/// Quickly moves an item from player inventory/hotbar to the first available/mergeable slot in the box.
#[spacetimedb::reducer]
pub fn quick_move_to_box(
    ctx: &ReducerContext, 
    box_id: u32, 
    item_instance_id: u64
) -> Result<(), String> {
    let sender_id = ctx.sender;
    // Get tables
    let mut boxes = ctx.db.wooden_storage_box();
    let inventory_items = ctx.db.inventory_item(); 
    let item_defs = ctx.db.item_definition();

    // --- Validations --- 
    let (_player, mut storage_box) = validate_box_interaction(ctx, box_id)?;
    let mut item_to_move = inventory_items.instance_id().find(item_instance_id).ok_or("Item instance not found")?;
    let item_def_to_move = item_defs.id().find(item_to_move.item_def_id).ok_or("Item definition not found")?;
    if item_to_move.player_identity != sender_id { return Err("Item does not belong to player".to_string()); }
    if item_to_move.inventory_slot.is_none() && item_to_move.hotbar_slot.is_none() { return Err("Item must be in player inventory or hotbar".to_string()); }

    // --- Call Handler --- 
    inventory_management::handle_quick_move_to_container(
        ctx, 
        &mut storage_box, 
        &mut item_to_move, 
        &item_def_to_move
    )?;

    // --- Commit Box Update --- 
    boxes.id().update(storage_box);
    Ok(())
}