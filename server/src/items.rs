use spacetimedb::{ReducerContext, SpacetimeType, Table};
use log;

// --- Item Enums and Structs ---

// Define categories or types for items
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, SpacetimeType)]
pub enum ItemCategory {
    Tool,
    Material,
    Placeable,
    // Add other categories as needed (Consumable, Wearable, etc.)
}

#[spacetimedb::table(name = item_definition, public)]
#[derive(Clone)]
pub struct ItemDefinition {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub name: String,          // Unique name used as an identifier too?
    pub description: String,   // Optional flavor text
    pub category: ItemCategory,
    pub icon_asset_name: String, // e.g., "stone_hatchet.png", used by client
    pub damage: Option<u32>,   // Damage dealt (e.g., by tools)
    pub is_stackable: bool,    // Can multiple instances exist in one inventory slot?
    pub stack_size: u32,       // Max number per stack (if stackable)
    pub is_equippable: bool,   // Can this item be visibly equipped by the player?
}

// --- Inventory Table ---

// Represents an instance of an item in a player's inventory
#[spacetimedb::table(name = inventory_item, public)]
#[derive(Clone)]
pub struct InventoryItem {
    #[primary_key]
    #[auto_inc]
    pub instance_id: u64,      // Unique ID for this specific item instance
    pub player_identity: spacetimedb::Identity, // Who owns this item
    pub item_def_id: u64,      // Links to ItemDefinition table (FK)
    pub quantity: u32,         // How many of this item
    pub hotbar_slot: Option<u8>, // Which hotbar slot (0-5), if any
    // Add other instance-specific data later (e.g., current_durability)
}

// --- Item Reducers ---

// Reducer to seed initial item definitions if the table is empty
#[spacetimedb::reducer]
pub fn seed_items(ctx: &ReducerContext) -> Result<(), String> {
    let items = ctx.db.item_definition();
    if items.iter().count() > 0 {
        log::info!("Item definitions already seeded ({}). Skipping.", items.iter().count());
        return Ok(());
    }

    log::info!("Seeding initial item definitions...");

    let initial_items = vec![
        ItemDefinition {
            id: 0,
            name: "Wood".to_string(),
            description: "A sturdy piece of wood.".to_string(),
            category: ItemCategory::Material,
            icon_asset_name: "wood.png".to_string(),
            damage: None,
            is_stackable: true,
            stack_size: 1000,
            is_equippable: false, // Materials are not equippable
        },
        ItemDefinition {
            id: 0,
            name: "Stone".to_string(),
            description: "A chunk of rock.".to_string(),
            category: ItemCategory::Material,
            icon_asset_name: "stone.png".to_string(),
            damage: None,
            is_stackable: true,
            stack_size: 1000,
            is_equippable: false, // Materials are not equippable
        },
        ItemDefinition {
            id: 0,
            name: "Stone Hatchet".to_string(),
            description: "A simple hatchet for chopping wood.".to_string(),
            category: ItemCategory::Tool,
            icon_asset_name: "wood_hatchet.png".to_string(),
            damage: Some(5),
            is_stackable: false,
            stack_size: 1,
            is_equippable: true, // Hatchet is equippable
        },
        ItemDefinition {
            id: 0,
            name: "Stone Pickaxe".to_string(),
            description: "A simple pickaxe for breaking rocks.".to_string(),
            category: ItemCategory::Tool,
            icon_asset_name: "pick_axe.png".to_string(),
            damage: Some(5),
            is_stackable: false,
            stack_size: 1,
            is_equippable: true, // Pickaxe is equippable
        },
        ItemDefinition {
            id: 0,
            name: "Camp Fire".to_string(),
            description: "Provides warmth and light. Requires fuel.".to_string(),
            category: ItemCategory::Placeable,
            icon_asset_name: "campfire.png".to_string(),
            damage: None,
            is_stackable: false,
            stack_size: 1,
            is_equippable: false, // Campfire is placeable, not equippable
        },
    ];

    let mut seeded_count = 0;
    for item_def in initial_items {
        match items.try_insert(item_def) {
            Ok(_) => seeded_count += 1,
            Err(e) => log::error!("Failed to insert item definition during seeding: {}", e),
        }
    }

    log::info!("Finished seeding {} item definitions.", seeded_count);
    Ok(())
} 