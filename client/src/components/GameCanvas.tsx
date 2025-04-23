import React, { useEffect, useRef, useCallback, useState, useMemo } from 'react';
import {
  Player as SpacetimeDBPlayer,
  Tree as SpacetimeDBTree,
  Stone as SpacetimeDBStone,
  Campfire as SpacetimeDBCampfire,
  Mushroom as SpacetimeDBMushroom,
  WorldState as SpacetimeDBWorldState,
  ActiveEquipment as SpacetimeDBActiveEquipment,
  InventoryItem as SpacetimeDBInventoryItem,
  ItemDefinition as SpacetimeDBItemDefinition,
  DroppedItem as SpacetimeDBDroppedItem,
  WoodenStorageBox as SpacetimeDBWoodenStorageBox
} from '../generated';

// --- Core Hooks ---
import { useAnimationCycle } from '../hooks/useAnimationCycle';
import { useAssetLoader } from '../hooks/useAssetLoader';
import { useGameViewport } from '../hooks/useGameViewport';
import { useMousePosition } from '../hooks/useMousePosition';
import { useDayNightCycle } from '../hooks/useDayNightCycle';
import { useInteractionFinder } from '../hooks/useInteractionFinder';
import { useGameLoop } from '../hooks/useGameLoop';
import { useInputHandler } from '../hooks/useInputHandler';

// --- Rendering Utilities ---
import { renderWorldBackground } from '../utils/worldRenderingUtils';
import { renderGroundEntities, renderYSortedEntities } from '../utils/renderingUtils';
import { renderInteractionLabels } from '../utils/labelRenderingUtils';
import { renderPlacementPreview } from '../utils/placementRenderingUtils';
import { drawInteractionIndicator } from '../utils/interactionIndicator';
import { drawMinimapOntoCanvas } from './Minimap';

// --- Other Components & Utils ---
import DeathScreen from './DeathScreen.tsx';
import { itemIcons } from '../utils/itemIconUtils';
import { PlacementItemInfo, PlacementActions } from '../hooks/usePlacementManager';
import {
    CAMPFIRE_LIGHT_RADIUS_BASE,
    CAMPFIRE_FLICKER_AMOUNT,
    HOLD_INTERACTION_DURATION_MS,
    CAMPFIRE_HEIGHT,
    BOX_HEIGHT,
    CAMPFIRE_LIGHT_INNER_COLOR,
    CAMPFIRE_LIGHT_OUTER_COLOR,
    PLAYER_BOX_INTERACTION_DISTANCE_SQUARED
} from '../config/gameConfig';
import {
    isPlayer, isWoodenStorageBox
} from '../utils/typeGuards';

// --- Prop Interface ---
interface GameCanvasProps {
  players: Map<string, SpacetimeDBPlayer>;
  trees: Map<string, SpacetimeDBTree>;
  stones: Map<string, SpacetimeDBStone>;
  campfires: Map<string, SpacetimeDBCampfire>;
  mushrooms: Map<string, SpacetimeDBMushroom>;
  droppedItems: Map<string, SpacetimeDBDroppedItem>;
  woodenStorageBoxes: Map<string, SpacetimeDBWoodenStorageBox>;
  inventoryItems: Map<string, SpacetimeDBInventoryItem>;
  itemDefinitions: Map<string, SpacetimeDBItemDefinition>;
  worldState: SpacetimeDBWorldState | null;
  localPlayerId?: string;
  connection: any | null;
  activeEquipments: Map<string, SpacetimeDBActiveEquipment>;
  placementInfo: PlacementItemInfo | null;
  placementActions: PlacementActions;
  placementError: string | null;
  onSetInteractingWith: (target: { type: string; id: number | bigint } | null) => void;
  updatePlayerPosition: (dx: number, dy: number, intendedDirection?: 'up' | 'down' | 'left' | 'right' | null) => void;
  callJumpReducer: () => void;
  callSetSprintingReducer: (isSprinting: boolean) => void;
  isMinimapOpen: boolean;
  setIsMinimapOpen: React.Dispatch<React.SetStateAction<boolean>>;
}

/**
 * GameCanvas Component
 *
 * The main component responsible for rendering the game world, entities, UI elements,
 * and handling the game loop orchestration. It integrates various custom hooks
 * to manage specific aspects like input, viewport, assets, day/night cycle, etc.
 */
const GameCanvas: React.FC<GameCanvasProps> = ({
  players,
  trees,
  stones,
  campfires,
  mushrooms,
  droppedItems,
  woodenStorageBoxes,
  itemDefinitions,
  worldState,
  localPlayerId,
  connection,
  activeEquipments,
  placementInfo,
  placementActions,
  placementError,
  onSetInteractingWith,
  updatePlayerPosition,
  callJumpReducer,
  callSetSprintingReducer,
  isMinimapOpen,
  setIsMinimapOpen,
}) => {

  // --- Refs ---
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const lastPositionsRef = useRef<Map<string, {x: number, y: number}>>(new Map());
  const placementActionsRef = useRef(placementActions);
  useEffect(() => {
    placementActionsRef.current = placementActions;
  }, [placementActions]);

  // --- Core Game State Hooks ---
  const localPlayer = useMemo(() => {
    if (!localPlayerId) return undefined;
    return players.get(localPlayerId);
  }, [players, localPlayerId]);

  const { canvasSize, cameraOffsetX, cameraOffsetY } = useGameViewport(localPlayer);
  const { heroImageRef, grassImageRef, itemImagesRef } = useAssetLoader();
  const { worldMousePos } = useMousePosition({ canvasRef, cameraOffsetX, cameraOffsetY, canvasSize });
  const { overlayRgba, maskCanvasRef } = useDayNightCycle({ worldState, campfires, cameraOffsetX, cameraOffsetY, canvasSize });
  const {
    closestInteractableMushroomId,
    closestInteractableCampfireId,
    closestInteractableDroppedItemId,
    closestInteractableBoxId,
    isClosestInteractableBoxEmpty,
  } = useInteractionFinder({ localPlayer, mushrooms, campfires, droppedItems, woodenStorageBoxes });
  const animationFrame = useAnimationCycle(150, 4);
  const { interactionProgress, processInputsAndActions } = useInputHandler({
      canvasRef, connection, localPlayerId, localPlayer: localPlayer ?? null,
      activeEquipments, placementInfo, placementActions, worldMousePos,
      closestInteractableMushroomId, closestInteractableCampfireId, closestInteractableDroppedItemId,
      closestInteractableBoxId, isClosestInteractableBoxEmpty, isMinimapOpen, setIsMinimapOpen,
      onSetInteractingWith, updatePlayerPosition, callJumpReducer, callSetSprintingReducer,
  });

  // --- UI State ---
  const [isMouseOverMinimap ] = useState(false);

  // --- Derived State ---
  const respawnTimestampMs = useMemo(() => {
    if (localPlayer?.isDead && localPlayer.respawnAt) {
      return Number(localPlayer.respawnAt.microsSinceUnixEpoch / 1000n);
    }
    return 0;
  }, [localPlayer?.isDead, localPlayer?.respawnAt]);
  const cursorStyle = placementInfo ? 'cell' : 'crosshair';

  // --- Effects ---
  useEffect(() => {
    console.log("Preloading item images based on itemDefinitions update...");
    itemDefinitions.forEach(itemDef => {
      const iconSrc = itemIcons[itemDef.iconAssetName];
      if (itemDef && iconSrc && typeof iconSrc === 'string' && !itemImagesRef.current.has(itemDef.iconAssetName)) {
        const img = new Image();
        img.src = iconSrc;
        img.onload = () => {
          itemImagesRef.current.set(itemDef.iconAssetName, img);
          console.log(`Preloaded item image: ${itemDef.iconAssetName} from ${img.src}`);
        };
        img.onerror = () => console.error(`Failed to preload item image asset: ${itemDef.iconAssetName} (Expected path/source: ${iconSrc})`);
        itemImagesRef.current.set(itemDef.iconAssetName, img);
      }
    });
  }, [itemDefinitions]);

  const renderGame = useCallback(() => {
    const canvas = canvasRef.current;
    const maskCanvas = maskCanvasRef.current;
    if (!canvas || !maskCanvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const now_ms = Date.now();
    const currentWorldMouseX = worldMousePos.x;
    const currentWorldMouseY = worldMousePos.y;
    const currentCanvasWidth = canvasSize.width;
    const currentCanvasHeight = canvasSize.height;

    ctx.clearRect(0, 0, currentCanvasWidth, currentCanvasHeight);
    ctx.fillStyle = '#000000';
    ctx.fillRect(0, 0, currentCanvasWidth, currentCanvasHeight);

    ctx.save();
    ctx.translate(cameraOffsetX, cameraOffsetY);
    renderWorldBackground(ctx, grassImageRef);

    const groundItems: (SpacetimeDBMushroom | SpacetimeDBDroppedItem | SpacetimeDBCampfire)[] = [];
    const ySortedEntities: (SpacetimeDBPlayer | SpacetimeDBTree | SpacetimeDBStone | SpacetimeDBWoodenStorageBox)[] = [];

    mushrooms.forEach(m => { if (m.respawnAt === null || m.respawnAt === undefined) groundItems.push(m); });
    droppedItems.forEach(i => groundItems.push(i));
    campfires.forEach(c => groundItems.push(c));

    players.forEach(p => ySortedEntities.push(p));
    trees.forEach(t => { if (t.health > 0) ySortedEntities.push(t); });
    stones.forEach(s => { if (s.health > 0) ySortedEntities.push(s); });
    woodenStorageBoxes.forEach(b => ySortedEntities.push(b));

    ySortedEntities.sort((a, b) => {
        const yA = isPlayer(a) ? a.positionY : (isWoodenStorageBox(a) ? a.posY : a.posY);
        const yB = isPlayer(b) ? b.positionY : (isWoodenStorageBox(b) ? b.posY : b.posY);
        return yA - yB;
    });

    let isPlacementTooFar = false;
    if (placementInfo && localPlayer && currentWorldMouseX !== null && currentWorldMouseY !== null) {
         const placeDistSq = (currentWorldMouseX - localPlayer.positionX)**2 + (currentWorldMouseY - localPlayer.positionY)**2;
         const clientPlacementRangeSq = PLAYER_BOX_INTERACTION_DISTANCE_SQUARED * 1.1;
         if (placeDistSq > clientPlacementRangeSq) {
             isPlacementTooFar = true;
         }
    }

    renderGroundEntities({ ctx, groundItems, itemDefinitions, itemImagesRef, nowMs: now_ms });
    renderYSortedEntities({
        ctx, ySortedEntities, heroImageRef, lastPositionsRef, activeEquipments,
        itemDefinitions, itemImagesRef, worldMouseX: currentWorldMouseX, worldMouseY: currentWorldMouseY,
        animationFrame, nowMs: now_ms
    });

    renderInteractionLabels({
        ctx, mushrooms, campfires, droppedItems, woodenStorageBoxes, itemDefinitions,
        closestInteractableMushroomId, closestInteractableCampfireId,
        closestInteractableDroppedItemId, closestInteractableBoxId, isClosestInteractableBoxEmpty,
    });
    renderPlacementPreview({
        ctx, placementInfo, itemImagesRef, worldMouseX: currentWorldMouseX,
        worldMouseY: currentWorldMouseY, isPlacementTooFar, placementError,
    });

    ctx.restore();

    if (overlayRgba !== 'transparent' && overlayRgba !== 'rgba(0,0,0,0.00)') {
         ctx.drawImage(maskCanvas, 0, 0);
    }

    const drawIndicatorIfNeeded = (entityType: 'campfire' | 'wooden_storage_box', entityId: number, entityPosX: number, entityPosY: number, entityHeight: number) => {
        if (interactionProgress && interactionProgress.targetId === entityId && interactionProgress.targetType === entityType) {
            const screenX = entityPosX + cameraOffsetX;
            const screenY = entityPosY + cameraOffsetY;
            const interactionDuration = Date.now() - interactionProgress.startTime;
            const progressPercent = Math.min(interactionDuration / HOLD_INTERACTION_DURATION_MS, 1);
            drawInteractionIndicator(ctx, screenX, screenY - (entityHeight / 2) - 15, progressPercent);
        }
    };

    campfires.forEach(fire => { drawIndicatorIfNeeded('campfire', fire.id, fire.posX, fire.posY, CAMPFIRE_HEIGHT); });
    woodenStorageBoxes.forEach(box => { if (interactionProgress && interactionProgress.targetId === box.id && isClosestInteractableBoxEmpty) { drawIndicatorIfNeeded('wooden_storage_box', box.id, box.posX, box.posY, BOX_HEIGHT); } });

    ctx.save();
    ctx.globalCompositeOperation = 'lighter';
    campfires.forEach(fire => {
        if (fire.isBurning) {
            const lightScreenX = fire.posX + cameraOffsetX;
            const lightScreenY = fire.posY + cameraOffsetY;
            const flicker = (Math.random() - 0.5) * 2 * CAMPFIRE_FLICKER_AMOUNT;
            const currentLightRadius = Math.max(0, CAMPFIRE_LIGHT_RADIUS_BASE + flicker);
            const lightGradient = ctx.createRadialGradient(lightScreenX, lightScreenY, 0, lightScreenX, lightScreenY, currentLightRadius);
            lightGradient.addColorStop(0, CAMPFIRE_LIGHT_INNER_COLOR);
            lightGradient.addColorStop(1, CAMPFIRE_LIGHT_OUTER_COLOR);
            ctx.fillStyle = lightGradient;
            ctx.beginPath();
            ctx.arc(lightScreenX, lightScreenY, currentLightRadius, 0, Math.PI * 2);
            ctx.fill();
        }
    });
    ctx.restore();

    if (isMinimapOpen) {
        drawMinimapOntoCanvas({ ctx, players, localPlayerId, canvasWidth: currentCanvasWidth, canvasHeight: currentCanvasHeight, isMouseOverMinimap });
    }
  }, [
      players, trees, stones, campfires, mushrooms, droppedItems, woodenStorageBoxes,
      worldState, localPlayerId, localPlayer, activeEquipments, itemDefinitions,
      itemImagesRef, heroImageRef, grassImageRef, cameraOffsetX, cameraOffsetY,
      canvasSize.width, canvasSize.height, worldMousePos.x, worldMousePos.y,
      animationFrame, placementInfo, placementError, overlayRgba, maskCanvasRef,
      closestInteractableMushroomId, closestInteractableCampfireId,
      closestInteractableDroppedItemId, closestInteractableBoxId, isClosestInteractableBoxEmpty,
      interactionProgress, isMinimapOpen, isMouseOverMinimap, lastPositionsRef
    ]);

  const gameLoopCallback = useCallback(() => {
    processInputsAndActions();
    renderGame();
  }, [processInputsAndActions, renderGame]);
  useGameLoop(gameLoopCallback);

  const handleRespawnRequest = useCallback(() => {
    if (!connection?.reducers) {
      console.error("Connection or reducers not available for respawn request.");
      return;
    }
    console.log("Requesting respawn via generated function...");
    try {
      connection.reducers.requestRespawn();
    } catch (err) {
      console.error("Error calling requestRespawn reducer:", err);
    }
  }, [connection]);

  return (
    <>
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
        style={{ cursor: cursorStyle }}
        onContextMenu={(e) => {
            if (placementInfo || (isMinimapOpen && isMouseOverMinimap)) {
                 e.preventDefault();
            }
        }}
      />
    </>
  );
};

export default GameCanvas; 