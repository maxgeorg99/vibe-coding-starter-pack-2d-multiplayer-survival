use spacetimedb::{ Identity, ReducerContext, Table, Timestamp };
use log;
use std::time::Duration;

// Import specific constants directly from their modules
use crate::tree::{TREE_COLLISION_Y_OFFSET, PLAYER_TREE_COLLISION_DISTANCE_SQUARED};
use crate::stone::{STONE_COLLISION_Y_OFFSET, PLAYER_STONE_COLLISION_DISTANCE_SQUARED};

// Import table traits needed for ctx.db access
use crate::tree::tree as TreeTableTrait;
use crate::stone::stone as StoneTableTrait;
use crate::items::item_definition as ItemDefinitionTableTrait;
use crate::items::inventory_item as InventoryItemTableTrait;
use crate::player as PlayerTableTrait;
use crate::active_equipment as ActiveEquipmentTableTrait;

// Import structs used
// use crate::environment::Tree; // Remove - Not used directly here
// use crate::items::ItemDefinition; // Remove - Not used directly here
// use crate::{Player, PLAYER_RADIUS}; // Remove - Not used directly here
use crate::PLAYER_RADIUS; // Add back the import for PLAYER_RADIUS
use std::f32::consts::PI;
use crate::items::{InventoryItem, ItemDefinition, ItemCategory, EquipmentSlot};
use crate::Player; // Corrected import path

// --- Constants ---
pub(crate) const RESPAWN_TIME_MS: u64 = 5000; // 5 seconds respawn time
const PVP_DAMAGE_MULTIPLIER: f32 = 6.0;
pub(crate) const RESOURCE_RESPAWN_DURATION_SECS: u64 = 300; // 5 minutes respawn time for trees/stones

const PLAYER_INTERACT_DISTANCE: f32 = 80.0;
const PLAYER_INTERACT_DISTANCE_SQUARED: f32 = PLAYER_INTERACT_DISTANCE * PLAYER_INTERACT_DISTANCE;

#[spacetimedb::table(name = active_equipment, public)]
#[derive(Clone, Default, Debug)]
pub struct ActiveEquipment {
    #[primary_key]
    pub player_identity: Identity,
    pub equipped_item_def_id: Option<u64>, // ID from ItemDefinition table
    pub equipped_item_instance_id: Option<u64>, // Instance ID from InventoryItem
    pub swing_start_time_ms: u64, // Timestamp (ms) when the current swing started, 0 if not swinging
    // Fields for worn armor
    pub head_item_instance_id: Option<u64>,
    pub chest_item_instance_id: Option<u64>,
    pub legs_item_instance_id: Option<u64>,
    pub feet_item_instance_id: Option<u64>,
    pub hands_item_instance_id: Option<u64>,
    pub back_item_instance_id: Option<u64>,
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

    // --- Get existing equipment or create default ---
    let mut equipment = get_or_create_active_equipment(ctx, sender_id)?;

    // Check if item is actually equippable using the field from ItemDefinition
    if !item_def.is_equippable || item_def.category == ItemCategory::Armor {
        // If not equippable OR if it's armor (handled by equip_armor), clear the main hand slot.
        log::info!("Player {:?} selected non-tool/weapon item {} or armor {}. Clearing main hand.", sender_id, item_def.name, item_instance_id);
        equipment.equipped_item_def_id = None;
        equipment.equipped_item_instance_id = None;
        equipment.swing_start_time_ms = 0;
        active_equipments.player_identity().update(equipment);
        return Ok(());
    }

    // --- Update the main hand equipment entry ---
    // Only update the fields related to the main hand item. Armor slots remain untouched.
    equipment.equipped_item_def_id = Some(item_def.id);
    equipment.equipped_item_instance_id = Some(item_instance_id);
    equipment.swing_start_time_ms = 0; // Reset swing state when equipping

    active_equipments.player_identity().update(equipment); // Update the existing row
    log::info!("Player {:?} equipped item: {} (Instance ID: {}) to main hand.", sender_id, item_def.name, item_instance_id);

    // --- REMOVED: Logic to insert inventory item, as equipping shouldn't create duplicates ---
    // ctx.db.inventory_item().insert(crate::items::InventoryItem { ... });

    Ok(())
}

// Reducer to explicitly unequip whatever item is active in the main hand
#[spacetimedb::reducer]
pub fn unequip_item(ctx: &ReducerContext) -> Result<(), String> {
    let sender_id = ctx.sender;
    let active_equipments = ctx.db.active_equipment();
    let inventory_items = ctx.db.inventory_item();

    if let Some(mut equipment) = active_equipments.player_identity().find(sender_id) {
        // Only clear the main hand fields. Leave armor slots untouched.
        if equipment.equipped_item_instance_id.is_some() {
             log::info!("Player {:?} explicitly unequipped main hand item.", sender_id);
             equipment.equipped_item_def_id = None;
             equipment.equipped_item_instance_id = None;
             equipment.swing_start_time_ms = 0;
             active_equipments.player_identity().update(equipment);
        }
    } else {
        log::info!("Player {:?} tried to unequip, but no ActiveEquipment row found.", sender_id);
        // No row exists, so nothing to unequip. Not an error.
    }
    Ok(())
}

// Reducer to trigger the 'use' action (swing) of the equipped item
#[spacetimedb::reducer]
pub fn use_equipped_item(ctx: &ReducerContext) -> Result<(), String> {
    let sender_id = ctx.sender;
    let now_ts = ctx.timestamp;
    let now_micros = now_ts.to_micros_since_unix_epoch();
    let now_ms = (now_micros / 1000) as u64;

    // Get tables
    let active_equipments = ctx.db.active_equipment();
    let players = ctx.db.player();
    let item_defs = ctx.db.item_definition();
    let trees = ctx.db.tree();
    let stones = ctx.db.stone(); // Get stones table
    let inventory_items = ctx.db.inventory_item(); // Get inventory table

    // --- Get Player and Equipment Info ---
    let player = players.identity().find(sender_id)
        .ok_or_else(|| "Player not found".to_string())?;
    let mut current_equipment = active_equipments.player_identity().find(sender_id)
        .ok_or_else(|| "No active equipment record found.".to_string())?;

    let item_def_id = current_equipment.equipped_item_def_id
        .ok_or_else(|| "No item equipped to use.".to_string())?;
    let item_def = item_defs.id().find(item_def_id)
        .ok_or_else(|| "Equipped item definition not found".to_string())?;

    // --- Update Swing Time ---
    // TODO: Add cooldown check?
    current_equipment.swing_start_time_ms = now_ms;
    active_equipments.player_identity().update(current_equipment.clone()); // Update swing time regardless of hitting anything
    log::debug!("Player {:?} started using item '{}' (ID: {})",
             sender_id, item_def.name, item_def_id);

    // --- Get Item Damage ---
    let item_damage = match item_def.damage {
        Some(dmg) if dmg > 0 => dmg,
        _ => return Ok(()), // Item has no damage, nothing more to do
    };

    // --- Attack Logic ---
    let attack_range = PLAYER_RADIUS * 4.0; // Increased range further
    let attack_angle_degrees = 90.0; // Widen attack arc to 90 degrees
    let attack_angle_rad = attack_angle_degrees * PI / 180.0;
    let half_attack_angle_rad = attack_angle_rad / 2.0;

    // Calculate player's forward vector based on direction
    let (forward_x, forward_y) = match player.direction.as_str() {
        "up" => (0.0, -1.0),
        "down" => (0.0, 1.0),
        "left" => (-1.0, 0.0),
        "right" => (1.0, 0.0),
        _ => (0.0, 1.0), // Default to down
    };

    let mut closest_tree_target: Option<(u64, f32)> = None; // (tree_id: u64, distance_sq)
    let mut closest_stone_target: Option<(u64, f32)> = None; // (stone_id: u64, distance_sq)
    let mut closest_player_target: Option<(Identity, f32)> = None; // (player_id, distance_sq)

    // Find closest Tree target
    for tree in trees.iter() {
        let dx = tree.pos_x - player.position_x;
        // Target the tree's defined collision Y coordinate
        let target_y = tree.pos_y - TREE_COLLISION_Y_OFFSET;
        let dy = target_y - player.position_y; 
        let dist_sq = dx * dx + dy * dy;

        if dist_sq < (attack_range * attack_range) && dist_sq > 0.0 {
            let distance = dist_sq.sqrt();
            let target_vec_x = dx / distance;
            let target_vec_y = dy / distance;

            // Calculate angle between player forward and target vector
            let dot_product: f32 = forward_x * target_vec_x + forward_y * target_vec_y;
            let angle_rad = dot_product.acos(); // Angle in radians

            if angle_rad <= half_attack_angle_rad {
                // Target is within range and angle
                if closest_tree_target.is_none() || dist_sq < closest_tree_target.unwrap().1 {
                    closest_tree_target = Some((tree.id, dist_sq));
                }
            }
        }
    }

    // Find closest Stone target
    for stone in stones.iter() {
        let dx = stone.pos_x - player.position_x;
        let target_y = stone.pos_y - STONE_COLLISION_Y_OFFSET;
        let dy = target_y - player.position_y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq < (attack_range * attack_range) && dist_sq > 0.0 {
            let distance = dist_sq.sqrt();
            let target_vec_x = dx / distance;
            let target_vec_y = dy / distance;
            let dot_product: f32 = forward_x * target_vec_x + forward_y * target_vec_y;
            let angle_rad = dot_product.acos();

            if angle_rad <= half_attack_angle_rad {
                if closest_stone_target.is_none() || dist_sq < closest_stone_target.unwrap().1 {
                    closest_stone_target = Some((stone.id, dist_sq));
                }
            }
        }
    }

    // Find closest Player target (excluding self)
    for other_player in players.iter() {
        if other_player.identity == sender_id { continue; } // Don't target self
        if other_player.is_dead { continue; } // Don't target dead players

        let dx = other_player.position_x - player.position_x;
        let dy = other_player.position_y - player.position_y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq < (attack_range * attack_range) && dist_sq > 0.0 {
            let distance = dist_sq.sqrt();
            let target_vec_x = dx / distance;
            let target_vec_y = dy / distance;
            let dot_product: f32 = forward_x * target_vec_x + forward_y * target_vec_y;
            let angle_rad = dot_product.acos();

            if angle_rad <= half_attack_angle_rad {
                if closest_player_target.is_none() || dist_sq < closest_player_target.unwrap().1 {
                    closest_player_target = Some((other_player.identity, dist_sq));
                }
            }
        }
    }

    // --- Apply Damage based on Tool Type and Target Priority ---
    let tool_name = item_def.name.as_str();
    let mut hit_something = false;

    if tool_name == "Stone Pickaxe" {
        // Pickaxe: Prioritize Stones > Players
        if let Some((stone_id, _)) = closest_stone_target {
            // --- Damage Stone ---
            let mut stone = stones.id().find(stone_id).ok_or("Target stone disappeared?")?;
            let old_health = stone.health;
            stone.health = stone.health.saturating_sub(item_damage);
            stone.last_hit_time = Some(now_ts); // Set last hit time for shake effect
            log::info!("Player {:?} hit Stone {} with {} for {} damage. Health: {} -> {}",
                    sender_id, stone_id, item_def.name, item_damage, old_health, stone.health);

            // --- Grant Stone Item --- 
            let stone_def_opt = item_defs.iter().find(|def| def.name == "Stone");
            if let Some(stone_def) = stone_def_opt {
                let stone_to_grant = item_damage as u32; 
                match crate::items::add_item_to_player_inventory(ctx, sender_id, stone_def.id, stone_to_grant) {
                    Ok(_) => log::debug!("Granted {} Stone to player {:?} via helper.", stone_to_grant, sender_id),
                    Err(e) => log::error!("Failed to grant Stone to player {:?}: {}", sender_id, e),
                }
            } else {
                log::error!("Stone item definition not found when granting stone.");
            }
            // --- End Grant Stone Item ---

            if stone.health == 0 {
                log::info!("Stone {} depleted by Player {:?}. Scheduling respawn.", stone_id, sender_id);
                let respawn_time = now_ts + Duration::from_secs(RESOURCE_RESPAWN_DURATION_SECS).into();
                stone.respawn_at = Some(respawn_time);
                stones.id().update(stone); // Update with health 0 and respawn time
                // stones.id().delete(stone_id); // Removed delete
            } else {
                stones.id().update(stone);
            }
            hit_something = true;

        } else if let Some((target_player_id, _)) = closest_player_target {
            // --- Damage Player ---
            let mut target_player = players.identity().find(target_player_id)
                .ok_or("Target player disappeared?")?;
            let old_health = target_player.health;
            // Apply PvP multiplier
            let actual_damage = (item_damage as f32 * PVP_DAMAGE_MULTIPLIER).max(0.0);
            target_player.health = (target_player.health - actual_damage).max(0.0);
            target_player.last_hit_time = Some(now_ts); // <-- Set last hit time
            log::info!("Player {:?} hit Player {:?} with {} for {:.1} ({} base * {}x) damage. Health: {:.1} -> {:.1}",
                     sender_id, target_player_id, item_def.name, actual_damage, item_damage, PVP_DAMAGE_MULTIPLIER, old_health, target_player.health);

            // Check for death
            if target_player.health <= 0.0 && !target_player.is_dead {
                target_player.is_dead = true;
                let respawn_micros = now_micros.saturating_add((RESPAWN_TIME_MS * 1000) as i64);
                target_player.respawn_at = Timestamp::from_micros_since_unix_epoch(respawn_micros);
                log::info!("Player {:?} killed Player {:?}. Respawn at {:?}", sender_id, target_player_id, target_player.respawn_at);
                // TODO: Drop items? Clear equipment?
            }

            players.identity().update(target_player);
            hit_something = true;
        }

    } else if tool_name == "Stone Hatchet" {
        // Hatchet: Prioritize Trees > Players
        if let Some((tree_id, _)) = closest_tree_target {
            // --- Damage Tree & Grant Wood ---
            let mut tree = trees.id().find(tree_id).ok_or("Target tree disappeared?")?;
            let old_health = tree.health;
            tree.health = tree.health.saturating_sub(item_damage);
            tree.last_hit_time = Some(now_ts);
            log::info!("Player {:?} hit Tree {} with {} for {} damage. Health: {} -> {}",
                     sender_id, tree_id, item_def.name, item_damage, old_health, tree.health);

            // --- Grant Wood Item ---
            let wood_def_opt = item_defs.iter().find(|def| def.name == "Wood");
            if let Some(wood_def) = wood_def_opt {
                let wood_to_grant = item_damage as u32; 
                match crate::items::add_item_to_player_inventory(ctx, sender_id, wood_def.id, wood_to_grant) {
                    Ok(_) => log::debug!("Granted {} Wood to player {:?} via helper.", wood_to_grant, sender_id),
                    Err(e) => log::error!("Failed to grant Wood to player {:?}: {}", sender_id, e),
                }
            } else {
                log::error!("Wood item definition not found when granting wood.");
            }
            // --- End Grant Wood Item ---
            
            if tree.health == 0 {
                log::info!("Tree {} destroyed by Player {:?}. Scheduling respawn.", tree_id, sender_id);
                let respawn_time = now_ts + Duration::from_secs(RESOURCE_RESPAWN_DURATION_SECS).into();
                tree.respawn_at = Some(respawn_time);
                trees.id().update(tree); // Update with health 0 and respawn time
                // trees.id().delete(tree_id); // REMOVED delete
            } else {
                trees.id().update(tree);
            }
            hit_something = true;

        } else if let Some((target_player_id, _)) = closest_player_target {
            // --- Damage Player ---
            let mut target_player = players.identity().find(target_player_id)
                .ok_or("Target player disappeared?")?;
            let old_health = target_player.health;
            // Apply PvP multiplier
            let actual_damage = (item_damage as f32 * PVP_DAMAGE_MULTIPLIER).max(0.0);
            target_player.health = (target_player.health - actual_damage).max(0.0);
            target_player.last_hit_time = Some(now_ts); // <-- Set last hit time
            log::info!("Player {:?} hit Player {:?} with {} for {:.1} ({} base * {}x) damage. Health: {:.1} -> {:.1}",
                     sender_id, target_player_id, item_def.name, actual_damage, item_damage, PVP_DAMAGE_MULTIPLIER, old_health, target_player.health);

            // Check for death
            if target_player.health <= 0.0 && !target_player.is_dead {
                target_player.is_dead = true;
                let respawn_micros = now_micros.saturating_add((RESPAWN_TIME_MS * 1000) as i64);
                target_player.respawn_at = Timestamp::from_micros_since_unix_epoch(respawn_micros);
                log::info!("Player {:?} killed Player {:?}. Respawn at {:?}", sender_id, target_player_id, target_player.respawn_at);
                // TODO: Drop items? Clear equipment?
            }

            players.identity().update(target_player);
            hit_something = true;
        }

    } else if tool_name == "Rock" {
        // Rock: Prioritize closest Stone, Tree, OR Player
        let mut closest_dist_sq = f32::MAX;
        let mut closest_target_type = None; // Option<"tree" | "stone" | "player">

        if let Some((_, dist_sq)) = closest_tree_target {
            if dist_sq < closest_dist_sq {
                closest_dist_sq = dist_sq;
                closest_target_type = Some("tree");
            }
        }
        if let Some((_, dist_sq)) = closest_stone_target {
             if dist_sq < closest_dist_sq {
                closest_dist_sq = dist_sq;
                closest_target_type = Some("stone");
            }
        }
        if let Some((_, dist_sq)) = closest_player_target {
             if dist_sq < closest_dist_sq {
                // closest_dist_sq = dist_sq; // Intentionally removed - player dist already calculated
                closest_target_type = Some("player");
            }
        }
        
        match closest_target_type {
            Some("tree") => {
                if let Some((tree_id, _)) = closest_tree_target { // Retrieve ID again
                    // --- Damage Tree & Grant Wood (Damage = 1) ---
                    let mut tree = trees.id().find(tree_id).ok_or("Target tree disappeared?")?;
                    let old_health = tree.health;
                    tree.health = tree.health.saturating_sub(1); // Rock damage = 1
                    tree.last_hit_time = Some(now_ts);
                    log::info!("Player {:?} hit Tree {} with {} for {} damage. Health: {} -> {}",
                            sender_id, tree_id, item_def.name, 1, old_health, tree.health);

                    // Grant 1 Wood - USE REFACTORED HELPER
                    if let Some(wood_def) = item_defs.iter().find(|def| def.name == "Wood") {
                        match crate::items::add_item_to_player_inventory(ctx, sender_id, wood_def.id, 1) {
                            Ok(_) => log::debug!("Granted 1 Wood to player {:?} via helper.", sender_id),
                            Err(e) => log::error!("Failed to grant Wood to player {:?}: {}", sender_id, e),
                        }
                    } else { 
                        log::error!("Wood item definition not found for Rock hit."); 
                    }

                    if tree.health == 0 {
                        log::info!("Tree {} destroyed by Player {:?}. Scheduling respawn.", tree_id, sender_id);
                        let respawn_time = now_ts + Duration::from_secs(RESOURCE_RESPAWN_DURATION_SECS).into();
                        tree.respawn_at = Some(respawn_time);
                        trees.id().update(tree); // Update with health 0 and respawn time
                        // trees.id().delete(tree_id); // REMOVED delete
                    } else {
                        trees.id().update(tree);
                    }
                    hit_something = true;
                }
            },
            Some("stone") => {
                if let Some((stone_id, _)) = closest_stone_target { // Retrieve ID again
                    // --- Damage Stone & Grant Stone (Damage = 1) ---
                    let mut stone = stones.id().find(stone_id).ok_or("Target stone disappeared?")?;
                    let old_health = stone.health;
                    stone.health = stone.health.saturating_sub(1); // Rock damage = 1
                    stone.last_hit_time = Some(now_ts);
                    log::info!("Player {:?} hit Stone {} with {} for {} damage. Health: {} -> {}",
                            sender_id, stone_id, item_def.name, 1, old_health, stone.health);

                    // Grant 1 Stone - USE REFACTORED HELPER
                    if let Some(stone_def) = item_defs.iter().find(|def| def.name == "Stone") {
                       match crate::items::add_item_to_player_inventory(ctx, sender_id, stone_def.id, 1) {
                           Ok(_) => log::debug!("Granted 1 Stone to player {:?} via helper.", sender_id),
                           Err(e) => log::error!("Failed to grant Stone to player {:?}: {}", sender_id, e),
                       }
                    } else { 
                        log::error!("Stone item definition not found for Rock hit."); 
                    }

                    if stone.health == 0 {
                        log::info!("Stone {} depleted by Player {:?}. Scheduling respawn.", stone_id, sender_id);
                        let respawn_time = now_ts + Duration::from_secs(RESOURCE_RESPAWN_DURATION_SECS).into();
                        stone.respawn_at = Some(respawn_time);
                        stones.id().update(stone); // Update with health 0 and respawn time
                        // stones.id().delete(stone_id);
                    } else {
                        stones.id().update(stone);
                    }
                    hit_something = true;
                }
            },
            Some("player") => {
                if let Some((player_id, _)) = closest_player_target { // Retrieve ID again
                    // --- Damage Player (Rock Damage = 1 * Multiplier) ---
                    let mut target_player = players.identity().find(player_id)
                        .ok_or("Target player disappeared?")?;
                    let old_health = target_player.health;
                    // Rock base damage is 1
                    let actual_damage = (1.0 * PVP_DAMAGE_MULTIPLIER).max(0.0);
                    target_player.health = (target_player.health - actual_damage).max(0.0);
                    target_player.last_hit_time = Some(now_ts);
                    log::info!("Player {:?} hit Player {:?} with {} for {:.1} (1 base * {}x) damage. Health: {:.1} -> {:.1}",
                            sender_id, player_id, item_def.name, actual_damage, PVP_DAMAGE_MULTIPLIER, old_health, target_player.health);

                    // Check for death
                    if target_player.health <= 0.0 && !target_player.is_dead {
                        target_player.is_dead = true;
                        let respawn_micros = now_micros.saturating_add((RESPAWN_TIME_MS * 1000) as i64);
                        target_player.respawn_at = Timestamp::from_micros_since_unix_epoch(respawn_micros);
                        log::info!("Player {:?} killed Player {:?}. Respawn at {:?}", sender_id, player_id, target_player.respawn_at);
                    }

                    players.identity().update(target_player);
                    hit_something = true;
                }
            },
            None => { /* No target found */ }, 
            _ => { /* Should not happen */ log::error!("Invalid closest_target_type"); }
        }

    } else {
        // Other Damage Tool (e.g., Sword later): Prioritize closest target overall
        let mut closest_dist_sq = f32::MAX;
        let mut closest_target_type = None; // Option<"tree" | "stone" | "player">

        if let Some((_, dist_sq)) = closest_tree_target {
            if dist_sq < closest_dist_sq {
                closest_dist_sq = dist_sq;
                closest_target_type = Some("tree");
            }
        }
        if let Some((_, dist_sq)) = closest_stone_target {
             if dist_sq < closest_dist_sq {
                closest_dist_sq = dist_sq;
                closest_target_type = Some("stone");
            }
        }
        if let Some((_, dist_sq)) = closest_player_target {
             if dist_sq < closest_dist_sq {
                closest_dist_sq = dist_sq;
                closest_target_type = Some("player");
            }
        }
        
        match closest_target_type {
            Some("tree") => {
                if let Some((tree_id, _)) = closest_tree_target { // Retrieve ID again
                    let mut tree = trees.id().find(tree_id).ok_or("Target tree disappeared?")?;
                    let old_health = tree.health;
                    tree.health = tree.health.saturating_sub(item_damage);
                    tree.last_hit_time = Some(now_ts);
                    log::info!("Player {:?} hit Tree {} with {} for {} damage. Health: {} -> {}",
                            sender_id, tree_id, item_def.name, item_damage, old_health, tree.health);
                    if tree.health == 0 {
                        log::info!("Tree {} destroyed by Player {:?}. Scheduling respawn.", tree_id, sender_id);
                        let respawn_time = now_ts + Duration::from_secs(RESOURCE_RESPAWN_DURATION_SECS).into();
                        tree.respawn_at = Some(respawn_time);
                        trees.id().update(tree); // Update with health 0 and respawn time
                        // trees.id().delete(tree_id); // REMOVED delete
                    } else {
                        trees.id().update(tree);
                    }
                    hit_something = true;
                }
            },
            Some("stone") => {
                if let Some((stone_id, _)) = closest_stone_target { // Retrieve ID again
                    let mut stone = stones.id().find(stone_id).ok_or("Target stone disappeared?")?;
                    let old_health = stone.health;
                    stone.health = stone.health.saturating_sub(item_damage);
                    stone.last_hit_time = Some(now_ts); // Set last hit time for shake effect
                    log::info!("Player {:?} hit Stone {} with {} for {} damage. Health: {} -> {}",
                            sender_id, stone_id, item_def.name, item_damage, old_health, stone.health);
                    if stone.health == 0 {
                        log::info!("Stone {} depleted by Player {:?}. Scheduling respawn.", stone_id, sender_id);
                        let respawn_time = now_ts + Duration::from_secs(RESOURCE_RESPAWN_DURATION_SECS).into();
                        stone.respawn_at = Some(respawn_time);
                        stones.id().update(stone); // Update with health 0 and respawn time
                        // stones.id().delete(stone_id);
                    } else {
                        stones.id().update(stone);
                    }
                    hit_something = true;
                }
            },
            Some("player") => {
                if let Some((player_id, _)) = closest_player_target { // Retrieve ID again
                    let mut target_player = players.identity().find(player_id)
                        .ok_or("Target player disappeared?")?;
                    let old_health = target_player.health;
                    // Apply PvP multiplier
                    let actual_damage = (item_damage as f32 * PVP_DAMAGE_MULTIPLIER).max(0.0);
                    target_player.health = (target_player.health - actual_damage).max(0.0);
                    target_player.last_hit_time = Some(now_ts); // <-- Set last hit time
                    log::info!("Player {:?} hit Player {:?} with {} for {:.1} ({} base * {}x) damage. Health: {:.1} -> {:.1}",
                            sender_id, player_id, item_def.name, actual_damage, item_damage, PVP_DAMAGE_MULTIPLIER, old_health, target_player.health);

                    // Check for death
                    if target_player.health <= 0.0 && !target_player.is_dead {
                        target_player.is_dead = true;
                        let respawn_micros = now_micros.saturating_add((RESPAWN_TIME_MS * 1000) as i64);
                        target_player.respawn_at = Timestamp::from_micros_since_unix_epoch(respawn_micros);
                        log::info!("Player {:?} killed Player {:?}. Respawn at {:?}", sender_id, player_id, target_player.respawn_at);
                        // TODO: Drop items? Clear equipment?
                    }

                    players.identity().update(target_player);
                    hit_something = true;
                }
            },
            None => { /* No target found */ }, 
            _ => { /* Should not happen */ log::error!("Invalid closest_target_type"); }
        }
    }

    if !hit_something {
        log::debug!("Player {:?} swung {} but hit nothing.", sender_id, item_def.name);
    }

    Ok(())
}

// Helper to find or create ActiveEquipment row
fn get_or_create_active_equipment(ctx: &ReducerContext, player_id: Identity) -> Result<ActiveEquipment, String> {
    let table = ctx.db.active_equipment();
    if let Some(existing) = table.player_identity().find(player_id) {
        Ok(existing)
    } else {
        log::info!("Creating new ActiveEquipment row for player {:?}", player_id);
        let new_equip = ActiveEquipment { 
            player_identity: player_id, 
            equipped_item_def_id: None, // Initialize hand slot
            equipped_item_instance_id: None,
            swing_start_time_ms: 0,
            // Initialize all armor slots to None
            head_item_instance_id: None,
            chest_item_instance_id: None,
            legs_item_instance_id: None,
            feet_item_instance_id: None,
            hands_item_instance_id: None,
            back_item_instance_id: None,
        };
        table.insert(new_equip.clone()); // Insert returns nothing useful here
        Ok(new_equip)
    }
}

#[spacetimedb::reducer]
pub fn equip_armor(ctx: &ReducerContext, item_instance_id: u64) -> Result<(), String> {
    let sender_id = ctx.sender;
    log::info!("Player {:?} attempting to equip armor item instance {}", sender_id, item_instance_id);

    // 1. Get the InventoryItem being equipped
    let mut item_to_equip = ctx.db.inventory_item().iter()
        .find(|i| i.instance_id == item_instance_id && i.player_identity == sender_id)
        .ok_or_else(|| format!("Item instance {} not found or not owned.", item_instance_id))?;
    let source_inv_slot = item_to_equip.inventory_slot; // Store original location
    let source_hotbar_slot = item_to_equip.hotbar_slot; // Store original location

    // 2. Get its ItemDefinition
    let item_def = ctx.db.item_definition().iter()
        .find(|def| def.id == item_to_equip.item_def_id)
        .ok_or_else(|| format!("Definition not found for item ID {}", item_to_equip.item_def_id))?;

    // 3. Validate: Must be Armor category and have a defined equipment_slot
    if item_def.category != ItemCategory::Armor {
        return Err(format!("Item '{}' is not Armor.", item_def.name));
    }
    let target_slot_type = item_def.equipment_slot
        .clone() // Clone the Option<EquipmentSlot>
        .ok_or_else(|| format!("Armor '{}' does not have a defined equipment slot.", item_def.name))?;

    // 4. Find or create the player's ActiveEquipment row
    let mut active_equipment = get_or_create_active_equipment(ctx, sender_id)?;

    // 5. Check if the target slot is already occupied & get old item ID
    let old_item_instance_id_opt = match target_slot_type {
         EquipmentSlot::Head => active_equipment.head_item_instance_id.take(), // .take() retrieves value and sets field to None
         EquipmentSlot::Chest => active_equipment.chest_item_instance_id.take(),
         EquipmentSlot::Legs => active_equipment.legs_item_instance_id.take(),
         EquipmentSlot::Feet => active_equipment.feet_item_instance_id.take(),
         EquipmentSlot::Hands => active_equipment.hands_item_instance_id.take(),
         EquipmentSlot::Back => active_equipment.back_item_instance_id.take(),
    };

    // 6. If occupied, move the old item back to the source slot of the item being equipped
    if let Some(old_item_instance_id) = old_item_instance_id_opt {
        log::info!("Slot {:?} was occupied by item {}. Moving it back to source slot (Inv: {:?}, Hotbar: {:?}).", 
                 target_slot_type, old_item_instance_id, source_inv_slot, source_hotbar_slot);
                 
        if let Some(mut old_item) = ctx.db.inventory_item().instance_id().find(old_item_instance_id) {
            old_item.inventory_slot = source_inv_slot; 
            old_item.hotbar_slot = source_hotbar_slot;
            ctx.db.inventory_item().instance_id().update(old_item);
        } else {
            // This shouldn't happen if data is consistent, but log an error if it does
            log::error!("Failed to find InventoryItem for previously equipped armor (ID: {})!", old_item_instance_id);
        }
    } else {
         log::info!("Slot {:?} was empty.", target_slot_type);
    }

    // 7. Update ActiveEquipment row with the new item ID in the correct slot
    match target_slot_type {
         EquipmentSlot::Head => active_equipment.head_item_instance_id = Some(item_instance_id),
         EquipmentSlot::Chest => active_equipment.chest_item_instance_id = Some(item_instance_id),
         EquipmentSlot::Legs => active_equipment.legs_item_instance_id = Some(item_instance_id),
         EquipmentSlot::Feet => active_equipment.feet_item_instance_id = Some(item_instance_id),
         EquipmentSlot::Hands => active_equipment.hands_item_instance_id = Some(item_instance_id),
         EquipmentSlot::Back => active_equipment.back_item_instance_id = Some(item_instance_id),
         // Note: The .take() above already cleared the field, so we just set the new value
    };
    ctx.db.active_equipment().player_identity().update(active_equipment); // Save ActiveEquipment changes

    // 8. Update the InventoryItem being equipped (remove from inventory/hotbar)
    item_to_equip.inventory_slot = None;
    item_to_equip.hotbar_slot = None;
    ctx.db.inventory_item().instance_id().update(item_to_equip);

    log::info!("Successfully equipped armor '{}' (ID: {}) to slot {:?}", 
             item_def.name, item_instance_id, target_slot_type);
             
    Ok(())
}
