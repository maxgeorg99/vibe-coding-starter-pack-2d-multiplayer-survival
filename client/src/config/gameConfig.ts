export const gameConfig = {
  // Tile settings
  tileSize: 48, // pixels
  
  // World dimensions (in tiles)
  worldWidth: 100,
  worldHeight: 100,
  
  // Player settings
  playerRadius: 24, // Radius for collision/hover checks (spriteWidth / 2)
  playerSpeed: 5, // pixels per frame
  spriteWidth: 48, // Intrinsic width of one sprite frame
  spriteHeight: 48, // Intrinsic height of one sprite frame
  
  // Viewport/Canvas size (fixed resolution)
  viewWidth: 800,  // Desired visible width in pixels
  viewHeight: 600, // Desired visible height in pixels
  
  // Game settings
  fps: 60,
}; 