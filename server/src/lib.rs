use spacetimedb::{Identity, Timestamp, ReducerContext, Table};
use log;

const PLAYER_RADIUS: f32 = 20.0;
const PLAYER_DIAMETER_SQUARED: f32 = (PLAYER_RADIUS * 2.0) * (PLAYER_RADIUS * 2.0);

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

// Update player movement with collision detection
#[spacetimedb::reducer]
pub fn update_player_position(
    ctx: &ReducerContext, 
    proposed_x: f32, // Rename to indicate it's a proposal
    proposed_y: f32
) -> Result<(), String> {
    let sender_id = ctx.sender;
    let players = ctx.db.player();
    
    // Find the current player
    let current_player = players.identity()
        .find(sender_id)
        .ok_or_else(|| "Player not found".to_string())?; // Use ok_or_else for String error

    let mut final_x = proposed_x;
    let mut final_y = proposed_y;
    let mut collision_detected = false;

    // Iterate through all other players to check for collisions
    for other_player in players.iter() {
        // Skip self-check
        if other_player.identity == sender_id {
            continue;
        }

        // Calculate squared distance between proposed position and other player
        let dx = proposed_x - other_player.position_x;
        let dy = proposed_y - other_player.position_y;
        let dist_sq = dx * dx + dy * dy;

        // Check for collision (overlap)
        if dist_sq < PLAYER_DIAMETER_SQUARED {
            collision_detected = true;
            // Simple resolution: Prevent movement by reverting to current position
            // More advanced: Calculate position just before collision or slide
            final_x = current_player.position_x;
            final_y = current_player.position_y;
            log::debug!("Collision detected between {:?} and {:?}. Movement reverted.", sender_id, other_player.identity);
            break; // Stop checking after first collision for this simple resolution
        }
    }

    // Only update if position actually changed and no collision stopped it
    if (final_x != current_player.position_x || final_y != current_player.position_y) && !collision_detected {
        // Determine direction based on movement
        let dx = final_x - current_player.position_x;
        let dy = final_y - current_player.position_y;
        let mut direction = current_player.direction.clone(); // Keep old direction if no move

        if dx.abs() > dy.abs() { // Horizontal movement is dominant
            if dx > 0.0 { direction = "right".to_string(); }
            else if dx < 0.0 { direction = "left".to_string(); }
        } else if dy.abs() > dx.abs() { // Vertical movement is dominant
            if dy > 0.0 { direction = "down".to_string(); }
            else if dy < 0.0 { direction = "up".to_string(); }
        } // If dx == dy == 0, direction remains unchanged

        let updated_player = Player {
            position_x: final_x,
            position_y: final_y,
            direction, // Set the calculated direction
            last_update: ctx.timestamp,
            ..current_player // Clone other fields (including the existing jump_start_time_ms)
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