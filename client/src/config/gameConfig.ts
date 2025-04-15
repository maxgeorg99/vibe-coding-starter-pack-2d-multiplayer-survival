export const gameConfig = {
  // Tile settings
  tileSize: 48, // pixels
  
  // World dimensions (in tiles)
  worldWidth: 20,
  worldHeight: 15,
  
  // Player settings
  playerRadius: 20,
  playerSpeed: 5, // pixels per frame
  
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