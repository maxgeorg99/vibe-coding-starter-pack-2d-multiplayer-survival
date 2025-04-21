use spacetimedb::{Table, ReducerContext, Identity, Timestamp};
// Add imports for required table traits
use crate::items::{inventory_item as InventoryItemTableTrait, item_definition as ItemDefinitionTableTrait};
use crate::player as PlayerTableTrait; // Assuming player table is defined in lib.rs
use log;

// Import the respawn duration constant
use crate::active_equipment::RESOURCE_RESPAWN_DURATION_SECS;
use std::time::Duration;

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
    pub respawn_at: Option<Timestamp>,
}

// --- Interaction Reducer ---

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

    // 5. Add Mushroom to Inventory (using helper from items module)
    crate::items::add_item_to_player_inventory(ctx, sender_id, mushroom_def.id, 1)?;

    // 6. Schedule Respawn instead of Deleting
    let respawn_time = ctx.timestamp + Duration::from_secs(RESOURCE_RESPAWN_DURATION_SECS);
    let mut mushroom_to_update = mushroom; // Clone the found mushroom to modify
    mushroom_to_update.respawn_at = Some(respawn_time);
    mushrooms.id().update(mushroom_to_update); // Update with respawn time
    log::info!("Player {:?} picked up mushroom {}. Scheduling respawn.", sender_id, mushroom_id);

    Ok(())
} 