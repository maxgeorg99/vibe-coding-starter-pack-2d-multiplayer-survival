// client/src/config/gameConfig.ts
// ------------------------------------
// Centralizes client-side configuration values primarily used for rendering.
// These values define how the game world *looks* on the client.
// The server maintains its own authoritative values for game logic and validation,
// so modifying these client-side values does not pose a security risk.
// ------------------------------------

export const gameConfig = {
  // Visual size of each grid tile in pixels.
  // Used for drawing the background grid and scaling visual elements.
  tileSize: 48,

  // Visual dimensions of the game world in tiles.
  // Used for rendering the background area and minimap calculations.
  worldWidth: 100,
  worldHeight: 100,

  // Intrinsic pixel dimensions of a single frame within player/entity spritesheets.
  // Essential for selecting and drawing the correct sprite visuals.
  spriteWidth: 48,
  spriteHeight: 48,
}; 