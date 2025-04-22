use spacetimedb::{Identity, ReducerContext, Table, Timestamp};
use log;
use rand::Rng;
use serde::{Serialize, Deserialize};

// --- Experience and Level Constants ---
const BASE_EXP_PER_KILL: f32 = 10.0;
const EXP_MULTIPLIER_PER_LEVEL: f32 = 1.2;
const BASE_EXP_TO_LEVEL: f32 = 100.0;
const EXP_TO_LEVEL_MULTIPLIER: f32 = 1.5;

// --- Buff Rarity Constants ---
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BuffRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

// --- Buff Types ---
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BuffType {
    Health(f32),           // Percentage increase
    Attack(f32),          // Percentage increase
    AttackSpeed(f32),     // Percentage increase
    MoveSpeed(f32),       // Percentage increase
    HpRegen(f32),         // Flat HP per 10 seconds
    Armor(f32),           // Percentage damage reduction
}

// --- Buff Struct ---
#[spacetimedb::table(name = buff, public)]
#[derive(Clone)]
pub struct Buff {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub player_id: Identity,
    pub buff_type: BuffType,
    pub rarity: BuffRarity,
}

// --- Player Stats Struct ---
#[spacetimedb::table(name = player_stats, public)]
#[derive(Clone)]
pub struct PlayerStats {
    #[primary_key]
    pub player_id: Identity,
    pub level: u32,
    pub experience: f32,
    pub experience_to_next_level: f32,
    pub base_health: f32,
    pub base_attack: f32,
    pub base_attack_speed: f32,
    pub base_move_speed: f32,
    pub base_hp_regen: f32,
    pub base_armor: f32,
}

// --- Helper Functions ---
fn calculate_exp_to_next_level(level: u32) -> f32 {
    BASE_EXP_TO_LEVEL * (EXP_TO_LEVEL_MULTIPLIER.powi(level as i32 - 1))
}

fn get_random_buff(rarity: BuffRarity) -> BuffType {
    let mut rng = rand::thread_rng();
    let buff_type = rng.gen_range(0..6);
    
    match (buff_type, rarity) {
        (0, BuffRarity::Common) => BuffType::Health(0.1),
        (0, BuffRarity::Uncommon) => BuffType::Health(0.2),
        (0, BuffRarity::Rare) => BuffType::Health(0.3),
        (0, BuffRarity::Epic) => BuffType::Health(0.4),
        (0, BuffRarity::Legendary) => BuffType::Health(0.5),
        
        (1, BuffRarity::Common) => BuffType::Attack(0.1),
        (1, BuffRarity::Uncommon) => BuffType::Attack(0.2),
        (1, BuffRarity::Rare) => BuffType::Attack(0.3),
        (1, BuffRarity::Epic) => BuffType::Attack(0.4),
        (1, BuffRarity::Legendary) => BuffType::Attack(0.5),
        
        (2, BuffRarity::Common) => BuffType::AttackSpeed(0.1),
        (2, BuffRarity::Uncommon) => BuffType::AttackSpeed(0.2),
        (2, BuffRarity::Rare) => BuffType::AttackSpeed(0.3),
        (2, BuffRarity::Epic) => BuffType::AttackSpeed(0.4),
        (2, BuffRarity::Legendary) => BuffType::AttackSpeed(0.5),
        
        (3, BuffRarity::Common) => BuffType::MoveSpeed(0.1),
        (3, BuffRarity::Uncommon) => BuffType::MoveSpeed(0.2),
        (3, BuffRarity::Rare) => BuffType::MoveSpeed(0.3),
        (3, BuffRarity::Epic) => BuffType::MoveSpeed(0.4),
        (3, BuffRarity::Legendary) => BuffType::MoveSpeed(0.5),
        
        (4, BuffRarity::Common) => BuffType::HpRegen(1.0),
        (4, BuffRarity::Uncommon) => BuffType::HpRegen(2.0),
        (4, BuffRarity::Rare) => BuffType::HpRegen(3.0),
        (4, BuffRarity::Epic) => BuffType::HpRegen(4.0),
        (4, BuffRarity::Legendary) => BuffType::HpRegen(5.0),
        
        (5, BuffRarity::Common) => BuffType::Armor(0.1),
        (5, BuffRarity::Uncommon) => BuffType::Armor(0.2),
        (5, BuffRarity::Rare) => BuffType::Armor(0.3),
        (5, BuffRarity::Epic) => BuffType::Armor(0.4),
        (5, BuffRarity::Legendary) => BuffType::Armor(0.5),
        
        _ => BuffType::Health(0.1), // Default case
    }
}

fn get_random_rarity() -> BuffRarity {
    let mut rng = rand::thread_rng();
    let roll = rng.gen_range(0..100);
    
    match roll {
        0..=49 => BuffRarity::Common,
        50..=74 => BuffRarity::Uncommon,
        75..=89 => BuffRarity::Rare,
        90..=97 => BuffRarity::Epic,
        98..=99 => BuffRarity::Legendary,
        _ => BuffRarity::Common,
    }
}

// --- Reducers ---
#[spacetimedb::reducer]
pub fn add_experience(ctx: &ReducerContext, amount: f32) -> Result<(), String> {
    let sender_id = ctx.sender;
    let player_stats = ctx.db.player_stats();
    let buffs = ctx.db.buff();
    
    let mut stats = player_stats.player_id().find(sender_id)
        .ok_or_else(|| "Player stats not found".to_string())?;
    
    stats.experience += amount;
    
    // Check for level up
    while stats.experience >= stats.experience_to_next_level {
        stats.level += 1;
        stats.experience -= stats.experience_to_next_level;
        stats.experience_to_next_level = calculate_exp_to_next_level(stats.level);
        
        // Generate random buffs for level up
        let buff_count = 3; // Number of buffs to choose from
        let mut available_buffs = Vec::new();
        
        for _ in 0..buff_count {
            let rarity = get_random_rarity();
            let buff_type = get_random_buff(rarity.clone());
            
            let buff = Buff {
                id: 0, // Auto-incremented
                player_id: sender_id,
                buff_type: buff_type.clone(),
                rarity: rarity.clone(),
            };
            
            available_buffs.push(buff);
        }
        
        // Store available buffs (they will be selected by the client)
        for buff in available_buffs {
            buffs.insert(buff);
        }
        
        log::info!("Player {:?} reached level {}!", sender_id, stats.level);
    }
    
    player_stats.player_id().update(stats);
    Ok(())
}

#[spacetimedb::reducer]
pub fn select_buff(ctx: &ReducerContext, buff_id: u64) -> Result<(), String> {
    let sender_id = ctx.sender;
    let buffs = ctx.db.buff();
    let player_stats = ctx.db.player_stats();
    
    // Get the selected buff
    let selected_buff = buffs.id().find(buff_id)
        .ok_or_else(|| "Buff not found".to_string())?;
    
    // Verify ownership
    if selected_buff.player_id != sender_id {
        return Err("Cannot select buff that doesn't belong to you".to_string());
    }
    
    // Get player stats
    let mut stats = player_stats.player_id().find(sender_id)
        .ok_or_else(|| "Player stats not found".to_string())?;
    
    // Apply buff effect
    match selected_buff.buff_type {
        BuffType::Health(amount) => stats.base_health *= (1.0 + amount),
        BuffType::Attack(amount) => stats.base_attack *= (1.0 + amount),
        BuffType::AttackSpeed(amount) => stats.base_attack_speed *= (1.0 + amount),
        BuffType::MoveSpeed(amount) => stats.base_move_speed *= (1.0 + amount),
        BuffType::HpRegen(amount) => stats.base_hp_regen += amount,
        BuffType::Armor(amount) => stats.base_armor += amount,
    }
    
    // Delete all available buffs for this player
    for buff in buffs.iter().filter(|b| b.player_id == sender_id) {
        buffs.id().delete(buff.id);
    }
    
    // Update player stats
    player_stats.player_id().update(stats);
    
    log::info!("Player {:?} selected buff: {:?} (Rarity: {:?})", 
        sender_id, selected_buff.buff_type, selected_buff.rarity);
    
    Ok(())
}

// --- Initialize Player Stats ---
pub fn initialize_player_stats(ctx: &ReducerContext, player_id: Identity) -> Result<(), String> {
    let player_stats = ctx.db.player_stats();
    
    let stats = PlayerStats {
        player_id,
        level: 1,
        experience: 0.0,
        experience_to_next_level: calculate_exp_to_next_level(1),
        base_health: 100.0,
        base_attack: 10.0,
        base_attack_speed: 1.0,
        base_move_speed: 1.0,
        base_hp_regen: 0.0,
        base_armor: 0.0,
    };
    
    player_stats.insert(stats);
    Ok(())
} 