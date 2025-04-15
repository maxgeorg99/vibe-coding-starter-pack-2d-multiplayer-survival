import React, { useEffect, useRef } from 'react';
import { gameConfig } from '../config/gameConfig';
import { Player as SpacetimeDBPlayer } from '../generated';

interface GameCanvasProps {
  players: Map<string, SpacetimeDBPlayer>;
  localPlayerId?: string;
  updatePlayerPosition: (x: number, y: number) => void;
}

const GameCanvas: React.FC<GameCanvasProps> = ({ 
  players, 
  localPlayerId,
  updatePlayerPosition 
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const keysPressed = useRef<Set<string>>(new Set());
  const requestIdRef = useRef<number>(0);
  
  // Get the local player
  const getLocalPlayer = (): SpacetimeDBPlayer | undefined => {
    if (!localPlayerId) return undefined;
    return players.get(localPlayerId);
  };
  
  // Set up keyboard event listeners
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      keysPressed.current.add(e.key.toLowerCase());
    };
    
    const handleKeyUp = (e: KeyboardEvent) => {
      keysPressed.current.delete(e.key.toLowerCase());
    };
    
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, []);
  
  // Update local player position based on keys pressed
  const updatePlayerBasedOnInput = () => {
    const localPlayer = getLocalPlayer();
    if (!localPlayer) return;
    
    let dx = 0;
    let dy = 0;
    
    // Handle WASD movement
    if (keysPressed.current.has('w') || keysPressed.current.has('arrowup')) {
      dy -= gameConfig.playerSpeed;
    }
    
    if (keysPressed.current.has('s') || keysPressed.current.has('arrowdown')) {
      dy += gameConfig.playerSpeed;
    }
    
    if (keysPressed.current.has('a') || keysPressed.current.has('arrowleft')) {
      dx -= gameConfig.playerSpeed;
    }
    
    if (keysPressed.current.has('d') || keysPressed.current.has('arrowright')) {
      dx += gameConfig.playerSpeed;
    }
    
    // Only update if there's movement
    if (dx !== 0 || dy !== 0) {
      // Calculate new position
      const newX = Math.max(
        gameConfig.playerRadius, 
        Math.min(
          gameConfig.canvasWidth - gameConfig.playerRadius, 
          localPlayer.positionX + dx
        )
      );
      
      const newY = Math.max(
        gameConfig.playerRadius, 
        Math.min(
          gameConfig.canvasHeight - gameConfig.playerRadius, 
          localPlayer.positionY + dy
        )
      );
      
      // Update position through reducer
      updatePlayerPosition(newX, newY);
    }
  };
  
  // Game loop
  const gameLoop = () => {
    // Get local player position *before* updating based on input
    // This helps keep the camera centered on the position sent to the server
    const localPlayer = getLocalPlayer(); 
    updatePlayerBasedOnInput();
    renderGame(localPlayer); // Pass local player to renderGame
    requestIdRef.current = requestAnimationFrame(gameLoop);
  };
  
  // Draw the game, centered on the local player
  const renderGame = (localPlayer: SpacetimeDBPlayer | undefined) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    
    // --- Pre-Camera Rendering --- 
    // Fill entire canvas with black background (area outside game world)
    ctx.fillStyle = '#000000';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Save the default state (before camera translation)
    ctx.save();

    // Calculate camera translation
    const cameraX = canvas.width / 2;
    const cameraY = canvas.height / 2;
    if (localPlayer) {
      ctx.translate(cameraX - localPlayer.positionX, cameraY - localPlayer.positionY);
    }
    
    // --- Post-Camera Rendering (World Space) --- 

    // Draw the green game world background
    ctx.fillStyle = '#8FBC8F'; // Soft grass green (DarkSeaGreen)
    ctx.fillRect(
      0, // World origin X
      0, // World origin Y
      gameConfig.worldWidth * gameConfig.tileSize, // World width in pixels
      gameConfig.worldHeight * gameConfig.tileSize // World height in pixels
    );
    
    // Draw grid (on top of green background)
    drawGrid(ctx);
    
    // Draw players (on top of green background)
    drawPlayers(ctx);

    // Restore the default state (removes the translation)
    ctx.restore();
  };
  
  // Draw the grid
  const drawGrid = (ctx: CanvasRenderingContext2D) => {
    ctx.strokeStyle = '#ddd';
    ctx.lineWidth = 1;
    
    // Draw vertical lines
    for (let x = 0; x <= gameConfig.worldWidth; x++) {
      ctx.beginPath();
      ctx.moveTo(x * gameConfig.tileSize, 0);
      ctx.lineTo(x * gameConfig.tileSize, gameConfig.canvasHeight);
      ctx.stroke();
    }
    
    // Draw horizontal lines
    for (let y = 0; y <= gameConfig.worldHeight; y++) {
      ctx.beginPath();
      ctx.moveTo(0, y * gameConfig.tileSize);
      ctx.lineTo(gameConfig.canvasWidth, y * gameConfig.tileSize);
      ctx.stroke();
    }
  };
  
  // Draw all players
  const drawPlayers = (ctx: CanvasRenderingContext2D) => {
    // Log the players map before drawing
    console.log('Drawing players:', players); 
    players.forEach(player => {
      // Draw player circle
      ctx.beginPath();
      ctx.arc(
        player.positionX,
        player.positionY,
        gameConfig.playerRadius,
        0,
        Math.PI * 2
      );
      ctx.fillStyle = player.color;
      ctx.fill();
      
      // Draw player name
      ctx.fillStyle = '#000';
      ctx.textAlign = 'center';
      ctx.font = '12px Arial';
      ctx.fillText(
        player.username,
        player.positionX,
        player.positionY - gameConfig.playerRadius - 5
      );
      
      // Highlight local player
      if (localPlayerId && player.identity.toHexString() === localPlayerId) {
        ctx.strokeStyle = '#000';
        ctx.lineWidth = 2;
        ctx.stroke();
      }
    });
  };
  
  // Effect for game loop
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    // Start the game loop
    requestIdRef.current = requestAnimationFrame(gameLoop);
    
    // Clean up game loop
    return () => {
      cancelAnimationFrame(requestIdRef.current);
    };
  }, [players, localPlayerId]); // Original dependencies
  
  return (
    <canvas
      ref={canvasRef}
      // Restore static width/height based on game config
      width={gameConfig.canvasWidth} 
      height={gameConfig.canvasHeight}
    />
  );
};

export default GameCanvas; 