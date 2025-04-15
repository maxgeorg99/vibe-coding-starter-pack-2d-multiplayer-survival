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
    pub last_update: Timestamp,
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
    
    // Generate a color for the player based on their username
    let color = random_color(&username);
    
    // Create a new player with initial position in the center of the map
    let player = Player {
        identity: sender_id,
        username: username.clone(), // Clone username here
        position_x: 640.0, // Center of a 20x64 tile world
        position_y: 480.0, // Center of a 15x64 tile world
        color,
        last_update: ctx.timestamp,
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
    if (final_x != current_player.position_x || final_y != current_player.position_y) || !collision_detected {
         // Create updated player data only if needed
        let updated_player = Player {
            position_x: final_x,
            position_y: final_y,
            last_update: ctx.timestamp,
            ..current_player // Clone other fields (identity, username, color)
        };
        
        // Update player in the database
        players.identity().update(updated_player);
    } // Else: Do nothing if collision reverted or position didn't change

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