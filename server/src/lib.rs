use spacetimedb::{Identity, Timestamp, ReducerContext, Table};
use log;

const PLAYER_RADIUS: f32 = 20.0;
const PLAYER_DIAMETER_SQUARED: f32 = (PLAYER_RADIUS * 2.0) * (PLAYER_RADIUS * 2.0);

// Passive Stat Drain Rates
const HUNGER_DRAIN_PER_SECOND: f32 = 100.0 / (30.0 * 60.0); // 100 hunger over 30 mins
const THIRST_DRAIN_PER_SECOND: f32 = 100.0 / (20.0 * 60.0); // 100 thirst over 20 mins
const STAMINA_DRAIN_PER_SECOND: f32 = 20.0; // Stamina drains fast while sprinting
const STAMINA_RECOVERY_PER_SECOND: f32 = 5.0;  // Stamina recovers slower
const SPRINT_SPEED_MULTIPLIER: f32 = 1.5;     // 50% faster sprint

// Player table to store position and color
#[spacetimedb::table(name = player, public)]
#[derive(Clone)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
    pub username: String,
    pub position_x: f32,
    pub position_y: f32,
    pub color: String, // Hex color code (e.g., "#FF0000" for red)
    pub direction: String, // "up", "down", "left", "right"
    pub last_update: Timestamp,
    pub jump_start_time_ms: u64, // Timestamp when the jump started (0 if not jumping)
    // New status fields
    pub health: f32,
    pub stamina: f32,
    pub thirst: f32,
    pub hunger: f32,
    pub warmth: f32,
    pub is_sprinting: bool, // Is the player currently trying to sprint?
}

// When a client connects, we need to create a player for them
#[spacetimedb::reducer]
pub fn identity_connected(_ctx: &ReducerContext) {
    // Identity connected handler is automatically called when a player connects
    // We'll handle player creation in the register_player reducer instead
}

// When a client disconnects, we need to clean up
#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    log::info!("identity_disconnected triggered for identity: {:?}", ctx.sender);
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    
    if let Some(player) = players.identity().find(sender_id) {
        let username = player.username.clone();
        players.identity().delete(sender_id);
        log::info!("Attempted delete for disconnected player: {} ({:?})", username, sender_id);
    }
}

// Register a new player
#[spacetimedb::reducer]
pub fn register_player(ctx: &ReducerContext, username: String) -> Result<(), String> {
    log::info!("register_player called by {:?} with username: {}", ctx.sender, username);
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    
    // Check if username is already taken by *any* player
    let username_taken = players.iter().any(|p| p.username == username);
    if username_taken {
        log::warn!("Username '{}' already taken. Registration failed for {:?}.", username, sender_id);
        return Err(format!("Username '{}' is already taken.", username));
    }
    
    // Check if this identity is already registered (shouldn't happen if disconnect works, but good safety check)
    if players.identity().find(sender_id).is_some() {
        log::warn!("Identity {:?} already registered. Registration failed.", sender_id);
        return Err("Player identity already registered".to_string());
    }
    
    // --- Find a valid spawn position --- 
    let initial_x = 640.0; 
    let initial_y = 480.0;
    let mut spawn_x = initial_x;
    let mut spawn_y = initial_y;
    let max_attempts = 10; // Prevent infinite loops
    let offset_step = PLAYER_RADIUS * 2.5; // How far to step when looking for space
    let mut attempt = 0;

    loop {
        let mut collision = false;
        for other_player in players.iter() {
            let dx = spawn_x - other_player.position_x;
            let dy = spawn_y - other_player.position_y;
            if (dx * dx + dy * dy) < PLAYER_DIAMETER_SQUARED {
                collision = true;
                break;
            }
        }

        if !collision || attempt >= max_attempts {
            if attempt >= max_attempts {
                 log::warn!("Could not find clear spawn point for {}, spawning at default.", username);
                 // Fallback to initial position even if colliding
                 spawn_x = initial_x;
                 spawn_y = initial_y;
            }
            break; // Found a spot or gave up
        }

        // Simple offset pattern: move right, down, left, up, then spiral out slightly?
        // This is basic, could be improved (random, spiral search)
        match attempt % 4 {
            0 => spawn_x += offset_step, // Try right
            1 => spawn_y += offset_step, // Try down
            2 => spawn_x -= offset_step * 2.0, // Try left (further)
            3 => spawn_y -= offset_step * 2.0, // Try up (further)
            _ => {}, // Should not happen
        }
        // Reset to center if offset gets too wild after a few attempts (basic safeguard)
        if attempt == 5 { 
             spawn_x = initial_x;
             spawn_y = initial_y;
             spawn_x += offset_step * 1.5; // Try a different diagonal
             spawn_y += offset_step * 1.5;
        }

        attempt += 1;
    }
    // --- End spawn position logic ---

    let color = random_color(&username);
    
    let player = Player {
        identity: sender_id,
        username: username.clone(), 
        position_x: spawn_x, // Use the found spawn position
        position_y: spawn_y, // Use the found spawn position
        color,
        direction: "down".to_string(), 
        last_update: ctx.timestamp,
        jump_start_time_ms: 0, // Initialize as not jumping
        // Initialize new status fields
        health: 100.0,
        stamina: 100.0,
        thirst: 100.0,
        hunger: 100.0,
        warmth: 100.0,
        is_sprinting: false, // Initialize sprint state
    };
    
    // Insert the new player
    match players.try_insert(player) {
        Ok(_) => {
            log::info!("Player registered: {}", username);
            Ok(())
        },
        Err(e) => {
            Err(format!("Failed to register player: {}", e))
        }
    }
}

// NEW Reducer: Called by the client to set the sprinting state
#[spacetimedb::reducer]
pub fn set_sprinting(ctx: &ReducerContext, sprinting: bool) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();

    if let Some(mut player) = players.identity().find(&sender_id) {
        // Only update if the state is actually changing
        if player.is_sprinting != sprinting {
            player.is_sprinting = sprinting;
            // Also update last_update time so stamina regen/drain calculation is accurate on next movement
            player.last_update = ctx.timestamp; 
            players.identity().update(player);
            log::debug!("Player {:?} set sprinting to {}", sender_id, sprinting);
        }
        Ok(())
    } else {
        Err("Player not found".to_string())
    }
}

// Update player movement, handle sprinting, stats, and collision
#[spacetimedb::reducer]
pub fn update_player_position(
    ctx: &ReducerContext, 
    move_dx: f32, // Delta X requested by client (based on base speed)
    move_dy: f32  // Delta Y requested by client (based on base speed)
) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    
    let current_player = players.identity()
        .find(sender_id)
        .ok_or_else(|| "Player not found".to_string())?;

    let now = ctx.timestamp;
    let last_update_time = current_player.last_update;
    let elapsed_micros = now.to_micros_since_unix_epoch().saturating_sub(last_update_time.to_micros_since_unix_epoch());
    let elapsed_seconds = (elapsed_micros as f64 / 1_000_000.0) as f32;

    // --- Calculate Stat Changes --- 
    let mut new_hunger = (current_player.hunger - (elapsed_seconds * HUNGER_DRAIN_PER_SECOND)).max(0.0);
    let mut new_thirst = (current_player.thirst - (elapsed_seconds * THIRST_DRAIN_PER_SECOND)).max(0.0);
    
    let mut new_stamina = current_player.stamina;
    let mut actual_speed_multiplier = 1.0;
    let mut current_sprinting_state = current_player.is_sprinting;
    let is_moving = move_dx != 0.0 || move_dy != 0.0;

    if current_sprinting_state && is_moving && new_stamina > 0.0 {
        // Drain stamina if sprinting, moving, and has stamina
        new_stamina = (new_stamina - (elapsed_seconds * STAMINA_DRAIN_PER_SECOND)).max(0.0);
        if new_stamina > 0.0 { // Can still sprint
            actual_speed_multiplier = SPRINT_SPEED_MULTIPLIER;
        } else { // Ran out of stamina
            current_sprinting_state = false; // Force stop sprinting this update cycle
            log::debug!("Player {:?} ran out of stamina.", sender_id);
        }
    } else if !current_sprinting_state {
        // Recover stamina if not trying to sprint
        new_stamina = (new_stamina + (elapsed_seconds * STAMINA_RECOVERY_PER_SECOND)).min(100.0);
    }
    // Note: If player stops moving while holding sprint, stamina doesn't drain but also doesn't recover here.
    // It will recover on the next update where is_sprinting is false or is_moving is false.

    // --- Calculate Target Position --- 
    let target_x = current_player.position_x + move_dx * actual_speed_multiplier;
    let target_y = current_player.position_y + move_dy * actual_speed_multiplier;

    // --- Collision Detection --- 
    let mut final_x = target_x;
    let mut final_y = target_y;
    let mut collision_detected = false;

    for other_player in players.iter() {
        if other_player.identity == sender_id {
            continue;
        }
        let dx = target_x - other_player.position_x;
        let dy = target_y - other_player.position_y;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq < PLAYER_DIAMETER_SQUARED {
            collision_detected = true;
            // Revert to original position - simple collision resolution
            final_x = current_player.position_x;
            final_y = current_player.position_y;
            log::debug!("Collision detected between {:?} and {:?}. Movement reverted.", sender_id, other_player.identity);
            break; 
        }
    }

    // --- Determine Update & Final State --- 
    let actual_dx = final_x - current_player.position_x;
    let actual_dy = final_y - current_player.position_y;
    let position_changed = actual_dx != 0.0 || actual_dy != 0.0;
    // Update if position changed, or if enough time passed for passive drains/regen
    let should_update = position_changed || elapsed_seconds > 0.1; 

    if should_update {
        let mut direction = current_player.direction.clone(); 
        if position_changed { // Only update direction if actual movement occurred
            if actual_dx.abs() > actual_dy.abs() { 
                if actual_dx > 0.0 { direction = "right".to_string(); }
                else if actual_dx < 0.0 { direction = "left".to_string(); }
            } else if actual_dy.abs() > actual_dx.abs() { 
                if actual_dy > 0.0 { direction = "down".to_string(); }
                else if actual_dy < 0.0 { direction = "up".to_string(); }
            } 
        }

        let updated_player = Player {
            position_x: final_x,
            position_y: final_y,
            direction, 
            last_update: now,
            hunger: new_hunger,
            thirst: new_thirst,
            stamina: new_stamina, // Update stamina
            is_sprinting: current_sprinting_state, // Reflect if we forced sprint off due to low stamina
            ..current_player // Clone other fields (health, warmth, color, username, jump_start_time_ms, identity)
        };
        
        players.identity().update(updated_player);
    } 

    Ok(())
}

// Helper function to generate a deterministic color based on username
fn random_color(username: &str) -> String {
    let colors = [
        "#FF0000", // Red
        "#00FF00", // Green
        "#0000FF", // Blue
        "#FFFF00", // Yellow
        "#FF00FF", // Magenta
        "#00FFFF", // Cyan
        "#FF8000", // Orange
        "#8000FF", // Purple
    ];
    
    // Use the username bytes to select a color
    // Sum the bytes and take modulo of the number of colors
    let username_bytes = username.as_bytes();
    let sum_of_bytes: u64 = username_bytes.iter().map(|&byte| byte as u64).sum();
    let color_index = (sum_of_bytes % colors.len() as u64) as usize;
    
    colors[color_index].to_string()
}

// Reducer called by the client to initiate a jump.
#[spacetimedb::reducer]
pub fn jump(ctx: &ReducerContext) -> Result<(), String> {
   let identity = ctx.sender;
   let players = ctx.db.player();

   if let Some(mut player) = players.identity().find(&identity) {
       // Get microseconds since epoch and convert to milliseconds u64
       let now_micros = ctx.timestamp.to_micros_since_unix_epoch();
       let now_ms = (now_micros / 1000) as u64;

       player.jump_start_time_ms = now_ms;
       player.last_update = ctx.timestamp;

       players.identity().update(player);
       Ok(())
   } else {
       Err("Player not found".to_string())
   }
} 