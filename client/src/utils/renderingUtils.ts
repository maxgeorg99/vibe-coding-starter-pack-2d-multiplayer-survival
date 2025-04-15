import { gameConfig } from '../config/gameConfig';
import { Player as SpacetimeDBPlayer } from '../generated';

// --- Constants --- 
const IDLE_FRAME_INDEX = 1; // Second frame is idle

// --- Helper Functions --- 

// Calculates sx, sy for the spritesheet
export const getSpriteCoordinates = (
  player: SpacetimeDBPlayer,
  isMoving: boolean,
  currentAnimationFrame: number
): { sx: number, sy: number } => {
  let spriteRow = 2; // Default Down
  switch (player.direction) {
    case 'up':    spriteRow = 0; break;
    case 'right': spriteRow = 1; break;
    case 'down':  spriteRow = 2; break;
    case 'left':  spriteRow = 3; break;
    default:      spriteRow = 2; break;
  }
  const frameIndex = isMoving ? currentAnimationFrame : IDLE_FRAME_INDEX;
  const sx = frameIndex * gameConfig.spriteWidth;
  const sy = spriteRow * gameConfig.spriteHeight;
  return { sx, sy };
};

// Checks if the mouse is hovering over the player
export const isPlayerHovered = (
  worldMouseX: number | null,
  worldMouseY: number | null,
  player: SpacetimeDBPlayer
): boolean => {
  if (worldMouseX === null || worldMouseY === null) return false;
  const hoverDX = worldMouseX - player.positionX;
  const hoverDY = worldMouseY - player.positionY;
  const distSq = hoverDX * hoverDX + hoverDY * hoverDY;
  return distSq < (gameConfig.playerRadius * gameConfig.playerRadius);
};

// Draws the styled name tag (Make exportable)
export const drawNameTag = (
  ctx: CanvasRenderingContext2D,
  player: SpacetimeDBPlayer,
  spriteTopY: number // dy from drawPlayer calculation
) => {
  ctx.font = '12px Arial';
  ctx.textAlign = 'center';
  const textWidth = ctx.measureText(player.username).width;
  const tagPadding = 4;
  const tagHeight = 16;
  const tagWidth = textWidth + tagPadding * 2;
  const tagX = player.positionX - tagWidth / 2;
  const tagY = spriteTopY - tagHeight - 2;

  ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
  ctx.beginPath();
  ctx.roundRect(tagX, tagY, tagWidth, tagHeight, 5);
  ctx.fill();

  ctx.fillStyle = '#FFFFFF';
  ctx.fillText(player.username, player.positionX, tagY + tagHeight / 2 + 4);
};

// Renders a complete player (sprite + conditional name tag)
export const renderPlayer = (
  ctx: CanvasRenderingContext2D,
  player: SpacetimeDBPlayer,
  heroImg: CanvasImageSource, // Use CanvasImageSource type
  isMoving: boolean,
  isHovered: boolean,
  currentAnimationFrame: number,
  jumpOffsetY: number = 0 // Default to 0 if not provided
) => {
  const { sx, sy } = getSpriteCoordinates(player, isMoving, currentAnimationFrame);
  
  const drawWidth = gameConfig.spriteWidth * 2;
  const drawHeight = gameConfig.spriteHeight * 2;
  const dx = player.positionX - drawWidth / 2;
  const dy = player.positionY - drawHeight / 2 - jumpOffsetY;

  // Draw the sprite
  ctx.drawImage(
    heroImg, 
    sx, sy, gameConfig.spriteWidth, gameConfig.spriteHeight, // Source
    dx, dy, drawWidth, drawHeight // Destination
  );

  // Draw name tag if hovered (position based on potentially offset dy)
  if (isHovered) {
    drawNameTag(ctx, player, dy);
  }
}; 