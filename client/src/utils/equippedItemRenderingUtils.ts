import { Player as SpacetimeDBPlayer, ActiveEquipment as SpacetimeDBActiveEquipment, ItemDefinition as SpacetimeDBItemDefinition } from '../generated';
import { gameConfig } from '../config/gameConfig';

// --- Constants (copied from GameCanvas for now, consider moving to config) ---
const SWING_DURATION_MS = 300;
const SWING_ANGLE_MAX_RAD = Math.PI / 2.5;

// --- Helper Function for Rendering Equipped Item ---
export const renderEquippedItem = (
  ctx: CanvasRenderingContext2D,
  player: SpacetimeDBPlayer, 
  equipment: SpacetimeDBActiveEquipment,
  itemDef: SpacetimeDBItemDefinition,
  itemImg: HTMLImageElement,
  now_ms: number,
  jumpOffset: number
) => {
  // --- Item Size and Position ---
  const scale = 0.05; // User's value
  const itemWidth = itemImg.width * scale;
  const itemHeight = itemImg.height * scale;
  let itemOffsetX = 0; 
  let itemOffsetY = 0; 
  let rotation = 0;

  let pivotX = player.positionX;
  let pivotY = player.positionY - jumpOffset; 
  
  const handOffsetX = gameConfig.spriteWidth * 0.2; 
  const handOffsetY = gameConfig.spriteHeight * 0.05;

  switch (player.direction) {
      case 'up': 
          itemOffsetX = -handOffsetX * -1.5;
          itemOffsetY = -handOffsetY * 2.0; 
          pivotX += itemOffsetX;
          pivotY += itemOffsetY; 
          break;
      case 'down': 
          itemOffsetX = handOffsetX * -2.5;
          itemOffsetY = handOffsetY * 1.5; 
          pivotX += itemOffsetX;
          pivotY += itemOffsetY; 
          break;
      case 'left': 
          itemOffsetX = -handOffsetX * 2.0; 
          itemOffsetY = handOffsetY;
          pivotX += itemOffsetX; 
          pivotY += itemOffsetY; 
          break;
      case 'right': 
          itemOffsetX = handOffsetX * 0.5; 
          itemOffsetY = handOffsetY;
          pivotX += itemOffsetX;
          pivotY += itemOffsetY; 
          break;
  }

  // --- Swing Animation --- 
  const swingStartTime = Number(equipment.swingStartTimeMs);
  const elapsedSwingTime = now_ms - swingStartTime;
  if (elapsedSwingTime < SWING_DURATION_MS) {
      const swingProgress = elapsedSwingTime / SWING_DURATION_MS;
      const currentAngle = Math.sin(swingProgress * Math.PI) * SWING_ANGLE_MAX_RAD;
      
      if (player.direction === 'right' || player.direction === 'up') {
        rotation = currentAngle; 
      } else {
        rotation = -currentAngle; 
      }
  }
  
  // Apply transformations
  ctx.save();
  ctx.translate(pivotX, pivotY);
  ctx.rotate(rotation);

  // Flip horizontally based on direction
  if (player.direction === 'right') { // Apply flip ONLY when facing right
      ctx.scale(-1, 1); 
  }

   // Flip horizontally based on direction
   if (player.direction === 'up') { // Apply flip ONLY when facing right
    ctx.scale(-1, 1); 
}

  
  // Draw image centered AT the pivot point
  ctx.drawImage(itemImg, -itemWidth / 2, -itemHeight / 2, itemWidth, itemHeight);
  ctx.restore();
}; 