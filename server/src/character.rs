use spacetimedb::{Identity, ReducerContext, Table};
use log;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::player_stats::player_stats;

// --- Character Types ---
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Copy, spacetimedb::SpacetimeType)]
pub enum CharacterType {
    Til,    // Tank - High HP
    Marc,   // Runner - High Move Speed
    Max,    // Fighter - High Attack Speed
    Chris,  // All-rounder - High HP Regen
}

// --- Character Stats Struct ---
#[spacetimedb::table(name = character, public)]
#[derive(Clone)]
pub struct Character {
    #[primary_key]
    pub player_id: Identity,
    pub character_type: CharacterType,
}

// --- Character Constants ---
const TIL_HP_BONUS: f32 = 1.5;        // 50% more HP
const MARC_SPEED_BONUS: f32 = 1.3;    // 30% more move speed
const MAX_ATTACK_SPEED_BONUS: f32 = 1.4; // 40% more attack speed
const CHRIS_HP_REGEN_BONUS: f32 = 2.0; // 2x HP regen

// --- Helper Functions ---
pub fn get_character_bonuses(character_type: CharacterType) -> HashMap<String, f32> {
    let mut bonuses = HashMap::new();
    match character_type {
        CharacterType::Til => {
            bonuses.insert("health".to_string(), TIL_HP_BONUS);
        },
        CharacterType::Marc => {
            bonuses.insert("move_speed".to_string(), MARC_SPEED_BONUS);
        },
        CharacterType::Max => {
            bonuses.insert("attack_speed".to_string(), MAX_ATTACK_SPEED_BONUS);
        },
        CharacterType::Chris => {
            bonuses.insert("hp_regen".to_string(), CHRIS_HP_REGEN_BONUS);
        },
    }
    bonuses
}

// --- Reducers ---
#[spacetimedb::reducer]
pub fn select_character(ctx: &ReducerContext, character_type: CharacterType) -> Result<(), String> {
    let player_id = ctx.sender;
    
    // Check if player already has a character
    let characters = ctx.db.character();
    if characters.player_id().find(player_id).is_some() {
        return Err("Player already has a character selected".to_string());
    }
    
    // Create new character entry
    let character = Character {
        player_id,
        character_type,
    };
    
    // Insert character
    match characters.try_insert(character) {
        Ok(_) => {
            // Apply character bonuses to player stats
            let bonuses = get_character_bonuses(character_type);
            let mut player_stats = ctx.db.player_stats();
            
            if let Some(stats) = player_stats.player_id().find(player_id) {
                let mut updated_stats = stats.clone();
                
                // Apply bonuses
                if let Some(health_bonus) = bonuses.get("health") {
                    updated_stats.health *= health_bonus;
                }
                if let Some(move_speed_bonus) = bonuses.get("move_speed") {
                    updated_stats.move_speed *= move_speed_bonus;
                }
                if let Some(attack_speed_bonus) = bonuses.get("attack_speed") {
                    updated_stats.attack_speed *= attack_speed_bonus;
                }
                if let Some(hp_regen_bonus) = bonuses.get("hp_regen") {
                    updated_stats.hp_regen *= hp_regen_bonus;
                }
                
                // Update player stats
                player_stats.player_id().update(updated_stats);
                
                log::info!("Character {:?} selected for player {:?} with bonuses: {:?}", 
                    character_type, player_id, bonuses);
            }
            
            Ok(())
        },
        Err(e) => Err(format!("Failed to select character: {}", e)),
    }
}