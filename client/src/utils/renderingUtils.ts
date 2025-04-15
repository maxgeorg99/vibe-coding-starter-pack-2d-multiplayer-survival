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

// Renders a complete player (sprite, shadow, and conditional name tag)
export const renderPlayer = (
  ctx: CanvasRenderingContext2D,
  player: SpacetimeDBPlayer,
  heroImg: CanvasImageSource,
  isMoving: boolean,
  isHovered: boolean,
  currentAnimationFrame: number,
  jumpOffsetY: number = 0
) => {
  const { sx, sy } = getSpriteCoordinates(player, isMoving, currentAnimationFrame);
  
  const drawWidth = gameConfig.spriteWidth * 2;
  const drawHeight = gameConfig.spriteHeight * 2;
  const spriteBaseX = player.positionX - drawWidth / 2;
  const spriteBaseY = player.positionY - drawHeight / 2;
  // The sprite's visual Y position, accounting for the jump
  const spriteDrawY = spriteBaseY - jumpOffsetY; 

  // --- Draw Shadow --- 
  const shadowBaseRadiusX = drawWidth * 0.3;
  const shadowBaseRadiusY = shadowBaseRadiusX * 0.4;
  const shadowMaxJumpOffset = 10; 
  const shadowYOffsetFromJump = jumpOffsetY * (shadowMaxJumpOffset / gameConfig.playerRadius); 
  // Add constant offset to place shadow below feet (approx half sprite height)
  const shadowBaseYOffset = drawHeight * 0.4; // Adjust this multiplier (0.4 to 0.5) as needed

  // Shadow gets smaller and fainter as player goes higher
  const jumpProgress = Math.min(1, jumpOffsetY / gameConfig.playerRadius); // 0 to 1 based on jump height
  const shadowScale = 1.0 - jumpProgress * 0.4; // Shrinks up to 40%
  const shadowOpacity = 0.5 - jumpProgress * 0.3; // Fades up to 60%

  ctx.fillStyle = `rgba(0, 0, 0, ${Math.max(0, shadowOpacity)})`;
  ctx.beginPath();
  ctx.ellipse(
    player.positionX, // Center X of shadow is player's X
    player.positionY + shadowBaseYOffset + shadowYOffsetFromJump, // Apply base offset AND jump offset
    shadowBaseRadiusX * shadowScale, 
    shadowBaseRadiusY * shadowScale, 
    0, 
    0, 
    Math.PI * 2 
  );
  ctx.fill();
  // --- End Draw Shadow ---

  // Draw the sprite (use the calculated spriteDrawY)
  ctx.drawImage(
    heroImg, 
    sx, sy, gameConfig.spriteWidth, gameConfig.spriteHeight, // Source
    spriteBaseX, spriteDrawY, drawWidth, drawHeight // Destination
  );

  // Draw name tag if hovered (position based on the sprite's visual top edge)
  if (isHovered) {
    drawNameTag(ctx, player, spriteDrawY);
  }
}; 