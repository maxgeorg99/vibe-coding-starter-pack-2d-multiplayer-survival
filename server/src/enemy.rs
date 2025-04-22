use spacetimedb::{Identity, ReducerContext, Table, Timestamp};
use log;
use std::time::Duration;
use rand::Rng;
use crate::Player;
use crate::PLAYER_RADIUS;
use crate::WORLD_WIDTH_PX;
use crate::WORLD_HEIGHT_PX;

// --- Enemy Constants ---
const ENEMY_SPAWN_RADIUS: f32 = 500.0; // Distance from player where enemies can spawn
const ENEMY_MOVE_SPEED: f32 = 50.0; // Pixels per second
const ENEMY_DAMAGE: f32 = 5.0; // Damage per hit
const ENEMY_ATTACK_COOLDOWN_MS: u64 = 1000; // 1 second between attacks
const ENEMY_SPAWN_INTERVAL_MS: u64 = 5000; // Spawn new enemies every 5 seconds
const MAX_ENEMIES: u32 = 50; // Maximum number of enemies in the world

// --- Enemy Types ---
#[derive(Clone, Debug)]
pub enum EnemyType {
    Basic,    // Basic enemy with standard stats
    Fast,     // Faster but less health
    Tank,     // Slower but more health
    Elite,    // Balanced but stronger
}

// --- Enemy Struct ---
#[spacetimedb::table(name = enemy, public)]
#[derive(Clone)]
pub struct Enemy {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub enemy_type: EnemyType,
    pub pos_x: f32,
    pub pos_y: f32,
    pub health: f32,
    pub max_health: f32,
    pub move_speed: f32,
    pub damage: f32,
    pub last_attack_time: Option<Timestamp>,
    pub target_player_id: Option<Identity>,
}

// --- Enemy Spawner ---
#[spacetimedb::reducer]
pub fn spawn_enemies(ctx: &ReducerContext) -> Result<(), String> {
    let now_ts = ctx.timestamp;
    let enemies = ctx.db.enemy();
    let players = ctx.db.player();

    // Count current enemies
    let current_enemy_count = enemies.iter().count() as u32;
    if current_enemy_count >= MAX_ENEMIES {
        return Ok(()); // Don't spawn more if at max
    }

    // Get all alive players
    let alive_players: Vec<&Player> = players.iter()
        .filter(|p| !p.is_dead)
        .collect();

    if alive_players.is_empty() {
        return Ok(()); // No players to spawn enemies around
    }

    // Spawn enemies around each player
    for player in alive_players {
        let mut rng = rand::thread_rng();
        
        // Random angle and distance for spawn position
        let angle = rng.gen_range(0.0..2.0 * std::f32::consts::PI);
        let distance = rng.gen_range(ENEMY_SPAWN_RADIUS * 0.8..ENEMY_SPAWN_RADIUS);
        
        let spawn_x = player.position_x + angle.cos() * distance;
        let spawn_y = player.position_y + angle.sin() * distance;

        // Clamp to world bounds
        let spawn_x = spawn_x.max(PLAYER_RADIUS).min(WORLD_WIDTH_PX - PLAYER_RADIUS);
        let spawn_y = spawn_y.max(PLAYER_RADIUS).min(WORLD_HEIGHT_PX - PLAYER_RADIUS);

        // Randomly choose enemy type
        let enemy_type = match rng.gen_range(0..4) {
            0 => EnemyType::Basic,
            1 => EnemyType::Fast,
            2 => EnemyType::Tank,
            3 => EnemyType::Elite,
            _ => EnemyType::Basic,
        };

        // Create enemy with type-specific stats
        let (health, move_speed, damage) = match enemy_type {
            EnemyType::Basic => (50.0, ENEMY_MOVE_SPEED, ENEMY_DAMAGE),
            EnemyType::Fast => (30.0, ENEMY_MOVE_SPEED * 1.5, ENEMY_DAMAGE * 0.8),
            EnemyType::Tank => (100.0, ENEMY_MOVE_SPEED * 0.7, ENEMY_DAMAGE * 1.2),
            EnemyType::Elite => (75.0, ENEMY_MOVE_SPEED * 1.2, ENEMY_DAMAGE * 1.5),
        };

        let enemy = Enemy {
            id: 0, // Auto-incremented
            enemy_type,
            pos_x: spawn_x,
            pos_y: spawn_y,
            health,
            max_health: health,
            move_speed,
            damage,
            last_attack_time: None,
            target_player_id: Some(player.identity),
        };

        enemies.insert(enemy);
    }

    Ok(())
}

// --- Enemy Movement and Attack ---
#[spacetimedb::reducer]
pub fn update_enemies(ctx: &ReducerContext) -> Result<(), String> {
    let now_ts = ctx.timestamp;
    let enemies = ctx.db.enemy();
    let players = ctx.db.player();
    let player_stats = ctx.db.player_stats();

    for mut enemy in enemies.iter() {
        // Skip if no target
        let target_player_id = match enemy.target_player_id {
            Some(id) => id,
            None => continue,
        };

        // Get target player
        let target_player = match players.identity().find(target_player_id) {
            Some(p) => p,
            None => continue, // Target player not found
        };

        if target_player.is_dead {
            continue; // Skip dead players
        }

        // Calculate direction to player
        let dx = target_player.position_x - enemy.pos_x;
        let dy = target_player.position_y - enemy.pos_y;
        let distance_sq = dx * dx + dy * dy;
        let distance = distance_sq.sqrt();

        // Move towards player
        if distance > PLAYER_RADIUS * 2.0 {
            let move_dx = (dx / distance) * enemy.move_speed;
            let move_dy = (dy / distance) * enemy.move_speed;
            
            enemy.pos_x += move_dx;
            enemy.pos_y += move_dy;
        }
        // Attack if close enough
        else if distance <= PLAYER_RADIUS * 2.0 {
            let can_attack = match enemy.last_attack_time {
                Some(last_attack) => {
                    let elapsed_ms = now_ts.duration_since(last_attack).as_millis() as u64;
                    elapsed_ms >= ENEMY_ATTACK_COOLDOWN_MS
                },
                None => true,
            };

            if can_attack {
                // Get player stats for armor calculation
                let player_stats = player_stats.player_id().find(target_player_id);
                let armor_reduction = player_stats.map(|stats| stats.base_armor).unwrap_or(0.0);
                
                // Calculate damage with armor reduction
                let damage = enemy.damage * (1.0 - armor_reduction.min(0.8)); // Cap armor at 80% reduction
                
                // Apply damage to player
                let mut player = target_player;
                player.health = (player.health - damage).max(0.0);
                player.last_hit_time = Some(now_ts);
                
                // Check for death
                if player.health <= 0.0 && !player.is_dead {
                    player.is_dead = true;
                    let respawn_micros = now_ts.to_micros_since_unix_epoch().saturating_add((5000 * 1000) as i64);
                    player.respawn_at = Timestamp::from_micros_since_unix_epoch(respawn_micros);
                }

                players.identity().update(player);
                enemy.last_attack_time = Some(now_ts);
            }
        }

        // Update enemy position
        enemies.id().update(enemy);
    }

    Ok(())
}

// --- Enemy Damage Handler ---
#[spacetimedb::reducer]
pub fn damage_enemy(ctx: &ReducerContext, enemy_id: u64, damage: f32) -> Result<(), String> {
    let enemies = ctx.db.enemy();
    let mut enemy = enemies.id().find(enemy_id)
        .ok_or_else(|| format!("Enemy {} not found", enemy_id))?;

    enemy.health = (enemy.health - damage).max(0.0);
    
    if enemy.health <= 0.0 {
        // Grant experience to the attacker
        if let Some(player_id) = ctx.sender {
            if let Some(mut player) = ctx.db.player().identity().find(player_id) {
                // Calculate experience based on enemy type
                let exp_gain = match enemy.enemy_type {
                    EnemyType::Basic => BASE_EXP_PER_KILL,
                    EnemyType::Fast => BASE_EXP_PER_KILL * 1.2,
                    EnemyType::Tank => BASE_EXP_PER_KILL * 1.5,
                    EnemyType::Elite => BASE_EXP_PER_KILL * 2.0,
                };

                // Add experience through the player_stats system
                if let Err(e) = crate::player_stats::add_experience(ctx, exp_gain) {
                    log::error!("Failed to add experience to player {:?}: {}", player_id, e);
                }

                log::info!("Player {:?} killed enemy {} (type: {:?}) and gained {} exp", 
                    player_id, enemy_id, enemy.enemy_type, exp_gain);
                ctx.db.player().identity().update(player);
            }
        }
        enemies.id().delete(enemy_id);
    } else {
        enemies.id().update(enemy);
    }

    Ok(())
} 