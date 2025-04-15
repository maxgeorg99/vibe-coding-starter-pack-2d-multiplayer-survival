use spacetimedb::{Identity, Table, Timestamp};

// --- Constants ---
pub(crate) const CAMPFIRE_COLLISION_RADIUS: f32 = 18.0; // Smaller than player radius
pub(crate) const CAMPFIRE_COLLISION_Y_OFFSET: f32 = 10.0; // Y offset for collision checking (relative to fire's center)
pub(crate) const PLAYER_CAMPFIRE_COLLISION_DISTANCE_SQUARED: f32 = (super::PLAYER_RADIUS + CAMPFIRE_COLLISION_RADIUS) * (super::PLAYER_RADIUS + CAMPFIRE_COLLISION_RADIUS);
pub(crate) const CAMPFIRE_CAMPFIRE_COLLISION_DISTANCE_SQUARED: f32 = (CAMPFIRE_COLLISION_RADIUS * 2.0) * (CAMPFIRE_COLLISION_RADIUS * 2.0); // Prevent placing campfires too close

pub(crate) const WARMTH_RADIUS: f32 = 150.0; // How far the warmth effect reaches
pub(crate) const WARMTH_RADIUS_SQUARED: f32 = WARMTH_RADIUS * WARMTH_RADIUS;
pub(crate) const WARMTH_PER_SECOND: f32 = 5.0; // How much warmth is gained per second near a fire

// TODO: Add lifetime/fuel later?

#[spacetimedb::table(name = campfire, public)]
#[derive(Clone)]
pub struct Campfire {
    #[primary_key]
    #[auto_inc]
    pub id: u32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub placed_by: Identity, // Track who placed it
    pub placed_at: Timestamp,
    // pub fuel: f32, // Example for future fuel mechanic
} 