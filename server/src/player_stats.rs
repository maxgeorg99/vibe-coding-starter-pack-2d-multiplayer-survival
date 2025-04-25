use spacetimedb::{Identity, ReducerContext, Table, Timestamp};
use spacetimedb::spacetimedb_lib::ScheduleAt;
use log;
use std::time::Duration;
// Define Constants locally
const HUNGER_DRAIN_PER_SECOND: f32 = 100.0 / (30.0 * 60.0);
const THIRST_DRAIN_PER_SECOND: f32 = 100.0 / (20.0 * 60.0);
// Make stat constants pub(crate) as well for consistency, although not strictly needed if only used here
pub(crate) const STAMINA_DRAIN_PER_SECOND: f32 = 5.0;
pub(crate) const STAMINA_RECOVERY_PER_SECOND: f32 = 1.0;
pub(crate) const HEALTH_LOSS_PER_SEC_LOW_THIRST: f32 = 0.5;
pub(crate) const HEALTH_LOSS_PER_SEC_LOW_HUNGER: f32 = 0.4;
pub(crate) const HEALTH_LOSS_MULTIPLIER_AT_ZERO: f32 = 2.0;
pub(crate) const HEALTH_RECOVERY_THRESHOLD: f32 = 80.0;
pub(crate) const HEALTH_RECOVERY_PER_SEC: f32 = 1.0;
pub(crate) const HEALTH_LOSS_PER_SEC_LOW_WARMTH: f32 = 0.6;

// Add the constants moved from lib.rs and make them pub(crate)
pub(crate) const SPRINT_SPEED_MULTIPLIER: f32 = 1.5;
pub(crate) const JUMP_COOLDOWN_MS: u64 = 500;
pub(crate) const LOW_NEED_THRESHOLD: f32 = 20.0;
pub(crate) const LOW_THIRST_SPEED_PENALTY: f32 = 0.75;
pub(crate) const LOW_WARMTH_SPEED_PENALTY: f32 = 0.8;

// --- Experience and Level Constants ---
pub(crate) const BASE_EXP_PER_KILL: f32 = 10.0;
pub(crate) const EXP_MULTIPLIER_PER_LEVEL: f32 = 1.2;
pub(crate) const BASE_EXP_TO_LEVEL: f32 = 100.0;
pub(crate) const EXP_TO_LEVEL_MULTIPLIER: f32 = 1.5;

// Import necessary items from the main lib module or other modules
use crate::{
    Player, // Player struct
    world_state::{self, TimeOfDay, BASE_WARMTH_DRAIN_PER_SECOND, WARMTH_DRAIN_MULTIPLIER_DAWN_DUSK, WARMTH_DRAIN_MULTIPLIER_NIGHT, WARMTH_DRAIN_MULTIPLIER_MIDNIGHT},
    campfire::{self, Campfire, WARMTH_RADIUS_SQUARED, WARMTH_PER_SECOND},
    active_equipment, // For unequipping on death
};

// Import table traits
use crate::Player as PlayerTableTrait;
use crate::world_state::world_state as WorldStateTableTrait;
use crate::campfire::campfire as CampfireTableTrait;
use crate::active_equipment::active_equipment as ActiveEquipmentTableTrait;
use crate::buff::{buff, Buff};
// Needed for unequip on death
use crate::player; // Added missing import for Player trait
use crate::player_stats::PlayerStatSchedule as PlayerStatScheduleTableTrait; // Added Self trait import

pub(crate) const PLAYER_STAT_UPDATE_INTERVAL_SECS: u64 = 1; // Update stats every second

// --- Player Stats Struct ---
#[spacetimedb::table(name = player_stats, public)]
#[derive(Clone)]
pub struct PlayerStats {
    #[primary_key]
    pub player_id: Identity,
    pub level: u32,
    pub experience: f32,
    pub experience_to_next_level: f32,
    pub health: f32,
    pub attack: f32,
    pub attack_speed: f32,
    pub move_speed: f32,
    pub hp_regen: f32,
    pub armor: f32,
    pub stamina: f32,
}

// --- Player Stat Schedule Table (Reverted to scheduled pattern) ---
#[spacetimedb::table(name = player_stat_schedule, scheduled(process_player_stats))]
#[derive(Clone)]
pub struct PlayerStatSchedule {
    #[primary_key]
    #[auto_inc]
    pub id: u64, // Changed PK name to id
    pub scheduled_at: ScheduleAt, // Added scheduled_at field
}

// --- Function to Initialize the Stat Update Schedule ---
pub fn init_player_stat_schedule(ctx: &ReducerContext) -> Result<(), String> {
    let schedule_table = ctx.db.player_stat_schedule();
    if schedule_table.iter().count() == 0 {
        log::info!(
            "Starting player stat update schedule (every {}s).",
            PLAYER_STAT_UPDATE_INTERVAL_SECS
        );
        let interval = Duration::from_secs(PLAYER_STAT_UPDATE_INTERVAL_SECS);
        // Use try_insert and handle potential errors (though unlikely for init schedule)
        match schedule_table.try_insert(PlayerStatSchedule {
            id: 0, // Auto-incremented
            scheduled_at: ScheduleAt::Interval(interval.into()),
        }) {
            Ok(_) => log::info!("Player stat schedule inserted."),
            Err(e) => log::error!("Failed to insert player stat schedule: {}", e),
        };
    } else {
        log::debug!("Player stat schedule already exists.");
    }
    Ok(())
}

// --- Reducer to Process ALL Player Stat Updates (Scheduled) ---
// --- Reducer to Process ALL Player Stat Updates (Scheduled) ---
#[spacetimedb::reducer]
pub fn process_player_stats(ctx: &ReducerContext, _schedule: PlayerStatSchedule) -> Result<(), String> {
    log::trace!("Processing player stats via schedule...");
    let current_time = ctx.timestamp;
    let players = ctx.db.player();
    let players_stats = ctx.db.player_stats();
    let world_states = ctx.db.world_state();
    let campfires = ctx.db.campfire();

    let world_state = world_states.iter().next()
        .ok_or_else(|| "WorldState not found during stat processing".to_string())?;

    for player_ref in players.iter() {
        let mut player = player_ref.clone();
        let mut player_stats = players_stats.player_id().find(player.identity).ok_or_else(|| "Player Stats not found during stat processing".to_string())?;
        let player_id = player.identity;

        if player.is_dead {
            continue;
        }

        // Use the dedicated stat update timestamp
        let last_stat_update_time = player.last_stat_update;
        let elapsed_micros = current_time.to_micros_since_unix_epoch().saturating_sub(last_stat_update_time.to_micros_since_unix_epoch());

        let elapsed_seconds = (elapsed_micros as f64 / 1_000_000.0) as f32;

        // Calculate Warmth
        let mut warmth_change_per_sec: f32 = 0.0;
        let drain_multiplier = match world_state.time_of_day {
            TimeOfDay::Morning | TimeOfDay::Noon | TimeOfDay::Afternoon => 0.0,
            TimeOfDay::Dawn | TimeOfDay::Dusk => WARMTH_DRAIN_MULTIPLIER_DAWN_DUSK,
            TimeOfDay::Night => WARMTH_DRAIN_MULTIPLIER_NIGHT * 1.25,
            TimeOfDay::Midnight => WARMTH_DRAIN_MULTIPLIER_MIDNIGHT * 1.33,
        };
        warmth_change_per_sec -= BASE_WARMTH_DRAIN_PER_SECOND * drain_multiplier;

        for fire in campfires.iter() {
            // Only gain warmth from burning campfires
            if fire.is_burning {
                let dx = player.position_x - fire.pos_x;
                let dy = player.position_y - fire.pos_y;
                if (dx * dx + dy * dy) < WARMTH_RADIUS_SQUARED {
                    warmth_change_per_sec += WARMTH_PER_SECOND;
                    log::trace!("Player {:?} gaining warmth from campfire {}", player_id, fire.id);
                }
            }
        }

        let mut new_sprinting_state = player.is_sprinting; // Start with current state
        let mut new_stamina = player_stats.stamina; // Initialize with current stamina

        // Check if player likely moved since last stat update
        let likely_moved = player.last_update > player.last_stat_update;

        if new_sprinting_state && likely_moved {
            // Apply drain if sprinting and likely moved
            new_stamina = (new_stamina - (elapsed_seconds * STAMINA_DRAIN_PER_SECOND)).max(0.0);
            if new_stamina <= 0.0 {
                new_sprinting_state = false; // Force sprinting off if out of stamina
                log::debug!("Player {:?} ran out of stamina (stat tick).", player_id);
            }
        } else if !new_sprinting_state {
            // Apply recovery only if not sprinting (or just stopped sprinting this tick)
            new_stamina = (new_stamina + (elapsed_seconds * STAMINA_RECOVERY_PER_SECOND)).min(100.0);
        }

        // Calculate Health
        let mut health_change_per_sec: f32 = 0.0;

        // Health recovery only if needs are met and not taking damage
        if health_change_per_sec == 0.0 && player_stats.health < 100.0 && player_stats.health > HEALTH_RECOVERY_THRESHOLD {
            health_change_per_sec += HEALTH_RECOVERY_PER_SEC;
        }

        let new_health = (player_stats.health + (health_change_per_sec * elapsed_seconds))
            .max(0.0).min(100.0);

        // --- Death Check ---
        let mut player_died = false;
        let mut calculated_respawn_at = player.respawn_at; // Keep existing value by default
        if player_stats.health > 0.0 && new_health <= 0.0 {
            player_died = true;
            calculated_respawn_at = ctx.timestamp + Duration::from_secs(5); // Set respawn time
            log::warn!("Player {} ({:?}) has died due to stat drain! Will be respawnable at {:?}",
                     player.username, player_id, calculated_respawn_at);

            // Unequip item on death
            // Call unequip using the context and the specific player's identity
            match active_equipment::unequip_item(ctx, player_id) {
                Ok(_) => log::info!("Unequipped item for dying player {:?}", player_id),
                Err(e) => log::error!("Failed to unequip item for dying player {:?}: {}", player_id, e),
            }
        }

        // --- Update Player Table ---
        // Only update if something actually changed
        let stats_changed = (player_stats.health - new_health).abs() > 0.01 ||
            (player_stats.stamina - new_stamina).abs() > 0.01 ||
            (player.is_sprinting != new_sprinting_state) || // Check if sprint state changed
            player_died; // Also update if other stats changed OR if player died

        if stats_changed {
            player_stats.health = new_health;
            player_stats.stamina = new_stamina;
            player.is_dead = player_died;
            player.respawn_at = calculated_respawn_at;
            player.is_sprinting = new_sprinting_state; // Update sprint state if changed
            // Note: We don't update position, direction here

            // ALWAYS update last_stat_update timestamp after processing
            player.last_stat_update = current_time;

            players_stats.player_id().update(player_stats);
            players.identity().update(player);
            log::debug!("Updated stats for player {:?}", player_id);
        } else {
            log::trace!("No significant stat changes for player {:?}, skipping update.", player_id);
            // Still update the stat timestamp even if nothing changed, to prevent large future deltas
            player.last_stat_update = current_time;
            players_stats.player_id().update(player_stats);
            players.identity().update(player);
            log::trace!("Updated player {:?} last_stat_update timestamp anyway.", player_id);
        }
    }

    // No rescheduling needed here, the table's ScheduleAt::Interval handles it
    Ok(())
}

// Implement the initialize_player_stats function
pub(crate) fn initialize_player_stats(ctx: &ReducerContext, player_id: Identity) -> Result<(), String> {
    let player_stats_table = ctx.db.player_stats();

    // Check if player stats already exist
    if player_stats_table.player_id().find(&player_id).is_some() {
        log::debug!("Player stats for {:?} already exist", player_id);
        return Ok(());
    }

    // Initialize new player stats add Character buffs
    let stats = PlayerStats {
        player_id,
        level: 1,
        experience: 0.0,
        experience_to_next_level: BASE_EXP_TO_LEVEL,
        health: 100.0,
        attack: 10.0,
        attack_speed: 1.0,
        move_speed: 5.0,
        hp_regen: 0.5,
        armor: 0.0,
        stamina: 100.0,
    };

    player_stats_table.try_insert(stats)
        .map_err(|e| format!("Failed to initialize player stats: {}", e))?;

    log::info!("Initialized stats for player {:?}", player_id);
    Ok(())
}