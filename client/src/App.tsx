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

    console.log('Setting up Player, Tree, Stone, Campfire, ItemDefinition, InventoryItem, and WorldState table subscriptions...');

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

    // --- Subscriptions --- 
    console.log('Subscribing to Player, Tree, Stone, Campfire, ItemDefinition, InventoryItem, and WorldState tables...');
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

    // Cleanup function for this effect
    return () => {
      console.log('Cleaning up Player, Tree, Stone, Campfire, ItemDefinition, InventoryItem, and WorldState table subscriptions...');
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
    try {
      // Call the SpacetimeDB reducer using camelCase
      connection.reducers.placeCampfire(worldX, worldY);
      setIsPlacingCampfire(false); // Exit placement mode on successful call attempt
      setPlacementError(null);
    } catch (err: any) {
      console.error('Failed to place campfire:', err);
      // Display the error message from the server if available
      const errorMessage = err?.message || "Failed to place campfire. Check logs.";
      setError(`Placement failed: ${errorMessage}`); // Show general error
      setPlacementError(errorMessage); // Keep specific error for potential placement UI
      // Do NOT exit placement mode on error, allow retry or cancel
    }
  };

  const cancelCampfirePlacement = () => {
    console.log("Cancelling campfire placement mode.");
    setIsPlacingCampfire(false);
    setPlacementError(null);
  };
  
  // --- LIFTED Drag/Drop Handlers --- 
  const handleItemDragStart = useCallback((info: DraggedItemInfo) => {
    console.log("[App] Drag Start:", info);
    setDraggedItemInfo(info); // Set state, which will update the ref via useEffect
    document.body.classList.add('item-dragging');
  }, []); // No dependencies needed here

  const handleItemDrop = useCallback((targetSlot: DragSourceSlotInfo | null) => { 
    console.log("[App] Drop Target:", targetSlot);
    document.body.classList.remove('item-dragging');
    const sourceInfo = draggedItemRef.current; 
    draggedItemRef.current = null;
    setDraggedItemInfo(null);

    if (!sourceInfo || !targetSlot || (sourceInfo.sourceSlot.type === targetSlot.type && sourceInfo.sourceSlot.index === targetSlot.index)) {
        console.log("[App] Drop cancelled: No source, no target, or dropped on self.");
        return; 
    }
    if (!connection?.reducers) {
        console.error("[App] Drop failed: Connection unavailable.");
        return; 
    }

    const itemInstanceId = BigInt(sourceInfo.item.instance.instanceId);
    console.log(`[App] Processing drop: Item ${itemInstanceId} from ${sourceInfo.sourceSlot.type}:${sourceInfo.sourceSlot.index} to ${targetSlot.type}:${targetSlot.index}`);

    try {
        // --- Handle Stack Splitting First (Highest Priority) ---
        if (sourceInfo.splitQuantity && sourceInfo.splitQuantity > 0) {
            console.log(`[App] Calling splitStack: Item ${itemInstanceId}, Qty ${sourceInfo.splitQuantity} to ${targetSlot.type}:${targetSlot.index}`);
            const targetIndexNum = typeof targetSlot.index === 'string' ? parseInt(targetSlot.index, 10) : targetSlot.index;
            if (isNaN(targetIndexNum)) { // Basic check if string index wasn't a number (e.g., equipment slot)
                 console.error("[App] Split failed: Target index is not a valid number for splitting.");
                 return; // Stop processing if target isn't numeric for split
            }
            connection.reducers.splitStack(itemInstanceId, sourceInfo.splitQuantity, targetSlot.type, targetIndexNum);
        }
        // --- Handle Normal Moves/Equips ---
        else {
            if (targetSlot.type === 'inventory') {
                // Target index is guaranteed to be number (u16) for inventory
                console.log(`[App] Calling moveItemToInventory: Item ${itemInstanceId} to slot ${targetSlot.index}`);
                connection.reducers.moveItemToInventory(itemInstanceId, targetSlot.index as number);
            } else if (targetSlot.type === 'hotbar') {
                // Target index is guaranteed to be number (u8) for hotbar
                 console.log(`[App] Calling moveItemToHotbar: Item ${itemInstanceId} to slot ${targetSlot.index}`);
                connection.reducers.moveItemToHotbar(itemInstanceId, targetSlot.index as number);
            } else if (targetSlot.type === 'equipment') {
                // Target index is string (slot name like 'Head')
                console.log(`[App] Calling equipArmorFromDrag: Item ${itemInstanceId} to slot ${targetSlot.index}`);
                connection.reducers.equipArmorFromDrag(itemInstanceId, targetSlot.index as string);
            }
            // --- NEW: Handle Campfire Fuel Target ---
            else if (targetSlot.type === 'campfire_fuel') {
                const targetSlotIndex = targetSlot.index as number; // Slot index (0-4)

                if (!interactingWith || interactingWith.type !== 'campfire') {
                    console.error("[App Drop] Cannot drop onto campfire slot without interaction context.");
                    return;
                }
                const campfireId = interactingWith.id as number;
                const targetCampfire = campfires.get(campfireId.toString());

                // --- Logging for debug ---
                console.log(`[App Drop Check] Campfire ${campfireId}, Target Slot ${targetSlotIndex}, State:`, targetCampfire);
                // Access specific field based on index for logging
                let currentFuelId: bigint | null | undefined = undefined;
                // Define NUM_FUEL_SLOTS_CONSTANT (replace with actual value if needed)
                const NUM_FUEL_SLOTS_CONSTANT = 5; 
                if (targetCampfire) {
                    switch(targetSlotIndex) {
                        case 0: currentFuelId = targetCampfire.fuelInstanceId0; break;
                        case 1: currentFuelId = targetCampfire.fuelInstanceId1; break;
                        case 2: currentFuelId = targetCampfire.fuelInstanceId2; break;
                        case 3: currentFuelId = targetCampfire.fuelInstanceId3; break;
                        case 4: currentFuelId = targetCampfire.fuelInstanceId4; break;
                    }
                }
                console.log(`[App Drop Check] currentFuelId in slot ${targetSlotIndex}:`, currentFuelId);
                // --- End Logging ---

                // Check if target slot index is valid and slot is empty
                if (targetCampfire && targetSlotIndex >= 0 && targetSlotIndex < NUM_FUEL_SLOTS_CONSTANT && currentFuelId == null) {
                    console.log(`[App] Calling addFuelToCampfire: Item ${itemInstanceId} (${sourceInfo.item.definition.name}) to campfire ${campfireId} slot ${targetSlotIndex}`);
                    // Pass campfireId, targetSlotIndex, and itemInstanceId
                    connection.reducers.addFuelToCampfire(campfireId, targetSlotIndex, itemInstanceId);
                } else {
                    // Rejection logic
                    if (!targetCampfire) {
                         console.warn(`[App] Drop rejected: Campfire ${campfireId} not found in state.`);
                    } else if (!(targetSlotIndex >= 0 && targetSlotIndex < NUM_FUEL_SLOTS_CONSTANT)) {
                         console.warn(`[App] Drop rejected: Invalid target slot index ${targetSlotIndex} for campfire ${campfireId}.`);
                    } else if (currentFuelId != null) { // Check the specific slot's state
                        console.warn(`[App] Drop rejected: Campfire ${campfireId} fuel slot ${targetSlotIndex} is already occupied.`);
                    }
                }
            }
            // --- END Campfire Logic ---
             else {
                console.warn(`[App] Unhandled drop target type: ${targetSlot.type}`);
            }
        }
    } catch (err: any) {
        console.error(`[App] Error processing drop (Item ${itemInstanceId} to ${targetSlot.type}:${targetSlot.index}):`, err);
        // TODO: Provide user feedback based on error?
    }
}, [connection, campfires, interactingWith]); // Ensure interactingWith is in dependencies
  
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
          />
        </div>
      )}
    </div>
  );
}

export default App;
