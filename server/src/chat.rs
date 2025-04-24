// server/src/chat.rs
//
// Module for managing chat functionality including messages and related
// operations in the multiplayer game.

use spacetimedb::{ReducerContext, Identity, Timestamp, Table};
use log;

// --- Table Definitions ---

#[spacetimedb::table(name = message, public)]
#[derive(Clone, Debug)]
pub struct Message {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub sender: Identity,
    pub text: String,
    pub sent: Timestamp, // Timestamp for sorting
}

// --- Reducers ---

/// Sends a chat message that will be visible to all players
#[spacetimedb::reducer]
pub fn send_message(ctx: &ReducerContext, text: String) -> Result<(), String> {
    if text.is_empty() {
        return Err("Message cannot be empty.".to_string());
    }
    if text.len() > 100 { // Match client-side max length
        return Err("Message too long (max 100 characters).".to_string());
    }

    let new_message = Message {
        id: 0, // Auto-incremented
        sender: ctx.sender,
        text: text.clone(), // Clone text for logging after potential move
        sent: ctx.timestamp,
    };

    log::info!("User {} sent message: {}", ctx.sender, text); // Log the message content
    
    // Use the database context handle to insert
    ctx.db.message().insert(new_message);

    Ok(())
}

// Could add more chat-related functionality in the future:
// - Private messages
// - Chat filtering
// - Chat commands/emotes
// - Chat history management (pruning old messages) 