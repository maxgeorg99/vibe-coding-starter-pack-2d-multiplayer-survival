import { gameConfig } from '../config/gameConfig';
import { Player as SpacetimeDBPlayer } from '../generated';

// --- Constants --- 
const IDLE_FRAME_INDEX = 1; // Second frame is idle
const PLAYER_SHAKE_DURATION_MS = 200; // How long the shake lasts
const PLAYER_SHAKE_AMOUNT_PX = 3;   // Max pixels to offset

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
  spriteTopY: number, // dy from drawPlayer calculation
  spriteX: number // Added new parameter for shaken X position
) => {
  ctx.font = '12px Arial';
  ctx.textAlign = 'center';
  const textWidth = ctx.measureText(player.username).width;
  const tagPadding = 4;
  const tagHeight = 16;
  const tagWidth = textWidth + tagPadding * 2;
  const tagX = spriteX - tagWidth / 2;
  const tagY = spriteTopY - tagHeight - 2;

  ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
  ctx.beginPath();
  ctx.roundRect(tagX, tagY, tagWidth, tagHeight, 5);
  ctx.fill();

  ctx.fillStyle = '#FFFFFF';
  ctx.fillText(player.username, spriteX, tagY + tagHeight / 2 + 4);
};

// Renders a complete player (sprite, shadow, and conditional name tag)
export const renderPlayer = (
  ctx: CanvasRenderingContext2D,
  player: SpacetimeDBPlayer,
  heroImg: CanvasImageSource,
  isMoving: boolean,
  isHovered: boolean,
  currentAnimationFrame: number,
  nowMs: number, // <-- Added current time in ms
  jumpOffsetY: number = 0
) => {
  const { sx, sy } = getSpriteCoordinates(player, isMoving, currentAnimationFrame);
  
  // --- Calculate Shake Offset (Only if alive) ---
  let shakeX = 0;
  let shakeY = 0;
  if (!player.isDead && player.lastHitTime) { // Check !player.isDead
    const lastHitMs = Number(player.lastHitTime.microsSinceUnixEpoch / 1000n);
    const elapsedSinceHit = nowMs - lastHitMs;
    if (elapsedSinceHit >= 0 && elapsedSinceHit < PLAYER_SHAKE_DURATION_MS) {
      shakeX = (Math.random() - 0.5) * 2 * PLAYER_SHAKE_AMOUNT_PX;
      shakeY = (Math.random() - 0.5) * 2 * PLAYER_SHAKE_AMOUNT_PX;
    }
  }
  // --- End Shake Offset ---

  const drawWidth = gameConfig.spriteWidth * 2;
  const drawHeight = gameConfig.spriteHeight * 2;
  const spriteBaseX = player.positionX - drawWidth / 2 + shakeX; // Includes shake if applicable
  const spriteBaseY = player.positionY - drawHeight / 2 + shakeY; // Includes shake if applicable
  const spriteDrawY = spriteBaseY - jumpOffsetY; 

  // --- Draw Shadow (Only if alive) --- 
  if (!player.isDead) { // Check !player.isDead
      const shadowBaseRadiusX = drawWidth * 0.3;
      const shadowBaseRadiusY = shadowBaseRadiusX * 0.4;
      const shadowMaxJumpOffset = 10; 
      const shadowYOffsetFromJump = jumpOffsetY * (shadowMaxJumpOffset / gameConfig.playerRadius); 
      const shadowBaseYOffset = drawHeight * 0.4; 
      const jumpProgress = Math.min(1, jumpOffsetY / gameConfig.playerRadius); 
      const shadowScale = 1.0 - jumpProgress * 0.4; 
      const shadowOpacity = 0.5 - jumpProgress * 0.3; 

      ctx.fillStyle = `rgba(0, 0, 0, ${Math.max(0, shadowOpacity)})`;
      ctx.beginPath();
      ctx.ellipse(
        player.positionX, // Use original player X for shadow center
        player.positionY + shadowBaseYOffset + shadowYOffsetFromJump, // Use original player Y for shadow center
        shadowBaseRadiusX * shadowScale, 
        shadowBaseRadiusY * shadowScale, 
        0, 
        0, 
        Math.PI * 2 
      );
      ctx.fill();
  }
  // --- End Draw Shadow ---

  // --- Draw Sprite --- 
  ctx.save(); // Save context state before potential rotation
  try {
    // Calculate center point for rotation (based on shaken position if alive)
    const centerX = spriteBaseX + drawWidth / 2;
    const centerY = spriteDrawY + drawHeight / 2;

    if (player.isDead) {
      // Determine rotation angle based on direction
      let rotationAngleRad = 0;
      switch (player.direction) {
        case 'up':    // Facing up, rotate CCW
        case 'right': // Facing right, rotate CCW
          rotationAngleRad = -Math.PI / 2; // -90 degrees
          break;
        case 'down':  // Facing down, rotate CW
        case 'left':  // Facing left, rotate CW
        default:
          rotationAngleRad = Math.PI / 2; // +90 degrees
          break;
      }
      
      // Translate origin to sprite center, rotate, translate back
      ctx.translate(centerX, centerY);
      ctx.rotate(rotationAngleRad);
      ctx.translate(-centerX, -centerY);
    }

    // Draw the sprite (respecting rotation if applied)
    ctx.drawImage(
      heroImg, 
      sx, sy, gameConfig.spriteWidth, gameConfig.spriteHeight, // Source
      spriteBaseX, spriteDrawY, drawWidth, drawHeight // Destination (includes shake if alive)
    );

  } finally {
      ctx.restore(); // Restore context state (removes rotation)
  }
  // --- End Draw Sprite ---

  // Draw name tag if hovered AND alive
  if (isHovered && !player.isDead) { // Check !player.isDead
    drawNameTag(ctx, player, spriteDrawY, spriteBaseX + drawWidth / 2);
  }
}; 