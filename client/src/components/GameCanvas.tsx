import React, { useEffect, useRef, useCallback, useState } from 'react';
import { gameConfig } from '../config/gameConfig';
import { Player as SpacetimeDBPlayer } from '../generated';
import heroSpriteSheet from '../assets/hero.png';
import grassTexture from '../assets/tiles/grass.png';
// Import helpers and hook
import { useAnimationCycle } from '../hooks/useAnimationCycle';
import { isPlayerHovered, renderPlayer } from '../utils/renderingUtils';
// Import Minimap drawing logic and dimensions
import { drawMinimapOntoCanvas, MINIMAP_DIMENSIONS } from './Minimap';

// Threshold for considering a player "recently moved" (in milliseconds)
const MOVEMENT_IDLE_THRESHOLD_MS = 200;
const ANIMATION_INTERVAL_MS = 150;

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
  const [canvasSize, setCanvasSize] = useState({ width: window.innerWidth, height: window.innerHeight });
  const keysPressed = useRef<Set<string>>(new Set());
  const requestIdRef = useRef<number>(0);
  const heroImageRef = useRef<HTMLImageElement | null>(null);
  const grassImageRef = useRef<HTMLImageElement | null>(null);
  const mousePosRef = useRef<{x: number | null, y: number | null}>({ x: null, y: null });
  const [isMinimapOpen, setIsMinimapOpen] = useState(false); // State for minimap visibility
  const [isMouseOverMinimap, setIsMouseOverMinimap] = useState(false); // State for minimap hover
  
  const animationFrame = useAnimationCycle(ANIMATION_INTERVAL_MS, 4);

  const getLocalPlayer = useCallback((): SpacetimeDBPlayer | undefined => {
    if (!localPlayerId) return undefined;
    return players.get(localPlayerId);
  }, [players, localPlayerId]);
  
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      keysPressed.current.add(e.key.toLowerCase());
      // Toggle minimap on 'g' press
      if (e.key.toLowerCase() === 'g' && !e.repeat) { // Prevent toggle spam on hold
        setIsMinimapOpen(prev => !prev);
      }
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
  
  const updatePlayerBasedOnInput = useCallback(() => {
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
      // Use world dimensions for boundary checks
      const worldPixelWidth = gameConfig.worldWidth * gameConfig.tileSize;
      const worldPixelHeight = gameConfig.worldHeight * gameConfig.tileSize;
      const newX = Math.max(
        gameConfig.playerRadius, 
        Math.min(
          worldPixelWidth - gameConfig.playerRadius, // World boundary
          localPlayer.positionX + dx
        )
      );
      const newY = Math.max(
        gameConfig.playerRadius, 
        Math.min(
          worldPixelHeight - gameConfig.playerRadius, // World boundary
          localPlayer.positionY + dy
        )
      );
      updatePlayerPosition(newX, newY);
    }
  }, [getLocalPlayer, updatePlayerPosition]);
  
  useEffect(() => { 
    // Remove pattern logic
    // Load Hero
    const heroImg = new Image();
    heroImg.src = heroSpriteSheet;
    heroImg.onload = () => {
      heroImageRef.current = heroImg;
      console.log('Hero spritesheet loaded.');
    };
    heroImg.onerror = () => console.error('Failed to load hero spritesheet.');

    // Load Grass
    const grassImg = new Image();
    grassImg.src = grassTexture;
    grassImg.onload = () => {
       grassImageRef.current = grassImg; // Store image in ref
       console.log('Grass texture loaded.');
    };
    grassImg.onerror = () => console.error('Failed to load grass texture.');

  }, []); // Runs once

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

      // Check if mouse is over the minimap (only if open)
      if (isMinimapOpen) {
        // Use imported dimensions and current canvas size state
        const currentCanvasWidth = canvasSize.width;
        const currentCanvasHeight = canvasSize.height;
        const minimapX = (currentCanvasWidth - MINIMAP_DIMENSIONS.width) / 2;
        const minimapY = (currentCanvasHeight - MINIMAP_DIMENSIONS.height) / 2;
        const mouseX = mousePosRef.current.x;
        const mouseY = mousePosRef.current.y;

        if (mouseX !== null && mouseY !== null &&
            mouseX >= minimapX && mouseX <= minimapX + MINIMAP_DIMENSIONS.width &&
            mouseY >= minimapY && mouseY <= minimapY + MINIMAP_DIMENSIONS.height) {
          setIsMouseOverMinimap(true);
        } else {
          setIsMouseOverMinimap(false);
        }
      } else {
        setIsMouseOverMinimap(false); // Ensure it's false if minimap is closed
      }
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
  }, [isMinimapOpen, canvasSize.width, canvasSize.height]); // Add canvas size dependencies

  const drawWorldBackground = useCallback((ctx: CanvasRenderingContext2D) => {
    const grassImg = grassImageRef.current;
    if (!grassImg) { // Draw fallback if image not loaded
      ctx.fillStyle = '#8FBC8F';
      ctx.fillRect(0, 0, gameConfig.worldWidth * gameConfig.tileSize, gameConfig.worldHeight * gameConfig.tileSize );
      return; 
    }

    // You can change this value to true if you want to enable grid lines again.
    const drawGridLines = false; // Keep grid lines off

    // --- Potential Fix: Draw slightly larger tiles ---
    const overlap = 1; // Overlap by 1 pixel

    for (let y = 0; y < gameConfig.worldHeight; y++) {
      for (let x = 0; x < gameConfig.worldWidth; x++) {
        // Draw image slightly larger to cover potential gaps
        ctx.drawImage(
          grassImg,
          x * gameConfig.tileSize,
          y * gameConfig.tileSize,
          gameConfig.tileSize + overlap, // Draw wider
          gameConfig.tileSize + overlap  // Draw taller
        );

        // Original grid line drawing (remains unchanged)
        if (drawGridLines) {
          ctx.strokeStyle = 'rgba(221, 221, 221, 0.5)';
          ctx.lineWidth = 1;
          ctx.strokeRect(x * gameConfig.tileSize, y * gameConfig.tileSize, gameConfig.tileSize, gameConfig.tileSize);
        }
      }
    }
  }, []);

  const drawPlayers = useCallback((ctx: CanvasRenderingContext2D, currentMousePos: {x: number | null, y: number | null}) => {
    const heroImg = heroImageRef.current;
    if (!heroImg) return; // Need image loaded to pass to renderPlayer
    const canvas = canvasRef.current;
    if (!canvas) return;
    const localPlayerData = getLocalPlayer();
    
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
      // Determine movement state
      const playerLastUpdate_micros = player.lastUpdate.__timestamp_micros_since_unix_epoch__;
      const playerLastUpdate_ms = Number(playerLastUpdate_micros / 1000n); 
      const isPlayerMoving = (now_ms - playerLastUpdate_ms) < MOVEMENT_IDLE_THRESHOLD_MS;
      
      // Check hover state
      const hovered = isPlayerHovered(worldMouseX, worldMouseY, player); // Use util
      
      // Call the unified renderPlayer function
      renderPlayer(ctx, player, heroImg, isPlayerMoving, hovered, animationFrame); 
    });
  }, [players, animationFrame, getLocalPlayer]);

  const renderGame = useCallback((currentMousePos: {x: number | null, y: number | null}) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    
    const localPlayerData = getLocalPlayer();

    // Use state for canvas dimensions
    const currentCanvasWidth = canvasSize.width;
    const currentCanvasHeight = canvasSize.height;

    ctx.fillStyle = '#000000';
    ctx.fillRect(0, 0, currentCanvasWidth, currentCanvasHeight);
    ctx.save();

    // Use state for camera centering
    const cameraX = currentCanvasWidth / 2;
    const cameraY = currentCanvasHeight / 2;
    if (localPlayerData) {
      ctx.translate(cameraX - localPlayerData.positionX, cameraY - localPlayerData.positionY);
    }
    
    drawWorldBackground(ctx);
    drawPlayers(ctx, currentMousePos); 
    ctx.restore();

    // Draw minimap if open (drawn in screen space, after restoring transform)
    if (isMinimapOpen) {
      // Call the imported drawing function
      drawMinimapOntoCanvas(
        ctx,
        players,
        localPlayerId,
        currentCanvasWidth,  // Pass state dimensions
        currentCanvasHeight, // Pass state dimensions
        isMouseOverMinimap
      );
    }
  }, [getLocalPlayer, drawWorldBackground, drawPlayers, isMinimapOpen, players, localPlayerId, isMouseOverMinimap, canvasSize.width, canvasSize.height]); // Added canvas size dependencies

  const gameLoop = useCallback(() => {
    updatePlayerBasedOnInput();
    renderGame(mousePosRef.current); 
    requestIdRef.current = requestAnimationFrame(gameLoop);
  }, [updatePlayerBasedOnInput, renderGame]);

  useEffect(() => {
    requestIdRef.current = requestAnimationFrame(gameLoop);
    return () => {
      cancelAnimationFrame(requestIdRef.current);
    };
  }, [gameLoop]); 

  // Effect to handle window resizing
  useEffect(() => {
    const handleResize = () => {
      setCanvasSize({ width: window.innerWidth, height: window.innerHeight });
    };

    window.addEventListener('resize', handleResize);
    handleResize(); // Initial size

    return () => window.removeEventListener('resize', handleResize);
  }, []);

  return (
    <canvas
      ref={canvasRef}
      width={canvasSize.width} 
      height={canvasSize.height}
    />
  );
};

export default GameCanvas; 