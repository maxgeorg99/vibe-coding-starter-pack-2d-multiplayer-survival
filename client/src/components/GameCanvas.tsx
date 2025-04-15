import React, { useEffect, useRef, useState } from 'react';
import { gameConfig } from '../config/gameConfig';
import { Player as SpacetimeDBPlayer } from '../generated';
import { Identity as SpacetimeDBIdentity, Timestamp as SpacetimeDBTimestamp } from '@clockworklabs/spacetimedb-sdk';
import heroSpriteSheet from '../assets/hero.png';

// Define sprite dimensions (adjust if needed)
const SPRITE_WIDTH = 48;
const SPRITE_HEIGHT = 48;
const PLAYER_RADIUS = 24; // Match sprite size / 2 (or use value from gameConfig if defined there)

// Threshold for considering a player "recently moved" (in milliseconds)
const MOVEMENT_IDLE_THRESHOLD_MS = 200;

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
  const heroImageRef = useRef<HTMLImageElement | null>(null);
  const [animationFrame, setAnimationFrame] = useState(0);
  const animationIntervalRef = useRef<number | null>(null);
  // Use a ref for mouse position to avoid state updates on move
  const mousePosRef = useRef<{x: number | null, y: number | null}>({ x: null, y: null }); 
  
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
  
  // Load the spritesheet image
  useEffect(() => { 
    const img = new Image();
    img.src = heroSpriteSheet;
    img.onload = () => {
      heroImageRef.current = img;
      console.log('Hero spritesheet loaded.');
    };
    img.onerror = () => {
      console.error('Failed to load hero spritesheet.');
    };
  }, []); // Runs once

  // Effect for setting up the animation timer
  useEffect(() => {
    // Always run animation timer
    animationIntervalRef.current = window.setInterval(() => {
      setAnimationFrame(frame => (frame + 1) % 4); // Cycle through 4 frames
    }, 150); // Adjust animation speed (ms)

    // Cleanup timer
    return () => { 
      if (animationIntervalRef.current) {
        clearInterval(animationIntervalRef.current);
      }
    };
  }, []); // Runs once to set up timer
  
  // Effect for Mouse Listeners (runs once)
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const handleMouseMove = (event: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      const scaleX = canvas.width / rect.width;
      const scaleY = canvas.height / rect.height;
      // Update ref directly
      mousePosRef.current = {
        x: (event.clientX - rect.left) * scaleX,
        y: (event.clientY - rect.top) * scaleY
      };
    };
    canvas.addEventListener('mousemove', handleMouseMove);

    const handleMouseLeave = () => {
      // Update ref directly
      mousePosRef.current = { x: null, y: null };
    };
    canvas.addEventListener('mouseleave', handleMouseLeave);

    // Cleanup listeners
    return () => {
      canvas.removeEventListener('mousemove', handleMouseMove);
      canvas.removeEventListener('mouseleave', handleMouseLeave);
    };
  }, []); // Empty dependency array - runs once

  // Game loop
  const gameLoop = () => {
    updatePlayerBasedOnInput();
    // Read current mouse position from ref
    renderGame(mousePosRef.current); 
    requestIdRef.current = requestAnimationFrame(gameLoop);
  };
  
  // Draw the game (accepts mouse position object)
  const renderGame = (currentMousePos: {x: number | null, y: number | null}) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    
    const localPlayerData = localPlayerId ? players.get(localPlayerId) : undefined;

    ctx.fillStyle = '#000000';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.save();

    const cameraX = canvas.width / 2;
    const cameraY = canvas.height / 2;
    if (localPlayerData) {
      ctx.translate(cameraX - localPlayerData.positionX, cameraY - localPlayerData.positionY);
    }
    
    ctx.fillStyle = '#8FBC8F'; 
    ctx.fillRect(0, 0, gameConfig.worldWidth * gameConfig.tileSize, gameConfig.worldHeight * gameConfig.tileSize );
    drawGrid(ctx);
    drawPlayers(ctx, currentMousePos); 
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
  
  // Draw all players (accepts mouse position object)
  const drawPlayers = (ctx: CanvasRenderingContext2D, currentMousePos: {x: number | null, y: number | null}) => {
    const img = heroImageRef.current;
    if (!img) return;
    const canvas = canvasRef.current;
    if (!canvas) return;

    const localPlayerData = localPlayerId ? players.get(localPlayerId) : undefined;

    let worldMouseX: number | null = null;
    let worldMouseY: number | null = null;
    if (currentMousePos.x !== null && currentMousePos.y !== null && localPlayerData) {
      const cameraOffsetX = canvas.width / 2 - localPlayerData.positionX;
      const cameraOffsetY = canvas.height / 2 - localPlayerData.positionY;
      worldMouseX = currentMousePos.x - cameraOffsetX;
      worldMouseY = currentMousePos.y - cameraOffsetY;
    }

    const now_ms = Date.now(); 

    players.forEach(player => {
      let spriteRow = 2; 
      switch (player.direction) {
        case 'up':    spriteRow = 0; break; 
        case 'right': spriteRow = 1; break; 
        case 'down':  spriteRow = 2; break; 
        case 'left':  spriteRow = 3; break; 
        default:      spriteRow = 2; break; 
      }

      const playerLastUpdate_micros = player.lastUpdate.__timestamp_micros_since_unix_epoch__;
      const playerLastUpdate_ms = Number(playerLastUpdate_micros / 1000n); 
      const isPlayerMoving = (now_ms - playerLastUpdate_ms) < MOVEMENT_IDLE_THRESHOLD_MS;
      
      const frameIndex = isPlayerMoving ? animationFrame : 1; 
      const sx = frameIndex * SPRITE_WIDTH;
      const sy = spriteRow * SPRITE_HEIGHT;

      const dx = player.positionX - SPRITE_WIDTH / 2;
      const dy = player.positionY - SPRITE_HEIGHT / 2;

      ctx.drawImage(img, sx, sy, SPRITE_WIDTH, SPRITE_HEIGHT, dx, dy, SPRITE_WIDTH, SPRITE_HEIGHT );
      
      let isHovering = false;
      if (worldMouseX !== null && worldMouseY !== null) {
        const hoverDX = worldMouseX - player.positionX;
        const hoverDY = worldMouseY - player.positionY;
        const distSq = hoverDX * hoverDX + hoverDY * hoverDY;
        if (distSq < (PLAYER_RADIUS * PLAYER_RADIUS)) {
          isHovering = true;
        }
      }

      if (isHovering) {
        ctx.font = '12px Arial';
        ctx.textAlign = 'center';
        const textWidth = ctx.measureText(player.username).width;
        const tagPadding = 4;
        const tagHeight = 16;
        const tagWidth = textWidth + tagPadding * 2;
        const tagX = player.positionX - tagWidth / 2; 
        const tagY = dy - tagHeight - 2; 

        ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
        ctx.beginPath();
        ctx.roundRect(tagX, tagY, tagWidth, tagHeight, 5);
        ctx.fill();

        ctx.fillStyle = '#FFFFFF';
        ctx.fillText(player.username, player.positionX, tagY + tagHeight / 2 + 4);
      }
    });
  };

  // useEffect starting game loop (Restore dependencies)
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    // Start the game loop
    requestIdRef.current = requestAnimationFrame(gameLoop);
    
    // Clean up
    return () => {
      cancelAnimationFrame(requestIdRef.current);
    };
  // Restore players/localPlayerId dependency 
  }, [players, localPlayerId]); 

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