import React from 'react';
import { gameConfig } from '../config/gameConfig';
import { Player as SpacetimeDBPlayer, Tree as SpacetimeDBTree, Stone as SpacetimeDBStone, Campfire as SpacetimeDBCampfire } from '../generated';

// Minimap constants
const MINIMAP_WIDTH = 200;
const MINIMAP_HEIGHT = 150;
const MINIMAP_PLAYER_DOT_SIZE = 3;
const MINIMAP_BG_COLOR_NORMAL = 'rgba(40, 40, 60, 0.2)';
const MINIMAP_BG_COLOR_HOVER = 'rgba(60, 60, 80, 0.2)';
const MINIMAP_BORDER_COLOR = '#a0a0c0';
const MINIMAP_LOCAL_PLAYER_COLOR = 'yellow';
const MINIMAP_OTHER_PLAYER_COLOR = 'red';
const MINIMAP_SCALE = 0.1;
const PLAYER_DOT_SIZE = 4;
const LOCAL_PLAYER_DOT_COLOR = '#FFFF00';
const OTHER_PLAYER_DOT_COLOR = '#FF0000';
const TREE_DOT_SIZE = 3;
const TREE_DOT_COLOR = '#00FF00';
const STONE_DOT_SIZE = 2;
const STONE_DOT_COLOR = '#808080';
const CAMPFIRE_DOT_SIZE = 3;
const CAMPFIRE_DOT_COLOR = '#FFA500';
const MINIMAP_WORLD_BG_COLOR = 'rgba(52, 88, 52, 0.2)';

interface MinimapProps {
  ctx: CanvasRenderingContext2D;
  players: Map<string, SpacetimeDBPlayer>;
  trees: Map<string, SpacetimeDBTree>;
  stones: Map<string, SpacetimeDBStone>;
  campfires: Map<string, SpacetimeDBCampfire>;
  localPlayerId?: string;
  canvasWidth: number;
  canvasHeight: number;
  isMouseOverMinimap: boolean;
}

const Minimap: React.FC<MinimapProps> = ({
  ctx,
  players,
  trees,
  stones,
  campfires,
  localPlayerId,
  canvasWidth,
  canvasHeight,
  isMouseOverMinimap,
}) => {
  const draw = (ctx: CanvasRenderingContext2D) => {
    // Calculate position and dimensions
    const mapX = (canvasWidth - MINIMAP_WIDTH) / 2;
    const mapY = (canvasHeight - MINIMAP_HEIGHT) / 2;
    const worldPixelWidth = gameConfig.worldWidth * gameConfig.tileSize;
    const worldPixelHeight = gameConfig.worldHeight * gameConfig.tileSize;
    const scaleX = MINIMAP_WIDTH / worldPixelWidth;
    const scaleY = MINIMAP_HEIGHT / worldPixelHeight;

    // Draw background (respecting hover state passed as prop)
    ctx.fillStyle = isMouseOverMinimap ? MINIMAP_BG_COLOR_HOVER : MINIMAP_BG_COLOR_NORMAL;
    ctx.fillRect(mapX, mapY, MINIMAP_WIDTH, MINIMAP_HEIGHT);

    // Draw border
    ctx.strokeStyle = MINIMAP_BORDER_COLOR;
    ctx.lineWidth = 1;
    ctx.strokeRect(mapX, mapY, MINIMAP_WIDTH, MINIMAP_HEIGHT);

    // Draw players
    players.forEach(player => {
      const playerMapX = mapX + player.positionX * scaleX;
      const playerMapY = mapY + player.positionY * scaleY;

      // Use different colors for local vs other players
      ctx.fillStyle = player.identity.toHexString() === localPlayerId
        ? MINIMAP_LOCAL_PLAYER_COLOR
        : MINIMAP_OTHER_PLAYER_COLOR;

      // Draw player dot
      ctx.beginPath();
      ctx.arc(playerMapX, playerMapY, MINIMAP_PLAYER_DOT_SIZE, 0, 2 * Math.PI);
      ctx.fill();
    });

    // Draw trees
    trees.forEach(tree => {
      const treeMinimapX = mapX + (tree.posX - (scaleX * (canvasWidth / 2))) * MINIMAP_SCALE;
      const treeMinimapY = mapY + (tree.posY - (scaleY * (canvasHeight / 2))) * MINIMAP_SCALE;

      if (treeMinimapX >= mapX && treeMinimapX <= mapX + MINIMAP_WIDTH &&
          treeMinimapY >= mapY && treeMinimapY <= mapY + MINIMAP_HEIGHT) {
        ctx.fillStyle = TREE_DOT_COLOR;
        ctx.beginPath();
        ctx.arc(treeMinimapX, treeMinimapY, TREE_DOT_SIZE, 0, 2 * Math.PI);
        ctx.fill();
      }
    });

    // Draw stones
    stones.forEach(stone => {
      const stoneMinimapX = mapX + (stone.posX - (scaleX * (canvasWidth / 2))) * MINIMAP_SCALE;
      const stoneMinimapY = mapY + (stone.posY - (scaleY * (canvasHeight / 2))) * MINIMAP_SCALE;

      if (stoneMinimapX >= mapX && stoneMinimapX <= mapX + MINIMAP_WIDTH &&
          stoneMinimapY >= mapY && stoneMinimapY <= mapY + MINIMAP_HEIGHT) {
        ctx.fillStyle = STONE_DOT_COLOR;
        ctx.beginPath();
        ctx.arc(stoneMinimapX, stoneMinimapY, STONE_DOT_SIZE, 0, 2 * Math.PI);
        ctx.fill();
      }
    });

    // Draw campfires
    campfires.forEach(fire => {
      const fireMinimapX = mapX + (fire.posX - (scaleX * (canvasWidth / 2))) * MINIMAP_SCALE;
      const fireMinimapY = mapY + (fire.posY - (scaleY * (canvasHeight / 2))) * MINIMAP_SCALE;

      if (fireMinimapX >= mapX && fireMinimapX <= mapX + MINIMAP_WIDTH &&
          fireMinimapY >= mapY && fireMinimapY <= mapY + MINIMAP_HEIGHT) {
        ctx.fillStyle = CAMPFIRE_DOT_COLOR;
        ctx.beginPath();
        ctx.arc(fireMinimapX, fireMinimapY, CAMPFIRE_DOT_SIZE, 0, 2 * Math.PI);
        ctx.fill();
      }
    });
  };

  // This component doesn't render directly to the DOM,
  // it provides a drawing function for the main canvas.
  // We might refactor this later if needed, but for now,
  // let's keep the drawing logic encapsulated here.
  // To make it "render", we'll rely on the parent GameCanvas
  // calling this component's draw logic.

  // We need a way for the parent to access the draw function.
  // We can expose it via a ref or pass it up.
  // For simplicity now, we'll assume GameCanvas will manage the drawing context.
  // Let's adjust GameCanvas to call a draw function from this component.

  // Returning null as this component doesn't render HTML itself.
  return null;
};

// We need a way for GameCanvas to actually *call* the drawing logic.
// Let's rethink the structure slightly. The Minimap component *should*
// perhaps manage its own canvas or be drawn onto the main one.
// Sticking with drawing onto the main one for now.

// How GameCanvas will use this:
// 1. Import Minimap.
// 2. Inside renderGame, if isMinimapOpen:
//    - Get the drawing context `ctx`.
//    - Call a function provided by Minimap, passing `ctx`.

// Let's export the drawing logic directly instead of making it a full component yet.
// This simplifies the integration for now.

export function drawMinimapOntoCanvas({
  ctx,
  players,
  trees,
  stones,
  campfires,
  localPlayerId,
  canvasWidth,
  canvasHeight,
  isMouseOverMinimap,
}: MinimapProps) {
  const minimapWidth = MINIMAP_WIDTH;
  const minimapHeight = MINIMAP_HEIGHT;
  
  // Calculate top-left corner for centering
  const minimapX = (canvasWidth - minimapWidth) / 2;
  const minimapY = (canvasHeight - minimapHeight) / 2;

  // --- Apply Retro Styling --- 
  ctx.save(); // Save context before applying shadow/styles

  // Apply shadow (draw rectangle slightly offset first, then the main one)
  // Note: Canvas shadows can be complex, this is a basic simulation
  const shadowOffset = 2;
  ctx.fillStyle = 'rgba(0,0,0,0.5)';
  ctx.fillRect(minimapX + shadowOffset, minimapY + shadowOffset, minimapWidth, minimapHeight);

  // 1. Draw Overall Minimap Background (Dark UI Color)
  ctx.fillStyle = isMouseOverMinimap ? MINIMAP_BG_COLOR_HOVER : MINIMAP_BG_COLOR_NORMAL;
  ctx.fillRect(minimapX, minimapY, minimapWidth, minimapHeight);

  // Draw border
  ctx.strokeStyle = MINIMAP_BORDER_COLOR;
  ctx.lineWidth = 1; // Match PlayerUI border style
  ctx.strokeRect(minimapX, minimapY, minimapWidth, minimapHeight);

  // Clip drawing to minimap bounds (optional, but good practice)
  ctx.beginPath();
  ctx.rect(minimapX, minimapY, minimapWidth, minimapHeight);
  ctx.clip();
  // --- End Initial Styling & Clip ---

  // Center view on local player if available
  const localPlayer = localPlayerId ? players.get(localPlayerId) : undefined;
  const centerX = localPlayer ? localPlayer.positionX : gameConfig.worldWidth * gameConfig.tileSize / 2;
  const centerY = localPlayer ? localPlayer.positionY : gameConfig.worldHeight * gameConfig.tileSize / 2;

  // Calculate the top-left corner of the minimap's view in world coordinates
  const viewOriginX = centerX - (minimapWidth / 2) / MINIMAP_SCALE;
  const viewOriginY = centerY - (minimapHeight / 2) / MINIMAP_SCALE;

  // 2. Calculate World Bounds Scaled onto Minimap
  const worldPxWidth = gameConfig.worldWidth * gameConfig.tileSize;
  const worldPxHeight = gameConfig.worldHeight * gameConfig.tileSize;
  const worldMinimapX = minimapX + (0 - viewOriginX) * MINIMAP_SCALE;
  const worldMinimapY = minimapY + (0 - viewOriginY) * MINIMAP_SCALE;
  const worldMinimapWidth = worldPxWidth * MINIMAP_SCALE;
  const worldMinimapHeight = worldPxHeight * MINIMAP_SCALE;

  // 3. Draw World Background Area (Dark Green)
  ctx.fillStyle = MINIMAP_WORLD_BG_COLOR;
  ctx.fillRect(worldMinimapX, worldMinimapY, worldMinimapWidth, worldMinimapHeight);

  // --- Draw Trees --- 
  ctx.fillStyle = TREE_DOT_COLOR;
  trees.forEach(tree => {
      // Calculate tree position relative to the minimap's view origin
      const treeMinimapX = minimapX + (tree.posX - viewOriginX) * MINIMAP_SCALE;
      const treeMinimapY = minimapY + (tree.posY - viewOriginY) * MINIMAP_SCALE;

      // Draw only if within minimap bounds (already clipped, but extra check)
      if (treeMinimapX >= minimapX && treeMinimapX <= minimapX + minimapWidth &&
          treeMinimapY >= minimapY && treeMinimapY <= minimapY + minimapHeight) 
      {
        ctx.fillRect(
          treeMinimapX - TREE_DOT_SIZE / 2,
          treeMinimapY - TREE_DOT_SIZE / 2,
          TREE_DOT_SIZE,
          TREE_DOT_SIZE
        );
      }
  });

  // --- Draw Stones --- 
  ctx.fillStyle = STONE_DOT_COLOR;
  stones.forEach(stone => {
    const stoneMinimapX = minimapX + (stone.posX - viewOriginX) * MINIMAP_SCALE;
    const stoneMinimapY = minimapY + (stone.posY - viewOriginY) * MINIMAP_SCALE;

    if (stoneMinimapX >= minimapX && stoneMinimapX <= minimapX + minimapWidth &&
        stoneMinimapY >= minimapY && stoneMinimapY <= minimapY + minimapHeight) 
    {
      ctx.fillRect(
        stoneMinimapX - STONE_DOT_SIZE / 2,
        stoneMinimapY - STONE_DOT_SIZE / 2,
        STONE_DOT_SIZE,
        STONE_DOT_SIZE
      );
    }
  });

  // --- Draw Campfires --- 
  ctx.fillStyle = CAMPFIRE_DOT_COLOR;
  campfires.forEach(fire => {
      const fireMinimapX = minimapX + (fire.posX - viewOriginX) * MINIMAP_SCALE;
      const fireMinimapY = minimapY + (fire.posY - viewOriginY) * MINIMAP_SCALE;

      if (fireMinimapX >= minimapX && fireMinimapX <= minimapX + minimapWidth &&
          fireMinimapY >= minimapY && fireMinimapY <= minimapY + minimapHeight) 
      {
          ctx.fillRect(
              fireMinimapX - CAMPFIRE_DOT_SIZE / 2,
              fireMinimapY - CAMPFIRE_DOT_SIZE / 2,
              CAMPFIRE_DOT_SIZE,
              CAMPFIRE_DOT_SIZE
          );
      }
  });

  // --- Draw Players --- 
  players.forEach(player => {
    const isLocal = player.identity.toHexString() === localPlayerId;
    ctx.fillStyle = isLocal ? LOCAL_PLAYER_DOT_COLOR : OTHER_PLAYER_DOT_COLOR;

    // Calculate player position relative to the minimap's view origin
    const playerMinimapX = minimapX + (player.positionX - viewOriginX) * MINIMAP_SCALE;
    const playerMinimapY = minimapY + (player.positionY - viewOriginY) * MINIMAP_SCALE;

    // Draw only if within minimap bounds
    if (playerMinimapX >= minimapX && playerMinimapX <= minimapX + minimapWidth &&
        playerMinimapY >= minimapY && playerMinimapY <= minimapY + minimapHeight) 
    {
      ctx.fillRect(
        playerMinimapX - PLAYER_DOT_SIZE / 2,
        playerMinimapY - PLAYER_DOT_SIZE / 2,
        PLAYER_DOT_SIZE,
        PLAYER_DOT_SIZE
      );
    }
  });

  // Restore context after drawing clipped content
  ctx.restore(); 
}

export const MINIMAP_DIMENSIONS = {
  width: MINIMAP_WIDTH,
  height: MINIMAP_HEIGHT,
};