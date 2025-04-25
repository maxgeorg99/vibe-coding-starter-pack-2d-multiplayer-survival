use spacetimedb::{Identity, ReducerContext, StdbRng, Table, Timestamp};
use log;
use serde::{Serialize, Deserialize};
use spacetimedb::rand::Rng;
use spacetimedb::rand::rngs::StdRng;
use crate::player_stats::player_stats;

// --- Buff Rarity Constants ---
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, spacetimedb::SpacetimeType)]
pub enum BuffRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

// --- Buff Types ---
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, spacetimedb::SpacetimeType)]
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

pub(crate) fn get_random_buff(rng: &mut StdRng,rarity: BuffRarity) -> BuffType {
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

pub(crate) fn get_random_rarity(rng: &mut StdRng) -> BuffRarity {
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
        BuffType::Health(amount) => stats.health *= (1.0 + amount),
        BuffType::Attack(amount) => stats.attack *= (1.0 + amount),
        BuffType::AttackSpeed(amount) => stats.attack_speed *= (1.0 + amount),
        BuffType::MoveSpeed(amount) => stats.move_speed *= (1.0 + amount),
        BuffType::HpRegen(amount) => stats.hp_regen += amount,
        BuffType::Armor(amount) => stats.armor += amount,
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