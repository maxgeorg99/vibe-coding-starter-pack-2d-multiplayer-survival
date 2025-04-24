import { gameConfig } from '../config/gameConfig';
import { Player as SpacetimeDBPlayer, Tree, Stone as SpacetimeDBStone } from '../generated';

// --- Calculate Proportional Dimensions ---
const worldPixelWidth = gameConfig.worldWidth * gameConfig.tileSize;
const worldPixelHeight = gameConfig.worldHeight * gameConfig.tileSize;
const worldAspectRatio = worldPixelHeight / worldPixelWidth;

const BASE_MINIMAP_WIDTH = 400; // Base width for calculation
const calculatedMinimapHeight = BASE_MINIMAP_WIDTH * worldAspectRatio;

// Minimap constants
const MINIMAP_WIDTH = BASE_MINIMAP_WIDTH;
const MINIMAP_HEIGHT = Math.round(calculatedMinimapHeight); // Use calculated height
const MINIMAP_BG_COLOR_NORMAL = 'rgba(40, 40, 60, 0.2)';
const MINIMAP_BG_COLOR_HOVER = 'rgba(60, 60, 80, 0.2)';
const MINIMAP_BORDER_COLOR = '#a0a0c0';
const PLAYER_DOT_SIZE = 3;
const LOCAL_PLAYER_DOT_COLOR = '#FFFF00';
// Add colors for trees and rocks
const TREE_DOT_COLOR = '#008000'; // Green
const ROCK_DOT_COLOR = '#808080'; // Grey
const ENTITY_DOT_SIZE = 2; // Slightly smaller dot size for world objects
// Unused constants removed
const MINIMAP_WORLD_BG_COLOR = 'rgba(52, 88, 52, 0.2)';

// Grid Constants - Divisions will be calculated dynamically
const GRID_LINE_COLOR = 'rgba(200, 200, 200, 0.3)';
const GRID_TEXT_COLOR = 'rgba(255, 255, 255, 0.5)';
const GRID_TEXT_FONT = '10px Arial';

// Props required for drawing the minimap
interface MinimapProps {
  ctx: CanvasRenderingContext2D;
  players: Map<string, SpacetimeDBPlayer>; // Map of player identities to player data
  trees: Map<string, Tree>; // Map of tree identities/keys to tree data
  stones: Map<string, SpacetimeDBStone>; // Add stones prop
  localPlayerId?: string; // Optional ID of the player viewing the map
  canvasWidth: number; // Width of the main game canvas
  canvasHeight: number; // Height of the main game canvas
  isMouseOverMinimap: boolean; // To change background on hover
}

/**
 * Draws the minimap overlay onto the provided canvas context.
 */
export function drawMinimapOntoCanvas({
  ctx,
  players,
  trees,
  stones,
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

  // --- Calculate Scale to Fit Entire World ---
  const worldPixelWidth = gameConfig.worldWidth * gameConfig.tileSize;
  const worldPixelHeight = gameConfig.worldHeight * gameConfig.tileSize;
  const scaleX = minimapWidth / worldPixelWidth;
  const scaleY = minimapHeight / worldPixelHeight;
  // Use the smaller scale factor to ensure the entire world fits without distortion
  const uniformScale = Math.min(scaleX, scaleY);

  // Adjust minimap drawing dimensions if aspect ratios don't match
  const effectiveMinimapDrawWidth = worldPixelWidth * uniformScale;
  const effectiveMinimapDrawHeight = worldPixelHeight * uniformScale;
  // Center the actual map drawing within the minimap background area
  const drawOffsetX = minimapX + (minimapWidth - effectiveMinimapDrawWidth) / 2;
  const drawOffsetY = minimapY + (minimapHeight - effectiveMinimapDrawHeight) / 2;

  // --- Apply Retro Styling --- 
  ctx.save(); // Save context before applying shadow/styles

  // Apply shadow (draw rectangle slightly offset first, then the main one)
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

  // 3. Draw World Background Area (Dark Green) within the effective drawing area
  ctx.fillStyle = MINIMAP_WORLD_BG_COLOR;
  // Use drawOffsetX/Y and effective dimensions
  ctx.fillRect(drawOffsetX, drawOffsetY, effectiveMinimapDrawWidth, effectiveMinimapDrawHeight);

  // --- Calculate Grid Divisions Dynamically ---
  const gridCellSize = gameConfig.minimapGridCellSizePixels > 0 ? gameConfig.minimapGridCellSizePixels : 1; // Avoid division by zero

  const gridDivisionsX = Math.max(1, Math.round(worldPixelWidth / gridCellSize)); // Ensure at least 1 division
  const gridDivisionsY = Math.max(1, Math.round(worldPixelHeight / gridCellSize)); // Ensure at least 1 division

  // --- Draw Grid ---
  const gridCellWidth = effectiveMinimapDrawWidth / gridDivisionsX;
  const gridCellHeight = effectiveMinimapDrawHeight / gridDivisionsY;

  ctx.strokeStyle = GRID_LINE_COLOR;
  ctx.lineWidth = 0.5;
  ctx.fillStyle = GRID_TEXT_COLOR;
  ctx.font = GRID_TEXT_FONT;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';

  // Draw Vertical Lines
  for (let i = 0; i <= gridDivisionsX; i++) {
    const x = drawOffsetX + i * gridCellWidth;
    ctx.beginPath();
    ctx.moveTo(x, drawOffsetY);
    ctx.lineTo(x, drawOffsetY + effectiveMinimapDrawHeight);
    ctx.stroke();
  }

  // Draw Horizontal Lines
  for (let i = 0; i <= gridDivisionsY; i++) {
    const y = drawOffsetY + i * gridCellHeight;
    ctx.beginPath();
    ctx.moveTo(drawOffsetX, y);
    ctx.lineTo(drawOffsetX + effectiveMinimapDrawWidth, y);
    ctx.stroke();
  }

  // Draw Cell Labels (e.g., A1, B1, A2) in the top-left corner of each cell
  for (let row = 0; row < gridDivisionsY; row++) {
    for (let col = 0; col < gridDivisionsX; col++) {
      const cellX = drawOffsetX + col * gridCellWidth;
      const cellY = drawOffsetY + row * gridCellHeight;
      
      const colLabel = String.fromCharCode(65 + col); // A, B, C...
      const rowLabel = (row + 1).toString(); // 1, 2, 3...
      const label = colLabel + rowLabel;

      ctx.fillText(label, cellX + 2, cellY + 2); // Draw label with small offset
    }
  }

  // --- End Grid Drawing ---

  // --- Draw Trees ---
  ctx.fillStyle = TREE_DOT_COLOR;
  trees.forEach(tree => {
      // Use posX/posY based on GameCanvas.tsx usage
      const treeMinimapX = drawOffsetX + tree.posX * uniformScale;
      const treeMinimapY = drawOffsetY + tree.posY * uniformScale;

      // Draw only if within minimap bounds
      if (treeMinimapX >= minimapX && treeMinimapX <= minimapX + minimapWidth &&
          treeMinimapY >= minimapY && treeMinimapY <= minimapY + minimapHeight) 
      {
        ctx.fillRect(
          treeMinimapX - ENTITY_DOT_SIZE / 2,
          treeMinimapY - ENTITY_DOT_SIZE / 2,
          ENTITY_DOT_SIZE,
          ENTITY_DOT_SIZE
        );
      }
  });

  // --- Draw Stones ---
  ctx.fillStyle = ROCK_DOT_COLOR; // Use ROCK_DOT_COLOR
  stones.forEach(stone => { // Use stones prop (type SpacetimeDBStone)
      // Use posX/posY based on GameCanvas.tsx usage
      const stoneMinimapX = drawOffsetX + stone.posX * uniformScale;
      const stoneMinimapY = drawOffsetY + stone.posY * uniformScale;

      // Draw only if within minimap bounds
      if (stoneMinimapX >= minimapX && stoneMinimapX <= minimapX + minimapWidth &&
          stoneMinimapY >= minimapY && stoneMinimapY <= minimapY + minimapHeight) 
      {
        ctx.fillRect(
          stoneMinimapX - ENTITY_DOT_SIZE / 2,
          stoneMinimapY - ENTITY_DOT_SIZE / 2,
          ENTITY_DOT_SIZE,
          ENTITY_DOT_SIZE
        );
      }
  });

  // --- Draw Local Player --- 
  players.forEach(player => {
    const isLocal = player.identity.toHexString() === localPlayerId;
    // Only draw if it's the local player
    if (isLocal) {
      ctx.fillStyle = LOCAL_PLAYER_DOT_COLOR;

      // Calculate player position relative to the world origin (0,0) and scale it
      const playerMinimapX = drawOffsetX + player.positionX * uniformScale;
      const playerMinimapY = drawOffsetY + player.positionY * uniformScale;

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
    }
  });

  // Restore context after drawing clipped content
  ctx.restore(); // Re-enable restore
}

// Export the calculated dimensions for potential use elsewhere (e.g., mouse interaction checks)
export const MINIMAP_DIMENSIONS = {
  width: MINIMAP_WIDTH,
  height: MINIMAP_HEIGHT,
};