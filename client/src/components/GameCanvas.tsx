import React, { useEffect, useRef, useCallback, useState, useMemo } from 'react';
// import { SpacetimeDBClient } from '@clockworklabs/spacetimedb-sdk';
import { gameConfig } from '../config/gameConfig';
import {
  Player as SpacetimeDBPlayer,
  Tree as SpacetimeDBTree,
  Stone as SpacetimeDBStone,
  Campfire as SpacetimeDBCampfire,
  Mushroom as SpacetimeDBMushroom, // Explicitly import Mushroom type
  WorldState as SpacetimeDBWorldState,
  ActiveEquipment as SpacetimeDBActiveEquipment,
  InventoryItem as SpacetimeDBInventoryItem,
  ItemDefinition as SpacetimeDBItemDefinition,
  DroppedItem as SpacetimeDBDroppedItem,
  WoodenStorageBox as SpacetimeDBWoodenStorageBox // Added import
} from '../generated';
import heroSpriteSheet from '../assets/hero.png';
import grassTexture from '../assets/tiles/grass.png';
import campfireSprite from '../assets/doodads/campfire.png';
// Import helpers and hook
import { useAnimationCycle } from '../hooks/useAnimationCycle';
import { isPlayerHovered, renderPlayer } from '../utils/renderingUtils';
// Import Minimap drawing logic and dimensions
import { drawMinimapOntoCanvas, MINIMAP_DIMENSIONS } from './Minimap';
// Import Tree rendering utils
import { renderTree, preloadTreeImages } from '../utils/treeRenderingUtils';
// Import Stone rendering utils
import { renderStone, preloadStoneImage } from '../utils/stoneRenderingUtils';
// Import Campfire rendering utils
import { renderCampfire, preloadCampfireImage, CAMPFIRE_HEIGHT } from '../utils/campfireRenderingUtils';
// Import Mushroom rendering utils
import { renderMushroom, preloadMushroomImages } from '../utils/mushroomRenderingUtils';
// Import Wooden Storage Box rendering utils
import { renderWoodenStorageBox, preloadWoodenStorageBoxImage, BOX_HEIGHT } from '../utils/woodenStorageBoxRenderingUtils';
// Import DeathScreen component with extension
import DeathScreen from './DeathScreen.tsx';
// Import item icon mapping
import { itemIcons } from '../utils/itemIconUtils';
// Import the new helper function
import { renderEquippedItem } from '../utils/equippedItemRenderingUtils';
// Import the reducer function type if needed (optional but good practice)
// import { requestRespawn } from '../generated'; // Removed - Not needed for callReducer by string
import { drawInteractionIndicator } from '../utils/interactionIndicator'; // Import drawing function
import { drawShadow } from '../utils/shadowUtils'; // Import shadow utility
// NEW: Import placement types
import { PlacementItemInfo, PlacementActions } from '../hooks/usePlacementManager';

// Threshold for movement animation (position delta)
const MOVEMENT_POSITION_THRESHOLD = 0.1; // Small threshold to account for float precision
const ANIMATION_INTERVAL_MS = 150;

// --- Jump Constants ---
const JUMP_DURATION_MS = 400; // Total duration of the jump animation
const JUMP_HEIGHT_PX = 40; // Maximum height the player reaches

// --- Day/Night Constants (Must match server/world_state.rs) ---
const FULL_MOON_CYCLE_INTERVAL = 3;
const CAMPFIRE_LIGHT_RADIUS_BASE = 150;
const CAMPFIRE_FLICKER_AMOUNT = 5; // Max pixels radius will change by
// Warmer light colors
const CAMPFIRE_LIGHT_INNER_COLOR = 'rgba(255, 180, 80, 0.35)'; // Warmer orange/yellow, slightly more opaque
const CAMPFIRE_LIGHT_OUTER_COLOR = 'rgba(255, 100, 0, 0.0)';  // Fade to transparent orange
const CAMPFIRE_WIDTH_PREVIEW = 64;
const CAMPFIRE_HEIGHT_PREVIEW = 64;

// --- Interaction Constants ---
const PLAYER_MUSHROOM_INTERACTION_DISTANCE_SQUARED = 64.0 * 64.0; // Matches server constant (64px)
const PLAYER_CAMPFIRE_INTERACTION_DISTANCE_SQUARED = 64.0 * 64.0; // Matches server constant (64px)
const PLAYER_BOX_INTERACTION_DISTANCE_SQUARED = 64.0 * 64.0; // <<< ADDED: Matches server constant

const HOLD_INTERACTION_DURATION_MS = 250; // Time to hold E for inventory (halved)

interface GameCanvasProps {
  players: Map<string, SpacetimeDBPlayer>;
  trees: Map<string, SpacetimeDBTree>;
  stones: Map<string, SpacetimeDBStone>;
  campfires: Map<string, SpacetimeDBCampfire>;
  mushrooms: Map<string, SpacetimeDBMushroom>; 
  droppedItems: Map<string, SpacetimeDBDroppedItem>;
  woodenStorageBoxes: Map<string, SpacetimeDBWoodenStorageBox>; // Added prop
  inventoryItems: Map<string, SpacetimeDBInventoryItem>;
  itemDefinitions: Map<string, SpacetimeDBItemDefinition>;
  worldState: SpacetimeDBWorldState | null;
  localPlayerId?: string;
  connection: any | null; 
  activeEquipments: Map<string, SpacetimeDBActiveEquipment>;
  updatePlayerPosition: (dx: number, dy: number, intendedDirection?: 'up' | 'down' | 'left' | 'right' | null) => void;
  callJumpReducer: () => void;
  callSetSprintingReducer: (isSprinting: boolean) => void;
  placementInfo: PlacementItemInfo | null;
  placementActions: PlacementActions;
  placementError: string | null; 
  onSetInteractingWith: (target: { type: string; id: number | bigint } | null) => void;
}

// Type guard for Player
function isPlayer(entity: any): entity is SpacetimeDBPlayer {
  return entity && typeof entity.identity !== 'undefined';
}

// Type guard for Tree
function isTree(entity: any): entity is SpacetimeDBTree {
  return entity && typeof entity.treeType !== 'undefined';
}

// Type guard for Stone
function isStone(entity: any): entity is SpacetimeDBStone {
  // Check for properties specific to Stone and not Player/Tree/Campfire
  return entity && typeof entity.health === 'number' && 
         typeof entity.posX === 'number' && typeof entity.posY === 'number' && 
         typeof entity.identity === 'undefined' && typeof entity.treeType === 'undefined' && 
         typeof entity.placed_by === 'undefined'; 
}

// Type guard for Campfire
function isCampfire(entity: any): entity is SpacetimeDBCampfire {
    return entity && typeof entity.placedBy !== 'undefined' && typeof entity.posX === 'number' && typeof entity.posY === 'number';
}

// Type guard for Mushroom
function isMushroom(entity: any): entity is SpacetimeDBMushroom {
    // Check for properties specific to Mushroom and not others
    return entity && typeof entity.posX === 'number' && typeof entity.posY === 'number' &&
           typeof entity.identity === 'undefined' && typeof entity.treeType === 'undefined' &&
           typeof entity.health === 'undefined' && typeof entity.placedBy === 'undefined';
}

// --- NEW: Type guard for WoodenStorageBox --- 
function isWoodenStorageBox(entity: any): entity is SpacetimeDBWoodenStorageBox {
  // Check for properties specific to WoodenStorageBox
  return entity && typeof entity.posX === 'number' && 
         typeof entity.posY === 'number' && 
         typeof entity.placedBy !== 'undefined' && // Check if placedBy exists
         typeof entity.isBurning === 'undefined'; // Differentiate from Campfire
}

// --- Interpolation Data ---
interface ColorPoint {
  r: number; g: number; b: number; a: number;
}

// --- Realistic Day/Night Color Palette ---
// Default night: Dark, desaturated blue/grey
const defaultPeakMidnightColor: ColorPoint = { r: 15, g: 20, b: 30, a: 0.92 };
const defaultTransitionNightColor: ColorPoint = { r: 40, g: 50, b: 70, a: 0.75 };

// Full Moon night: Brighter, cooler grey/blue, less saturated
const fullMoonPeakMidnightColor: ColorPoint =    { r: 90, g: 110, b: 130, a: 0.48 }; // Slightly brighter, less saturated blue-grey
const fullMoonTransitionNightColor: ColorPoint = { r: 75, g: 100, b: 125, a: 0.58 }; // Slightly desaturated cooler transition

const keyframes: Record<number, ColorPoint> = {
  // Use default peak midnight color as the base for 0.00/1.00
  0.00: defaultPeakMidnightColor,
  // Use default transition night color as base for 0.20 and 0.95
  0.20: defaultTransitionNightColor,
  // Dawn: Soft pink/orange hues
  0.35: { r: 255, g: 180, b: 120, a: 0.25 },
  // Noon: Clear (transparent)
  0.50: { r: 0, g: 0, b: 0, a: 0.0 },
  // Afternoon: Warm golden tint
  0.65: { r: 255, g: 210, b: 150, a: 0.15 },
  // Dusk: Softer orange/purple hues
  0.75: { r: 255, g: 150, b: 100, a: 0.35 },
  // Fading Dusk: Muted deep purple/grey
  0.85: { r: 80, g: 70, b: 90, a: 0.60 },
  // Use default transition night color as base for 0.20 and 0.95
  0.95: defaultTransitionNightColor,
  // Use default peak midnight color as the base for 0.00/1.00
  1.00: defaultPeakMidnightColor,
};

// Helper function for linear interpolation
function lerp(start: number, end: number, t: number): number {
  return start * (1 - t) + end * t;
}

// Function to interpolate RGBA color between keyframes
function interpolateRgba(progress: number, currentKeyframes: Record<number, ColorPoint>): string {
  const sortedKeys = Object.keys(currentKeyframes).map(Number).sort((a, b) => a - b);

  let startKey = 0;
  let endKey = 1;

  // Find the two keyframes surrounding the progress
  for (let i = 0; i < sortedKeys.length - 1; i++) {
    if (progress >= sortedKeys[i] && progress <= sortedKeys[i + 1]) {
      startKey = sortedKeys[i];
      endKey = sortedKeys[i + 1];
      break;
    }
  }

  const startTime = startKey;
  const endTime = endKey;
  const startColor = currentKeyframes[startKey];
  const endColor = currentKeyframes[endKey];

  // Calculate interpolation factor (t) between 0 and 1
  const t = (endTime === startTime) ? 0 : (progress - startTime) / (endTime - startTime);

  // Interpolate each color component
  const r = Math.round(lerp(startColor.r, endColor.r, t));
  const g = Math.round(lerp(startColor.g, endColor.g, t));
  const b = Math.round(lerp(startColor.b, endColor.b, t));
  const a = lerp(startColor.a, endColor.a, t);

  return `rgba(${r},${g},${b},${a.toFixed(2)})`; // Return RGBA string without spaces
}
// --- End Interpolation Data ---

// --- NEW: Type guard for DroppedItem ---
function isDroppedItem(entity: any): entity is SpacetimeDBDroppedItem {
    // Check for properties specific to DroppedItem and not others
    return entity && typeof entity.posX === 'number' && typeof entity.posY === 'number' &&
           typeof entity.itemDefId !== 'undefined' && // Check for itemDefId
           typeof entity.identity === 'undefined' && 
           typeof entity.treeType === 'undefined' &&
           typeof entity.health === 'undefined' && 
           typeof entity.placedBy === 'undefined';
}

const GameCanvas: React.FC<GameCanvasProps> = ({
  players,
  trees,
  stones,
  campfires,
  mushrooms,
  droppedItems,
  woodenStorageBoxes,
  inventoryItems,
  itemDefinitions,
  worldState,
  localPlayerId,
  connection,
  activeEquipments,
  updatePlayerPosition,
  callJumpReducer,
  callSetSprintingReducer,
  placementInfo,
  placementActions,
  placementError,
  onSetInteractingWith,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const maskCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const [canvasSize, setCanvasSize] = useState({ width: window.innerWidth, height: window.innerHeight });
  const keysPressed = useRef<Set<string>>(new Set());
  const requestIdRef = useRef<number>(0);
  const heroImageRef = useRef<HTMLImageElement | null>(null);
  const grassImageRef = useRef<HTMLImageElement | null>(null);
  const campfireImageRef = useRef<HTMLImageElement | null>(null);
  const itemImagesRef = useRef<Map<string, HTMLImageElement>>(new Map()); // Cache for item images
  const mousePosRef = useRef<{x: number | null, y: number | null}>({ x: null, y: null });
  const worldMousePosRef = useRef<{x: number | null, y: number | null}>({ x: null, y: null });
  const [isMinimapOpen, setIsMinimapOpen] = useState(false); // State for minimap visibility
  const [isMouseOverMinimap, setIsMouseOverMinimap] = useState(false); // State for minimap hover
  const isSprintingRef = useRef<boolean>(false); // Track current sprint state
  const lastPositionsRef = useRef<Map<string, {x: number, y: number}>>(new Map()); // Store last known positions
  const isInputDisabled = useRef<boolean>(false); // Ref to track input disable state
  const closestInteractableMushroomIdRef = useRef<bigint | null>(null); // Ref for nearby mushroom ID
  const closestInteractableCampfireIdRef = useRef<number | null>(null); // Ref for nearby campfire ID (u32 maps to number)
  const closestInteractableDroppedItemIdRef = useRef<bigint | null>(null); // Ref for closest interactable dropped item ID
  const closestInteractableBoxIdRef = useRef<number | null>(null); // <<< ADDED: Ref for nearby box ID (u32)
  const isEHeldDownRef = useRef<boolean>(false);
  const isMouseDownRef = useRef<boolean>(false);
  const lastClientSwingAttemptRef = useRef<number>(0); // Store timestamp of last attempt
  const eKeyDownTimestampRef = useRef<number>(0);
  const eKeyHoldTimerRef = useRef<number | null>(null);
  const [interactionProgress, setInteractionProgress] = useState<{ targetId: number | bigint | null; startTime: number } | null>(null);

  // --- Ref for Placement Actions Object --- 
  const placementActionsRef = useRef(placementActions);

  // Update ref when placement actions object prop changes
  useEffect(() => {
    placementActionsRef.current = placementActions;
  }, [placementActions]);
  // --- End Refs --- 

  const animationFrame = useAnimationCycle(ANIMATION_INTERVAL_MS, 4);

  // Memoize the keyframes based on full moon state, anticipating the next cycle if needed
  const currentKeyframes = useMemo(() => {
    const currentProgress = worldState?.cycleProgress ?? 0.25; // Default if no state
    const currentCycleCount = worldState?.cycleCount ?? 0;
    const currentIsFullMoon = worldState?.isFullMoon ?? false;
    const anticipationThreshold = 0.75; // When to start anticipating next cycle's moon

    // Anticipate next cycle's moon status if we're in late dusk/night
    let effectiveIsFullMoon = currentIsFullMoon;
    if (currentProgress > anticipationThreshold) {
        const nextCycleIsFullMoon = ((currentCycleCount + 1) % FULL_MOON_CYCLE_INTERVAL) === 0;
        effectiveIsFullMoon = nextCycleIsFullMoon;
    }

    // Choose peak/transition colors based on the effective moon state
    const peakNight = effectiveIsFullMoon ? fullMoonPeakMidnightColor : defaultPeakMidnightColor;
    const transitionNight = effectiveIsFullMoon ? fullMoonTransitionNightColor : defaultTransitionNightColor;

    // Start with the default keyframes
    let adjustedKeyframes = { ...keyframes };

    // Override pure night colors using selected colors
    adjustedKeyframes[0.00] = peakNight;
    adjustedKeyframes[0.20] = transitionNight;
    adjustedKeyframes[0.95] = transitionNight;
    adjustedKeyframes[1.00] = peakNight;

    // Adjust adjacent keyframes ONLY if the effective moon state is full moon
    if (effectiveIsFullMoon) {
        const dusk = keyframes[0.75];
        adjustedKeyframes[0.85] = {
             r: Math.round(lerp(dusk.r, transitionNight.r, 0.5)),
             g: Math.round(lerp(dusk.g, transitionNight.g, 0.5)),
             b: Math.round(lerp(dusk.b, transitionNight.b, 0.5)),
             a: lerp(dusk.a, transitionNight.a, 0.5)
        };
        const dawn = keyframes[0.35];
        adjustedKeyframes[0.35] = {
             r: Math.round(lerp(transitionNight.r, dawn.r, 0.7)),
             g: Math.round(lerp(transitionNight.g, dawn.g, 0.7)),
             b: Math.round(lerp(transitionNight.b, dawn.b, 0.7)),
             a: lerp(transitionNight.a, dawn.a, 0.6)
         };
    }
    return adjustedKeyframes;

  }, [worldState?.cycleProgress, worldState?.cycleCount, worldState?.isFullMoon]);

  const getLocalPlayer = useCallback((): SpacetimeDBPlayer | undefined => {
    if (!localPlayerId) return undefined;
    return players.get(localPlayerId);
  }, [players, localPlayerId]);

  // Update input disabled state based on local player death status
  useEffect(() => {
    const localPlayer = getLocalPlayer();
    isInputDisabled.current = !!localPlayer?.isDead;
    if (isInputDisabled.current) {
      keysPressed.current.clear(); // Clear keys if player is dead
      // Ensure sprint state is reset if player dies while sprinting
      if (isSprintingRef.current) {
           isSprintingRef.current = false;
           // Optionally call reducer if needed, but player state reset on respawn might cover this
           // callSetSprintingReducer(false);
      }
    }
  }, [players, localPlayerId, getLocalPlayer]);

  // --- Calculate Respawn Time ---
  const localPlayer = getLocalPlayer();
  const respawnTimestampMs = useMemo(() => {
    if (localPlayer?.isDead && localPlayer.respawnAt) {
      // Use the correct method name and convert BigInt microseconds to number milliseconds
      return Number(localPlayer.respawnAt.microsSinceUnixEpoch / 1000n);
    }
    return 0;
  }, [localPlayer?.isDead, localPlayer?.respawnAt]);

  // --- NEW: Swing Attempt Helper (Moved Before useEffect) ---
  const attemptSwing = useCallback(() => {
    if (!connection?.reducers || !localPlayerId) return;
    
    const localEquipment = activeEquipments.get(localPlayerId);
    if (!localEquipment || localEquipment.equippedItemDefId === null) {
        return; 
    }

    const now = Date.now();
    const SWING_COOLDOWN_MS = 500;

    // Client-side cooldown
    if (now - lastClientSwingAttemptRef.current < SWING_COOLDOWN_MS) {
      return;
    }
    
    // Server-side cooldown
    if (now - Number(localEquipment.swingStartTimeMs) < SWING_COOLDOWN_MS) {
       return; 
    }

    // Attempt the swing
    try {
        connection.reducers.useEquippedItem();
        lastClientSwingAttemptRef.current = now;
    } catch (err) {
        console.error("[AttemptSwing] Error calling useEquippedItem reducer:", err);
    }
  }, [connection, localPlayerId, activeEquipments]); // Dependencies

  // --- Input Handling useEffect --- 
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (isInputDisabled.current && e.key.toLowerCase() !== 'escape') return;
      const key = e.key.toLowerCase();
      if (key === 'escape' && placementInfo) { 
        if (placementActionsRef.current) {
             placementActionsRef.current.cancelPlacement();
        }
        return; 
      }
      if (isInputDisabled.current) return;

      // Avoid processing modifier keys if they are the only key pressed initially
      if (key === 'shift' || key === 'control' || key === 'alt' || key === 'meta') {
        // Handle Shift for sprinting start
        if (key === 'shift' && !isSprintingRef.current && !e.repeat) {
            isSprintingRef.current = true;
            callSetSprintingReducer(true);
        }
        // Don't add modifier keys themselves to keysPressed
        return;
      }

      keysPressed.current.add(key);

      // Handle jump on Spacebar press
      if (key === ' ' && !e.repeat) {
        const localPlayer = getLocalPlayer();
        if (localPlayer) {
          callJumpReducer();
        }
      }

      // Toggle minimap on 'g' press
      if (key === 'g' && !e.repeat) {
        setIsMinimapOpen(prev => !prev);
      }

      // Handle interaction key ('e')
      if (key === 'e' && !e.repeat && !isEHeldDownRef.current) {
        const closestCampfireId = closestInteractableCampfireIdRef.current;
        const closestMushroomId = closestInteractableMushroomIdRef.current;
        const closestDroppedItemId = closestInteractableDroppedItemIdRef.current;
        const closestBoxId = closestInteractableBoxIdRef.current;
        
        // Prioritize interaction: DroppedItem > Box > Campfire > Mushroom
        if (closestDroppedItemId !== null && connection?.reducers) {
             try {
                 connection.reducers.pickupDroppedItem(closestDroppedItemId);
             } catch (err) {
                 console.error("Error calling pickupDroppedItem reducer:", err);
                 // TODO: Display error feedback to the user?
             }
             return; // Handled dropped item, prevent other interactions
        } else if (closestBoxId !== null && connection?.reducers) {
            console.log(`[GameCanvas KeyDown] Interacting with Box ID: ${closestBoxId}`);
            try {
                // Call reducer to validate interaction server-side
                connection.reducers.interactWithStorageBox(closestBoxId);
                // If no error, proceed to open UI via App state
                onSetInteractingWith({ type: 'wooden_storage_box', id: closestBoxId });
            } catch (err) {
                console.error("[GameCanvas KeyDown] Error calling interactWithStorageBox reducer:", err);
                // TODO: Show error to user? (e.g., too far away)
            }
            return; // Interaction handled
        } else if (closestCampfireId !== null) {
          isEHeldDownRef.current = true;
          eKeyDownTimestampRef.current = Date.now();
          setInteractionProgress({ targetId: closestCampfireId, startTime: Date.now() }); // Start visual indicator

          // Clear any existing timer before setting a new one
          if (eKeyHoldTimerRef.current) {
            clearTimeout(eKeyHoldTimerRef.current);
          }

          // Set timer for hold action
          eKeyHoldTimerRef.current = setTimeout(() => {
            if (isEHeldDownRef.current) { // Check if still held
              onSetInteractingWith({ type: 'campfire', id: closestCampfireId }); 
              // Reset state AFTER triggering action
              isEHeldDownRef.current = false;
              setInteractionProgress(null);
              eKeyHoldTimerRef.current = null;
            }
          }, HOLD_INTERACTION_DURATION_MS);
          // Do NOT call other reducers here anymore
          return; // Prevent adding 'e' to keysPressed for movement
        } else if (closestMushroomId !== null && connection?.reducers) {
            try {
                connection.reducers.interactWithMushroom(closestMushroomId);
            } catch (err) {
                console.error("Error calling interactWithMushroom reducer:", err);
            }
             return; // Prevent adding 'e' to keysPressed
        }
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      // Always allow keyup processing to clear keysPressed, even if dead
      const key = e.key.toLowerCase();
      // Handle Shift key release for sprinting end
      if (key === 'shift') {
        if (isSprintingRef.current) {
            isSprintingRef.current = false;
            // Only call reducer if player isn't dead (respawn resets state anyway)
            if (!isInputDisabled.current) {
                 callSetSprintingReducer(false);
            }
        }
      }
      keysPressed.current.delete(key);

      // --- 'E' Key Up Logic ---
      if (key === 'e') {
          if (isEHeldDownRef.current) {
              isEHeldDownRef.current = false;
              // Clear the hold timer if it exists
              if (eKeyHoldTimerRef.current) {
                  clearTimeout(eKeyHoldTimerRef.current);
                  eKeyHoldTimerRef.current = null;
              }
              // Always hide indicator on key up
              setInteractionProgress(null);
              
              const closestCampfireId = closestInteractableCampfireIdRef.current;
              if (closestCampfireId !== null) {
                  const holdDuration = Date.now() - eKeyDownTimestampRef.current;
                  
                  // If held for less than the threshold, trigger toggle
                  if (holdDuration < HOLD_INTERACTION_DURATION_MS) {
                       if(connection?.reducers) {
                           try {
                              connection.reducers.toggleCampfireBurning(closestCampfireId);
                           } catch (err) {
                               console.error("Error calling toggleCampfireBurning reducer:", err);
                           }
                       } else {
                           console.warn("Cannot toggle campfire: Connection or reducers unavailable.");
                       }
                  } // Else (hold was long enough), the timeout callback handled opening inventory
              } 
              // Reset timestamp just in case
              eKeyDownTimestampRef.current = 0; 
          }
      }
      // --- End 'E' Key Up Logic ---
    };

    const handleCanvasClick = (event: MouseEvent) => {
        if (isInputDisabled.current) return;
        if (placementInfo && worldMousePosRef.current.x !== null && worldMousePosRef.current.y !== null) {
             if (event.button === 0) {
                 if (typeof placementActionsRef.current?.attemptPlacement === 'function') {
                    placementActionsRef.current.attemptPlacement(worldMousePosRef.current.x, worldMousePosRef.current.y);
                 } else {
                    console.error("[GameCanvas handleCanvasClick] attemptPlacement function missing on ref!");
                 }
                 return; 
             }
         }
    };

    const handleContextMenu = (event: MouseEvent) => {
         if (isInputDisabled.current) return;
        if (placementInfo) {
            event.preventDefault(); 
            if (placementActionsRef.current) {
                placementActionsRef.current.cancelPlacement();
            }
        }
    };

    const handleWheel = (event: WheelEvent) => {
        if (placementInfo) {
            if (placementActionsRef.current) {
               placementActionsRef.current.cancelPlacement(); 
            }
        }
    };

    // --- NEW: Mouse Down/Up Listeners for Swinging ---
    const handleMouseDown = (event: MouseEvent) => {
         if (isInputDisabled.current || event.button !== 0 || placementInfo) return; 
        isMouseDownRef.current = true;
        attemptSwing(); 
    };

    const handleMouseUp = (event: MouseEvent) => {
        if (event.button === 0) {
           isMouseDownRef.current = false;
        }
    };

    // Add listeners
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    window.addEventListener('wheel', handleWheel, { passive: true });
    window.addEventListener('mousedown', handleMouseDown);
    window.addEventListener('mouseup', handleMouseUp);
    const canvas = canvasRef.current;
    if (canvas) {
        canvas.addEventListener('click', handleCanvasClick);
        canvas.addEventListener('contextmenu', handleContextMenu);
    }

    const handleBlur = () => {
        if (isSprintingRef.current) {
            isSprintingRef.current = false;
            callSetSprintingReducer(false);
        }
        keysPressed.current.clear(); // Clear all keys on blur
    };
    window.addEventListener('blur', handleBlur);

    // Remove listeners
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
      window.removeEventListener('wheel', handleWheel);
      window.removeEventListener('blur', handleBlur);
      window.removeEventListener('mousedown', handleMouseDown);
      window.removeEventListener('mouseup', handleMouseUp);
       if (canvas) {
        canvas.removeEventListener('click', handleCanvasClick);
        canvas.removeEventListener('contextmenu', handleContextMenu);
      }
    };
  }, [getLocalPlayer, callJumpReducer, callSetSprintingReducer, placementInfo, attemptSwing]);

  const updatePlayerBasedOnInput = useCallback(() => {
    if (isInputDisabled.current) return; // Skip input processing if dead

    const localPlayer = getLocalPlayer();
    if (!localPlayer) return;

    let dx = 0;
    let dy = 0;
    const speed = 5; // Use base speed
    let intendedDirection: 'up' | 'down' | 'left' | 'right' | null = null; // Initialize intended direction

    // Handle WASD movement to determine direction and base delta
    // We send 1 or -1 to indicate direction, server calculates actual distance
    if (keysPressed.current.has('w') || keysPressed.current.has('arrowup')) {
      dy -= speed;
      intendedDirection = 'up';
    }
    if (keysPressed.current.has('s') || keysPressed.current.has('arrowdown')) {
      dy += speed;
      intendedDirection = 'down';
    }
    if (keysPressed.current.has('a') || keysPressed.current.has('arrowleft')) {
      dx -= speed;
      intendedDirection = 'left';
    }
    if (keysPressed.current.has('d') || keysPressed.current.has('arrowright')) {
      dx += speed;
      intendedDirection = 'right';
    }

    // Normalize diagonal movement (optional but good practice)
    if (dx !== 0 && dy !== 0) {
        const magnitude = Math.sqrt(dx * dx + dy * dy);
        dx = (dx / magnitude) * 5;
        dy = (dy / magnitude) * 5;
    }

    // Always call updatePlayerPosition to allow server to process passive changes (like stamina regen) AND direction changes
    // Pass intendedDirection as the third argument
    // Send the normalized (or 0, +/-1) dx/dy values
    updatePlayerPosition(dx, dy, intendedDirection);
    // Client no longer calculates absolute position or checks boundaries

  }, [getLocalPlayer, updatePlayerPosition]);

  useEffect(() => {
    // Load Hero
    const heroImg = new Image();
    heroImg.src = heroSpriteSheet;
    heroImg.onload = () => {
      heroImageRef.current = heroImg;
      console.log('Hero spritesheet loaded.');
    };
    heroImg.onerror = () => console.error('Failed to load hero spritesheet.');

    // Load Grass
    const grassImg = new Image();
    grassImg.src = grassTexture;
    grassImg.onload = () => {
       grassImageRef.current = grassImg; // Store image in ref
       console.log('Grass texture loaded.');
    };
    grassImg.onerror = () => console.error('Failed to load grass texture.');

    // Preload Tree Images
    preloadTreeImages();
    // Preload Stone Image
    preloadStoneImage();
    // Preload Campfire Image
    preloadCampfireImage();
    // Preload Mushroom Images
    preloadMushroomImages();
    // Preload Wooden Storage Box Image
    preloadWoodenStorageBoxImage();
    // Also load the main campfire image for placement preview
    const fireImg = new Image();
    fireImg.src = campfireSprite;
    fireImg.onload = () => { campfireImageRef.current = fireImg; console.log('Campfire sprite loaded.'); };
    fireImg.onerror = () => console.error('Failed to load campfire sprite.');

  }, []); // Runs once

  // --- NEW: Effect for Preloading Item Images based on Definitions ---
  useEffect(() => {
    console.log("Preloading item images based on itemDefinitions update...");
    itemDefinitions.forEach(itemDef => {
      // Check if the icon name exists in our mapping and hasn't been loaded yet
      const iconSrc = itemIcons[itemDef.iconAssetName]; // Get potential source
      if (itemDef && iconSrc && typeof iconSrc === 'string' && !itemImagesRef.current.has(itemDef.iconAssetName)) {
        const img = new Image();
        img.src = iconSrc; // Assign the verified string source
        img.onload = () => {
          itemImagesRef.current.set(itemDef.iconAssetName, img);
          console.log(`Preloaded item image: ${itemDef.iconAssetName} from ${img.src}`);
        };
        img.onerror = () => console.error(`Failed to preload item image asset: ${itemDef.iconAssetName} (Expected path/source: ${iconSrc})`);
        // Add placeholder immediately to prevent repeated load attempts in the same cycle
        itemImagesRef.current.set(itemDef.iconAssetName, img);
      }
    });
  }, [itemDefinitions]); // Re-run whenever itemDefinitions changes

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const handleMouseMove = (event: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      const scaleX = canvas.width / rect.width;
      const scaleY = canvas.height / rect.height;
      const screenX = (event.clientX - rect.left) * scaleX;
      const screenY = (event.clientY - rect.top) * scaleY;

      // Update screen mouse position ref
      mousePosRef.current = { x: screenX, y: screenY };

      // Calculate and update world mouse position ref
      const localPlayerData = getLocalPlayer();
      if (localPlayerData) {
          const cameraOffsetX = canvasSize.width / 2 - localPlayerData.positionX;
          const cameraOffsetY = canvasSize.height / 2 - localPlayerData.positionY;
          worldMousePosRef.current = {
              x: screenX - cameraOffsetX,
              y: screenY - cameraOffsetY
          };
      } else {
          worldMousePosRef.current = { x: null, y: null };
      }

      // Check if mouse is over the minimap (only if open)
      if (isMinimapOpen) {
        // Use imported dimensions and current canvas size state
        const currentCanvasWidth = canvasSize.width;
        const currentCanvasHeight = canvasSize.height;
        const minimapX = (currentCanvasWidth - MINIMAP_DIMENSIONS.width) / 2;
        const minimapY = (currentCanvasHeight - MINIMAP_DIMENSIONS.height) / 2;
        const mouseX = mousePosRef.current.x;
        const mouseY = mousePosRef.current.y;

        if (mouseX !== null && mouseY !== null &&
            mouseX >= minimapX && mouseX <= minimapX + MINIMAP_DIMENSIONS.width &&
            mouseY >= minimapY && mouseY <= minimapY + MINIMAP_DIMENSIONS.height) {
          setIsMouseOverMinimap(true);
        } else {
          setIsMouseOverMinimap(false);
        }
      } else {
        setIsMouseOverMinimap(false); // Ensure it's false if minimap is closed
      }
    };
    canvas.addEventListener('mousemove', handleMouseMove);

    const handleMouseLeave = () => {
      // Update ref directly
      mousePosRef.current = { x: null, y: null };
      worldMousePosRef.current = { x: null, y: null }; // Clear world pos too
    };
    canvas.addEventListener('mouseleave', handleMouseLeave);

    // Cleanup listeners
    return () => {
      canvas.removeEventListener('mousemove', handleMouseMove);
      canvas.removeEventListener('mouseleave', handleMouseLeave);
    };
  }, [isMinimapOpen, canvasSize.width, canvasSize.height, getLocalPlayer]);

  const drawWorldBackground = useCallback((ctx: CanvasRenderingContext2D) => {
    const grassImg = grassImageRef.current;
    if (!grassImg) { // Draw fallback if image not loaded
      ctx.fillStyle = '#8FBC8F';
      ctx.fillRect(0, 0, gameConfig.worldWidth * gameConfig.tileSize, gameConfig.worldHeight * gameConfig.tileSize );
      return;
    }

    // You can change this value to true if you want to enable grid lines again.
    const drawGridLines = false; // Keep grid lines off

    // --- Potential Fix: Draw slightly larger tiles ---
    const overlap = 1; // Overlap by 1 pixel

    for (let y = 0; y < gameConfig.worldHeight; y++) {
      for (let x = 0; x < gameConfig.worldWidth; x++) {
        // Draw image slightly larger to cover potential gaps
        ctx.drawImage(
          grassImg,
          x * gameConfig.tileSize,
          y * gameConfig.tileSize,
          gameConfig.tileSize + overlap, // Draw wider
          gameConfig.tileSize + overlap  // Draw taller
        );

        // Original grid line drawing (remains unchanged)
        if (drawGridLines) {
          ctx.strokeStyle = 'rgba(221, 221, 221, 0.5)';
          ctx.lineWidth = 1;
          ctx.strokeRect(x * gameConfig.tileSize, y * gameConfig.tileSize, gameConfig.tileSize, gameConfig.tileSize);
        }
      }
    }
  }, []);

  // Initialize the off-screen canvas
  useEffect(() => {
    if (canvasRef.current && !maskCanvasRef.current) {
      maskCanvasRef.current = document.createElement('canvas');
      maskCanvasRef.current.width = canvasRef.current.width;
      maskCanvasRef.current.height = canvasRef.current.height;
      console.log('Off-screen mask canvas created.');
    }
    // Resize mask canvas if main canvas resizes
    else if (canvasRef.current && maskCanvasRef.current) {
      if (maskCanvasRef.current.width !== canvasRef.current.width || maskCanvasRef.current.height !== canvasRef.current.height) {
        maskCanvasRef.current.width = canvasRef.current.width;
        maskCanvasRef.current.height = canvasRef.current.height;
         console.log('Off-screen mask canvas resized.');
      }
    }
  }, [canvasSize]); // Run when canvasSize changes

  const renderGame = useCallback(() => {
    const canvas = canvasRef.current;
    const maskCanvas = maskCanvasRef.current;
    if (!canvas || !maskCanvas) return;
    const ctx = canvas.getContext('2d');
    const maskCtx = maskCanvas.getContext('2d');
    if (!ctx || !maskCtx) return;

    const localPlayerData = getLocalPlayer();
    const now_ms = Date.now();
    const currentWorldMouseX = worldMousePosRef.current.x;
    const currentWorldMouseY = worldMousePosRef.current.y;
    const currentCanvasWidth = canvasSize.width;
    const currentCanvasHeight = canvasSize.height;
    const cameraOffsetX = localPlayerData ? (canvasSize.width / 2 - localPlayerData.positionX) : 0;
    const cameraOffsetY = localPlayerData ? (canvasSize.height / 2 - localPlayerData.positionY) : 0;

    // Clear canvas
    ctx.clearRect(0, 0, currentCanvasWidth, currentCanvasHeight);
    ctx.fillStyle = '#000000';
    ctx.fillRect(0, 0, currentCanvasWidth, currentCanvasHeight);

    // --- World Rendering ---
    ctx.save();
    ctx.translate(cameraOffsetX, cameraOffsetY);
    drawWorldBackground(ctx);

    // 1. Gather and Categorize Entities
    const groundItems: (SpacetimeDBMushroom | SpacetimeDBDroppedItem | SpacetimeDBCampfire)[] = []; 
    const ySortableEntities: (SpacetimeDBPlayer | SpacetimeDBTree | SpacetimeDBStone | SpacetimeDBWoodenStorageBox)[] = []; 

    // Add Mushrooms and Dropped Items to groundItems
    mushrooms.forEach(m => { if (m.respawnAt === null || m.respawnAt === undefined) groundItems.push(m); });
    droppedItems.forEach(i => groundItems.push(i));
    // ADD Campfires to groundItems
    campfires.forEach(c => groundItems.push(c));

    // Add Players, Trees, Stones to ySortableEntities
    players.forEach(p => ySortableEntities.push(p));
    trees.forEach(t => { if (t.health > 0) ySortableEntities.push(t); });
    stones.forEach(s => { if (s.health > 0) ySortableEntities.push(s); });
    // ADD Wooden Storage Boxes to ySortableEntities
    woodenStorageBoxes.forEach(b => ySortableEntities.push(b));

    // 2. Sort Y-Sortable Entities
    ySortableEntities.sort((a, b) => {
        // Need to handle different position property names
        const yA = isPlayer(a) ? a.positionY : (isWoodenStorageBox(a) ? a.posY : a.posY); // Use posY for Box, Tree, Stone
        const yB = isPlayer(b) ? b.positionY : (isWoodenStorageBox(b) ? b.posY : b.posY); // Use posY for Box, Tree, Stone
        return yA - yB;
    });

    // 3. Find Closest Interactables (Mushrooms, Campfires, DroppedItems, Boxes)
    let closestDistSq = PLAYER_MUSHROOM_INTERACTION_DISTANCE_SQUARED;
    closestInteractableMushroomIdRef.current = null; // Reset each frame
    if (localPlayerData) {
      // Filter mushrooms before checking distance
      mushrooms.forEach((mushroom) => {
        // Ignore mushrooms waiting to respawn
        if (mushroom.respawnAt !== null && mushroom.respawnAt !== undefined) {
             return; // Skip this mushroom
        }

        const dx = localPlayerData.positionX - mushroom.posX;
        const dy = localPlayerData.positionY - mushroom.posY;
        const distSq = dx * dx + dy * dy;

        if (distSq < closestDistSq) {
          closestDistSq = distSq;
          closestInteractableMushroomIdRef.current = mushroom.id; // Store the BigInt ID
        }
      });
    }

    // --- Find Closest Interactable Campfire ---
    let closestCampfireDistSq = PLAYER_CAMPFIRE_INTERACTION_DISTANCE_SQUARED;
    closestInteractableCampfireIdRef.current = null; // Reset each frame
    if (localPlayerData) {
      campfires.forEach((campfire) => {
        // Potential future check: Ignore if campfire is extinguished and has no fuel?
        // For now, allow interaction even if off.

        const dx = localPlayerData.positionX - campfire.posX;
        const dy = localPlayerData.positionY - campfire.posY;
        const distSq = dx * dx + dy * dy;

        if (distSq < closestCampfireDistSq) {
          closestCampfireDistSq = distSq;
          closestInteractableCampfireIdRef.current = campfire.id; // Store the number ID
        }
      });
    }
    // --- End Finding Closest Campfire ---

    // --- NEW: Find Closest Interactable DroppedItem ---
    let closestDroppedItemDistSq = PLAYER_MUSHROOM_INTERACTION_DISTANCE_SQUARED; // Reuse distance for now
    closestInteractableDroppedItemIdRef.current = null; // Reset each frame
    if (localPlayerData) {
      droppedItems.forEach((item) => {
        const dx = localPlayerData.positionX - item.posX;
        const dy = localPlayerData.positionY - item.posY;
        const distSq = dx * dx + dy * dy;

        if (distSq < closestDroppedItemDistSq) {
          closestDroppedItemDistSq = distSq;
          closestInteractableDroppedItemIdRef.current = item.id; // Store the BigInt ID
        }
      });
    }
    // --- End Finding Closest DroppedItem ---

    // --- Find Closest Interactable WoodenStorageBox <<< ADDED Block ---
    let closestBoxDistSq = PLAYER_BOX_INTERACTION_DISTANCE_SQUARED;
    closestInteractableBoxIdRef.current = null; // Reset each frame
    if (localPlayerData) {
      woodenStorageBoxes.forEach((box) => {
        const dx = localPlayerData.positionX - box.posX;
        const dy = localPlayerData.positionY - box.posY;
        const distSq = dx * dx + dy * dy;

        if (distSq < closestBoxDistSq) {
          closestBoxDistSq = distSq;
          closestInteractableBoxIdRef.current = box.id; // Store the number ID
        }
      });
    }
    // --- End Finding Closest Box ---

    // --- UPDATED Placement Preview Rendering --- 
    let isPlacementTooFar = false;
    // Check generic placementInfo
    if (placementInfo && localPlayerData && currentWorldMouseX !== null && currentWorldMouseY !== null) {
         const placeDistSq = (currentWorldMouseX - localPlayerData.positionX)**2 + (currentWorldMouseY - localPlayerData.positionY)**2;
         const clientPlacementRangeSq = (96.0 * 96.0) * 1.05; 
         if (placeDistSq > clientPlacementRangeSq) {
             isPlacementTooFar = true;
         }
    }

    // 4. Render Ground Items (Mushrooms, Dropped Items, Campfires)
    groundItems.forEach(entity => {
        // Check for DroppedItem FIRST
        if (isDroppedItem(entity)) {
            // --- Correct Dropped Item Rendering Logic --- 
            const itemDef = itemDefinitions.get(entity.itemDefId.toString());
            let itemImg: HTMLImageElement | null = null;
            let iconAssetName: string | null = null;
            if (itemDef) {
                iconAssetName = itemDef.iconAssetName;
                itemImg = itemImagesRef.current.get(iconAssetName) ?? null;
            } else {
                 console.warn(`[Render DroppedItem] Definition not found for ID: ${entity.itemDefId}`);
            }
            const canRenderIcon = itemDef && iconAssetName && itemImg && itemImg.complete && itemImg.naturalHeight !== 0;

            const drawWidth = 64;
            const drawHeight = 64;
            if (canRenderIcon && iconAssetName) {
               // Draw shadow first
               const centerX = entity.posX;
               const baseY = entity.posY;
               const shadowRadiusX = drawWidth * 0.3;
               const shadowRadiusY = shadowRadiusX * 0.4;
               const shadowOffsetY = -drawHeight * -0.2; // Push shadow UP slightly (10% of item height)
               drawShadow(ctx, centerX, baseY + shadowOffsetY, shadowRadiusX, shadowRadiusY);

               ctx.drawImage(itemImg!, entity.posX - drawWidth / 2, entity.posY - drawHeight / 2, drawWidth, drawHeight);
                // Eagerly load image if not already cached
                if (!itemImagesRef.current.has(iconAssetName)) {
                     const iconSrc = itemIcons[iconAssetName] || '';
                     if (iconSrc) {
                         const img = new Image();
                         img.src = iconSrc;
                         img.onload = () => itemImagesRef.current.set(iconAssetName!, img);
                         itemImagesRef.current.set(iconAssetName, img); // Add placeholder immediately
                     }
                }
            } else {
                // Fallback rendering if icon isn't ready or definition missing
                ctx.fillStyle = '#CCCCCC'; // Grey square fallback
                ctx.fillRect(entity.posX - drawWidth / 2, entity.posY - drawHeight / 2, drawWidth, drawHeight);
            }
        // Check for Mushroom SECOND
        } else if (isMushroom(entity)) {
            renderMushroom(ctx, entity, now_ms);
        // Check for Campfire THIRD
        } else if (isCampfire(entity)) {
            // Render the campfire sprite using its world coordinates
            renderCampfire(ctx, entity.posX, entity.posY, entity.isBurning);
        }
    });

    // 5. Render Y-Sorted Entities (Players, Trees, Stones) - On Top of Ground Items
    ySortableEntities.forEach(entity => {
       if (isPlayer(entity)) {
           // --- Player Rendering Logic (No Label) ---
           const playerId = entity.identity.toHexString();
           const lastPos = lastPositionsRef.current.get(playerId);
           let isPlayerMoving = false;
           if (lastPos) {
             const dx = Math.abs(entity.positionX - lastPos.x);
             const dy = Math.abs(entity.positionY - lastPos.y);
             if (dx > MOVEMENT_POSITION_THRESHOLD || dy > MOVEMENT_POSITION_THRESHOLD) {
               isPlayerMoving = true;
             }
           } else {
             isPlayerMoving = false;
           }
           lastPositionsRef.current.set(playerId, { x: entity.positionX, y: entity.positionY });

           let jumpOffset = 0;
           const jumpStartTime = entity.jumpStartTimeMs;
           if (jumpStartTime > 0) {
               const elapsedJumpTime = now_ms - Number(jumpStartTime);
               if (elapsedJumpTime < JUMP_DURATION_MS) {
                   const d = JUMP_DURATION_MS;
                   const h = JUMP_HEIGHT_PX;
                   const x = elapsedJumpTime;
                   jumpOffset = (-4 * h / (d * d)) * x * (x - d);
               }
           }
           const hovered = isPlayerHovered(currentWorldMouseX, currentWorldMouseY, entity);
           const heroImg = heroImageRef.current;

           // --- Get Equipment Data ---
           const equipment = activeEquipments.get(playerId);
           let itemDef: SpacetimeDBItemDefinition | null = null;
           let itemImg: HTMLImageElement | null = null;

           if (equipment && equipment.equippedItemDefId) {
             itemDef = itemDefinitions.get(equipment.equippedItemDefId.toString()) || null;
             itemImg = (itemDef ? itemImagesRef.current.get(itemDef.iconAssetName) : null) || null;
           }
           const canRenderItem = itemDef && itemImg && itemImg.complete && itemImg.naturalHeight !== 0;

           // --- Conditional Rendering Order ---
           if (entity.direction === 'left' || entity.direction === 'up') {
              // Draw Item BEHIND Player
              if (canRenderItem && equipment) {
                renderEquippedItem(ctx, entity, equipment, itemDef!, itemImg!, now_ms, jumpOffset);
              }
              // Draw Player
              if (heroImg) {
                renderPlayer(ctx, entity, heroImg, isPlayerMoving, hovered, animationFrame, now_ms, jumpOffset);
              }
           } else { // direction === 'right' or 'down'
              // Draw Player FIRST
              if (heroImg) {
                renderPlayer(ctx, entity, heroImg, isPlayerMoving, hovered, animationFrame, now_ms, jumpOffset);
              }
              // Draw Item IN FRONT of Player
              if (canRenderItem && equipment) {
                 renderEquippedItem(ctx, entity, equipment, itemDef!, itemImg!, now_ms, jumpOffset);
              }
           }
           // --- End Conditional Rendering Order ---
       } else if (isTree(entity)) { 
           renderTree(ctx, entity, now_ms);
       } else if (isStone(entity)) { 
           renderStone(ctx, entity, now_ms);
       } else if (isWoodenStorageBox(entity)) {
           // Use the dedicated rendering function
            renderWoodenStorageBox(ctx, entity.posX, entity.posY);
       } 
    });

    // 6. Render Interaction Labels (World Space - On Top of Everything)
    mushrooms.forEach(mushroom => {
        if (closestInteractableMushroomIdRef.current === mushroom.id) {
            // Draw mushroom label
            const text = "Press E to Collect";
            const textX = mushroom.posX;
            const textY = mushroom.posY - 60;
            ctx.fillStyle = "white";
            ctx.strokeStyle = "black";
            ctx.lineWidth = 2;
            ctx.font = '14px "Press Start 2P", cursive';
            ctx.textAlign = "center";
            ctx.strokeText(text, textX, textY);
            ctx.fillText(text, textX, textY);
        }
    });
    droppedItems.forEach(item => {
         if (closestInteractableDroppedItemIdRef.current === item.id) {
            const itemDef = itemDefinitions.get(item.itemDefId.toString());
            const itemName = itemDef ? itemDef.name : 'Item';
            const text = `Press E to pick up ${itemName} (x${item.quantity})`;
            const textX = item.posX;
            const textY = item.posY - 25;
            ctx.fillStyle = "white";
            ctx.strokeStyle = "black";
            ctx.lineWidth = 2;
            ctx.font = '14px "Press Start 2P", cursive';
            ctx.textAlign = "center";
            ctx.strokeText(text, textX, textY);
            ctx.fillText(text, textX, textY);
        }
    });
    campfires.forEach(fire => {
         if (closestInteractableCampfireIdRef.current === fire.id) {
            const text = "Press E to Use";
            const textX = fire.posX;
            const textY = fire.posY - (CAMPFIRE_HEIGHT / 2) - 10; // 10px above the sprite top
            ctx.fillStyle = "white";
            ctx.strokeStyle = "black";
            ctx.lineWidth = 2;
            ctx.font = '14px "Press Start 2P", cursive';
            ctx.textAlign = "center";
            ctx.strokeText(text, textX, textY);
            ctx.fillText(text, textX, textY);
         }
    });

    // <<< ADDED: Render Wooden Storage Box Interaction Label >>>
    woodenStorageBoxes.forEach(box => {
         if (closestInteractableBoxIdRef.current === box.id) {
            const text = "Press E to Open";
            const textX = box.posX;
            const textY = box.posY - (BOX_HEIGHT / 2) - 10; // Adjust Y offset as needed
            ctx.fillStyle = "white";
            ctx.strokeStyle = "black";
            ctx.lineWidth = 2;
            ctx.font = '14px "Press Start 2P", cursive';
            ctx.textAlign = "center";
            ctx.strokeText(text, textX, textY);
            ctx.fillText(text, textX, textY);
         }
    });

    // Render placement preview (generic)
    if (placementInfo && currentWorldMouseX !== null && currentWorldMouseY !== null) {
        // ... (preview rendering using placementInfo.iconAssetName) ...
         // Get the image using the iconAssetName from placementInfo
         const previewImg = itemImagesRef.current.get(placementInfo.iconAssetName);
         
         if (previewImg && previewImg.complete && previewImg.naturalHeight !== 0) {
             const drawWidth = CAMPFIRE_WIDTH_PREVIEW; // TODO: Make this dynamic based on item type?
             const drawHeight = CAMPFIRE_HEIGHT_PREVIEW;
             
             ctx.save(); 
             let placementMessage = placementError; // Use error from hook
             if (isPlacementTooFar) {
                 ctx.filter = 'grayscale(80%) brightness(1.2) contrast(0.8) opacity(50%)';
                 placementMessage = "Too far away"; 
             } else {
                 ctx.globalAlpha = 0.6;
             }
 
             ctx.drawImage(previewImg, currentWorldMouseX - drawWidth / 2, currentWorldMouseY - drawHeight / 2, drawWidth, drawHeight);
             
             if (placementMessage) {
                 ctx.fillStyle = isPlacementTooFar ? 'orange' : 'red'; 
                 ctx.font = '12px "Press Start 2P", cursive';
                 ctx.textAlign = 'center';
                 ctx.filter = 'none'; 
                 ctx.globalAlpha = 1.0;
                 ctx.fillText(placementMessage, currentWorldMouseX, currentWorldMouseY - drawHeight / 2 - 5);
             }
             ctx.restore(); 
         } else {
             // Fallback if image not loaded yet
             ctx.fillStyle = "rgba(255, 255, 255, 0.3)";
             ctx.fillRect(currentWorldMouseX - 32, currentWorldMouseY - 32, 64, 64);
         }
    }
    ctx.restore(); // Restore from world space rendering

    // --- Screen Space Effects ---

    // 1. Prepare Off-screen Mask
    maskCtx.clearRect(0, 0, maskCanvas.width, maskCanvas.height);
    let overlayRgba = 'transparent';
    if (worldState) {
        overlayRgba = interpolateRgba(worldState.cycleProgress, currentKeyframes);
    }
    if (overlayRgba !== 'transparent' && overlayRgba !== 'rgba(0,0,0,0.00)') {
        // Fill mask canvas with overlay color
        maskCtx.fillStyle = overlayRgba;
        maskCtx.fillRect(0, 0, maskCanvas.width, maskCanvas.height);

        // Cut holes using destination-out with a radial gradient for soft edges
        maskCtx.globalCompositeOperation = 'destination-out';
        campfires.forEach(fire => {
            // --- Only cut hole if burning ---
            if (fire.isBurning) {
                const screenX = fire.posX + cameraOffsetX;
                const screenY = fire.posY + cameraOffsetY;
                const radius = CAMPFIRE_LIGHT_RADIUS_BASE; // Use base radius for mask shape

                // Create gradient: Opaque white in inner half -> Transparent white at edge
                const maskGradient = maskCtx.createRadialGradient(screenX, screenY, radius * 0.5, screenX, screenY, radius);
                maskGradient.addColorStop(0, 'rgba(255, 255, 255, 1)'); // Opaque white starts at 50% radius
                maskGradient.addColorStop(1, 'rgba(255, 255, 255, 0)'); // Transparent white at the edge

                maskCtx.fillStyle = maskGradient; // Use the gradient to fill
                maskCtx.beginPath();
                maskCtx.arc(screenX, screenY, radius, 0, Math.PI * 2);
                maskCtx.fill();
            }
            // --- End burning check ---
        });
        maskCtx.globalCompositeOperation = 'source-over'; // Reset
    }

    // 2. Draw the Masked Overlay onto the main canvas
    if (overlayRgba !== 'transparent' && overlayRgba !== 'rgba(0,0,0,0.00)') {
         ctx.drawImage(maskCanvas, 0, 0);
    }

    // 3. Draw Campfire Sprites (AFTER Masked Overlay)
    // --- MOVED: Campfire sprites are now rendered in the world space ground layer ---
    /* 
    campfires.forEach(fire => {
        const screenX = fire.posX + cameraOffsetX;
        const screenY = fire.posY + cameraOffsetY;
        // Pass the isBurning state from the campfire data
        renderCampfire(ctx, screenX, screenY, fire.isBurning);

        // --- NEW: Draw Interaction Indicator ---
        if (interactionProgress && interactionProgress.targetId === fire.id) {
            const interactionDuration = Date.now() - interactionProgress.startTime;
            const progressPercent = Math.min(interactionDuration / HOLD_INTERACTION_DURATION_MS, 1);
            // Use the helper function to draw the indicator above the campfire
            drawInteractionIndicator(ctx, screenX, screenY - CAMPFIRE_HEIGHT / 2 - 15, progressPercent);
        }
        // --- End Interaction Indicator ---
    });
    */
    
    // --- NEW: Draw Campfire Interaction Indicator (Screen Space) ---
    // We draw this *after* the overlay, but before the glow, so the indicator itself isn't shaded.
    campfires.forEach(fire => {
        if (interactionProgress && interactionProgress.targetId === fire.id) {
             const screenX = fire.posX + cameraOffsetX;
             const screenY = fire.posY + cameraOffsetY;
             const interactionDuration = Date.now() - interactionProgress.startTime;
             const progressPercent = Math.min(interactionDuration / HOLD_INTERACTION_DURATION_MS, 1);
             drawInteractionIndicator(ctx, screenX, screenY - CAMPFIRE_HEIGHT / 2 - 15, progressPercent);
         }
    });
    // --- End Interaction Indicator ---

    // 4. Draw Campfire Light Glow (Additively)
    ctx.save();
    ctx.globalCompositeOperation = 'lighter';
    campfires.forEach(fire => {
        // --- Only draw glow if burning ---
        if (fire.isBurning) {
            const lightScreenX = fire.posX + cameraOffsetX;
            const lightScreenY = fire.posY + cameraOffsetY;
            const flicker = (Math.random() - 0.5) * 2 * CAMPFIRE_FLICKER_AMOUNT;
            const currentLightRadius = Math.max(0, CAMPFIRE_LIGHT_RADIUS_BASE + flicker);

            // Use the WARNER gradient colors defined earlier
            const lightGradient = ctx.createRadialGradient(lightScreenX, lightScreenY, 0, lightScreenX, lightScreenY, currentLightRadius);
            lightGradient.addColorStop(0, CAMPFIRE_LIGHT_INNER_COLOR);
            lightGradient.addColorStop(1, CAMPFIRE_LIGHT_OUTER_COLOR);
            ctx.fillStyle = lightGradient;
            ctx.beginPath();
            ctx.arc(lightScreenX, lightScreenY, currentLightRadius, 0, Math.PI * 2);
            ctx.fill();
        }
        // --- End burning check ---
    });
    ctx.restore();

    // 5. Draw UI (Minimap)
    if (isMinimapOpen) {
        drawMinimapOntoCanvas({
            ctx, players, trees, stones, campfires, localPlayerId,
            canvasWidth: currentCanvasWidth, canvasHeight: currentCanvasHeight, isMouseOverMinimap
        });
    }
  }, [
      getLocalPlayer,
      drawWorldBackground,
      players,
      trees,
      stones,
      campfires,
      mushrooms, // Add mushrooms to dependencies
      worldState,
      currentKeyframes,
      localPlayerId,
      isMinimapOpen,
      isMouseOverMinimap,
      canvasSize.width,
      canvasSize.height,
      animationFrame,
      placementError,
      activeEquipments,
      itemDefinitions,
      interactionProgress, // Still needed for campfire interaction indicator
      droppedItems,
      woodenStorageBoxes // Added dependency
    ]);

  const gameLoop = useCallback(() => {
    // Only update player input if not dead
    if (!isInputDisabled.current) {
        updatePlayerBasedOnInput();
        // --- NEW: Check for continuous swing --- 
        if (isMouseDownRef.current) {
            attemptSwing();
        }
        // --- END Check ---
    }
    // Always render the game world (or background behind death screen)
    renderGame();
    requestIdRef.current = requestAnimationFrame(gameLoop);
  }, [updatePlayerBasedOnInput, renderGame, attemptSwing]); // Added attemptSwing dependency

  useEffect(() => {
    requestIdRef.current = requestAnimationFrame(gameLoop);
    return () => {
      cancelAnimationFrame(requestIdRef.current);
    };
  }, [gameLoop]);

  // Effect to handle window resizing
  useEffect(() => {
    const handleResize = () => {
      setCanvasSize({ width: window.innerWidth, height: window.innerHeight });
    };

    window.addEventListener('resize', handleResize);
    handleResize(); // Initial size

    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // --- Respawn Handler ---
  const handleRespawnRequest = useCallback(() => {
    if (!connection?.reducers) { // Check if connection and reducers exist
      console.error("Connection or reducers not available for respawn request.");
      return;
    }
    console.log("Requesting respawn via generated function...");
    try {
      // Call the generated reducer function directly
      connection.reducers.requestRespawn();
    } catch (err) {
      console.error("Error calling requestRespawn reducer:", err);
      // Handle potential errors during the call itself
    }
    // Client state will update automatically via subscription when player.isDead changes
  }, [connection]);

  // Add console logs for debugging
  // console.log(`[DeathCheck] isPlayerDead: ${localPlayer?.isDead}, respawnTimestampMs: ${respawnTimestampMs}, connection exists: ${!!connection}`);
  if (localPlayer) {
      // console.log('[DeathCheck] localPlayer state:', localPlayer);
  }

  return (
    <> {/* Use Fragment to return multiple elements */}
      {/* Conditionally render Death Screen */}
      {localPlayer?.isDead && respawnTimestampMs > 0 && connection && (
        <DeathScreen
          respawnAt={respawnTimestampMs}
          onRespawn={handleRespawnRequest}
        />
      )}

      <canvas
        ref={canvasRef}
        width={canvasSize.width}
        height={canvasSize.height}
        style={{ cursor: isInputDisabled.current ? 'default' : (placementInfo ? 'cell' : 'crosshair') }}
        onContextMenu={placementInfo ? (e) => e.preventDefault() : undefined}
      />
    </>
  );
};

export default GameCanvas; 