export const gameConfig = {
  // Tile settings
  tileSize: 48, // pixels
  
  // World dimensions (in tiles)
  worldWidth: 20,
  worldHeight: 15,
  
  // Player settings
  playerRadius: 24, // Radius for collision/hover checks (spriteWidth / 2)
  playerSpeed: 5, // pixels per frame
  spriteWidth: 48, // Intrinsic width of one sprite frame
  spriteHeight: 48, // Intrinsic height of one sprite frame
  
  // Game settings
  fps: 60,
  
  // Canvas will be calculated based on tile size and world dimensions
  get canvasWidth() {
    return this.tileSize * this.worldWidth;
  },
  
  get canvasHeight() {
    return this.tileSize * this.worldHeight;
  }
}; 