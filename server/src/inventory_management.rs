use spacetimedb::{ReducerContext, Identity, Table};
use log;

// Import necessary types and Table Traits
use crate::items::{InventoryItem, ItemDefinition, calculate_merge_result, add_item_to_player_inventory};
use crate::items::{inventory_item as InventoryItemTableTrait, item_definition as ItemDefinitionTableTrait};
// Remove specific container imports
// use crate::wooden_storage_box::{WoodenStorageBox, NUM_BOX_SLOTS};
// use crate::wooden_storage_box::wooden_storage_box as WoodenStorageBoxTableTrait;

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
}

// --- Helper: Check if Container is Empty --- 

/// Checks if all slots in an ItemContainer are empty.
pub(crate) fn is_container_empty<C: ItemContainer>(container: &C) -> bool {
    for i in 0..container.num_slots() as u8 {
        if container.get_slot_instance_id(i).is_some() {
            return false; // Found an item, not empty
        }
    }
    true // Went through all slots, all were empty
}

// --- Container Item Search Helper Interface --- 

/// Trait for clearing an item from a container type.
/// Each container module should implement this trait for its container type.
pub(crate) trait ContainerItemClearer {
    /// Search for and remove the specified item instance from this container type.
    /// Returns true if the item was found and removed.
    fn clear_item(ctx: &ReducerContext, item_instance_id: u64) -> bool;
}

// Note: The clear_item_from_any_container function has been moved to items.rs
// to keep inventory_management.rs container-agnostic.

// --- Core Logic Handlers (Refactored to handle more validation) --- 

/// Handles moving an item from player inventory/hotbar INTO a container slot.
pub(crate) fn handle_move_to_container_slot<C: ItemContainer>(
    ctx: &ReducerContext,
    container: &mut C, 
    target_slot_index: u8,
    item_instance_id: u64,
) -> Result<(), String> {
    // Get tables inside handler
    let inventory_table = ctx.db.inventory_item();
    let item_def_table = ctx.db.item_definition();
    let sender_id = ctx.sender;

    // --- Fetch and Validate Item to Move --- 
    let mut item_to_move = inventory_table.instance_id().find(item_instance_id)
        .ok_or(format!("Item instance {} not found", item_instance_id))?;
    let item_def_to_move = item_def_table.id().find(item_to_move.item_def_id)
        .ok_or(format!("Definition missing for item {}", item_to_move.item_def_id))?;
    
    // --- Determine Original Location --- 
    let original_location_was_equipment = item_to_move.inventory_slot.is_none() && item_to_move.hotbar_slot.is_none();
    if original_location_was_equipment {
        log::debug!("[MoveToContainer] Item {} is potentially coming from an equipment slot.", item_instance_id);
    } else {
        // If it's NOT from equipment, it MUST be from inv/hotbar and owned by the sender.
        if item_to_move.player_identity != sender_id { 
            return Err("Item does not belong to player".to_string()); 
        }
    }

    // --- Validate Target Slot --- 
    if target_slot_index >= container.num_slots() as u8 {
        return Err(format!("Target slot index {} out of bounds.", target_slot_index));
    }
    let target_instance_id_opt = container.get_slot_instance_id(target_slot_index);
    
    // --- Merge/Swap/Place Logic --- 
    if let Some(target_instance_id) = target_instance_id_opt {
        // Target occupied: Merge or Swap
        let mut target_item = inventory_table.instance_id().find(target_instance_id)
                                .ok_or_else(|| format!("Target item instance {} in container slot {} not found!", target_instance_id, target_slot_index))?;

        match calculate_merge_result(&item_to_move, &target_item, &item_def_to_move) {
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

    // --- Clear Original Equipment Slot if Necessary --- 
    if original_location_was_equipment {
        log::info!("[MoveToContainer] Clearing original equipment slot for item {}.", item_instance_id);
        // Call helper using crate path
        crate::items::clear_specific_item_from_equipment_slots(ctx, sender_id, item_instance_id);
    }

    Ok(())
}

/// Handles moving an item FROM a container slot TO the player's inventory.
pub(crate) fn handle_move_from_container_slot<C: ItemContainer>(
    ctx: &ReducerContext, 
    container: &mut C, 
    source_slot_index: u8,
    target_slot_type: String, 
    target_slot_index: u32 // Use u32 to match split args
) -> Result<(), String> {
    let sender_id = ctx.sender;

    // --- Fetch and Validate Item to Move --- 
    if source_slot_index >= container.num_slots() as u8 {
        return Err(format!("Source slot index {} out of bounds.", source_slot_index));
    }
    let source_instance_id = container.get_slot_instance_id(source_slot_index)
        .ok_or_else(|| format!("Source slot {} in container is empty", source_slot_index))?;

    log::info!("[InvManager FromContainer] Attempting move item {} from container slot {} to player {:?} {} slot {}", 
             source_instance_id, source_slot_index, sender_id, target_slot_type, target_slot_index);
    
    // --- Call specific move function from items.rs --- 
    let move_result = match target_slot_type.as_str() {
        "inventory" => {
            if target_slot_index >= 24 { return Err("Invalid inventory target index".to_string()); }
            crate::items::move_item_to_inventory(ctx, source_instance_id, target_slot_index as u16)
        },
        "hotbar" => {
            if target_slot_index >= 6 { return Err("Invalid hotbar target index".to_string()); }
            crate::items::move_item_to_hotbar(ctx, source_instance_id, target_slot_index as u8)
        },
        _ => Err(format!("Invalid target slot type '{}'", target_slot_type)),
    };

    // --- If move successful, clear source slot in container --- 
    if move_result.is_ok() {
        log::debug!("[InvManager FromContainer] Move successful, clearing container slot {}", source_slot_index);
        container.set_slot(source_slot_index, None, None);
    } else {
        log::error!("[InvManager FromContainer] Failed to move item {} to player: {:?}. Container slot {} unchanged.",
                 source_instance_id, move_result.as_ref().err(), source_slot_index);
    }

    move_result // Return the result of the move operation
}

/// Handles moving an item BETWEEN slots within the same container.
pub(crate) fn handle_move_within_container<C: ItemContainer>(
    ctx: &ReducerContext,
    container: &mut C,
    source_slot_index: u8,
    target_slot_index: u8
) -> Result<(), String> {
    // Get tables inside handler
    let inventory_table = ctx.db.inventory_item();
    let item_def_table = ctx.db.item_definition();

    // --- Validate Slots & Fetch Items --- 
    if source_slot_index >= container.num_slots() as u8 
        || target_slot_index >= container.num_slots() as u8 
        || source_slot_index == target_slot_index {
        return Err("Invalid source or target slot index".to_string());
    }
    let source_instance_id = container.get_slot_instance_id(source_slot_index)
        .ok_or(format!("Source slot {} is empty", source_slot_index))?;
    let source_def_id = container.get_slot_def_id(source_slot_index)
        .ok_or("Source definition ID missing")?;
    let mut source_item = inventory_table.instance_id().find(source_instance_id).ok_or("Source item not found")?;
    
    let target_instance_id_opt = container.get_slot_instance_id(target_slot_index);
    let target_def_id_opt = container.get_slot_def_id(target_slot_index);

    // --- Merge/Swap/Move Logic --- 
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
    // NOTE: Source item validation (ownership, location, quantity, stackability) is done in the REDUCER before calling this.
    // This handler assumes the split is valid and just performs the split + placement/merge.
    log::info!("[InvManager SplitToContainer] Splitting {} from item {} into container slot {}", 
             quantity_to_split, source_item.instance_id, target_slot_index);

    // --- Validate Target Slot Index --- 
    if target_slot_index >= container.num_slots() as u8 {
        return Err(format!("Target slot index {} out of bounds.", target_slot_index));
    }

    let inventory_table = ctx.db.inventory_item();
    let item_def_table = ctx.db.item_definition();

    // 1. Perform split using helper from items.rs
    // This updates source_item quantity and creates a new item instance.
    let new_item_instance_id = crate::items::split_stack_helper(ctx, source_item, quantity_to_split)?;
    let new_item_def_id = source_item.item_def_id; // Get def_id from potentially updated source_item
    // Find the newly created item (needed for merging)
    let mut new_item = inventory_table.instance_id().find(new_item_instance_id)
                       .ok_or("Failed to find newly split item instance")?;
    let new_item_def = item_def_table.id().find(new_item_def_id)
                        .ok_or("Failed to find definition for new item")?;

    // 2. Check if target slot is occupied
    if let Some(target_instance_id) = container.get_slot_instance_id(target_slot_index) {
        // --- Target Occupied: Attempt Merge --- 
        log::debug!("[InvManager SplitToContainer] Target slot {} occupied by {}, attempting merge.", target_slot_index, target_instance_id);
        let mut target_item = inventory_table.instance_id().find(target_instance_id)
                            .ok_or_else(|| format!("Target item {} in container slot {} not found!", target_instance_id, target_slot_index))?;

        match calculate_merge_result(&new_item, &target_item, &new_item_def) {
            Ok((_, _source_new_qty, target_new_qty, delete_source)) => {
                // Merge successful
                log::info!("[InvManager SplitToContainer Merge] Merging new item {} onto target {}. Target new qty: {}", 
                         new_item_instance_id, target_instance_id, target_new_qty);
                target_item.quantity = target_new_qty;
                inventory_table.instance_id().update(target_item);
                if delete_source { 
                    // The new item was fully merged, delete it
                    inventory_table.instance_id().delete(new_item_instance_id);
                    log::debug!("[InvManager SplitToContainer Merge] New item {} deleted after merge.", new_item_instance_id);
                } else {
                    // Should not happen if merging the *entire* new stack, but handle defensively
                    log::warn!("[InvManager SplitToContainer Merge] New item {} not deleted after merge? New Qty: {}", 
                             new_item_instance_id, _source_new_qty); 
                    // Update the container slot anyway, overwriting the old target
                    container.set_slot(target_slot_index, Some(new_item_instance_id), Some(new_item_def_id));
                }
                // Container state for the target slot doesn't change if merge succeeded on existing item
            },
            Err(e) => {
                // Merge Failed (different types, target full, etc.) - Cannot place split item here.
                // Revert the split by giving quantity back? No, helper already updated source.
                // We must delete the newly created item and return error.
                log::warn!("[InvManager SplitToContainer Merge Failed] Cannot merge split item {} onto target {}: {}. Deleting split item.",
                         new_item_instance_id, target_instance_id, e);
                inventory_table.instance_id().delete(new_item_instance_id);
                return Err(format!("Cannot merge split stack onto item in slot {}: {}", target_slot_index, e));
            }
        }
    } else {
        // --- Target Empty: Place --- 
        log::debug!("[InvManager SplitToContainer] Target slot {} empty. Placing new item {}.", target_slot_index, new_item_instance_id);
        // Update the container struct state with the NEW item using trait method
        container.set_slot(target_slot_index, Some(new_item_instance_id), Some(new_item_def_id));
    }

    Ok(())
}

/// Handles splitting a stack FROM a container slot TO player inventory/hotbar.
pub(crate) fn handle_split_from_container<C: ItemContainer>(
    ctx: &ReducerContext,
    container: &mut C, 
    source_slot_index: u8,
    quantity_to_split: u32,
    target_slot_type: String, 
    target_slot_index: u32
) -> Result<(), String> {
    // Get tables inside handler
    let inventory_table = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition(); // Needed for stackability check
    let sender_id = ctx.sender;

    // --- Fetch and Validate Source Item --- 
    if source_slot_index >= container.num_slots() as u8 {
        return Err(format!("Source slot index {} out of bounds.", source_slot_index));
    }
     let source_instance_id = container.get_slot_instance_id(source_slot_index)
        .ok_or(format!("Source slot {} is empty", source_slot_index))?;
    let source_def_id = container.get_slot_def_id(source_slot_index)
        .ok_or("Missing definition ID in source slot")?;
    let mut source_item = inventory_table.instance_id().find(source_instance_id)
        .ok_or("Source item instance not found")?;
    if quantity_to_split == 0 || quantity_to_split >= source_item.quantity {
        return Err("Invalid split quantity".to_string());
    }
    let item_def = item_defs.id().find(source_def_id).ok_or("Item definition not found")?;
    if !item_def.is_stackable { return Err("Source item is not stackable".to_string()); }

    // --- Validate Target --- 
    let target_is_inventory = match target_slot_type.as_str() {
        "inventory" => true,
        "hotbar" => false,
        _ => return Err("Invalid target_slot_type".to_string()),
    };
    if target_is_inventory && target_slot_index >= 24 { return Err("Invalid inventory target index".to_string()); }
    if !target_is_inventory && target_slot_index >= 6 { return Err("Invalid hotbar target index".to_string()); }

    log::info!("[InvManager SplitFromContainer] Splitting {} from container slot {} to player {} slot {}",
             quantity_to_split, source_slot_index, target_slot_type, target_slot_index);

    // 1. Perform split using helper
    let new_item_instance_id = crate::items::split_stack_helper(ctx, &mut source_item, quantity_to_split)?;

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
    // container.set_slot(source_slot_index, None, None); // REMOVED: This was incorrect, split_stack_helper updates original item qty.
    Ok(())
}

/// Handles splitting a stack BETWEEN two slots within the same container.
pub(crate) fn handle_split_within_container<C: ItemContainer>(
    ctx: &ReducerContext,
    container: &mut C,
    source_slot_index: u8,
    target_slot_index: u8,
    quantity_to_split: u32
) -> Result<(), String> {
    // Get tables inside handler
    let inventory_table = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();

     log::info!("[InvManager SplitWithinContainer] Splitting {} from slot {} to slot {} within container",
             quantity_to_split, source_slot_index, target_slot_index);

    // --- Fetch and Validate Source & Target --- 
    if source_slot_index >= container.num_slots() as u8 
        || target_slot_index >= container.num_slots() as u8 
        || source_slot_index == target_slot_index {
        return Err("Invalid source or target slot index".to_string());
    }
    let source_instance_id = container.get_slot_instance_id(source_slot_index)
        .ok_or(format!("Source slot {} is empty", source_slot_index))?;
    let mut source_item = inventory_table.instance_id().find(source_instance_id)
        .ok_or("Source item instance not found")?;
    if quantity_to_split == 0 || quantity_to_split >= source_item.quantity {
        return Err("Invalid split quantity".to_string());
    }
    let item_def = item_defs.id().find(source_item.item_def_id).ok_or("Item definition not found")?;
    if !item_def.is_stackable { return Err("Source item is not stackable".to_string()); }

    // --- Perform Split --- 
    let new_item_instance_id = crate::items::split_stack_helper(ctx, &mut source_item, quantity_to_split)?;
    let new_item_def_id = source_item.item_def_id;
    // Find the newly created item (needed for merging)
    let mut new_item = inventory_table.instance_id().find(new_item_instance_id)
                       .ok_or("Failed to find newly split item instance")?;
    let new_item_def = item_defs.id().find(new_item_def_id)
                        .ok_or("Failed to find definition for new item")?;

    // --- Place New Stack or Merge --- 
    if let Some(target_instance_id) = container.get_slot_instance_id(target_slot_index) {
        // --- Target Occupied: Attempt Merge --- 
        log::debug!("[InvManager SplitWithinContainer] Target slot {} occupied by {}, attempting merge.", target_slot_index, target_instance_id);
        let mut target_item = inventory_table.instance_id().find(target_instance_id)
                            .ok_or_else(|| format!("Target item {} in container slot {} not found!", target_instance_id, target_slot_index))?;

        match calculate_merge_result(&new_item, &target_item, &new_item_def) {
            Ok((_, _source_new_qty, target_new_qty, delete_source)) => {
                // Merge successful
                log::info!("[InvManager SplitWithinContainer Merge] Merging new item {} onto target {}. Target new qty: {}", 
                         new_item_instance_id, target_instance_id, target_new_qty);
                target_item.quantity = target_new_qty;
                inventory_table.instance_id().update(target_item);
                if delete_source { 
                    inventory_table.instance_id().delete(new_item_instance_id);
                    log::debug!("[InvManager SplitWithinContainer Merge] New item {} deleted after merge.", new_item_instance_id);
                } else {
                     log::warn!("[InvManager SplitWithinContainer Merge] New item {} not deleted after merge? New Qty: {}", 
                             new_item_instance_id, _source_new_qty); 
                    // Overwrite target slot if merge didn't delete source (unexpected)
                     container.set_slot(target_slot_index, Some(new_item_instance_id), Some(new_item_def_id));
                }
            },
            Err(e) => {
                 // Merge Failed - Error out, delete the split stack
                log::warn!("[InvManager SplitWithinContainer Merge Failed] Cannot merge split item {} onto target {}: {}. Deleting split item.",
                         new_item_instance_id, target_instance_id, e);
                inventory_table.instance_id().delete(new_item_instance_id);
                return Err(format!("Cannot merge split stack onto item in slot {}: {}", target_slot_index, e));
            }
        }

    } else {
        // --- Target Empty: Place --- 
        log::debug!("[InvManager SplitWithinContainer] Target slot {} empty. Placing new item {}.", target_slot_index, new_item_instance_id);
        container.set_slot(target_slot_index, Some(new_item_instance_id), Some(new_item_def_id));
    }

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
    let inventory_table = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition(); // Needed for stacking check
    let sender_id = ctx.sender;

    // Get item info using trait methods
    let source_instance_id = container.get_slot_instance_id(source_slot_index)
        .ok_or_else(|| format!("Source slot {} in container is empty", source_slot_index))?;
    let source_def_id = container.get_slot_def_id(source_slot_index)
        .ok_or_else(|| format!("Missing definition ID in source slot {}", source_slot_index))?;
    
    // Fetch the item to move
    let mut item_to_move = inventory_table.instance_id().find(source_instance_id)
        .ok_or("Item instance in container slot not found in inventory table")?;
    let item_def = item_defs.id().find(source_def_id)
        .ok_or("Item definition not found")?;

    log::info!("[InvManager QuickFromContainer] Moving item {} (Def {}) from container slot {} to player {:?} inventory", 
             source_instance_id, source_def_id, source_slot_index, sender_id);

    // --- Logic to add/merge item into player inventory --- 
    let mut remaining_quantity = item_to_move.quantity;
    let mut item_deleted_from_container = false;

    // 1. Try merging onto existing stacks (Hotbar first, then Inventory)
    if item_def.is_stackable {
        let mut items_to_update: Vec<InventoryItem> = Vec::new();
        // Hotbar merge attempt
        for mut target_item in inventory_table.iter().filter(|i| i.player_identity == sender_id && i.item_def_id == source_def_id && i.hotbar_slot.is_some()) {
            let space_available = item_def.stack_size.saturating_sub(target_item.quantity);
            if space_available > 0 {
                let transfer_qty = std::cmp::min(remaining_quantity, space_available);
                target_item.quantity += transfer_qty;
                remaining_quantity -= transfer_qty;
                items_to_update.push(target_item); // Stage update
                if remaining_quantity == 0 { break; }
            }
        }
        // Inventory merge attempt
        if remaining_quantity > 0 {
            for mut target_item in inventory_table.iter().filter(|i| i.player_identity == sender_id && i.item_def_id == source_def_id && i.inventory_slot.is_some()) {
                 let space_available = item_def.stack_size.saturating_sub(target_item.quantity);
                 if space_available > 0 {
                    let transfer_qty = std::cmp::min(remaining_quantity, space_available);
                    target_item.quantity += transfer_qty;
                    remaining_quantity -= transfer_qty;
                    items_to_update.push(target_item); // Stage update
                    if remaining_quantity == 0 { break; }
                }
            }
        }
        // Apply merged updates
        for updated_item in items_to_update {
             inventory_table.instance_id().update(updated_item);
        }
    }

    // 2. If quantity remains, find empty slot (Hotbar first, then Inventory)
    if remaining_quantity > 0 {
        let target_slot: Option<(String, u32)> = find_first_empty_player_slot(ctx, sender_id);

        if let Some((slot_type, slot_index)) = target_slot {
            // Assign the *original item* to the empty slot
            item_to_move.player_identity = sender_id; // Ensure ownership
            item_to_move.quantity = remaining_quantity; // Update quantity if partially merged
            if slot_type == "hotbar" {
                item_to_move.hotbar_slot = Some(slot_index as u8);
                item_to_move.inventory_slot = None;
            } else {
                item_to_move.hotbar_slot = None;
                item_to_move.inventory_slot = Some(slot_index as u16);
            }
            inventory_table.instance_id().update(item_to_move);
            log::info!("[InvManager QuickFromContainer] Placed item {} (Qty {}) into {} slot {}", source_instance_id, remaining_quantity, slot_type, slot_index);
            item_deleted_from_container = true; // The item instance is now fully owned by the player
        } else {
             log::warn!("[InvManager QuickFromContainer] Inventory full for player {:?}. Could not place remaining {} of item {}. Item remains in container.", 
                      sender_id, remaining_quantity, source_instance_id);
            return Err("Inventory is full".to_string());
        }
    } else {
        // Item fully merged, delete the original instance
        log::info!("[InvManager QuickFromContainer] Item {} fully merged. Deleting instance.", source_instance_id);
        inventory_table.instance_id().delete(source_instance_id);
        item_deleted_from_container = true;
    }

    // --- If item was successfully moved/merged/deleted, clear container slot --- 
    if item_deleted_from_container {
        container.set_slot(source_slot_index, None, None);
    }
    
    Ok(()) 
}

// Helper to find the first available slot (hotbar preferred)
pub(crate) fn find_first_empty_player_slot(ctx: &ReducerContext, player_id: Identity) -> Option<(String, u32)> {
    let inventory = ctx.db.inventory_item();
    // Check Hotbar (0-5)
    let occupied_hotbar: std::collections::HashSet<u8> = inventory.iter()
        .filter(|i| i.player_identity == player_id && i.hotbar_slot.is_some())
        .map(|i| i.hotbar_slot.unwrap())
        .collect();
    if let Some(empty_slot) = (0..6).find(|slot| !occupied_hotbar.contains(slot)) {
        return Some(("hotbar".to_string(), empty_slot as u32));
    }
    // Check Inventory (0-23)
    let occupied_inventory: std::collections::HashSet<u16> = inventory.iter()
        .filter(|i| i.player_identity == player_id && i.inventory_slot.is_some())
        .map(|i| i.inventory_slot.unwrap())
        .collect();
    if let Some(empty_slot) = (0..24).find(|slot| !occupied_inventory.contains(slot)) {
        return Some(("inventory".to_string(), empty_slot as u32));
    }
    None // No empty slots found
}

/// Handles quickly moving an item FROM the player inventory/hotbar INTO the first
/// available/mergeable slot in the container.
pub(crate) fn handle_quick_move_to_container<C: ItemContainer>(
    ctx: &ReducerContext,
    container: &mut C,
    item_instance_id: u64,
) -> Result<(), String> {
    // Get tables
    let inventory_table = ctx.db.inventory_item();
    let item_def_table = ctx.db.item_definition();
    let sender_id = ctx.sender;
    
    // --- Fetch and Validate Item --- 
    let mut item_to_move = inventory_table.instance_id().find(item_instance_id)
        .ok_or(format!("Item instance {} not found", item_instance_id))?;
    let item_def_to_move = item_def_table.id().find(item_to_move.item_def_id)
        .ok_or(format!("Definition missing for item {}", item_to_move.item_def_id))?;
    
    // --- Determine Original Location --- 
    let original_location_was_equipment = item_to_move.inventory_slot.is_none() && item_to_move.hotbar_slot.is_none();
    if original_location_was_equipment {
        log::debug!("[MoveToContainer] Item {} is potentially coming from an equipment slot.", item_instance_id);
    }

    let mut operation_occured = false; 

    // 1. Attempt to merge with existing stacks
    if item_def_to_move.is_stackable {
        for slot_index in 0..container.num_slots() as u8 {
            if let Some(target_instance_id) = container.get_slot_instance_id(slot_index) {
                if container.get_slot_def_id(slot_index) == Some(item_def_to_move.id) { // Check if same item type
                    let mut target_item = inventory_table.instance_id().find(target_instance_id)
                                            .ok_or_else(|| format!("Target item {} in slot {} missing!", target_instance_id, slot_index))?;
                    
                    match calculate_merge_result(&item_to_move, &target_item, &item_def_to_move) {
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

    // --- Clear Original Equipment Slot if Necessary --- 
    if original_location_was_equipment {
        log::info!("[MoveToContainer] Clearing original equipment slot for item {}.", item_instance_id);
        // Call helper using crate path
        crate::items::clear_specific_item_from_equipment_slots(ctx, sender_id, item_instance_id);
    }

    Ok(())
}