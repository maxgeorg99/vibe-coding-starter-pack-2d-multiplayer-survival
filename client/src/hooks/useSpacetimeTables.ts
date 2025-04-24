import { useState, useEffect, useRef } from 'react';
import * as SpacetimeDB from '../generated';
import { DbConnection } from '../generated'; // Import the connection type
import { getChunkIndicesForViewport } from '../utils/chunkUtils'; // Import the chunk utility

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
    viewport: { minX: number; minY: number; maxX: number; maxY: number } | null; // New viewport prop
}

// Helper type for subscription handles (adjust if SDK provides a specific type)
type SubscriptionHandle = { unsubscribe: () => void } | null;

// Function to compare arrays of chunk indices
function areChunkArraysEqual(a: number[], b: number[]): boolean {
    if (a.length !== b.length) return false;
    
    // Sort both arrays to ensure consistent comparison
    const sortedA = [...a].sort((x, y) => x - y);
    const sortedB = [...b].sort((x, y) => x - y);
    
    return sortedA.every((val, idx) => val === sortedB[idx]);
}

export const useSpacetimeTables = ({
    connection,
    cancelPlacement,
    viewport, // Get viewport from props
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

    // Ref to hold the cancelPlacement function
    const cancelPlacementRef = useRef(cancelPlacement);
    useEffect(() => { cancelPlacementRef.current = cancelPlacement; }, [cancelPlacement]);

    // Keep viewport in a ref for use in callbacks
    const viewportRef = useRef(viewport);
    useEffect(() => { viewportRef.current = viewport; }, [viewport]);

    // Track current chunk indices to avoid unnecessary resubscriptions
    const currentChunksRef = useRef<number[]>([]);
    
    // --- Refs for Subscription Management ---
    const nonSpatialHandlesRef = useRef<SubscriptionHandle[]>([]);
    const spatialHandlesRef = useRef<SubscriptionHandle[]>([]);
    const callbacksRegisteredRef = useRef(false);
    
    // Flag to prevent duplicate subscription callbacks during React's StrictMode
    const spatialSubscriptionsActiveRef = useRef(false);

    // Helper function for safely unsubscribing
    const safeUnsubscribe = (sub: SubscriptionHandle) => {
        if (sub) {
            try {
                sub.unsubscribe();
            } catch (e) {
                console.warn('[useSpacetimeTables] Error unsubscribing:', e);
            }
        }
    };

    // --- Effect for Subscriptions and Callbacks ---
    useEffect(() => {
        // --- Callback Registration & Initial Subscriptions (Only Once Per Connection Instance) ---
        if (connection && !callbacksRegisteredRef.current) {
            console.log("[useSpacetimeTables] Connection available. Registering callbacks & initial subs..."); // Log combined action

            // --- Define Callbacks --- (Keep definitions here)
             const handlePlayerInsert = (ctx: any, player: SpacetimeDB.Player) => {
                 console.log('[useSpacetimeTables] handlePlayerInsert CALLED for:', player.username, player.identity.toHexString());
                 setPlayers(prev => new Map(prev).set(player.identity.toHexString(), player));
                 if (connection.identity) {
                    const isLocalPlayer = player.identity.isEqual(connection.identity);
                    console.log(`[useSpacetimeTables] Comparing identities: Player=${player.identity.toHexString()}, Connection=${connection.identity.toHexString()}, Match=${isLocalPlayer}`);
                    if (isLocalPlayer && !localPlayerRegistered) {
                         console.log('[useSpacetimeTables] Local player matched! Setting localPlayerRegistered = true.');
                         setLocalPlayerRegistered(true);
                     }
                 } else {
                    console.warn('[useSpacetimeTables] handlePlayerInsert: connection.identity is null during check?');
                 }
             };
            // ... (Keep ALL other handle... callback definitions here) ...
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
                     if (localPlayerRegistered) {
                        console.warn('[useSpacetimeTables] Local player deleted from server.');
                        setLocalPlayerRegistered(false);
                     }
                 }
             };
             const handleTreeInsert = (ctx: any, tree: SpacetimeDB.Tree) => setTrees(prev => new Map(prev).set(tree.id.toString(), tree));
             const handleTreeUpdate = (ctx: any, oldTree: SpacetimeDB.Tree, newTree: SpacetimeDB.Tree) => {
                 // Only update if there are visible changes that would affect rendering
                 // Compare relevant fields directly instead of JSON.stringify
                 const changed = oldTree.posX !== newTree.posX ||
                                 oldTree.posY !== newTree.posY ||
                                 oldTree.health !== newTree.health ||
                                 oldTree.treeType !== newTree.treeType ||
                                 oldTree.lastHitTime !== newTree.lastHitTime ||
                                 oldTree.respawnAt !== newTree.respawnAt;

                 if (changed) {
                     // Convert BigInt id to string for map key
                     setTrees(prev => new Map(prev).set(newTree.id.toString(), newTree));
                 }
             };
             const handleTreeDelete = (ctx: any, tree: SpacetimeDB.Tree) => setTrees(prev => { const newMap = new Map(prev); newMap.delete(tree.id.toString()); return newMap; });
             const handleStoneInsert = (ctx: any, stone: SpacetimeDB.Stone) => setStones(prev => new Map(prev).set(stone.id.toString(), stone));
             const handleStoneUpdate = (ctx: any, oldStone: SpacetimeDB.Stone, newStone: SpacetimeDB.Stone) => {
                 // Compare relevant fields directly
                 const changed = oldStone.posX !== newStone.posX ||
                                 oldStone.posY !== newStone.posY ||
                                 oldStone.health !== newStone.health ||
                                 oldStone.lastHitTime !== newStone.lastHitTime ||
                                 oldStone.respawnAt !== newStone.respawnAt;

                 if (changed) {
                     // Convert BigInt id to string for map key
                     setStones(prev => new Map(prev).set(newStone.id.toString(), newStone));
                 }
             };
             const handleStoneDelete = (ctx: any, stone: SpacetimeDB.Stone) => setStones(prev => { const newMap = new Map(prev); newMap.delete(stone.id.toString()); return newMap; });
             const handleCampfireInsert = (ctx: any, campfire: SpacetimeDB.Campfire) => {
                 setCampfires(prev => new Map(prev).set(campfire.id.toString(), campfire));
                 if (connection.identity && campfire.placedBy.isEqual(connection.identity)) {
                    console.log("[useSpacetimeTables handleCampfireInsert] Calling cancelPlacement via ref...");
                    cancelPlacementRef.current();
                }
             };
             const handleCampfireUpdate = (ctx: any, oldFire: SpacetimeDB.Campfire, newFire: SpacetimeDB.Campfire) => setCampfires(prev => new Map(prev).set(newFire.id.toString(), newFire));
             const handleCampfireDelete = (ctx: any, campfire: SpacetimeDB.Campfire) => setCampfires(prev => { const newMap = new Map(prev); newMap.delete(campfire.id.toString()); return newMap; });
             const handleItemDefInsert = (ctx: any, itemDef: SpacetimeDB.ItemDefinition) => setItemDefinitions(prev => new Map(prev).set(itemDef.id.toString(), itemDef));
             const handleItemDefUpdate = (ctx: any, oldDef: SpacetimeDB.ItemDefinition, newDef: SpacetimeDB.ItemDefinition) => setItemDefinitions(prev => new Map(prev).set(newDef.id.toString(), newDef));
             const handleItemDefDelete = (ctx: any, itemDef: SpacetimeDB.ItemDefinition) => setItemDefinitions(prev => { const newMap = new Map(prev); newMap.delete(itemDef.id.toString()); return newMap; });
             const handleInventoryInsert = (ctx: any, invItem: SpacetimeDB.InventoryItem) => setInventoryItems(prev => new Map(prev).set(invItem.instanceId.toString(), invItem));
             const handleInventoryUpdate = (ctx: any, oldItem: SpacetimeDB.InventoryItem, newItem: SpacetimeDB.InventoryItem) => setInventoryItems(prev => new Map(prev).set(newItem.instanceId.toString(), newItem));
             const handleInventoryDelete = (ctx: any, invItem: SpacetimeDB.InventoryItem) => setInventoryItems(prev => { const newMap = new Map(prev); newMap.delete(invItem.instanceId.toString()); return newMap; });
             const handleWorldStateInsert = (ctx: any, state: SpacetimeDB.WorldState) => setWorldState(state);
             const handleWorldStateUpdate = (ctx: any, oldState: SpacetimeDB.WorldState, newState: SpacetimeDB.WorldState) => {
                 const significantChange = oldState.timeOfDay !== newState.timeOfDay || oldState.isFullMoon !== newState.isFullMoon || oldState.cycleCount !== newState.cycleCount;
                 if (significantChange) setWorldState(newState);
             };
             const handleWorldStateDelete = (ctx: any, state: SpacetimeDB.WorldState) => setWorldState(null);
             const handleActiveEquipmentInsert = (ctx: any, equip: SpacetimeDB.ActiveEquipment) => setActiveEquipments(prev => new Map(prev).set(equip.playerIdentity.toHexString(), equip));
             const handleActiveEquipmentUpdate = (ctx: any, oldEquip: SpacetimeDB.ActiveEquipment, newEquip: SpacetimeDB.ActiveEquipment) => setActiveEquipments(prev => new Map(prev).set(newEquip.playerIdentity.toHexString(), newEquip));
             const handleActiveEquipmentDelete = (ctx: any, equip: SpacetimeDB.ActiveEquipment) => setActiveEquipments(prev => { const newMap = new Map(prev); newMap.delete(equip.playerIdentity.toHexString()); return newMap; });
             const handleMushroomInsert = (ctx: any, mushroom: SpacetimeDB.Mushroom) => setMushrooms(prev => new Map(prev).set(mushroom.id.toString(), mushroom));
             const handleMushroomUpdate = (ctx: any, oldMushroom: SpacetimeDB.Mushroom, newMushroom: SpacetimeDB.Mushroom) => {
                 // Compare relevant fields directly
                 const changed = oldMushroom.posX !== newMushroom.posX ||
                                 oldMushroom.posY !== newMushroom.posY ||
                                 oldMushroom.respawnAt !== newMushroom.respawnAt;

                 if (changed) {
                     // Convert BigInt id to string for map key
                     setMushrooms(prev => new Map(prev).set(newMushroom.id.toString(), newMushroom));
                 }
             };
             const handleMushroomDelete = (ctx: any, mushroom: SpacetimeDB.Mushroom) => setMushrooms(prev => { const newMap = new Map(prev); newMap.delete(mushroom.id.toString()); return newMap; });
             const handleDroppedItemInsert = (ctx: any, item: SpacetimeDB.DroppedItem) => setDroppedItems(prev => new Map(prev).set(item.id.toString(), item));
             const handleDroppedItemUpdate = (ctx: any, oldItem: SpacetimeDB.DroppedItem, newItem: SpacetimeDB.DroppedItem) => setDroppedItems(prev => new Map(prev).set(newItem.id.toString(), newItem));
             const handleDroppedItemDelete = (ctx: any, item: SpacetimeDB.DroppedItem) => setDroppedItems(prev => { const newMap = new Map(prev); newMap.delete(item.id.toString()); return newMap; });
             const handleWoodenStorageBoxInsert = (ctx: any, box: SpacetimeDB.WoodenStorageBox) => {
                 setWoodenStorageBoxes(prev => new Map(prev).set(box.id.toString(), box));
                 if (connection.identity && box.placedBy.isEqual(connection.identity)) {
                    console.log("[useSpacetimeTables handleWoodenStorageBoxInsert] Calling cancelPlacement via ref...");
                    cancelPlacementRef.current();
                 }
             };
             const handleWoodenStorageBoxUpdate = (ctx: any, oldBox: SpacetimeDB.WoodenStorageBox, newBox: SpacetimeDB.WoodenStorageBox) => setWoodenStorageBoxes(prev => new Map(prev).set(newBox.id.toString(), newBox));
             const handleWoodenStorageBoxDelete = (ctx: any, box: SpacetimeDB.WoodenStorageBox) => setWoodenStorageBoxes(prev => { const newMap = new Map(prev); newMap.delete(box.id.toString()); return newMap; });
             const handleRecipeInsert = (ctx: any, recipe: SpacetimeDB.Recipe) => setRecipes(prev => new Map(prev).set(recipe.recipeId.toString(), recipe));
             const handleRecipeUpdate = (ctx: any, oldRecipe: SpacetimeDB.Recipe, newRecipe: SpacetimeDB.Recipe) => setRecipes(prev => new Map(prev).set(newRecipe.recipeId.toString(), newRecipe));
             const handleRecipeDelete = (ctx: any, recipe: SpacetimeDB.Recipe) => setRecipes(prev => { const newMap = new Map(prev); newMap.delete(recipe.recipeId.toString()); return newMap; });
             const handleCraftingQueueInsert = (ctx: any, queueItem: SpacetimeDB.CraftingQueueItem) => setCraftingQueueItems(prev => new Map(prev).set(queueItem.queueItemId.toString(), queueItem));
             const handleCraftingQueueUpdate = (ctx: any, oldItem: SpacetimeDB.CraftingQueueItem, newItem: SpacetimeDB.CraftingQueueItem) => setCraftingQueueItems(prev => new Map(prev).set(newItem.queueItemId.toString(), newItem));
             const handleCraftingQueueDelete = (ctx: any, queueItem: SpacetimeDB.CraftingQueueItem) => setCraftingQueueItems(prev => { const newMap = new Map(prev); newMap.delete(queueItem.queueItemId.toString()); return newMap; });
             // --- End Callback Definitions ---

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
            callbacksRegisteredRef.current = true;

            // --- Create Initial Non-Spatial Subscriptions ---
            // Unsubscribe any lingering non-spatial handles from a previous connection (safety net)
            nonSpatialHandlesRef.current.forEach(sub => safeUnsubscribe(sub)); 
            nonSpatialHandlesRef.current = []; // Clear after unsubscribing
            
            console.log("[useSpacetimeTables] Setting up initial non-spatial subscriptions.");
            const currentInitialSubs = [
                 connection.subscriptionBuilder()
                    .onApplied(() => console.log("[useSpacetimeTables] Non-spatial PLAYER subscription applied.")) 
                    .onError((err) => console.error("[useSpacetimeTables] Non-spatial PLAYER subscription error:", err)) 
                    .subscribe('SELECT * FROM player'), // Fixed: lowercase table name
                 connection.subscriptionBuilder().subscribe('SELECT * FROM item_definition'), // Fixed: lowercase snake_case
                 connection.subscriptionBuilder().subscribe('SELECT * FROM recipe'), // Fixed: lowercase
                 connection.subscriptionBuilder().subscribe('SELECT * FROM world_state'), // Fixed: lowercase snake_case
                 connection.subscriptionBuilder()
                    .onApplied(() => console.log("[useSpacetimeTables] Non-spatial INVENTORY subscription applied.")) 
                    .onError((err) => console.error("[useSpacetimeTables] Non-spatial INVENTORY subscription error:", err))
                    .subscribe('SELECT * FROM inventory_item'), // Fixed: lowercase snake_case
                 connection.subscriptionBuilder()
                    .onApplied(() => console.log("[useSpacetimeTables] Non-spatial EQUIPMENT subscription applied.")) 
                    .onError((err) => console.error("[useSpacetimeTables] Non-spatial EQUIPMENT subscription error:", err))
                    .subscribe('SELECT * FROM active_equipment'), // Fixed: lowercase snake_case
                 connection.subscriptionBuilder()
                    .onApplied(() => console.log("[useSpacetimeTables] Non-spatial CRAFTING subscription applied.")) 
                    .onError((err) => console.error("[useSpacetimeTables] Non-spatial CRAFTING subscription error:", err))
                    .subscribe('SELECT * FROM crafting_queue_item'), // Fixed: lowercase snake_case
            ];
            nonSpatialHandlesRef.current = currentInitialSubs; // Store handles in ref
        }

        // --- Spatial Subscription Management ---
        if (connection && viewport) {
            // Get new viewport chunk indices
            const newChunkIndices = getChunkIndicesForViewport(viewport);
            
            if (newChunkIndices.length === 0) {
                console.log("[useSpacetimeTables] No chunk indices in viewport range. Skipping spatial subscriptions.");
                return;
            }
            
            // OPTIMIZATION: Only update subscriptions if the chunks have actually changed
            const chunksChanged = !areChunkArraysEqual(currentChunksRef.current, newChunkIndices);
            
            if (!chunksChanged && spatialHandlesRef.current.length > 0) {
                // If chunks haven't changed and we already have subscriptions, skip re-subscribing
                // This prevents flickering and reduces unnecessary subscription churn
                console.log(`[useSpacetimeTables] Viewport update detected but chunk set unchanged. Skipping re-subscription.`);
                return;
            }
            
            // Check if we already have active spatial subscriptions to avoid duplicate subscriptions
            // This helps prevent multiple sets of subscription callbacks from running in parallel
            if (spatialSubscriptionsActiveRef.current) {
                console.log("[useSpacetimeTables] Spatial subscriptions already in progress. Skipping.");
                return;
            }
            
            // Chunks changed - update spatial subscriptions
            // Update our reference to current chunks
            currentChunksRef.current = newChunkIndices;
            
            // Safely unsubscribe previous spatial subscriptions
            spatialHandlesRef.current.forEach(sub => safeUnsubscribe(sub));
            spatialHandlesRef.current = []; // Clear after unsubscribing
            
            const indicesString = newChunkIndices.join(',');
            console.log(`[useSpacetimeTables] Viewport chunks changed. Setting up spatial subscriptions for ${newChunkIndices.length} chunks: ${indicesString}`);
            
            // Set flag to indicate we're in the process of subscribing
            spatialSubscriptionsActiveRef.current = true;
            
            const newSpatialSubs = [];
            
            try {
                console.log(`[useSpacetimeTables] Creating ${newChunkIndices.length * 3} individual chunk subscriptions for resources.`);
                let completedResourceSubs = 0;
                const totalResourceSubs = newChunkIndices.length * 3;

                // --- Create individual subscriptions for each chunk --- 
                newChunkIndices.forEach(chunkIndex => {
                    const treeQuery = `SELECT * FROM tree WHERE chunk_index = ${chunkIndex}`;
                    newSpatialSubs.push(
                        connection.subscriptionBuilder()
                            .onApplied(() => { completedResourceSubs++; /* console.log(`[Sub Debug] Tree chunk ${chunkIndex} applied.`); */ })
                            .onError((err) => { 
                                 console.error(`[useSpacetimeTables] Spatial TREE subscription error (Chunk ${chunkIndex}):`, err);
                                 console.error("[useSpacetimeTables] Failed query was:", treeQuery);
                                 completedResourceSubs++; // Count error as 'complete' to avoid hanging
                             })
                            .subscribe(treeQuery)
                    );

                    const stoneQuery = `SELECT * FROM stone WHERE chunk_index = ${chunkIndex}`;
                    newSpatialSubs.push(
                        connection.subscriptionBuilder()
                            .onApplied(() => { completedResourceSubs++; /* console.log(`[Sub Debug] Stone chunk ${chunkIndex} applied.`); */ })
                            .onError((err) => {
                                 console.error(`[useSpacetimeTables] Spatial STONE subscription error (Chunk ${chunkIndex}):`, err);
                                 console.error("[useSpacetimeTables] Failed query was:", stoneQuery);
                                 completedResourceSubs++; 
                             })
                            .subscribe(stoneQuery)
                    );

                    const mushroomQuery = `SELECT * FROM mushroom WHERE chunk_index = ${chunkIndex}`;
                    newSpatialSubs.push(
                        connection.subscriptionBuilder()
                            .onApplied(() => { completedResourceSubs++; /* console.log(`[Sub Debug] Mushroom chunk ${chunkIndex} applied.`); */ })
                            .onError((err) => { 
                                console.error(`[useSpacetimeTables] Spatial MUSHROOM subscription error (Chunk ${chunkIndex}):`, err);
                                console.error("[useSpacetimeTables] Failed query was:", mushroomQuery);
                                completedResourceSubs++;
                            })
                            .subscribe(mushroomQuery)
                    );
                });
                
                // Player subscription (not currently chunk-indexed)
                newSpatialSubs.push(
                    connection.subscriptionBuilder()
                        .onApplied((ctx) => console.log(`[useSpacetimeTables] Spatial PLAYER subscription applied. Initial rows: ${ctx.db.player.count()}`))
                        .onError((err) => console.error("[useSpacetimeTables] Spatial PLAYER subscription error:", err))
                        .subscribe(`SELECT * FROM player`)
                );
                
                // Campfire subscription (not currently chunk-indexed)
                newSpatialSubs.push(
                    connection.subscriptionBuilder()
                        .onApplied((ctx) => console.log(`[useSpacetimeTables] Spatial CAMPFIRE subscription applied. Initial rows: ${ctx.db.campfire.count()}`))
                        .onError((err) => console.error("[useSpacetimeTables] Spatial CAMPFIRE subscription error:", err))
                        .subscribe(`SELECT * FROM campfire`)
                );
                
                // Dropped item subscription (not currently chunk-indexed)
                newSpatialSubs.push(
                    connection.subscriptionBuilder()
                        .onApplied((ctx) => console.log(`[useSpacetimeTables] Spatial DROPPED_ITEM subscription applied. Initial rows: ${ctx.db.droppedItem.count()}`))
                        .onError((err) => console.error("[useSpacetimeTables] Spatial DROPPED_ITEM subscription error:", err))
                        .subscribe(`SELECT * FROM dropped_item`)
                );
                
                // Wooden storage box subscription (not currently chunk-indexed)
                newSpatialSubs.push(
                    connection.subscriptionBuilder()
                        .onApplied((ctx) => {
                            console.log(`[useSpacetimeTables] Spatial WOODEN_BOX subscription applied. Initial rows: ${ctx.db.woodenStorageBox.count()}`);
                            // Check if all resource subscriptions have reported back (applied or error)
                            // This approach is approximate but helps signal when loading might be complete
                            if (completedResourceSubs >= totalResourceSubs) {
                                 console.log("[useSpacetimeTables] All tracked resource chunk subscriptions completed (applied or error).");
                            }
                            spatialSubscriptionsActiveRef.current = false;
                        })
                        .onError((err) => {
                            console.error("[useSpacetimeTables] Spatial WOODEN_BOX subscription error:", err);
                            // Reset flag on error too
                            spatialSubscriptionsActiveRef.current = false;
                        })
                        .subscribe(`SELECT * FROM wooden_storage_box`)
                );
                
                // Store the new subscription handles
                spatialHandlesRef.current = newSpatialSubs;
            } catch (error) {
                console.error("[useSpacetimeTables] Error creating spatial subscriptions:", error);
                spatialSubscriptionsActiveRef.current = false;
            }
            
        } else if (connection && !viewport) {
            // If viewport becomes null while connected, clean up spatial subs
            console.log("[useSpacetimeTables] Viewport removed. Cleaning up spatial subscriptions.");
            spatialHandlesRef.current.forEach(sub => safeUnsubscribe(sub));
            spatialHandlesRef.current = [];
            currentChunksRef.current = [];
            spatialSubscriptionsActiveRef.current = false;
        }

        // --- Cleanup Function --- 
        // This function runs when the component unmounts OR BEFORE the effect re-runs due to dependency change.
        return () => {
             // Determine if cleanup is for unmount/connection loss or just a viewport change
             const isConnectionLost = !connection; // Check the connection prop passed to the hook

             console.log(`[useSpacetimeTables] Running cleanup. Connection Lost: ${isConnectionLost}, Viewport was present: ${!!viewport}`);

             // ONLY unsubscribe NON-SPATIAL handles and reset state if the connection is actually lost
             if (isConnectionLost) {
                 console.log("[useSpacetimeTables] Cleanup due to connection loss: Unsubscribing non-spatial, resetting state.");
                 nonSpatialHandlesRef.current.forEach(sub => safeUnsubscribe(sub));
                 nonSpatialHandlesRef.current = []; // Clear ref
                 
                 spatialHandlesRef.current.forEach(sub => safeUnsubscribe(sub));
                 spatialHandlesRef.current = []; // Also clear spatial ref here
                 
                 callbacksRegisteredRef.current = false;
                 currentChunksRef.current = [];
                 spatialSubscriptionsActiveRef.current = false;
                 setLocalPlayerRegistered(false);
                 // Reset table states
                 setPlayers(new Map()); setTrees(new Map()); setStones(new Map()); setCampfires(new Map());
                 setMushrooms(new Map()); setItemDefinitions(new Map()); setRecipes(new Map());
                 setInventoryItems(new Map()); setWorldState(null); setActiveEquipments(new Map());
                 setDroppedItems(new Map()); setWoodenStorageBoxes(new Map()); setCraftingQueueItems(new Map());
             }
             // Don't try to unsubscribe spatial subscriptions here - already handled at the beginning of the effect
        };

    }, [connection, viewport]); // Re-run when connection or viewport changes

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