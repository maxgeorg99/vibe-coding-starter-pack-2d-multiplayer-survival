import React, { useState, useEffect, useRef, useCallback } from 'react';
import './App.css'; // Removed potentially missing import
import GameCanvas from './components/GameCanvas';
import PlayerUI from './components/PlayerUI';
import { DraggedItemInfo, DragSourceSlotInfo } from './types/dragDropTypes';
import Hotbar from './components/Hotbar'; // Import the new Hotbar component
import githubLogo from '../public/github.png'; // Import the logo
import * as SpacetimeDB from './generated';
import { Identity as SpacetimeDBIdentity } from '@clockworklabs/spacetimedb-sdk';
import { DbConnection } from './generated'; // Correct import source

// SpacetimeDB connection parameters
const SPACETIME_DB_ADDRESS = 'ws://localhost:3000';
const SPACETIME_DB_NAME = 'vibe-survival-game'; // Adjust if your module name is different

// --- Style Constants (similar to PlayerUI/Minimap) --- 
const UI_BG_COLOR = 'rgba(40, 40, 60, 0.85)';
const UI_BORDER_COLOR = '#a0a0c0';
const UI_SHADOW = '2px 2px 0px rgba(0,0,0,0.5)';
const UI_FONT_FAMILY = '"Press Start 2P", cursive';

function App() {
  // Use the generated Player type
  const [players, setPlayers] = useState<Map<string, SpacetimeDB.Player>>(new Map());
  // Add state for trees
  const [trees, setTrees] = useState<Map<string, SpacetimeDB.Tree>>(new Map());
  // Add state for stones
  const [stones, setStones] = useState<Map<string, SpacetimeDB.Stone>>(new Map());
  // Add state for campfires
  const [campfires, setCampfires] = useState<Map<string, SpacetimeDB.Campfire>>(new Map());
  // Add state for mushrooms
  const [mushrooms, setMushrooms] = useState<Map<string, SpacetimeDB.Mushroom>>(new Map());
  // Add state for item definitions and inventory
  const [itemDefinitions, setItemDefinitions] = useState<Map<string, SpacetimeDB.ItemDefinition>>(new Map());
  const [inventoryItems, setInventoryItems] = useState<Map<string, SpacetimeDB.InventoryItem>>(new Map());
  // Add state for WorldState (nullable, as it might not exist initially)
  const [worldState, setWorldState] = useState<SpacetimeDB.WorldState | null>(null);
  // Add state for ActiveEquipment
  const [activeEquipments, setActiveEquipments] = useState<Map<string, SpacetimeDB.ActiveEquipment>>(new Map());
  // Add state for Dropped Items
  const [droppedItems, setDroppedItems] = useState<Map<string, SpacetimeDB.DroppedItem>>(new Map());
  // State holds the generated connection type after successful connection
  const [connection, setConnection] = useState<SpacetimeDB.DbConnection | null>(null);
  const [username, setUsername] = useState<string>('');
  const [isConnected, setIsConnected] = useState<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const usernameInputRef = useRef<HTMLInputElement>(null); // Ref for autofocus
  
  // --- Campfire Placement State ---
  const [isPlacingCampfire, setIsPlacingCampfire] = useState<boolean>(false);
  const [placementError, setPlacementError] = useState<string | null>(null); // Error message for placement
  // Re-add state for tracking current interaction target
  const [interactingWith, setInteractingWith] = useState<{ type: string; id: number | bigint } | null>(null);
  
  // LIFTED STATE: Custom Drag/Drop State
  const [draggedItemInfo, setDraggedItemInfo] = useState<DraggedItemInfo | null>(null);
  // Ref to hold the latest dragged info for callbacks
  const draggedItemRef = useRef<DraggedItemInfo | null>(null);

  // Effect to keep the ref synchronized with the state
  useEffect(() => {
      draggedItemRef.current = draggedItemInfo;
  }, [draggedItemInfo]);

  useEffect(() => {
    // Use generated DbConnection type
    let connectionInstance: SpacetimeDB.DbConnection | null = null; 

    const connectToSpacetimeDB = async () => {
      try {
        console.log(`Attempting to connect to SpacetimeDB at ${SPACETIME_DB_ADDRESS}, module: ${SPACETIME_DB_NAME}...`);
        
        // Use builder with correct methods from SDK definition
        connectionInstance = DbConnection.builder()
          .withUri(SPACETIME_DB_ADDRESS)       // Correct method for address
          .withModuleName(SPACETIME_DB_NAME)     // Correct method for db name
          .onConnect((conn: SpacetimeDB.DbConnection, identity: SpacetimeDBIdentity, token: string) => { // Correct method
            console.log('Connected!', { identity: identity.toHexString(), token });
            setConnection(conn);
            setError(null);
            // DO NOT set isConnected here; wait for player registration
            // setIsConnected(true); 
          })
          .onDisconnect((context: any, error?: Error) => { // Correct method
            console.log('Disconnected.', error?.message);
            setConnection(null);
            setIsConnected(false);
            setError(`Disconnected${error ? ': ' + error.message : ''}. Please refresh.`);
          })
          // No general onError, use onConnectError for initial connection issues
          .onConnectError((context: any, error: Error) => { // Correct method for initial errors
            console.error('Initial Connection Error:', error);
            setConnection(null);
            setIsConnected(false);
            setError(`Connection failed: ${error.message || error}`);
          })
          .build(); // build() takes no args
          
      } catch (err: any) {
        // Catch errors during the builder.build() call itself (e.g., invalid URI format)
        console.error('Failed to build SpacetimeDB connection:', err);
        setError(`Failed to build connection: ${err.message || err}`);
        setConnection(null);
        setIsConnected(false);
      }
    };

    connectToSpacetimeDB();
    
    return () => {
      // Cleanup: Use disconnect() suggested by linter
      if (connectionInstance) {
        console.log('Closing SpacetimeDB connection.');
        connectionInstance.disconnect(); // Use disconnect()
      }
    };
  }, []);
  
  // Log inventory items when they change
  useEffect(() => {
    console.log("Inventory Items Updated:", inventoryItems);
  }, [inventoryItems]);
  
  // Effect for Subscribing to ALL table data and handling updates
  useEffect(() => {
    if (!connection) return;

    // Declare subscription variables - using any for simplicity if specific type isn't exported/known
    let playerSubscription: any = null;
    let treeSubscription: any = null;
    let stoneSubscription: any = null;
    let campfireSubscription: any = null; // Subscription for Campfires
    let itemDefSubscription: any = null;
    let inventorySubscription: any = null;
    let worldStateSubscription: any = null; // Subscription for WorldState
    let activeEquipmentSubscription: any = null; // Subscription for ActiveEquipment
    let mushroomSubscription: any = null; // Subscription for Mushroom
    // NEW: Subscription variable for DroppedItem
    let droppedItemSubscription: any = null;

    console.log('Setting up Player, Tree, Stone, Campfire, ItemDefinition, InventoryItem, WorldState, DroppedItem table subscriptions...');

    // --- Player Callbacks --- 
    const handlePlayerInsert = (ctx: any, player: SpacetimeDB.Player) => {
      console.log('Player Inserted:', player.username, player.identity.toHexString());
      // Create a new map for inserts
      setPlayers(prev => {
        const newMap = new Map(prev);
        newMap.set(player.identity.toHexString(), player);
        return newMap;
      });
      
      if (connection && connection.identity && player.identity.isEqual(connection.identity)) {
        console.log('Local player registered, switching to game view.');
        setIsConnected(true); 
      }
    };

    const handlePlayerUpdate = (ctx: any, oldPlayer: SpacetimeDB.Player, newPlayer: SpacetimeDB.Player) => {
      // Use a small threshold for float comparisons
      const EPSILON = 0.01;
      const positionChanged = Math.abs(oldPlayer.positionX - newPlayer.positionX) > EPSILON || Math.abs(oldPlayer.positionY - newPlayer.positionY) > EPSILON;
      // Explicitly compare rounded values
      const healthChanged = Math.round(oldPlayer.health) !== Math.round(newPlayer.health);
      const staminaChanged = Math.round(oldPlayer.stamina) !== Math.round(newPlayer.stamina);
      const hungerChanged = Math.round(oldPlayer.hunger) !== Math.round(newPlayer.hunger);
      const thirstChanged = Math.round(oldPlayer.thirst) !== Math.round(newPlayer.thirst);
      const warmthChanged = Math.round(oldPlayer.warmth) !== Math.round(newPlayer.warmth); 
      const jumpTimeChanged = oldPlayer.jumpStartTimeMs !== newPlayer.jumpStartTimeMs;
      const otherStateChanged = oldPlayer.isSprinting !== newPlayer.isSprinting || oldPlayer.direction !== newPlayer.direction;

      // Combine checks
      const significantChange = positionChanged || healthChanged || staminaChanged || hungerChanged || thirstChanged || warmthChanged || jumpTimeChanged || otherStateChanged;

      if (significantChange) {
          // Log exactly what changed
          // console.log(`[handlePlayerUpdate] Player ${newPlayer.username} updated. Changes: pos=${positionChanged}, hp=${healthChanged}, stam=${staminaChanged}, hung=${hungerChanged}, thir=${thirstChanged}, warm=${warmthChanged}, jump=${jumpTimeChanged}, other=${otherStateChanged}`);
          setPlayers(prev => {
            const newMap = new Map(prev);
            newMap.set(newPlayer.identity.toHexString(), newPlayer);
            return newMap;
          });
      } else {
        // Optional: Log ignored updates
        // console.log(`[handlePlayerUpdate] Player ${newPlayer.username} update ignored (no significant change).`);
      }
    };

    const handlePlayerDelete = (ctx: any, player: SpacetimeDB.Player) => {
      console.log('Player Deleted:', player.username, player.identity.toHexString());
      // Ensure a new map is created for deletes
      setPlayers(prev => {
        const newMap = new Map(prev);
        newMap.delete(player.identity.toHexString());
        return newMap;
      });
    };

    // --- Tree Callbacks --- 
    const handleTreeInsert = (ctx: any, tree: SpacetimeDB.Tree) => {
      console.log('Tree Inserted:', tree.id);
      setTrees(prev => {
        const newMap = new Map(prev);
        newMap.set(tree.id.toString(), tree); // Use tree.id as key
        return newMap;
      });
    };

    const handleTreeUpdate = (ctx: any, oldTree: SpacetimeDB.Tree, newTree: SpacetimeDB.Tree) => {
      // Fix log message - Tree no longer has 'state'
      console.log(`Tree Updated: ${newTree.id}, Health: ${newTree.health}`); 
      setTrees(prev => {
        const newMap = new Map(prev);
        newMap.set(newTree.id.toString(), newTree);
        return newMap;
      });
    };

    const handleTreeDelete = (ctx: any, tree: SpacetimeDB.Tree) => {
      console.log('Tree Deleted:', tree.id);
      setTrees(prev => {
        const newMap = new Map(prev);
        newMap.delete(tree.id.toString());
        return newMap;
      });
    };

    // --- Stone Callbacks --- 
    const handleStoneInsert = (ctx: any, stone: SpacetimeDB.Stone) => {
      console.log('Stone Inserted:', stone.id);
      setStones(prev => {
        const newMap = new Map(prev);
        newMap.set(stone.id.toString(), stone);
        return newMap;
      });
    };

    const handleStoneUpdate = (ctx: any, oldStone: SpacetimeDB.Stone, newStone: SpacetimeDB.Stone) => {
      // Simple update for now
      console.log(`Stone Updated: ${newStone.id}, Health: ${newStone.health}`);
      setStones(prev => {
        const newMap = new Map(prev);
        newMap.set(newStone.id.toString(), newStone);
        return newMap;
      });
    };

    const handleStoneDelete = (ctx: any, stone: SpacetimeDB.Stone) => {
      console.log('Stone Deleted:', stone.id);
      setStones(prev => {
        const newMap = new Map(prev);
        newMap.delete(stone.id.toString());
        return newMap;
      });
    };

    // --- Campfire Callbacks ---
    const handleCampfireInsert = (ctx: any, campfire: SpacetimeDB.Campfire) => {
      console.log('Campfire Inserted:', campfire.id);
      setCampfires(prev => new Map(prev).set(campfire.id.toString(), campfire));
    };

    const handleCampfireUpdate = (ctx: any, oldFire: SpacetimeDB.Campfire, newFire: SpacetimeDB.Campfire) => {
      // Likely no updates needed yet unless fuel is added
      console.log(`Campfire Updated: ${newFire.id}`);
      setCampfires(prev => new Map(prev).set(newFire.id.toString(), newFire));
    };

    const handleCampfireDelete = (ctx: any, campfire: SpacetimeDB.Campfire) => {
      console.log('Campfire Deleted:', campfire.id);
      setCampfires(prev => {
          const newMap = new Map(prev);
          newMap.delete(campfire.id.toString());
          return newMap;
      });
    };

    // --- ItemDefinition Callbacks ---
    const handleItemDefInsert = (ctx: any, itemDef: SpacetimeDB.ItemDefinition) => {
      console.log('Item Definition Inserted:', itemDef.name, itemDef.id);
      setItemDefinitions(prev => new Map(prev).set(itemDef.id.toString(), itemDef));
    };
    const handleItemDefUpdate = (ctx: any, oldDef: SpacetimeDB.ItemDefinition, newDef: SpacetimeDB.ItemDefinition) => {
      console.log('Item Definition Updated:', newDef.name, newDef.id);
      setItemDefinitions(prev => new Map(prev).set(newDef.id.toString(), newDef));
    };
    const handleItemDefDelete = (ctx: any, itemDef: SpacetimeDB.ItemDefinition) => {
      console.log('Item Definition Deleted:', itemDef.name, itemDef.id);
      setItemDefinitions(prev => {
          const newMap = new Map(prev);
          newMap.delete(itemDef.id.toString());
          return newMap;
      });
    };

    // --- InventoryItem Callbacks ---
    const handleInventoryInsert = (ctx: any, invItem: SpacetimeDB.InventoryItem) => {
      const instanceIdStr = invItem.instanceId.toString();
      console.log(`[handleInventoryInsert] Received insert for item instance: ${instanceIdStr}, Player: ${invItem.playerIdentity.toHexString()}, InvSlot: ${invItem.inventorySlot}, HotbarSlot: ${invItem.hotbarSlot}`); 

      // Always update the state map, regardless of owner
      setInventoryItems(prev => {
          const newMap = new Map(prev);
          newMap.set(instanceIdStr, invItem);
          // Log map update - check if it belongs to local player for info
          const isLocalPlayerItem = connection?.identity && invItem.playerIdentity.isEqual(connection.identity);
          console.log(`[handleInventoryInsert] Updated inventoryItems map. Item belongs to local player? ${isLocalPlayerItem}. New size: ${newMap.size}. Contains key ${instanceIdStr}? ${newMap.has(instanceIdStr)}`); 
          return newMap;
      });
    };
    const handleInventoryUpdate = (ctx: any, oldItem: SpacetimeDB.InventoryItem, newItem: SpacetimeDB.InventoryItem) => {
      const instanceIdStr = newItem.instanceId.toString(); 
      
      // *** SPECIFIC LOGGING FOR CAMPFIRE DEBUG ***
      const oldInvSlot = oldItem.inventorySlot;
      const oldHotbarSlot = oldItem.hotbarSlot;
      const newInvSlot = newItem.inventorySlot;
      const newHotbarSlot = newItem.hotbarSlot;

      // Check if the item was moved *out* of a regular slot (potentially into a campfire)
      if ((oldInvSlot !== null || oldHotbarSlot !== null) && (newInvSlot === null && newHotbarSlot === null)) {
          console.log(`[DEBUG_CAMPFIRE] Item ${instanceIdStr} slots changed FROM (Inv: ${oldInvSlot}, Hotbar: ${oldHotbarSlot}) TO (Inv: ${newInvSlot}, Hotbar: ${newHotbarSlot}). This item might be going into a campfire.`);
      }
      // *** END SPECIFIC LOGGING ***

      // Original verbose log (can be removed later)
      console.log(`[handleInventoryUpdate] Received update for item instance: ${instanceIdStr}, Player: ${newItem.playerIdentity.toHexString()}, InvSlot: ${newItem.inventorySlot}, HotbarSlot: ${newItem.hotbarSlot}`);

      // Always update the state map, regardless of owner
      setInventoryItems(prev => {
          const newMap = new Map(prev);
          newMap.set(instanceIdStr, newItem);
          // Log map update - check if it belongs to local player for info
          const isLocalPlayerItem = connection?.identity && newItem.playerIdentity.isEqual(connection.identity);
          console.log(`[handleInventoryUpdate] Updated inventoryItems map. Item belongs to local player? ${isLocalPlayerItem}. New size: ${newMap.size}. Contains key ${instanceIdStr}? ${newMap.has(instanceIdStr)}`); 
          return newMap;
      });
    };
    const handleInventoryDelete = (ctx: any, invItem: SpacetimeDB.InventoryItem) => {
      const instanceIdStr = invItem.instanceId.toString(); 
       // Log ALL deletes
      console.log(`[handleInventoryDelete] Received delete for item instance: ${instanceIdStr}, Player: ${invItem.playerIdentity.toHexString()}`);

      if (connection?.identity && invItem.playerIdentity.isEqual(connection.identity)) {
          setInventoryItems(prev => {
              const newMap = new Map(prev);
              const deleted = newMap.delete(instanceIdStr);
               // Log map update
              console.log(`[handleInventoryDelete] Updated inventoryItems map for local player. Deleted key ${instanceIdStr}? ${deleted}. New size: ${newMap.size}.`);
              return newMap;
          });
       } 
    };

    // --- WorldState Callbacks --- 
    const handleWorldStateInsert = (ctx: any, state: SpacetimeDB.WorldState) => {
      console.log('WorldState Inserted:', state);
      setWorldState(state);
    };

    const handleWorldStateUpdate = (ctx: any, oldState: SpacetimeDB.WorldState, newState: SpacetimeDB.WorldState) => {
      // Only update state if key properties have changed to avoid infinite loops
      const significantChange = 
        oldState.timeOfDay !== newState.timeOfDay || 
        oldState.isFullMoon !== newState.isFullMoon ||
        oldState.cycleCount !== newState.cycleCount; // Add cycleCount check?
        // Add other critical fields if necessary

      if (significantChange) {
        // console.log(`WorldState significant update: ${JSON.stringify(newState.timeOfDay)}, Full Moon: ${newState.isFullMoon}, Cycle: ${newState.cycleCount}`);
        setWorldState(newState);
      } else {
        // Log minor updates if needed for debugging, but don't set state
        // console.log("WorldState minor update ignored (e.g., cycleProgress)");
      }
    };

    // WorldState is a singleton, so delete shouldn't really happen, but good to have
    const handleWorldStateDelete = (ctx: any, state: SpacetimeDB.WorldState) => {
      console.warn('WorldState Deleted:', state);
      setWorldState(null);
    };

    // --- ActiveEquipment Callbacks ---
    const handleActiveEquipmentInsert = (ctx: any, equip: SpacetimeDB.ActiveEquipment) => {
      console.log('ActiveEquipment Inserted for player:', equip.playerIdentity.toHexString());
      setActiveEquipments(prev => new Map(prev).set(equip.playerIdentity.toHexString(), equip));
    };
    const handleActiveEquipmentUpdate = (ctx: any, oldEquip: SpacetimeDB.ActiveEquipment, newEquip: SpacetimeDB.ActiveEquipment) => {
      console.log('ActiveEquipment Updated for player:', newEquip.playerIdentity.toHexString());
      setActiveEquipments(prev => new Map(prev).set(newEquip.playerIdentity.toHexString(), newEquip));
    };
    const handleActiveEquipmentDelete = (ctx: any, equip: SpacetimeDB.ActiveEquipment) => {
      console.log('ActiveEquipment Deleted for player:', equip.playerIdentity.toHexString());
      setActiveEquipments(prev => {
          const newMap = new Map(prev);
          newMap.delete(equip.playerIdentity.toHexString());
          return newMap;
      });
    };

    // --- Mushroom Callbacks ---
    const handleMushroomInsert = (ctx: any, mushroom: SpacetimeDB.Mushroom) => {
      console.log('Mushroom Inserted:', mushroom.id);
      setMushrooms(prev => new Map(prev).set(mushroom.id.toString(), mushroom));
    };
    const handleMushroomUpdate = (ctx: any, oldMushroom: SpacetimeDB.Mushroom, newMushroom: SpacetimeDB.Mushroom) => {
      // Mushrooms currently have no updatable fields, but include for future
      console.log('Mushroom Updated:', newMushroom.id);
      setMushrooms(prev => new Map(prev).set(newMushroom.id.toString(), newMushroom));
    };
    const handleMushroomDelete = (ctx: any, mushroom: SpacetimeDB.Mushroom) => {
      console.log('Mushroom Deleted:', mushroom.id);
      setMushrooms(prev => {
          const newMap = new Map(prev);
          newMap.delete(mushroom.id.toString());
          return newMap;
      });
    };

    // --- NEW: DroppedItem Callbacks ---
    const handleDroppedItemInsert = (ctx: any, item: SpacetimeDB.DroppedItem) => {
      console.log('DroppedItem Inserted:', item.id, 'DefID:', item.itemDefId, 'Qty:', item.quantity);
      setDroppedItems(prev => new Map(prev).set(item.id.toString(), item));
    };
    const handleDroppedItemUpdate = (ctx: any, oldItem: SpacetimeDB.DroppedItem, newItem: SpacetimeDB.DroppedItem) => {
      // Updates might happen if quantity changes later, but unlikely for now
      console.log('DroppedItem Updated:', newItem.id);
      setDroppedItems(prev => new Map(prev).set(newItem.id.toString(), newItem));
    };
    const handleDroppedItemDelete = (ctx: any, item: SpacetimeDB.DroppedItem) => {
      console.log('DroppedItem Deleted:', item.id);
      setDroppedItems(prev => {
          const newMap = new Map(prev);
          newMap.delete(item.id.toString());
          return newMap;
      });
    };

    // Register the callbacks for Player
    connection.db.player.onInsert(handlePlayerInsert);
    connection.db.player.onUpdate(handlePlayerUpdate);
    connection.db.player.onDelete(handlePlayerDelete);
    
    // Register the callbacks for Tree
    connection.db.tree.onInsert(handleTreeInsert);
    connection.db.tree.onUpdate(handleTreeUpdate);
    connection.db.tree.onDelete(handleTreeDelete);

    // Register the callbacks for Stone
    connection.db.stone.onInsert(handleStoneInsert);
    connection.db.stone.onUpdate(handleStoneUpdate);
    connection.db.stone.onDelete(handleStoneDelete);

    // Register the callbacks for Campfire
    connection.db.campfire.onInsert(handleCampfireInsert);
    connection.db.campfire.onUpdate(handleCampfireUpdate);
    connection.db.campfire.onDelete(handleCampfireDelete);

    // Register the callbacks for ItemDefinition
    connection.db.itemDefinition.onInsert(handleItemDefInsert);
    connection.db.itemDefinition.onUpdate(handleItemDefUpdate);
    connection.db.itemDefinition.onDelete(handleItemDefDelete);

    // Register the callbacks for InventoryItem
    connection.db.inventoryItem.onInsert(handleInventoryInsert);
    connection.db.inventoryItem.onUpdate(handleInventoryUpdate);
    connection.db.inventoryItem.onDelete(handleInventoryDelete);

    // Register the callbacks for WorldState
    connection.db.worldState.onInsert(handleWorldStateInsert);
    connection.db.worldState.onUpdate(handleWorldStateUpdate);
    connection.db.worldState.onDelete(handleWorldStateDelete);

    // Register the callbacks for ActiveEquipment
    connection.db.activeEquipment.onInsert(handleActiveEquipmentInsert);
    connection.db.activeEquipment.onUpdate(handleActiveEquipmentUpdate);
    connection.db.activeEquipment.onDelete(handleActiveEquipmentDelete);

    // Register Mushroom callbacks
    connection.db.mushroom.onInsert(handleMushroomInsert);
    connection.db.mushroom.onUpdate(handleMushroomUpdate);
    connection.db.mushroom.onDelete(handleMushroomDelete);

    // NEW: Register DroppedItem callbacks
    connection.db.droppedItem.onInsert(handleDroppedItemInsert);
    connection.db.droppedItem.onUpdate(handleDroppedItemUpdate);
    connection.db.droppedItem.onDelete(handleDroppedItemDelete);

    // --- Subscriptions --- 
    console.log('Subscribing to Player, Tree, Stone, Campfire, ItemDefinition, InventoryItem, WorldState, DroppedItem tables...');
    playerSubscription = connection.subscriptionBuilder()
      .onApplied(() => console.log('Subscription to Player table APPLIED.'))
      .onError((ctx: SpacetimeDB.ErrorContext) => {
        console.error('Subscription to Player table FAILED. Context:', ctx);
        setError(`Player subscription failed. Check console.`);
      })
      .subscribe('SELECT * FROM player');

    treeSubscription = connection.subscriptionBuilder()
      .onApplied(() => console.log('Subscription to Tree table APPLIED.'))
      .onError((ctx: SpacetimeDB.ErrorContext) => {
        console.error('Subscription to Tree table FAILED. Context:', ctx);
        setError(`Tree subscription failed. Check console.`);
      })
      .subscribe('SELECT * FROM tree');

    stoneSubscription = connection.subscriptionBuilder()
      .onApplied(() => console.log('Subscription to Stone table APPLIED.'))
      .onError((ctx: SpacetimeDB.ErrorContext) => {
        console.error('Subscription to Stone table FAILED. Context:', ctx);
        setError(`Stone subscription failed. Check console.`);
      })
      .subscribe('SELECT * FROM stone');

    // Subscribe to Campfire
    campfireSubscription = connection.subscriptionBuilder()
      .onApplied(() => console.log('Subscription to Campfire table APPLIED.'))
      .onError((ctx: SpacetimeDB.ErrorContext) => {
        console.error('Subscription to Campfire table FAILED. Context:', ctx);
        setError(`Campfire subscription failed. Check console.`);
      })
      .subscribe('SELECT * FROM campfire');

    itemDefSubscription = connection.subscriptionBuilder()
      .onApplied(() => console.log('Subscription to ItemDefinition table APPLIED.'))
      .onError((ctx: SpacetimeDB.ErrorContext) => {
        console.error('Subscription to ItemDefinition table FAILED. Context:', ctx);
        setError(`ItemDefinition subscription failed. Check console.`);
      })
      .subscribe('SELECT * FROM item_definition');

    // Subscribe to ALL inventory items - filtering happens in callbacks/rendering
    inventorySubscription = connection.subscriptionBuilder()
      .onApplied(() => console.log('Subscription to InventoryItem table APPLIED.'))
      .onError((ctx: SpacetimeDB.ErrorContext) => {
        console.error('Subscription to InventoryItem table FAILED. Context:', ctx);
        setError(`InventoryItem subscription failed. Check console.`);
      })
      .subscribe('SELECT * FROM inventory_item');

    // Subscribe to WorldState (singleton)
    worldStateSubscription = connection.subscriptionBuilder()
      .onApplied(() => console.log('Subscription to WorldState table APPLIED.'))
      .onError((ctx: SpacetimeDB.ErrorContext) => {
        console.error('Subscription to WorldState table FAILED. Context:', ctx);
        setError(`WorldState subscription failed. Check console.`);
      })
      .subscribe('SELECT * FROM world_state');

    // Subscribe to ActiveEquipment
    activeEquipmentSubscription = connection.subscriptionBuilder()
      .onApplied(() => console.log('Subscription to ActiveEquipment table APPLIED.'))
      .onError((ctx: SpacetimeDB.ErrorContext) => {
        console.error('Subscription to ActiveEquipment table FAILED. Context:', ctx);
        setError(`ActiveEquipment subscription failed. Check console.`);
      })
      .subscribe('SELECT * FROM active_equipment');

    // Subscribe to Mushroom
    mushroomSubscription = connection.subscriptionBuilder()
      .onApplied(() => console.log('Subscription to Mushroom table APPLIED.'))
      .onError((ctx: SpacetimeDB.ErrorContext) => {
        console.error('Subscription to Mushroom table FAILED. Context:', ctx);
        setError(`Mushroom subscription failed. Check console.`);
      })
      .subscribe('SELECT * FROM mushroom');

    // NEW: Subscribe to DroppedItem
    droppedItemSubscription = connection.subscriptionBuilder()
      .onApplied(() => console.log('Subscription to DroppedItem table APPLIED.'))
      .onError((ctx: SpacetimeDB.ErrorContext) => {
        console.error('Subscription to DroppedItem table FAILED. Context:', ctx);
        setError(`DroppedItem subscription failed. Check console.`);
      })
      .subscribe('SELECT * FROM dropped_item');

    // Cleanup function for this effect
    return () => {
      console.log('Cleaning up Player, Tree, Stone, Campfire, ItemDefinition, InventoryItem, WorldState, DroppedItem table subscriptions...');
      // Remove Player listeners
      connection.db.player.removeOnInsert(handlePlayerInsert);
      connection.db.player.removeOnUpdate(handlePlayerUpdate);
      connection.db.player.removeOnDelete(handlePlayerDelete);
      // Remove Tree listeners
      connection.db.tree.removeOnInsert(handleTreeInsert);
      connection.db.tree.removeOnUpdate(handleTreeUpdate);
      connection.db.tree.removeOnDelete(handleTreeDelete);
      // Remove Stone listeners
      connection.db.stone.removeOnInsert(handleStoneInsert);
      connection.db.stone.removeOnUpdate(handleStoneUpdate);
      connection.db.stone.removeOnDelete(handleStoneDelete);
      // Remove Campfire listeners
      connection.db.campfire.removeOnInsert(handleCampfireInsert);
      connection.db.campfire.removeOnUpdate(handleCampfireUpdate);
      connection.db.campfire.removeOnDelete(handleCampfireDelete);
      // Remove ItemDefinition listeners
      connection.db.itemDefinition.removeOnInsert(handleItemDefInsert);
      connection.db.itemDefinition.removeOnUpdate(handleItemDefUpdate);
      connection.db.itemDefinition.removeOnDelete(handleItemDefDelete);
      
      // Remove InventoryItem listeners
      connection.db.inventoryItem.removeOnInsert(handleInventoryInsert);
      connection.db.inventoryItem.removeOnUpdate(handleInventoryUpdate);
      connection.db.inventoryItem.removeOnDelete(handleInventoryDelete);
      
      // Remove WorldState listeners
      connection.db.worldState.removeOnInsert(handleWorldStateInsert);
      connection.db.worldState.removeOnUpdate(handleWorldStateUpdate);
      connection.db.worldState.removeOnDelete(handleWorldStateDelete);
      
      // Remove ActiveEquipment listeners
      connection.db.activeEquipment.removeOnInsert(handleActiveEquipmentInsert);
      connection.db.activeEquipment.removeOnUpdate(handleActiveEquipmentUpdate);
      connection.db.activeEquipment.removeOnDelete(handleActiveEquipmentDelete);
      
      // Remove Mushroom listeners
      connection.db.mushroom.removeOnInsert(handleMushroomInsert);
      connection.db.mushroom.removeOnUpdate(handleMushroomUpdate);
      connection.db.mushroom.removeOnDelete(handleMushroomDelete);
      
      // NEW: Remove DroppedItem listeners
      connection.db.droppedItem.removeOnInsert(handleDroppedItemInsert);
      connection.db.droppedItem.removeOnUpdate(handleDroppedItemUpdate);
      connection.db.droppedItem.removeOnDelete(handleDroppedItemDelete);
      
      // Unsubscribe from queries
      if (playerSubscription) playerSubscription.unsubscribe();
      if (treeSubscription) treeSubscription.unsubscribe();
      if (stoneSubscription) stoneSubscription.unsubscribe();
      if (campfireSubscription) campfireSubscription.unsubscribe();
      if (itemDefSubscription) itemDefSubscription.unsubscribe();
      if (inventorySubscription) inventorySubscription.unsubscribe();
      if (worldStateSubscription) worldStateSubscription.unsubscribe();
      if (activeEquipmentSubscription) activeEquipmentSubscription.unsubscribe();
      if (mushroomSubscription) mushroomSubscription.unsubscribe(); // Unsubscribe mushroom
      // NEW: Unsubscribe droppedItem
      if (droppedItemSubscription) droppedItemSubscription.unsubscribe();
    };
    
  }, [connection]); // Re-run this effect if the connection object changes
  
  // Handle player registration
  const handleRegisterPlayer = () => {
    if (connection && username.trim()) {
      setIsLoading(true);
      console.log(`Calling registerPlayer reducer with username: ${username}`);
      try {
        connection.reducers.registerPlayer(username);
      } catch (err) {
        console.error('Failed to register player:', err);
        setError('Failed to register player. Please try again.');
      } finally {
        // Keep isLoading true until successful connection confirmed by subscription update
        // setIsLoading(false); 
      }
    }
  };
  
  // --- Handle Enter key submission --- 
  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter' && !isLoading && username.trim()) {
      handleRegisterPlayer();
    }
  };

  // --- Autofocus on initial render --- 
  useEffect(() => {
    if (!isConnected && usernameInputRef.current) {
      usernameInputRef.current.focus();
    }
  }, [isConnected]); // Re-run if connection status changes (e.g., disconnect)
  
  // --- Prevent global context menu --- 
  useEffect(() => {
    const handleGlobalContextMenu = (event: MouseEvent) => {
      event.preventDefault();
    };
    window.addEventListener('contextmenu', handleGlobalContextMenu);

    // Cleanup listener on component unmount
    return () => {
      window.removeEventListener('contextmenu', handleGlobalContextMenu);
    };
  }, []); // Empty dependency array ensures this runs only once on mount
  
  // --- Reducer Calls ---
  // Function to call the updatePlayerPosition reducer
  const updatePlayerPosition = (dx: number, dy: number, intendedDirection: 'up' | 'down' | 'left' | 'right' | null = null) => {
    if (!connection?.reducers) return;

    // Log the direction being sent
    // console.log(`Calling updatePlayerPosition with dx: ${dx}, dy: ${dy}, direction: ${intendedDirection}`);

    try {
      // Call the reducer, passing the optional direction
      connection.reducers.updatePlayerPosition(dx, dy, intendedDirection ?? undefined);
    } catch (error) {
      console.error("Error calling updatePlayerPosition reducer:", error);
    }
  };
  
  // Function to call the setSprinting reducer
  const callSetSprintingReducer = (isSprinting: boolean) => {
    if (connection) {
      try {
        // Call the new SpacetimeDB reducer using camelCase
        connection.reducers.setSprinting(isSprinting);
      } catch (err) {
        console.error('Failed to call setSprinting reducer:', err);
      }
    }
  };
  
  // --- Jump Reducer Call ---
  const callJumpReducer = () => {
    if (connection) {
      try {
        // Call the actual SpacetimeDB reducer using camelCase
        connection.reducers.jump();
      } catch (err) {
        console.error('Failed to call jump reducer:', err);
        // Optionally set an error state here
      }
    }
  };
  
  // --- Campfire Placement Handlers ---
  const startCampfirePlacement = () => {
    console.log("Starting campfire placement mode.");
    setIsPlacingCampfire(true);
    setPlacementError(null); // Clear previous errors
  };

  const handlePlaceCampfire = (worldX: number, worldY: number) => {
    if (!connection || !isPlacingCampfire) return;

    console.log(`Attempting to place campfire at (${worldX}, ${worldY})`);
    setPlacementError(null); // Clear previous error before attempting
    try {
      // Call the SpacetimeDB reducer using camelCase
      connection.reducers.placeCampfire(worldX, worldY);
    } catch (err: any) {
      console.error('Failed to call place campfire reducer (client-side error):', err);
      // Display the error message 
      const errorMessage = err?.message || "Failed to place campfire. Check logs.";
      setError(`Placement failed: ${errorMessage}`); // Show general error
      setPlacementError(errorMessage); // Keep specific error for potential placement UI
      // Do NOT exit placement mode on error
    }
  };

  const cancelCampfirePlacement = () => {
    console.log("Cancelling campfire placement mode.");
    setIsPlacingCampfire(false);
    setPlacementError(null); // Clear error on cancel
  };
  
  // --- LIFTED Drag/Drop Handlers --- 
  const handleItemDragStart = useCallback((info: DraggedItemInfo) => {
    console.log("[App] Drag Start:", info);
    setDraggedItemInfo(info); // Set state, which will update the ref via useEffect
    document.body.classList.add('item-dragging');
  }, []); // No dependencies needed here

  // Use useCallback to memoize the drop handler
  const handleItemDrop = useCallback((targetSlot: DragSourceSlotInfo | null) => {
    console.log("[App] Drop Target:", targetSlot);
    document.body.classList.remove('item-dragging');
    const sourceInfo = draggedItemRef.current;
    // Always clear drag state, regardless of whether drop is valid
    draggedItemRef.current = null;
    setDraggedItemInfo(null);

    if (!sourceInfo) { 
      console.log("[App Drop] No source info found, ignoring drop.");
      return; 
    } // Early exit if no source
    if (!connection?.reducers) { 
        console.log("[App Drop] No reducers connection, ignoring drop.");
        return; 
    } // Early exit if no connection

    const itemInstanceId = BigInt(sourceInfo.item.instance.instanceId);

    // --- Handle Dropping Item into the World --- 
    if (targetSlot === null) {
      console.log(`[App Drop] Target is NULL. Dropping item ${itemInstanceId} into the world.`);
      const quantityToDrop = sourceInfo.splitQuantity ?? sourceInfo.item.instance.quantity;
      try {
          connection.reducers.dropItem(itemInstanceId, quantityToDrop);
      } catch (error) {
          console.error("[App Drop] Error calling dropItem reducer:", error);
          setError(`Failed to drop item: ${error}`); // Show user feedback
      }
      return; // Drop handled, exit
    }

    // --- Proceed with existing logic for dropping onto a slot --- 
    console.log(`[App Drop] Processing drop onto slot: Item ${itemInstanceId} from ${sourceInfo.sourceSlot.type}:${sourceInfo.sourceSlot.index} to ${targetSlot.type}:${targetSlot.index}`);

    try {
        // --- Handle Stack Splitting First (if target is valid slot) --- 
        if (sourceInfo.splitQuantity && sourceInfo.splitQuantity > 0) {
            const quantityToSplit = sourceInfo.splitQuantity;
            const sourceSlotType = sourceInfo.sourceSlot.type;
            const targetSlotType = targetSlot.type;
            // We need the Campfire ID if either source or target is a campfire slot
            // Let's assume it's passed via sourceSlot.parentId or targetSlot.parentId for now
            // This might need adjustment based on how InventoryUI passes the info.
            const campfireId = sourceInfo.sourceSlot.parentId ?? targetSlot.parentId;
            const campfireIdNum = campfireId ? Number(campfireId) : null;

            console.log(`[App Drop] Initiating SPLIT: Qty ${quantityToSplit} onto ${targetSlotType}:${targetSlot.index} (Campfire context: ${campfireIdNum})`);

            // Ensure target index is a number (can be string for equipment)
            let targetIndexNum: number | null = null;
            if (typeof targetSlot.index === 'number') {
                targetIndexNum = targetSlot.index;
            } else if (typeof targetSlot.index === 'string') {
                if (targetSlotType === 'inventory' || targetSlotType === 'hotbar') {
                    targetIndexNum = parseInt(targetSlot.index, 10);
                    if (isNaN(targetIndexNum)) {
                        console.error("[App Drop] Split failed: Target index string is not a valid number for inv/hotbar.");
                        setError("Invalid split target slot.");
                        return;
                    }
                } else if (targetSlotType !== 'equipment') { // Allow string index only for equipment
                    console.error("[App Drop] Split failed: Target index string invalid for type:", targetSlotType);
                    setError("Invalid split target slot.");
                    return;
                }
            } else {
                 console.error("[App Drop] Split failed: Target index is invalid type.");
                 setError("Invalid split target slot.");
                 return;
            }

            // Call appropriate split reducer based on source and target
            if (sourceSlotType === 'inventory' || sourceSlotType === 'hotbar') {
                 if (targetSlotType === 'inventory' || targetSlotType === 'hotbar') {
                     if (targetIndexNum === null) { console.error("Target index null for inv/hotbar split"); return; }
                     console.log(`Calling splitStack with target ${targetSlotType}, index ${targetIndexNum}`);
                     connection.reducers.splitStack(itemInstanceId, quantityToSplit, targetSlotType, targetIndexNum);
                 } else if (targetSlotType === 'campfire_fuel') {
                     // --- FIX #2: Use interactingWith for splitting INTO campfire --- 
                     const targetFuelIndex = typeof targetSlot.index === 'number' ? targetSlot.index : parseInt(targetSlot.index.toString(), 10);

                     // Get campfire ID from interaction context when splitting INTO it
                     const targetCampfireId = interactingWith?.type === 'campfire' ? Number(interactingWith.id) : null;
                     
                     if (targetCampfireId === null || isNaN(targetFuelIndex)) { 
                         console.error("[App Drop] Missing CampfireID (from interactingWith) or TargetIndex for split INTO campfire"); 
                         setError("Could not determine target campfire slot for split.");
                         return; 
                     }
                     
                     console.log(`Calling splitStackIntoCampfire: Item ${itemInstanceId}, Qty ${quantityToSplit} to Campfire ${targetCampfireId} Slot ${targetFuelIndex}`);
                     if (connection.reducers.splitStackIntoCampfire) {
                         connection.reducers.splitStackIntoCampfire(itemInstanceId, quantityToSplit, targetCampfireId, targetFuelIndex);
                     } else {
                         console.error("Reducer 'splitStackIntoCampfire' not found!");
                         setError("Splitting into campfire not supported.");
                     }
                 } else {
                      console.warn(`[App Drop] Split ignored: Cannot split from ${sourceSlotType} to ${targetSlotType}`);
                      setError("Cannot split item to that location.");
                 }
            } else if (sourceSlotType === 'campfire_fuel') {
                 // Splitting FROM campfire should use parentId from sourceSlot
                 const sourceCampfireId = sourceInfo.sourceSlot.parentId ? Number(sourceInfo.sourceSlot.parentId) : null;
                 const sourceIndexNum = typeof sourceInfo.sourceSlot.index === 'number' ? sourceInfo.sourceSlot.index : parseInt(sourceInfo.sourceSlot.index.toString(), 10);
                 
                 if (sourceCampfireId === null || isNaN(sourceIndexNum)) { 
                    console.error("[App Drop] Missing CampfireID or SourceIndex for split FROM campfire"); 
                    setError("Could not determine source campfire slot for split.");
                    return; 
                 }

                 if (targetSlotType === 'inventory' || targetSlotType === 'hotbar') {
                     if (targetIndexNum === null) { console.error("Target index null for inv/hotbar split from campfire"); return; }
                     console.log(`Calling splitStackFromCampfire: Campfire ${sourceCampfireId} Slot ${sourceIndexNum}, Item ${itemInstanceId}, Qty ${quantityToSplit} to ${targetSlotType}:${targetIndexNum}`);
                     connection.reducers.splitStackFromCampfire(sourceCampfireId, sourceIndexNum, quantityToSplit, targetSlotType, targetIndexNum);
                 } else if (targetSlotType === 'campfire_fuel') {
                     // Splitting WITHIN campfire - Target index already parsed, sourceCampfireId is correct.
                     if (targetIndexNum === null) { console.error("Target index null for within-campfire split"); return; }
                     console.log(`Calling splitStackWithinCampfire: Campfire ${sourceCampfireId}, Source Slot ${sourceIndexNum}, Item ${itemInstanceId}, Qty ${quantityToSplit} to Target Slot ${targetIndexNum}`);
                     if (connection.reducers.splitStackWithinCampfire) {
                         connection.reducers.splitStackWithinCampfire(sourceCampfireId, sourceIndexNum, quantityToSplit, targetIndexNum);
                     } else {
                         console.error("Reducer 'splitStackWithinCampfire' not found!");
                         setError("Splitting within campfire not supported.");
                     }
                 } else {
                      console.warn(`[App Drop] Split ignored: Cannot split from ${sourceSlotType} to ${targetSlotType}`);
                      setError("Cannot split item to that location.");
                 }
            } else {
                console.warn(`[App Drop] Split ignored: Cannot split from source type ${sourceSlotType}`);
                setError("Cannot split from this item source.");
            }
            return; // Split attempt handled, exit
        }

        // --- Standard Item Move (Full Stack) ---
        // Based on target slot type, call the appropriate reducer
        if (targetSlot.type === 'inventory') {
            // Ensure target index is a number
            const targetIndexNum = typeof targetSlot.index === 'number' ? targetSlot.index : parseInt(targetSlot.index.toString(), 10);
            if (isNaN(targetIndexNum)) { console.error("Invalid inventory index", targetSlot.index); return; }
            connection.reducers.moveItemToInventory(itemInstanceId, targetIndexNum);
        } else if (targetSlot.type === 'hotbar') {
            const targetIndexNum = typeof targetSlot.index === 'number' ? targetSlot.index : parseInt(targetSlot.index.toString(), 10);
            if (isNaN(targetIndexNum)) { console.error("Invalid hotbar index", targetSlot.index); return; }
             connection.reducers.moveItemToHotbar(itemInstanceId, targetIndexNum);
        } else if (targetSlot.type === 'equipment' && typeof targetSlot.index === 'string') {
            // Equipment uses string index ('Head', 'Chest', etc.)
            connection.reducers.equipArmorFromDrag(itemInstanceId, targetSlot.index);
        } else if (targetSlot.type === 'campfire_fuel') {
            const targetIndexNum = typeof targetSlot.index === 'number' ? targetSlot.index : parseInt(targetSlot.index.toString(), 10);
            if (isNaN(targetIndexNum)) { console.error("Invalid campfire fuel index", targetSlot.index); return; }
            
            // Determine Campfire ID (using parentId if available, otherwise fallback to interactionTarget)
            let campfireIdNum: number | null = null;
            if (targetSlot.parentId) {
                campfireIdNum = Number(targetSlot.parentId);
            } else if (sourceInfo.sourceSlot.parentId) { // Check source slot if target doesn't have it
                 campfireIdNum = Number(sourceInfo.sourceSlot.parentId);
            } else if (interactingWith?.type === 'campfire') { // Fallback to interaction target
                campfireIdNum = Number(interactingWith.id);
            }

            if (campfireIdNum === null || isNaN(campfireIdNum)) {
                console.error("[App Drop] Cannot move to/within campfire slot: Campfire ID could not be determined.");
                setError("Cannot move item: Campfire context lost.");
                return;
            }

            // --- Differentiate between adding fuel and moving fuel --- 
            if (sourceInfo.sourceSlot.type === 'campfire_fuel') {
                 // Moving FROM a campfire slot TO another campfire slot
                 const sourceIndexNum = typeof sourceInfo.sourceSlot.index === 'number' ? sourceInfo.sourceSlot.index : parseInt(sourceInfo.sourceSlot.index.toString(), 10);
                 if (isNaN(sourceIndexNum)) { console.error("Invalid source campfire fuel index", sourceInfo.sourceSlot.index); return; }
                 
                 console.log(`[App Drop] Calling moveFuelWithinCampfire for campfire ${campfireIdNum} from slot ${sourceIndexNum} to ${targetIndexNum}`);
                 connection.reducers.moveFuelWithinCampfire(campfireIdNum, sourceIndexNum, targetIndexNum);
             } else {
                 // Moving FROM inventory/hotbar TO a campfire slot
                 console.log(`[App Drop] Calling addFuelToCampfire for item ${itemInstanceId} to campfire ${campfireIdNum} slot ${targetIndexNum}`);
                 connection.reducers.addFuelToCampfire(campfireIdNum, targetIndexNum, itemInstanceId);
             }
        } else {
            console.warn("Unhandled drop target type or index:", targetSlot);
            setError("Cannot drop item here.");
        }

    } catch (error: any) {
        console.error("[App Drop] Error calling reducer:", error);
         setError(`Action failed: ${error?.message || error}`); // Show user feedback
    }

  }, [connection, interactingWith]); // Add interactingWith as dependency
  
  return (
    <div className="App" style={{ backgroundColor: '#111' }}>
      {error && <div className="error-message">{error}</div>}
      
      {!isConnected ? (
        <div style={{ /* Centering styles */
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          minHeight: '100vh',
          width: '100%',
          fontFamily: UI_FONT_FAMILY,
        }}>
          <div style={{ /* Login Box Styles */
            backgroundColor: UI_BG_COLOR,
            color: 'white',
            padding: '40px',
            borderRadius: '4px',
            border: `1px solid ${UI_BORDER_COLOR}`,
            boxShadow: UI_SHADOW,
            textAlign: 'center',
            minWidth: '350px',
          }}>
            <img
              src={githubLogo}
              alt="GitHub Logo"
              style={{ 
                width: '300px',
                height: '200px',
                marginBottom: '25px',
              }}
            />
            <input
              ref={usernameInputRef}
              type="text"
              placeholder="Enter Username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              onKeyDown={handleKeyDown}
              disabled={isLoading}
              style={{ /* Input Styles */
                padding: '10px',
                marginBottom: '15px',
                border: `1px solid ${UI_BORDER_COLOR}`,
                backgroundColor: '#333',
                color: 'white',
                fontFamily: UI_FONT_FAMILY,
                fontSize: '14px',
                display: 'block',
                width: 'calc(100% - 22px)',
                textAlign: 'center',
              }}
            />
            <button
              onClick={handleRegisterPlayer}
              disabled={isLoading || !username.trim()}
              style={{ /* Button Styles */
                padding: '10px 20px',
                border: `1px solid ${UI_BORDER_COLOR}`,
                backgroundColor: isLoading ? '#555' : '#777',
                color: isLoading ? '#aaa' : 'white',
                fontFamily: UI_FONT_FAMILY,
                fontSize: '14px',
                cursor: (isLoading || !username.trim()) ? 'not-allowed' : 'pointer',
                boxShadow: UI_SHADOW,
              }}
            >
              {isLoading ? 'Joining...' : 'Join Game'}
            </button>
          </div>
        </div>
      ) : (
        <div className="game-container">
          <GameCanvas 
            players={players}
            trees={trees}
            stones={stones}
            campfires={campfires}
            mushrooms={mushrooms}
            droppedItems={droppedItems}
            inventoryItems={inventoryItems}
            itemDefinitions={itemDefinitions}
            worldState={worldState}
            localPlayerId={connection?.identity?.toHexString() ?? undefined}
            connection={connection}
            activeEquipments={activeEquipments}
            updatePlayerPosition={updatePlayerPosition}
            callJumpReducer={callJumpReducer}
            callSetSprintingReducer={callSetSprintingReducer}
            isPlacingCampfire={isPlacingCampfire}
            handlePlaceCampfire={handlePlaceCampfire}
            cancelCampfirePlacement={cancelCampfirePlacement}
            placementError={placementError}
            onSetInteractingWith={setInteractingWith}
          />
          <PlayerUI 
            identity={connection?.identity || null}
            players={players}
            inventoryItems={inventoryItems}
            itemDefinitions={itemDefinitions}
            connection={connection}
            startCampfirePlacement={startCampfirePlacement}
            cancelCampfirePlacement={cancelCampfirePlacement}
            onItemDragStart={handleItemDragStart}
            onItemDrop={handleItemDrop}
            draggedItemInfo={draggedItemInfo}
            activeEquipments={activeEquipments}
            campfires={campfires}
            interactingWith={interactingWith}
            onSetInteractingWith={setInteractingWith}
          />
          <Hotbar 
            playerIdentity={connection?.identity || null}
            itemDefinitions={itemDefinitions}
            inventoryItems={inventoryItems}
            startCampfirePlacement={startCampfirePlacement}
            cancelCampfirePlacement={cancelCampfirePlacement}
            connection={connection}
            onItemDragStart={handleItemDragStart}
            onItemDrop={handleItemDrop}
            draggedItemInfo={draggedItemInfo}
            interactingWith={interactingWith}
            campfires={campfires}
          />
        </div>
      )}
    </div>
  );
}

export default App;
