use spacetimedb::{ Identity, ReducerContext, Table };
use log;

// Import table traits needed for ctx.db access
use crate::player;
use crate::environment::tree;
use crate::environment::stone;
use crate::items::inventory_item;
use crate::items::item_definition as item_definition_table_trait; // Alias to avoid conflict with struct
// Import structs used
// use crate::environment::Tree; // Remove - Not used directly here
// use crate::items::ItemDefinition; // Remove - Not used directly here
// use crate::{Player, PLAYER_RADIUS}; // Remove - Not used directly here
use crate::environment::{TREE_COLLISION_Y_OFFSET, STONE_COLLISION_Y_OFFSET}; // Import offset constants
use crate::PLAYER_RADIUS; // Add back the import for PLAYER_RADIUS
use std::f32::consts::PI;

#[spacetimedb::table(name = active_equipment, public)]
#[derive(Clone, Default, Debug)]
pub struct ActiveEquipment {
    #[primary_key]
    pub player_identity: Identity,
    pub equipped_item_def_id: Option<u64>, // ID from ItemDefinition table
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
        equipped_item_def_id: Some(item_def.id), // Store the u64 ID directly
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
    let attack_range = PLAYER_RADIUS * 3.0; // Increased range slightly
    let attack_angle_degrees = 70.0; // Width of the attack arc (degrees)
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
            let stone_def = item_defs.iter().find(|def| def.name == "Stone")
                .ok_or("Stone item definition not found")?;
            let stone_to_grant = item_damage as u32; 
            let existing_stone_stack = inventory_items.iter()
                .find(|item| item.player_identity == sender_id && item.item_def_id == stone_def.id);

            match existing_stone_stack {
                Some(mut stone_stack) => {
                    stone_stack.quantity = stone_stack.quantity.saturating_add(stone_to_grant);
                    log::debug!("Player {:?} collected {} stone. New stack size: {}", 
                            sender_id, stone_to_grant, stone_stack.quantity);
                    inventory_items.instance_id().update(stone_stack);
                },
                None => {
                    let new_stone_item = crate::items::InventoryItem {
                        instance_id: 0, player_identity: sender_id, item_def_id: stone_def.id,
                        quantity: stone_to_grant, hotbar_slot: None,
                    };
                    match inventory_items.try_insert(new_stone_item) {
                        Ok(_) => log::debug!("Player {:?} collected {} stone. New stack created.", 
                                           sender_id, stone_to_grant),
                        Err(e) => log::error!("Failed to grant new stone stack to player {:?}: {}", sender_id, e),
                    }
                }
            }
            // --- End Grant Stone Item ---

            if stone.health == 0 {
                log::info!("Stone {} destroyed by Player {:?}", stone_id, sender_id);
                stones.id().delete(stone_id);
                // TODO: Spawn extra stone items?
            } else {
                stones.id().update(stone);
            }
            hit_something = true;

        } else if let Some((target_player_id, _)) = closest_player_target {
            // --- Damage Player ---
            let mut target_player = players.identity().find(target_player_id)
                .ok_or("Target player disappeared?")?;
            let old_health = target_player.health;
            target_player.health = (target_player.health - item_damage as f32).max(0.0);
            log::info!("Player {:?} hit Player {:?} with {} for {} damage. Health: {:.1} -> {:.1}",
                     sender_id, target_player_id, item_def.name, item_damage, old_health, target_player.health);
            players.identity().update(target_player);
            hit_something = true;
        }

    } else if tool_name == "Stone Hatchet" {
        // Hatchet: Prioritize Trees > Players
        if let Some((tree_id, _)) = closest_tree_target {
            // --- Damage Tree & Grant Wood --- (Code is duplicated from previous version)
            let mut tree = trees.id().find(tree_id).ok_or("Target tree disappeared?")?;
            let old_health = tree.health;
            tree.health = tree.health.saturating_sub(item_damage);
            tree.last_hit_time = Some(now_ts);
            log::info!("Player {:?} hit Tree {} with {} for {} damage. Health: {} -> {}",
                     sender_id, tree_id, item_def.name, item_damage, old_health, tree.health);

            let wood_def = item_defs.iter().find(|def| def.name == "Wood")
                .ok_or("Wood item definition not found")?;
            let wood_to_grant = item_damage as u32; 
            let existing_wood_stack = inventory_items.iter()
                .find(|item| item.player_identity == sender_id && item.item_def_id == wood_def.id);
            match existing_wood_stack {
                Some(mut wood_stack) => {
                    wood_stack.quantity = wood_stack.quantity.saturating_add(wood_to_grant);
                    log::debug!("Player {:?} collected {} wood. New stack size: {}", 
                             sender_id, wood_to_grant, wood_stack.quantity);
                    inventory_items.instance_id().update(wood_stack);
                },
                None => {
                    let new_wood_item = crate::items::InventoryItem {
                        instance_id: 0, player_identity: sender_id, item_def_id: wood_def.id,
                        quantity: wood_to_grant, hotbar_slot: None,
                    };
                    match inventory_items.try_insert(new_wood_item) {
                        Ok(_) => log::debug!("Player {:?} collected {} wood. New stack created.", 
                                           sender_id, wood_to_grant),
                        Err(e) => log::error!("Failed to grant new wood stack to player {:?}: {}", sender_id, e),
                    }
                }
            }
            if tree.health == 0 {
                log::info!("Tree {} destroyed by Player {:?}", tree_id, sender_id);
                trees.id().delete(tree_id);
            } else {
                trees.id().update(tree);
            }
            hit_something = true;

        } else if let Some((target_player_id, _)) = closest_player_target {
            // --- Damage Player ---
            let mut target_player = players.identity().find(target_player_id)
                .ok_or("Target player disappeared?")?;
            let old_health = target_player.health;
            target_player.health = (target_player.health - item_damage as f32).max(0.0);
            log::info!("Player {:?} hit Player {:?} with {} for {} damage. Health: {:.1} -> {:.1}",
                     sender_id, target_player_id, item_def.name, item_damage, old_health, target_player.health);
            players.identity().update(target_player);
            hit_something = true;
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
                        trees.id().delete(tree_id);
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
                        stones.id().delete(stone_id);
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
                    target_player.health = (target_player.health - item_damage as f32).max(0.0);
                    log::info!("Player {:?} hit Player {:?} with {} for {} damage. Health: {:.1} -> {:.1}",
                            sender_id, player_id, item_def.name, item_damage, old_health, target_player.health);
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
