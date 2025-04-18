use spacetimedb::{ReducerContext, Identity, Table};
use log;

// Import necessary types and Table Traits
use crate::items::{InventoryItem, ItemDefinition, calculate_merge_result, add_item_to_player_inventory};
use crate::items::{inventory_item as InventoryItemTableTrait, item_definition as ItemDefinitionTableTrait};
use crate::wooden_storage_box::{WoodenStorageBox, NUM_BOX_SLOTS}; // Import Box struct and constant

// --- Generic Item Container Trait --- 

/// Trait for entities that can hold items in indexed slots.
pub(crate) trait ItemContainer {
    /// Returns the total number of slots in this container.
    fn num_slots(&self) -> usize;

    /// Gets the item instance ID from a specific slot index.
    /// Returns None if the slot index is invalid or the slot is empty.
    fn get_slot_instance_id(&self, slot_index: u8) -> Option<u64>;

    /// Gets the item definition ID from a specific slot index.
    /// Returns None if the slot index is invalid or the slot is empty.
    fn get_slot_def_id(&self, slot_index: u8) -> Option<u64>;

    /// Sets the instance and definition IDs for a specific slot index.
    /// Implementations should handle invalid indices gracefully (e.g., do nothing).
    fn set_slot(&mut self, slot_index: u8, instance_id: Option<u64>, def_id: Option<u64>);

    // We could add more methods later if needed, e.g., find_first_empty_slot
}

// --- Helper functions for getting/setting box slots by index --- 

/// Gets the instance ID from a specific slot index in a WoodenStorageBox.
pub(crate) fn get_box_slot_instance_id(storage_box: &WoodenStorageBox, slot_index: u8) -> Option<u64> {
    if slot_index >= NUM_BOX_SLOTS as u8 { return None; }
    match slot_index {
        0 => storage_box.slot_instance_id_0,
        1 => storage_box.slot_instance_id_1,
        2 => storage_box.slot_instance_id_2,
        3 => storage_box.slot_instance_id_3,
        4 => storage_box.slot_instance_id_4,
        5 => storage_box.slot_instance_id_5,
        6 => storage_box.slot_instance_id_6,
        7 => storage_box.slot_instance_id_7,
        8 => storage_box.slot_instance_id_8,
        9 => storage_box.slot_instance_id_9,
        10 => storage_box.slot_instance_id_10,
        11 => storage_box.slot_instance_id_11,
        12 => storage_box.slot_instance_id_12,
        13 => storage_box.slot_instance_id_13,
        14 => storage_box.slot_instance_id_14,
        15 => storage_box.slot_instance_id_15,
        16 => storage_box.slot_instance_id_16,
        17 => storage_box.slot_instance_id_17,
        _ => None, // Should be unreachable due to check above
    }
}

/// Gets the definition ID from a specific slot index in a WoodenStorageBox.
pub(crate) fn get_box_slot_def_id(storage_box: &WoodenStorageBox, slot_index: u8) -> Option<u64> {
    if slot_index >= NUM_BOX_SLOTS as u8 { return None; }
    match slot_index {
        0 => storage_box.slot_def_id_0,
        1 => storage_box.slot_def_id_1,
        2 => storage_box.slot_def_id_2,
        3 => storage_box.slot_def_id_3,
        4 => storage_box.slot_def_id_4,
        5 => storage_box.slot_def_id_5,
        6 => storage_box.slot_def_id_6,
        7 => storage_box.slot_def_id_7,
        8 => storage_box.slot_def_id_8,
        9 => storage_box.slot_def_id_9,
        10 => storage_box.slot_def_id_10,
        11 => storage_box.slot_def_id_11,
        12 => storage_box.slot_def_id_12,
        13 => storage_box.slot_def_id_13,
        14 => storage_box.slot_def_id_14,
        15 => storage_box.slot_def_id_15,
        16 => storage_box.slot_def_id_16,
        17 => storage_box.slot_def_id_17,
        _ => None,
    }
}

/// Sets the instance and definition IDs for a specific slot index in a mutable WoodenStorageBox.
pub(crate) fn set_box_slot(
    storage_box: &mut WoodenStorageBox, 
    slot_index: u8, 
    instance_id: Option<u64>, 
    def_id: Option<u64>
) {
    if slot_index >= NUM_BOX_SLOTS as u8 { return; } // Ignore invalid index
    match slot_index {
        0 => { storage_box.slot_instance_id_0 = instance_id; storage_box.slot_def_id_0 = def_id; }
        1 => { storage_box.slot_instance_id_1 = instance_id; storage_box.slot_def_id_1 = def_id; }
        2 => { storage_box.slot_instance_id_2 = instance_id; storage_box.slot_def_id_2 = def_id; }
        3 => { storage_box.slot_instance_id_3 = instance_id; storage_box.slot_def_id_3 = def_id; }
        4 => { storage_box.slot_instance_id_4 = instance_id; storage_box.slot_def_id_4 = def_id; }
        5 => { storage_box.slot_instance_id_5 = instance_id; storage_box.slot_def_id_5 = def_id; }
        6 => { storage_box.slot_instance_id_6 = instance_id; storage_box.slot_def_id_6 = def_id; }
        7 => { storage_box.slot_instance_id_7 = instance_id; storage_box.slot_def_id_7 = def_id; }
        8 => { storage_box.slot_instance_id_8 = instance_id; storage_box.slot_def_id_8 = def_id; }
        9 => { storage_box.slot_instance_id_9 = instance_id; storage_box.slot_def_id_9 = def_id; }
        10 => { storage_box.slot_instance_id_10 = instance_id; storage_box.slot_def_id_10 = def_id; }
        11 => { storage_box.slot_instance_id_11 = instance_id; storage_box.slot_def_id_11 = def_id; }
        12 => { storage_box.slot_instance_id_12 = instance_id; storage_box.slot_def_id_12 = def_id; }
        13 => { storage_box.slot_instance_id_13 = instance_id; storage_box.slot_def_id_13 = def_id; }
        14 => { storage_box.slot_instance_id_14 = instance_id; storage_box.slot_def_id_14 = def_id; }
        15 => { storage_box.slot_instance_id_15 = instance_id; storage_box.slot_def_id_15 = def_id; }
        16 => { storage_box.slot_instance_id_16 = instance_id; storage_box.slot_def_id_16 = def_id; }
        17 => { storage_box.slot_instance_id_17 = instance_id; storage_box.slot_def_id_17 = def_id; }
        _ => {}, // Should be unreachable
    }
}


// --- Core Logic Handlers (Accessing ctx.db directly) --- 

/// Handles moving an item from player inventory/hotbar INTO a container slot.
/// Performs merge if possible, otherwise places/swaps.
/// Updates the `container` struct directly, but caller must commit the change to the DB.
pub(crate) fn handle_move_to_container_slot<C: ItemContainer>(
    ctx: &ReducerContext,
    container: &mut C, 
    target_slot_index: u8,
    item_to_move: &mut InventoryItem, 
    item_def_to_move: &ItemDefinition
) -> Result<(), String> {
    // Get tables inside handler
    let inventory_table = ctx.db.inventory_item();

    let target_instance_id_opt = container.get_slot_instance_id(target_slot_index);
    let sender_id = item_to_move.player_identity;
    let item_instance_id = item_to_move.instance_id;

    if target_slot_index >= container.num_slots() as u8 {
        return Err(format!("Target slot index {} out of bounds.", target_slot_index));
    }

    if let Some(target_instance_id) = target_instance_id_opt {
        // Target occupied: Merge or Swap
        let mut target_item = inventory_table.instance_id().find(target_instance_id)
                                .ok_or_else(|| format!("Target item instance {} in container slot {} not found!", target_instance_id, target_slot_index))?;

        match calculate_merge_result(item_to_move, &target_item, item_def_to_move) {
            Ok((_, source_new_qty, target_new_qty, delete_source)) => {
                // Merge successful
                log::info!("[InvManager MergeToContainer] Merging item {} onto item {}.", item_instance_id, target_instance_id);
                target_item.quantity = target_new_qty;
                inventory_table.instance_id().update(target_item);
                if delete_source {
                    inventory_table.instance_id().delete(item_instance_id);
                } else {
                    item_to_move.quantity = source_new_qty;
                    item_to_move.inventory_slot = None; 
                    item_to_move.hotbar_slot = None;
                    inventory_table.instance_id().update(item_to_move.clone());
                }
                // Container state unchanged on merge
            },
            Err(_) => {
                // Merge Failed: Swap
                log::info!("[InvManager SwapToContainer] Cannot merge. Swapping slot {}.", target_slot_index);
                let source_inv_slot = item_to_move.inventory_slot;
                let source_hotbar_slot = item_to_move.hotbar_slot;
                
                // Move target item to player
                target_item.inventory_slot = source_inv_slot;
                target_item.hotbar_slot = source_hotbar_slot;
                target_item.player_identity = sender_id;
                inventory_table.instance_id().update(target_item);
                
                // Move source item to container
                item_to_move.inventory_slot = None;
                item_to_move.hotbar_slot = None;
                inventory_table.instance_id().update(item_to_move.clone()); 
                
                // Update container state using trait method
                container.set_slot(target_slot_index, Some(item_instance_id), Some(item_def_to_move.id));
            }
        }
    } else {
        // Target Empty: Place
        log::info!("[InvManager PlaceInContainer] Moving item {} to empty slot {}", item_instance_id, target_slot_index);
        item_to_move.inventory_slot = None;
        item_to_move.hotbar_slot = None;
        inventory_table.instance_id().update(item_to_move.clone());
        // Update container state using trait method
        container.set_slot(target_slot_index, Some(item_instance_id), Some(item_def_to_move.id));
    }
    Ok(())
}

/// Handles moving an item FROM a container slot TO the player's inventory.
/// Updates the `container` struct directly, but caller must commit the change to the DB.
pub(crate) fn handle_move_from_container_slot<C: ItemContainer>(
    ctx: &ReducerContext,
    container: &mut C, 
    source_slot_index: u8
) -> Result<(), String> {
    // Get tables inside handler
    let inventory_table = ctx.db.inventory_item();

    let sender_id = ctx.sender;
    let source_instance_id = container.get_slot_instance_id(source_slot_index)
        .ok_or_else(|| format!("Source slot {} in container is empty", source_slot_index))?;
    let source_def_id = container.get_slot_def_id(source_slot_index)
        .ok_or_else(|| format!("Missing definition ID in source slot {}", source_slot_index))?;
    
    let mut item_to_move = inventory_table.instance_id().find(source_instance_id)
        .ok_or("Item instance in container slot not found in inventory table")?;

    // Clear Slot in container struct using trait method
    container.set_slot(source_slot_index, None, None);
    
    log::info!("[InvManager FromContainer] Moving item {} (Def {}) from container slot {} to player {:?} inventory", 
             source_instance_id, source_def_id, source_slot_index, sender_id);
    
    // Update item ownership 
    item_to_move.player_identity = sender_id;
    inventory_table.instance_id().update(item_to_move.clone()); 

    // Call helper from items.rs 
    add_item_to_player_inventory(ctx, sender_id, source_def_id, item_to_move.quantity)
}

/// Handles moving an item BETWEEN slots within the same container.
/// Updates the `container` struct directly, but caller must commit the change to the DB.
pub(crate) fn handle_move_within_container<C: ItemContainer>(
    ctx: &ReducerContext,
    container: &mut C,
    source_slot_index: u8,
    target_slot_index: u8
) -> Result<(), String> {
    // Get tables inside handler
    let inventory_table = ctx.db.inventory_item();
    let item_def_table = ctx.db.item_definition();

    // Get Source Info using trait methods
    let source_instance_id = container.get_slot_instance_id(source_slot_index)
        .ok_or(format!("Source slot {} is empty", source_slot_index))?;
    let source_def_id = container.get_slot_def_id(source_slot_index)
        .ok_or("Source definition ID missing")?;

    // Get Target Info (Optional) using trait methods
    let target_instance_id_opt = container.get_slot_instance_id(target_slot_index);
    let target_def_id_opt = container.get_slot_def_id(target_slot_index);

    if target_slot_index >= container.num_slots() as u8 {
        return Err(format!("Target slot index {} out of bounds.", target_slot_index));
    }

    if let Some(target_instance_id) = target_instance_id_opt {
        // Target occupied: Try Merge then Swap
        let mut source_item = inventory_table.instance_id().find(source_instance_id).ok_or("Source item not found")?;
        let mut target_item = inventory_table.instance_id().find(target_instance_id).ok_or("Target item not found")?;
        let item_def = item_def_table.id().find(source_def_id).ok_or("Item definition not found")?;

        match calculate_merge_result(&source_item, &target_item, &item_def) {
            Ok((_, source_new_qty, target_new_qty, delete_source)) => {
                // Merge Possible
                log::info!("[InvManager WithinContainer Merge] Merging slot {} onto slot {}", source_slot_index, target_slot_index);
                target_item.quantity = target_new_qty;
                inventory_table.instance_id().update(target_item);
                if delete_source {
                    inventory_table.instance_id().delete(source_instance_id);
                } else {
                    source_item.quantity = source_new_qty;
                    inventory_table.instance_id().update(source_item);
                }
                container.set_slot(source_slot_index, None, None); // Clear source slot
            },
            Err(_) => {
                // Merge Failed: Swap
                log::info!("[InvManager WithinContainer Swap] Swapping slot {} and {}", source_slot_index, target_slot_index);
                container.set_slot(target_slot_index, Some(source_instance_id), Some(source_def_id));
                container.set_slot(source_slot_index, target_instance_id_opt, target_def_id_opt);
            }
        }
    } else {
        // Target Empty: Move
        log::info!("[InvManager WithinContainer Move] Moving from slot {} to empty slot {}", source_slot_index, target_slot_index);
        container.set_slot(target_slot_index, Some(source_instance_id), Some(source_def_id));
        container.set_slot(source_slot_index, None, None);
    }
    Ok(())
}

// --- Split Handlers (Accessing ctx.db directly) --- 

/// Handles splitting a stack FROM player inventory INTO an empty container slot.
/// Updates the `container` struct directly, but caller must commit the change to the DB.
pub(crate) fn handle_split_into_container<C: ItemContainer>(
    ctx: &ReducerContext,
    container: &mut C, 
    target_slot_index: u8,
    source_item: &mut InventoryItem, 
    quantity_to_split: u32
) -> Result<(), String> {
    log::info!("[InvManager SplitToContainer] Splitting {} from item {} into container slot {}", 
             quantity_to_split, source_item.instance_id, target_slot_index);

    if target_slot_index >= container.num_slots() as u8 {
        return Err(format!("Target slot index {} out of bounds.", target_slot_index));
    }
    if container.get_slot_instance_id(target_slot_index).is_some() {
        return Err(format!("Target slot {} is already occupied.", target_slot_index));
    }

    // 1. Perform split using helper from items.rs
    let new_item_instance_id = crate::items::split_stack_helper(ctx, source_item, quantity_to_split)?;
    let new_item_def_id = source_item.item_def_id;

    // 2. Update the container struct state with the NEW item using trait method
    container.set_slot(target_slot_index, Some(new_item_instance_id), Some(new_item_def_id));

    Ok(())
}

/// Handles splitting a stack FROM a container slot TO player inventory/hotbar.
/// Updates the `container` struct directly, but caller must commit the change to the DB.
pub(crate) fn handle_split_from_container<C: ItemContainer>(
    ctx: &ReducerContext,
    container: &mut C, 
    source_slot_index: u8,
    source_item: &mut InventoryItem, 
    quantity_to_split: u32,
    target_slot_type: String, 
    target_slot_index: u32
) -> Result<(), String> {
    log::info!("[InvManager SplitFromContainer] Splitting {} from container slot {} to player {} slot {}",
             quantity_to_split, source_slot_index, target_slot_type, target_slot_index);

    // 1. Perform split using helper
    let new_item_instance_id = crate::items::split_stack_helper(ctx, source_item, quantity_to_split)?;

    // 2. Move the NEWLY CREATED stack to the target player slot
    log::debug!("[InvManager SplitFromContainer] Moving new item {} to player", new_item_instance_id);
    let mut new_item_stack = ctx.db.inventory_item().instance_id().find(new_item_instance_id)
                            .ok_or("Newly split item stack not found!")?;
    new_item_stack.player_identity = ctx.sender; 

    // Call appropriate move function from items.rs 
    let move_result = if target_slot_type == "inventory" {
        crate::items::move_item_to_inventory(ctx, new_item_instance_id, target_slot_index as u16)
    } else if target_slot_type == "hotbar" {
        crate::items::move_item_to_hotbar(ctx, new_item_instance_id, target_slot_index as u8)
    } else {
        ctx.db.inventory_item().instance_id().delete(new_item_instance_id); 
        Err(format!("Invalid target slot type '{}' in split handler", target_slot_type))
    };

    // If move to player failed (e.g., full inventory), log the error and return it.
    if let Err(ref e) = move_result { // Borrow the error for logging
        log::error!("[InvManager SplitFromContainer] Failed to move split stack {} to player: {:?}. Original stack quantity remains reduced.", 
                  new_item_instance_id, e); // Log the borrowed error `e`
        return move_result; // Return the original error Result
    }

    // If move was successful, clear the source slot in the container struct
    container.set_slot(source_slot_index, None, None);
    Ok(())
}

/// Handles splitting a stack BETWEEN two slots within the same container.
/// Updates the `container` struct directly, but caller must commit the change to the DB.
pub(crate) fn handle_split_within_container<C: ItemContainer>(
    ctx: &ReducerContext,
    container: &mut C,
    source_slot_index: u8,
    target_slot_index: u8,
    source_item: &mut InventoryItem, 
    quantity_to_split: u32
) -> Result<(), String> {
     log::info!("[InvManager SplitWithinContainer] Splitting {} from slot {} to slot {} within container",
             quantity_to_split, source_slot_index, target_slot_index);

    // Validations specific to this operation
    if target_slot_index >= container.num_slots() as u8 {
        return Err(format!("Target slot index {} out of bounds.", target_slot_index));
    }
    if container.get_slot_instance_id(target_slot_index).is_some() {
        return Err(format!("Target slot {} is already occupied.", target_slot_index));
    }

    // 1. Perform the split
    let new_item_instance_id = crate::items::split_stack_helper(ctx, source_item, quantity_to_split)?;
    let new_item_def_id = source_item.item_def_id;

    // 2. Place the NEW stack in the target container slot using trait method
    log::debug!("[InvManager SplitWithinContainer] Placing new item {} in target slot {}", new_item_instance_id, target_slot_index);
    container.set_slot(target_slot_index, Some(new_item_instance_id), Some(new_item_def_id));

    Ok(())
}

/// Handles quickly moving an item FROM a container slot TO the player inventory.
/// Assumes validation (distance, etc.) is done by the calling reducer.
/// Updates the `container` struct directly, but caller must commit the change to the DB.
pub(crate) fn handle_quick_move_from_container<C: ItemContainer>(
    ctx: &ReducerContext, 
    container: &mut C, 
    source_slot_index: u8
) -> Result<(), String> {
    // Get inventory table handle
    let inventory_table = ctx.db.inventory_item();
    let sender_id = ctx.sender;

    // Get item info using trait methods
    let source_instance_id = container.get_slot_instance_id(source_slot_index)
        .ok_or_else(|| format!("Source slot {} in container is empty", source_slot_index))?;
    let source_def_id = container.get_slot_def_id(source_slot_index)
        .ok_or_else(|| format!("Missing definition ID in source slot {}", source_slot_index))?;
    
    let mut item_to_move = inventory_table.instance_id().find(source_instance_id)
        .ok_or("Item instance in container slot not found in inventory table")?;

    // Clear Slot in container struct using trait method
    container.set_slot(source_slot_index, None, None);
    
    log::info!("[InvManager QuickFromContainer] Moving item {} (Def {}) from container slot {} to player {:?} inventory", 
             source_instance_id, source_def_id, source_slot_index, sender_id);
    
    // Update item ownership 
    item_to_move.player_identity = sender_id;
    inventory_table.instance_id().update(item_to_move.clone()); // No ? needed

    // Call helper from items.rs to add to player inventory (handles stacking/finding slots)
    add_item_to_player_inventory(ctx, sender_id, source_def_id, item_to_move.quantity)
}

/// Handles quickly moving an item FROM the player inventory/hotbar INTO the first
/// available/mergeable slot in the container.
/// Updates the `container` struct directly, but caller must commit the change to the DB.
pub(crate) fn handle_quick_move_to_container<C: ItemContainer>(
    ctx: &ReducerContext,
    container: &mut C,
    item_to_move: &mut InventoryItem, // Item coming from player inventory
    item_def_to_move: &ItemDefinition,
) -> Result<(), String> {
    let inventory_table = ctx.db.inventory_item();
    let sender_id = ctx.sender;
    let item_instance_id = item_to_move.instance_id;
    let original_quantity = item_to_move.quantity;
    let mut operation_occured = false; // Track if we merged or placed

    // 1. Attempt to merge with existing stacks
    if item_def_to_move.is_stackable {
        for slot_index in 0..container.num_slots() as u8 {
            if let Some(target_instance_id) = container.get_slot_instance_id(slot_index) {
                if container.get_slot_def_id(slot_index) == Some(item_def_to_move.id) { // Check if same item type
                    let mut target_item = inventory_table.instance_id().find(target_instance_id)
                                            .ok_or_else(|| format!("Target item {} in slot {} missing!", target_instance_id, slot_index))?;
                    
                    match calculate_merge_result(item_to_move, &target_item, item_def_to_move) {
                        Ok((qty_transfer, source_new_qty, target_new_qty, delete_source)) => {
                            if qty_transfer > 0 { // Only proceed if merge actually happened
                                log::info!("[InvManager QuickToContainer Merge] Merging {} from item {} onto item {} in slot {}",
                                        qty_transfer, item_instance_id, target_instance_id, slot_index);
                                target_item.quantity = target_new_qty;
                                inventory_table.instance_id().update(target_item);
                                if delete_source {
                                    inventory_table.instance_id().delete(item_instance_id);
                                    item_to_move.quantity = 0; // Mark as fully merged
                                } else {
                                    item_to_move.quantity = source_new_qty;
                                    // Don't clear player slots yet, might need them if placing remainder fails
                                }
                                operation_occured = true;
                                // If source fully merged, we are done
                                if delete_source { return Ok(()); }
                                // Continue loop to merge into other stacks if possible
                            }
                        },
                        Err(_) => { /* Merge not possible (e.g., target full), continue loop */ }
                    }
                }
            }
        }
    }

    // 2. If item still has quantity, find first empty slot and place it
    if item_to_move.quantity > 0 {
        let mut empty_slot_found: Option<u8> = None;
        for slot_index in 0..container.num_slots() as u8 {
            if container.get_slot_instance_id(slot_index).is_none() {
                empty_slot_found = Some(slot_index);
                break;
            }
        }

        if let Some(target_slot_index) = empty_slot_found {
            log::info!("[InvManager QuickToContainer Place] Placing remaining {} of item {} into empty slot {}",
                    item_to_move.quantity, item_instance_id, target_slot_index);
            // Now clear original player slot and update item state
            let original_inv_slot = item_to_move.inventory_slot;
            let original_hotbar_slot = item_to_move.hotbar_slot;
            item_to_move.inventory_slot = None;
            item_to_move.hotbar_slot = None;
            inventory_table.instance_id().update(item_to_move.clone());
            // Update container state
            container.set_slot(target_slot_index, Some(item_instance_id), Some(item_def_to_move.id));
            operation_occured = true;
        } else {
            // No empty slot found. If we partially merged, that's okay.
            // If NO operation occurred (no merge, no place), return error.
            if !operation_occured {
                log::warn!("[InvManager QuickToContainer] Failed: No stack to merge onto and no empty slots for item {}", item_instance_id);
                return Err("Container is full".to_string());
            } else {
                 log::info!("[InvManager QuickToContainer] Partially merged item {}, but no empty slot for remainder {}.", item_instance_id, item_to_move.quantity);
                 // Item remains partially in player inventory, that's intended outcome.
            }
        }
    }

    Ok(())
}