import { gameConfig } from '../config/gameConfig';
import { Player as SpacetimeDBPlayer } from '../generated';

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
// Unused constants removed
const MINIMAP_WORLD_BG_COLOR = 'rgba(52, 88, 52, 0.2)';

// Grid Constants
const GRID_DIVISIONS_X = 2;
const GRID_DIVISIONS_Y = 2;
const GRID_LINE_COLOR = 'rgba(200, 200, 200, 0.3)';
const GRID_TEXT_COLOR = 'rgba(255, 255, 255, 0.5)';
const GRID_TEXT_FONT = '10px Arial';

// Props required for drawing the minimap
interface MinimapProps {
  ctx: CanvasRenderingContext2D;
  players: Map<string, SpacetimeDBPlayer>; // Map of player identities to player data
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

  // --- Draw Grid --- 
  const gridCellWidth = effectiveMinimapDrawWidth / GRID_DIVISIONS_X;
  const gridCellHeight = effectiveMinimapDrawHeight / GRID_DIVISIONS_Y;

  ctx.strokeStyle = GRID_LINE_COLOR;
  ctx.lineWidth = 0.5;
  ctx.fillStyle = GRID_TEXT_COLOR;
  ctx.font = GRID_TEXT_FONT;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';

  // Draw Vertical Lines
  for (let i = 0; i <= GRID_DIVISIONS_X; i++) {
    const x = drawOffsetX + i * gridCellWidth;
    ctx.beginPath();
    ctx.moveTo(x, drawOffsetY);
    ctx.lineTo(x, drawOffsetY + effectiveMinimapDrawHeight);
    ctx.stroke();
  }

  // Draw Horizontal Lines
  for (let i = 0; i <= GRID_DIVISIONS_Y; i++) {
    const y = drawOffsetY + i * gridCellHeight;
    ctx.beginPath();
    ctx.moveTo(drawOffsetX, y);
    ctx.lineTo(drawOffsetX + effectiveMinimapDrawWidth, y);
    ctx.stroke();
  }

  // Draw Cell Labels (e.g., A1, B1, A2) in the top-left corner of each cell
  for (let row = 0; row < GRID_DIVISIONS_Y; row++) {
    for (let col = 0; col < GRID_DIVISIONS_X; col++) {
      const cellX = drawOffsetX + col * gridCellWidth;
      const cellY = drawOffsetY + row * gridCellHeight;
      
      const colLabel = String.fromCharCode(65 + col); // A, B, C...
      const rowLabel = (row + 1).toString(); // 1, 2, 3...
      const label = colLabel + rowLabel;

      ctx.fillText(label, cellX + 2, cellY + 2); // Draw label with small offset
    }
  }

  // --- End Grid Drawing ---

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
  ctx.restore(); 
}

// Export the calculated dimensions for potential use elsewhere (e.g., mouse interaction checks)
export const MINIMAP_DIMENSIONS = {
  width: MINIMAP_WIDTH,
  height: MINIMAP_HEIGHT,
};