import { useState, useEffect, useRef } from 'react';
import * as SpacetimeDB from '../generated';
import { DbConnection } from '../generated'; // Import the connection type

// Define the shape of the state returned by the hook
export interface SpacetimeTableStates {
    players: Map<string, SpacetimeDB.Player>;
    trees: Map<string, SpacetimeDB.Tree>;
    stones: Map<string, SpacetimeDB.Stone>;
    campfires: Map<string, SpacetimeDB.Campfire>;
    mushrooms: Map<string, SpacetimeDB.Mushroom>;
    itemDefinitions: Map<string, SpacetimeDB.ItemDefinition>;
    inventoryItems: Map<string, SpacetimeDB.InventoryItem>;
    worldState: SpacetimeDB.WorldState | null;
    activeEquipments: Map<string, SpacetimeDB.ActiveEquipment>;
    droppedItems: Map<string, SpacetimeDB.DroppedItem>;
    woodenStorageBoxes: Map<string, SpacetimeDB.WoodenStorageBox>;
    recipes: Map<string, SpacetimeDB.Recipe>;
    craftingQueueItems: Map<string, SpacetimeDB.CraftingQueueItem>;
    localPlayerRegistered: boolean; // Flag indicating local player presence
}

// Define the props the hook accepts
interface UseSpacetimeTablesProps {
    connection: DbConnection | null;
    cancelPlacement: () => void; // Function to cancel placement mode
}

export const useSpacetimeTables = ({
    connection,
    cancelPlacement,
}: UseSpacetimeTablesProps): SpacetimeTableStates => {
    // --- State Management for Tables ---
    const [players, setPlayers] = useState<Map<string, SpacetimeDB.Player>>(new Map());
    const [trees, setTrees] = useState<Map<string, SpacetimeDB.Tree>>(new Map());
    const [stones, setStones] = useState<Map<string, SpacetimeDB.Stone>>(new Map());
    const [campfires, setCampfires] = useState<Map<string, SpacetimeDB.Campfire>>(new Map());
    const [mushrooms, setMushrooms] = useState<Map<string, SpacetimeDB.Mushroom>>(new Map());
    const [itemDefinitions, setItemDefinitions] = useState<Map<string, SpacetimeDB.ItemDefinition>>(new Map());
    const [inventoryItems, setInventoryItems] = useState<Map<string, SpacetimeDB.InventoryItem>>(new Map());
    const [worldState, setWorldState] = useState<SpacetimeDB.WorldState | null>(null);
    const [activeEquipments, setActiveEquipments] = useState<Map<string, SpacetimeDB.ActiveEquipment>>(new Map());
    const [droppedItems, setDroppedItems] = useState<Map<string, SpacetimeDB.DroppedItem>>(new Map());
    const [woodenStorageBoxes, setWoodenStorageBoxes] = useState<Map<string, SpacetimeDB.WoodenStorageBox>>(new Map());
    const [recipes, setRecipes] = useState<Map<string, SpacetimeDB.Recipe>>(new Map());
    const [craftingQueueItems, setCraftingQueueItems] = useState<Map<string, SpacetimeDB.CraftingQueueItem>>(new Map());
    const [localPlayerRegistered, setLocalPlayerRegistered] = useState<boolean>(false);

    // Ref to hold the cancelPlacement function to avoid re-running effect if it changes
    const cancelPlacementRef = useRef(cancelPlacement);
    useEffect(() => {
        cancelPlacementRef.current = cancelPlacement;
    }, [cancelPlacement]);

    // --- Effect for Subscriptions and Callbacks ---
    useEffect(() => {
        if (!connection) {
            // If connection drops, reset registration status
            setLocalPlayerRegistered(false);
            // Optionally clear table states? Or keep them cached?
            // For now, keep them cached.
            return;
        }

        // --- Callbacks (moved from App.tsx) ---
        // Define ALL handle...Insert/Update/Delete callbacks here
        // Make sure they use the state setters defined within this hook (setPlayers, setTrees, etc.)

        // --- Player Callbacks (with registration logic) ---
        const handlePlayerInsert = (ctx: any, player: SpacetimeDB.Player) => {
            console.log('[useSpacetimeTables] Player Inserted:', player.username, player.identity.toHexString());
            setPlayers(prev => new Map(prev).set(player.identity.toHexString(), player));
            if (connection.identity && player.identity.isEqual(connection.identity)) {
                console.log('[useSpacetimeTables] Local player registered.');
                setLocalPlayerRegistered(true);
            }
        };
        const handlePlayerUpdate = (ctx: any, oldPlayer: SpacetimeDB.Player, newPlayer: SpacetimeDB.Player) => {
            const EPSILON = 0.01;
            const posChanged = Math.abs(oldPlayer.positionX - newPlayer.positionX) > EPSILON || Math.abs(oldPlayer.positionY - newPlayer.positionY) > EPSILON;
            const statsChanged = Math.round(oldPlayer.health) !== Math.round(newPlayer.health) || Math.round(oldPlayer.stamina) !== Math.round(newPlayer.stamina) || Math.round(oldPlayer.hunger) !== Math.round(newPlayer.hunger) || Math.round(oldPlayer.thirst) !== Math.round(newPlayer.thirst) || Math.round(oldPlayer.warmth) !== Math.round(newPlayer.warmth);
            const stateChanged = oldPlayer.isSprinting !== newPlayer.isSprinting || oldPlayer.direction !== newPlayer.direction || oldPlayer.jumpStartTimeMs !== newPlayer.jumpStartTimeMs || oldPlayer.isDead !== newPlayer.isDead;
            if (posChanged || statsChanged || stateChanged) {
                setPlayers(prev => new Map(prev).set(newPlayer.identity.toHexString(), newPlayer));
            }
        };
        const handlePlayerDelete = (ctx: any, deletedPlayer: SpacetimeDB.Player) => {
            console.log('[useSpacetimeTables] Player Deleted:', deletedPlayer.username, deletedPlayer.identity.toHexString());
            setPlayers(prev => { const newMap = new Map(prev); newMap.delete(deletedPlayer.identity.toHexString()); return newMap; });
            if (connection.identity && deletedPlayer.identity.isEqual(connection.identity)) {
                console.warn('[useSpacetimeTables] Local player deleted from server.');
                setLocalPlayerRegistered(false); // Local player gone
            }
        };

        // --- Tree Callbacks ---
        const handleTreeInsert = (ctx: any, tree: SpacetimeDB.Tree) => {
            setTrees(prev => new Map(prev).set(tree.id.toString(), tree));
        };
        const handleTreeUpdate = (ctx: any, oldTree: SpacetimeDB.Tree, newTree: SpacetimeDB.Tree) => {
            setTrees(prev => new Map(prev).set(newTree.id.toString(), newTree));
        };
        const handleTreeDelete = (ctx: any, tree: SpacetimeDB.Tree) => {
            setTrees(prev => { const newMap = new Map(prev); newMap.delete(tree.id.toString()); return newMap; });
        };

        // --- Stone Callbacks ---
        const handleStoneInsert = (ctx: any, stone: SpacetimeDB.Stone) => {
            setStones(prev => new Map(prev).set(stone.id.toString(), stone));
        };
        const handleStoneUpdate = (ctx: any, oldStone: SpacetimeDB.Stone, newStone: SpacetimeDB.Stone) => {
            setStones(prev => new Map(prev).set(newStone.id.toString(), newStone));
        };
        const handleStoneDelete = (ctx: any, stone: SpacetimeDB.Stone) => {
            setStones(prev => { const newMap = new Map(prev); newMap.delete(stone.id.toString()); return newMap; });
        };

        // --- Campfire Callbacks (with placement cancellation) ---
        const handleCampfireInsert = (ctx: any, campfire: SpacetimeDB.Campfire) => {
            console.log('[useSpacetimeTables] Campfire Inserted:', campfire.id);
            setCampfires(prev => new Map(prev).set(campfire.id.toString(), campfire));
            // Cancel placement if this client just placed this item (check identity)
            if (connection.identity && campfire.placedBy.isEqual(connection.identity)) {
                // Check item name? Currently not possible here without more context.
                // Assume any insert by local player might warrant cancellation for now.
                // TODO: Make this more robust if needed (e.g., pass placementInfo)
                console.log("[useSpacetimeTables handleCampfireInsert] Calling cancelPlacement via ref...");
                cancelPlacementRef.current(); // Call action via ref
            }
        };
        const handleCampfireUpdate = (ctx: any, oldFire: SpacetimeDB.Campfire, newFire: SpacetimeDB.Campfire) => {
            setCampfires(prev => new Map(prev).set(newFire.id.toString(), newFire));
        };
        const handleCampfireDelete = (ctx: any, campfire: SpacetimeDB.Campfire) => {
            console.log('[useSpacetimeTables] Campfire Deleted:', campfire.id);
            setCampfires(prev => { const newMap = new Map(prev); newMap.delete(campfire.id.toString()); return newMap; });
        };

        // --- ItemDefinition Callbacks ---
        const handleItemDefInsert = (ctx: any, itemDef: SpacetimeDB.ItemDefinition) => {
            setItemDefinitions(prev => new Map(prev).set(itemDef.id.toString(), itemDef));
        };
        const handleItemDefUpdate = (ctx: any, oldDef: SpacetimeDB.ItemDefinition, newDef: SpacetimeDB.ItemDefinition) => {
            setItemDefinitions(prev => new Map(prev).set(newDef.id.toString(), newDef));
        };
        const handleItemDefDelete = (ctx: any, itemDef: SpacetimeDB.ItemDefinition) => {
            setItemDefinitions(prev => { const newMap = new Map(prev); newMap.delete(itemDef.id.toString()); return newMap; });
        };

        // --- InventoryItem Callbacks ---
        const handleInventoryInsert = (ctx: any, invItem: SpacetimeDB.InventoryItem) => {
            setInventoryItems(prev => new Map(prev).set(invItem.instanceId.toString(), invItem));
        };
        const handleInventoryUpdate = (ctx: any, oldItem: SpacetimeDB.InventoryItem, newItem: SpacetimeDB.InventoryItem) => {
            setInventoryItems(prev => new Map(prev).set(newItem.instanceId.toString(), newItem));
        };
        const handleInventoryDelete = (ctx: any, invItem: SpacetimeDB.InventoryItem) => {
            setInventoryItems(prev => {
                const newMap = new Map(prev);
                newMap.delete(invItem.instanceId.toString());
                return newMap;
            });
        };

        // --- WorldState Callbacks ---
        const handleWorldStateInsert = (ctx: any, state: SpacetimeDB.WorldState) => {
            setWorldState(state);
        };
        const handleWorldStateUpdate = (ctx: any, oldState: SpacetimeDB.WorldState, newState: SpacetimeDB.WorldState) => {
            const significantChange = oldState.timeOfDay !== newState.timeOfDay || oldState.isFullMoon !== newState.isFullMoon || oldState.cycleCount !== newState.cycleCount;
            if (significantChange) {
                setWorldState(newState);
            }
        };
        const handleWorldStateDelete = (ctx: any, state: SpacetimeDB.WorldState) => {
            console.warn('[useSpacetimeTables] WorldState Deleted:', state);
            setWorldState(null);
        };

        // --- ActiveEquipment Callbacks ---
        const handleActiveEquipmentInsert = (ctx: any, equip: SpacetimeDB.ActiveEquipment) => {
            setActiveEquipments(prev => new Map(prev).set(equip.playerIdentity.toHexString(), equip));
        };
        const handleActiveEquipmentUpdate = (ctx: any, oldEquip: SpacetimeDB.ActiveEquipment, newEquip: SpacetimeDB.ActiveEquipment) => {
            setActiveEquipments(prev => new Map(prev).set(newEquip.playerIdentity.toHexString(), newEquip));
        };
        const handleActiveEquipmentDelete = (ctx: any, equip: SpacetimeDB.ActiveEquipment) => {
            setActiveEquipments(prev => { const newMap = new Map(prev); newMap.delete(equip.playerIdentity.toHexString()); return newMap; });
        };

        // --- Mushroom Callbacks ---
        const handleMushroomInsert = (ctx: any, mushroom: SpacetimeDB.Mushroom) => {
            setMushrooms(prev => new Map(prev).set(mushroom.id.toString(), mushroom));
        };
        const handleMushroomUpdate = (ctx: any, oldMushroom: SpacetimeDB.Mushroom, newMushroom: SpacetimeDB.Mushroom) => {
            setMushrooms(prev => new Map(prev).set(newMushroom.id.toString(), newMushroom));
        };
        const handleMushroomDelete = (ctx: any, mushroom: SpacetimeDB.Mushroom) => {
            setMushrooms(prev => { const newMap = new Map(prev); newMap.delete(mushroom.id.toString()); return newMap; });
        };

        // --- DroppedItem Callbacks ---
        const handleDroppedItemInsert = (ctx: any, item: SpacetimeDB.DroppedItem) => {
            setDroppedItems(prev => new Map(prev).set(item.id.toString(), item));
        };
        const handleDroppedItemUpdate = (ctx: any, oldItem: SpacetimeDB.DroppedItem, newItem: SpacetimeDB.DroppedItem) => {
            setDroppedItems(prev => new Map(prev).set(newItem.id.toString(), newItem));
        };
        const handleDroppedItemDelete = (ctx: any, item: SpacetimeDB.DroppedItem) => {
            setDroppedItems(prev => { const newMap = new Map(prev); newMap.delete(item.id.toString()); return newMap; });
        };

        // --- WoodenStorageBox Callbacks (with placement cancellation) ---
        const handleWoodenStorageBoxInsert = (ctx: any, box: SpacetimeDB.WoodenStorageBox) => {
            console.log('[useSpacetimeTables] WoodenStorageBox Inserted:', box.id);
            setWoodenStorageBoxes(prev => new Map(prev).set(box.id.toString(), box));
             // Cancel placement if this client just placed this item (check identity)
             if (connection.identity && box.placedBy.isEqual(connection.identity)) {
                // TODO: Make this more robust if needed
                console.log("[useSpacetimeTables handleWoodenStorageBoxInsert] Calling cancelPlacement via ref...");
                cancelPlacementRef.current();
            }
        };
        const handleWoodenStorageBoxUpdate = (ctx: any, oldBox: SpacetimeDB.WoodenStorageBox, newBox: SpacetimeDB.WoodenStorageBox) => {
            setWoodenStorageBoxes(prev => new Map(prev).set(newBox.id.toString(), newBox));
        };
        const handleWoodenStorageBoxDelete = (ctx: any, box: SpacetimeDB.WoodenStorageBox) => {
            console.log('[useSpacetimeTables] WoodenStorageBox Deleted:', box.id);
            setWoodenStorageBoxes(prev => { const newMap = new Map(prev); newMap.delete(box.id.toString()); return newMap; });
        };

        // --- Recipe Callbacks ---
        const handleRecipeInsert = (ctx: any, recipe: SpacetimeDB.Recipe) => {
            setRecipes(prev => new Map(prev).set(recipe.recipeId.toString(), recipe));
        };
        const handleRecipeUpdate = (ctx: any, oldRecipe: SpacetimeDB.Recipe, newRecipe: SpacetimeDB.Recipe) => {
            setRecipes(prev => new Map(prev).set(newRecipe.recipeId.toString(), newRecipe));
        };
        const handleRecipeDelete = (ctx: any, recipe: SpacetimeDB.Recipe) => {
            setRecipes(prev => { const newMap = new Map(prev); newMap.delete(recipe.recipeId.toString()); return newMap; });
        };

        // --- CraftingQueueItem Callbacks ---
        const handleCraftingQueueInsert = (ctx: any, queueItem: SpacetimeDB.CraftingQueueItem) => {
            setCraftingQueueItems(prev => new Map(prev).set(queueItem.queueItemId.toString(), queueItem));
        };
        const handleCraftingQueueUpdate = (ctx: any, oldItem: SpacetimeDB.CraftingQueueItem, newItem: SpacetimeDB.CraftingQueueItem) => {
            setCraftingQueueItems(prev => new Map(prev).set(newItem.queueItemId.toString(), newItem));
        };
        const handleCraftingQueueDelete = (ctx: any, queueItem: SpacetimeDB.CraftingQueueItem) => {
            setCraftingQueueItems(prev => { const newMap = new Map(prev); newMap.delete(queueItem.queueItemId.toString()); return newMap; });
        };

        // --- Register Callbacks ---
        connection.db.player.onInsert(handlePlayerInsert); connection.db.player.onUpdate(handlePlayerUpdate); connection.db.player.onDelete(handlePlayerDelete);
        connection.db.tree.onInsert(handleTreeInsert); connection.db.tree.onUpdate(handleTreeUpdate); connection.db.tree.onDelete(handleTreeDelete);
        connection.db.stone.onInsert(handleStoneInsert); connection.db.stone.onUpdate(handleStoneUpdate); connection.db.stone.onDelete(handleStoneDelete);
        connection.db.campfire.onInsert(handleCampfireInsert); connection.db.campfire.onUpdate(handleCampfireUpdate); connection.db.campfire.onDelete(handleCampfireDelete);
        connection.db.itemDefinition.onInsert(handleItemDefInsert); connection.db.itemDefinition.onUpdate(handleItemDefUpdate); connection.db.itemDefinition.onDelete(handleItemDefDelete);
        connection.db.inventoryItem.onInsert(handleInventoryInsert); connection.db.inventoryItem.onUpdate(handleInventoryUpdate); connection.db.inventoryItem.onDelete(handleInventoryDelete);
        connection.db.worldState.onInsert(handleWorldStateInsert); connection.db.worldState.onUpdate(handleWorldStateUpdate); connection.db.worldState.onDelete(handleWorldStateDelete);
        connection.db.activeEquipment.onInsert(handleActiveEquipmentInsert); connection.db.activeEquipment.onUpdate(handleActiveEquipmentUpdate); connection.db.activeEquipment.onDelete(handleActiveEquipmentDelete);
        connection.db.mushroom.onInsert(handleMushroomInsert); connection.db.mushroom.onUpdate(handleMushroomUpdate); connection.db.mushroom.onDelete(handleMushroomDelete);
        connection.db.droppedItem.onInsert(handleDroppedItemInsert); connection.db.droppedItem.onUpdate(handleDroppedItemUpdate); connection.db.droppedItem.onDelete(handleDroppedItemDelete);
        connection.db.woodenStorageBox.onInsert(handleWoodenStorageBoxInsert); connection.db.woodenStorageBox.onUpdate(handleWoodenStorageBoxUpdate); connection.db.woodenStorageBox.onDelete(handleWoodenStorageBoxDelete);
        connection.db.recipe.onInsert(handleRecipeInsert); connection.db.recipe.onUpdate(handleRecipeUpdate); connection.db.recipe.onDelete(handleRecipeDelete);
        connection.db.craftingQueueItem.onInsert(handleCraftingQueueInsert); connection.db.craftingQueueItem.onUpdate(handleCraftingQueueUpdate); connection.db.craftingQueueItem.onDelete(handleCraftingQueueDelete);

        // --- Subscriptions ---
        console.log('[useSpacetimeTables] Subscribing to all tables...');
        const subs = [
            connection.subscriptionBuilder().subscribe('SELECT * FROM player'),
            connection.subscriptionBuilder().subscribe('SELECT * FROM tree'),
            connection.subscriptionBuilder().subscribe('SELECT * FROM stone'),
            connection.subscriptionBuilder().subscribe('SELECT * FROM campfire'),
            connection.subscriptionBuilder().subscribe('SELECT * FROM item_definition'),
            connection.subscriptionBuilder().subscribe('SELECT * FROM inventory_item'),
            connection.subscriptionBuilder().subscribe('SELECT * FROM world_state'),
            connection.subscriptionBuilder().subscribe('SELECT * FROM active_equipment'),
            connection.subscriptionBuilder().subscribe('SELECT * FROM mushroom'),
            connection.subscriptionBuilder().subscribe('SELECT * FROM dropped_item'),
            connection.subscriptionBuilder().subscribe('SELECT * FROM wooden_storage_box'),
            connection.subscriptionBuilder().subscribe('SELECT * FROM recipe'),
            connection.subscriptionBuilder().subscribe('SELECT * FROM crafting_queue_item'),
        ];

        // Cleanup function
        return () => {
            console.log('[useSpacetimeTables] Cleaning up all table listeners and subscriptions...');
            // Remove ALL listeners
            connection?.db?.player?.removeOnInsert(handlePlayerInsert); connection?.db?.player?.removeOnUpdate(handlePlayerUpdate); connection?.db?.player?.removeOnDelete(handlePlayerDelete);
            connection?.db?.tree?.removeOnInsert(handleTreeInsert); connection?.db?.tree?.removeOnUpdate(handleTreeUpdate); connection?.db?.tree?.removeOnDelete(handleTreeDelete);
            connection?.db?.stone?.removeOnInsert(handleStoneInsert); connection?.db?.stone?.removeOnUpdate(handleStoneUpdate); connection?.db?.stone?.removeOnDelete(handleStoneDelete);
            connection?.db?.campfire?.removeOnInsert(handleCampfireInsert); connection?.db?.campfire?.removeOnUpdate(handleCampfireUpdate); connection?.db?.campfire?.removeOnDelete(handleCampfireDelete);
            connection?.db?.itemDefinition?.removeOnInsert(handleItemDefInsert); connection?.db?.itemDefinition?.removeOnUpdate(handleItemDefUpdate); connection?.db?.itemDefinition?.removeOnDelete(handleItemDefDelete);
            connection?.db?.inventoryItem?.removeOnInsert(handleInventoryInsert); connection?.db?.inventoryItem?.removeOnUpdate(handleInventoryUpdate); connection?.db?.inventoryItem?.removeOnDelete(handleInventoryDelete);
            connection?.db?.worldState?.removeOnInsert(handleWorldStateInsert); connection?.db?.worldState?.removeOnUpdate(handleWorldStateUpdate); connection?.db?.worldState?.removeOnDelete(handleWorldStateDelete);
            connection?.db?.activeEquipment?.removeOnInsert(handleActiveEquipmentInsert); connection?.db?.activeEquipment?.removeOnUpdate(handleActiveEquipmentUpdate); connection?.db?.activeEquipment?.removeOnDelete(handleActiveEquipmentDelete);
            connection?.db?.mushroom?.removeOnInsert(handleMushroomInsert); connection?.db?.mushroom?.removeOnUpdate(handleMushroomUpdate); connection?.db?.mushroom?.removeOnDelete(handleMushroomDelete);
            connection?.db?.droppedItem?.removeOnInsert(handleDroppedItemInsert); connection?.db?.droppedItem?.removeOnUpdate(handleDroppedItemUpdate); connection?.db?.droppedItem?.removeOnDelete(handleDroppedItemDelete);
            connection?.db?.woodenStorageBox?.removeOnInsert(handleWoodenStorageBoxInsert); connection?.db?.woodenStorageBox?.removeOnUpdate(handleWoodenStorageBoxUpdate); connection?.db?.woodenStorageBox?.removeOnDelete(handleWoodenStorageBoxDelete);
            connection?.db?.recipe?.removeOnInsert(handleRecipeInsert); connection?.db?.recipe?.removeOnUpdate(handleRecipeUpdate); connection?.db?.recipe?.removeOnDelete(handleRecipeDelete);
            connection?.db?.craftingQueueItem?.removeOnInsert(handleCraftingQueueInsert); connection?.db?.craftingQueueItem?.removeOnUpdate(handleCraftingQueueUpdate); connection?.db?.craftingQueueItem?.removeOnDelete(handleCraftingQueueDelete);

            // Unsubscribe
            subs.forEach(sub => sub?.unsubscribe());
        };

    }, [connection]); // Re-run ONLY when the connection object changes

    // --- Return Hook State ---
    return {
        players,
        trees,
        stones,
        campfires,
        mushrooms,
        itemDefinitions,
        inventoryItems,
        worldState,
        activeEquipments,
        droppedItems,
        woodenStorageBoxes,
        recipes,
        craftingQueueItems,
        localPlayerRegistered,
    };
}; 