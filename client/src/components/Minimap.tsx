import React from 'react';
import { gameConfig } from '../config/gameConfig';
import { Player as SpacetimeDBPlayer } from '../generated';

// Minimap constants
const MINIMAP_WIDTH = 200;
const MINIMAP_HEIGHT = 150;
const MINIMAP_PLAYER_DOT_SIZE = 3;
const MINIMAP_BG_COLOR_NORMAL = 'rgba(100, 100, 100, 0.6)';
const MINIMAP_BG_COLOR_HOVER = 'rgba(100, 100, 100, 0.9)'; // Keep hover color for potential future use
const MINIMAP_BORDER_COLOR = 'white';
const MINIMAP_LOCAL_PLAYER_COLOR = 'yellow';
const MINIMAP_OTHER_PLAYER_COLOR = 'red';

interface MinimapProps {
  players: Map<string, SpacetimeDBPlayer>;
  localPlayerId?: string;
  canvasWidth: number;
  canvasHeight: number;
  isMouseOver: boolean; // Receive hover state as prop
}

const Minimap: React.FC<MinimapProps> = ({
  players,
  localPlayerId,
  canvasWidth,
  canvasHeight,
  isMouseOver,
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
    ctx.fillStyle = isMouseOver ? MINIMAP_BG_COLOR_HOVER : MINIMAP_BG_COLOR_NORMAL;
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

export const drawMinimapOntoCanvas = (
  ctx: CanvasRenderingContext2D,
  players: Map<string, SpacetimeDBPlayer>,
  localPlayerId: string | undefined,
  canvasWidth: number,
  canvasHeight: number,
  isMouseOver: boolean
) => {
  // Calculate position and dimensions
  const mapX = (canvasWidth - MINIMAP_WIDTH) / 2;
  const mapY = (canvasHeight - MINIMAP_HEIGHT) / 2;
  const worldPixelWidth = gameConfig.worldWidth * gameConfig.tileSize;
  const worldPixelHeight = gameConfig.worldHeight * gameConfig.tileSize;
  const scaleX = MINIMAP_WIDTH / worldPixelWidth;
  const scaleY = MINIMAP_HEIGHT / worldPixelHeight;

  // Draw background (respecting hover state passed as prop)
  ctx.fillStyle = isMouseOver ? MINIMAP_BG_COLOR_HOVER : MINIMAP_BG_COLOR_NORMAL;
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
};


export const MINIMAP_DIMENSIONS = {
  width: MINIMAP_WIDTH,
  height: MINIMAP_HEIGHT,
};