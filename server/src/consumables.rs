// server/src/consumables.rs
use spacetimedb::{ReducerContext, Identity, Table};
use log;

// Import table traits needed for ctx.db access
use crate::player as PlayerTableTrait;
use crate::items::{InventoryItem, inventory_item as InventoryItemTableTrait};
use crate::items::{ItemDefinition, item_definition as ItemDefinitionTableTrait};
use crate::items::ItemCategory;
use crate::player_stats::player_stats;
// Import the enum itself

// --- Consumable Effect Constants ---
const MUSHROOM_HEALTH_GAIN: f32 = 5.0;
const MUSHROOM_HUNGER_GAIN: f32 = 10.0;
const MUSHROOM_THIRST_GAIN: f32 = 5.0;
const MAX_STAT_VALUE: f32 = 100.0; // Max value for health, hunger, thirst

#[spacetimedb::reducer]
pub fn consume_item(ctx: &ReducerContext, item_instance_id: u64) -> Result<(), String> {
    let sender_id = ctx.sender;
    let inventory = ctx.db.inventory_item();
    let item_defs = ctx.db.item_definition();
    let players_stats = ctx.db.player_stats();

    log::info!("[ConsumeItem] Player {:?} attempting to consume item instance {}", sender_id, item_instance_id);

    // 1. Get the InventoryItem being consumed
    let mut item_to_consume = inventory.instance_id().find(item_instance_id)
        .ok_or_else(|| format!("Item instance {} not found.", item_instance_id))?;

    // 2. Verify ownership
    if item_to_consume.player_identity != sender_id {
        return Err("Cannot consume an item that does not belong to you.".to_string());
    }

    // 3. Get its ItemDefinition
    let item_def = item_defs.id().find(item_to_consume.item_def_id)
        .ok_or_else(|| format!("Definition not found for item ID {}", item_to_consume.item_def_id))?;

    // 4. Validate: Must be Consumable category
    if item_def.category != ItemCategory::Consumable {
        return Err(format!("Item '{}' is not consumable.", item_def.name));
    }

    // 5. Find the player stats to apply effects to
    let mut player_stat = players_stats.player_id().find(sender_id)
        .ok_or_else(|| "Player not found to apply consumable effects.".to_string())?;

    // 6. Apply Effects (Specific to Mushroom for now)
    // TODO: Refactor this to use data from ItemDefinition if more consumables are added
    let mut stat_changed = false;
    if item_def.name == "Mushroom" {
        let old_health = player_stat.health;

        player_stat.health = (player_stat.health + MUSHROOM_HEALTH_GAIN).min(MAX_STAT_VALUE);

        stat_changed = true; // Assume stats changed if it's a mushroom

        log::info!(
            "[ConsumeItem] Player {:?} consumed {}. Stats: H {:.1}->{:.1}",
            sender_id, item_def.name, 
            old_health, player_stat.health,
        );

    } else {
        log::warn!("[ConsumeItem] Consumed item '{}' has no defined effect.", item_def.name);
        // Return Ok even if no effect, item is still consumed
    }

    // 7. Decrease quantity or delete item stack
    item_to_consume.quantity -= 1;
    if item_to_consume.quantity == 0 {
        log::debug!("[ConsumeItem] Item instance {} stack depleted, deleting.", item_instance_id);
        inventory.instance_id().delete(item_instance_id);
    } else {
        log::debug!("[ConsumeItem] Item instance {} quantity reduced to {}.", item_instance_id, item_to_consume.quantity);
        inventory.instance_id().update(item_to_consume);
    }

    // 8. Update Player state only if stats changed
    if stat_changed {
         players_stats.player_id().update(player_stat);
    }

    Ok(())
} 