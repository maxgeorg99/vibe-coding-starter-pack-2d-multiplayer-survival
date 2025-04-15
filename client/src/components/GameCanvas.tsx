import React, { useEffect, useRef, useCallback, useState } from 'react';
import { gameConfig } from '../config/gameConfig';
import { Player as SpacetimeDBPlayer } from '../generated';
import heroSpriteSheet from '../assets/hero.png';
import grassTexture from '../assets/tiles/grass.png';
// Import helpers and hook
import { useAnimationCycle } from '../hooks/useAnimationCycle';
import { isPlayerHovered, renderPlayer, getSpriteCoordinates } from '../utils/renderingUtils';
// Import Minimap drawing logic and dimensions
import { drawMinimapOntoCanvas, MINIMAP_DIMENSIONS } from './Minimap';

// Threshold for movement animation (position delta)
const MOVEMENT_POSITION_THRESHOLD = 0.1; // Small threshold to account for float precision
const ANIMATION_INTERVAL_MS = 150;

// --- Jump Constants ---
const JUMP_DURATION_MS = 400; // Total duration of the jump animation
const JUMP_HEIGHT_PX = 40; // Maximum height the player reaches

interface GameCanvasProps {
  players: Map<string, SpacetimeDBPlayer>;
  localPlayerId?: string;
  updatePlayerPosition: (dx: number, dy: number) => void;
  callJumpReducer: () => void;
  callSetSprintingReducer: (isSprinting: boolean) => void;
}

const GameCanvas: React.FC<GameCanvasProps> = ({ 
  players, 
  localPlayerId,
  updatePlayerPosition,
  callJumpReducer,
  callSetSprintingReducer
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
  const isSprintingRef = useRef<boolean>(false); // Track current sprint state
  const lastPositionsRef = useRef<Map<string, {x: number, y: number}>>(new Map()); // Store last known positions
  
  const animationFrame = useAnimationCycle(ANIMATION_INTERVAL_MS, 4);

  const getLocalPlayer = useCallback((): SpacetimeDBPlayer | undefined => {
    if (!localPlayerId) return undefined;
    return players.get(localPlayerId);
  }, [players, localPlayerId]);
  
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const key = e.key.toLowerCase();
      // Avoid processing modifier keys if they are the only key pressed initially
      if (key === 'shift' || key === 'control' || key === 'alt' || key === 'meta') {
        // Handle Shift for sprinting start
        if (key === 'shift' && !isSprintingRef.current && !e.repeat) {
            isSprintingRef.current = true;
            callSetSprintingReducer(true);
        }
        // Don't add modifier keys themselves to keysPressed
        return; 
      }

      keysPressed.current.add(key);

      // Handle jump on Spacebar press
      if (key === ' ' && !e.repeat) {
        const localPlayer = getLocalPlayer();
        if (localPlayer) {
          callJumpReducer(); 
        }
      }

      // Toggle minimap on 'g' press
      if (key === 'g' && !e.repeat) { 
        setIsMinimapOpen(prev => !prev);
      }
    };
    
    const handleKeyUp = (e: KeyboardEvent) => {
      const key = e.key.toLowerCase();
      // Handle Shift key release for sprinting end
      if (key === 'shift') {
        if (isSprintingRef.current) { // Only call if we were sprinting
            isSprintingRef.current = false;
            callSetSprintingReducer(false);
        }
      }
      keysPressed.current.delete(key);
    };
    
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    
    // Cleanup function to handle edge case: window losing focus while shift is held
    const handleBlur = () => {
        if (isSprintingRef.current) {
            isSprintingRef.current = false;
            callSetSprintingReducer(false);
        }
        keysPressed.current.clear(); // Clear all keys on blur
    };
    window.addEventListener('blur', handleBlur);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
      window.removeEventListener('blur', handleBlur);
    };
  }, [getLocalPlayer, callJumpReducer, callSetSprintingReducer]);
  
  const updatePlayerBasedOnInput = useCallback(() => {
    const localPlayer = getLocalPlayer();
    if (!localPlayer) return;
    
    let dx = 0;
    let dy = 0;
    const speed = gameConfig.playerSpeed; // Use base speed
    
    // Handle WASD movement to calculate deltas
    if (keysPressed.current.has('w') || keysPressed.current.has('arrowup')) {
      dy -= speed;
    }
    if (keysPressed.current.has('s') || keysPressed.current.has('arrowdown')) {
      dy += speed;
    }
    if (keysPressed.current.has('a') || keysPressed.current.has('arrowleft')) {
      dx -= speed;
    }
    if (keysPressed.current.has('d') || keysPressed.current.has('arrowright')) {
      dx += speed;
    }
    
    // Normalize diagonal movement (optional but good practice)
    if (dx !== 0 && dy !== 0) {
        const magnitude = Math.sqrt(dx * dx + dy * dy);
        dx = (dx / magnitude) * speed;
        dy = (dy / magnitude) * speed;
    }

    // Always call updatePlayerPosition to allow server to process passive changes (like stamina regen)
    updatePlayerPosition(dx, dy);
    // Client no longer calculates absolute position or checks boundaries

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

  // Updated drawPlayers to use position check for animation
  const drawPlayersWithJump = useCallback((ctx: CanvasRenderingContext2D, currentMousePos: {x: number | null, y: number | null}) => {
    const heroImg = heroImageRef.current;
    if (!heroImg) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const localPlayerData = getLocalPlayer();

    let worldMouseX: number | null = null;
    let worldMouseY: number | null = null;
    // Use state for canvas dimensions when calculating world mouse position
    if (currentMousePos.x !== null && currentMousePos.y !== null && localPlayerData) {
        const cameraOffsetX = canvasSize.width / 2 - localPlayerData.positionX;
        const cameraOffsetY = canvasSize.height / 2 - localPlayerData.positionY;
        worldMouseX = currentMousePos.x - cameraOffsetX;
        worldMouseY = currentMousePos.y - cameraOffsetY;
    }

    const now_ms = Date.now(); 

    players.forEach(player => {
      const playerId = player.identity.toHexString();
      const lastPos = lastPositionsRef.current.get(playerId);
      let isPlayerMoving = false;

      if (lastPos) {
        const dx = Math.abs(player.positionX - lastPos.x);
        const dy = Math.abs(player.positionY - lastPos.y);
        // Check if position changed significantly
        if (dx > MOVEMENT_POSITION_THRESHOLD || dy > MOVEMENT_POSITION_THRESHOLD) {
          isPlayerMoving = true;
        }
      } else {
        // If no last position, assume not moving initially for animation purposes
        isPlayerMoving = false; 
      }

      // Update last known position for next frame
      lastPositionsRef.current.set(playerId, { x: player.positionX, y: player.positionY });

      // --- Jump Calculation (remains the same) ---
      let jumpOffset = 0;
      const jumpStartTime = player.jumpStartTimeMs;
      if (jumpStartTime > 0) {
          const elapsedJumpTime = now_ms - Number(jumpStartTime); 
          if (elapsedJumpTime < JUMP_DURATION_MS) {
              const d = JUMP_DURATION_MS;
              const h = JUMP_HEIGHT_PX;
              const x = elapsedJumpTime;
              jumpOffset = (-4 * h / (d * d)) * x * (x - d);
          } 
      }
      // --- End Jump Calculation ---

      // Check hover state
      const hovered = isPlayerHovered(worldMouseX, worldMouseY, player);
      
      // Call the unified renderPlayer function
      // We pass isPlayerMoving determined by position change
      renderPlayer(ctx, player, heroImg, isPlayerMoving, hovered, animationFrame, jumpOffset);
    });
  }, [players, animationFrame, getLocalPlayer, canvasSize.width, canvasSize.height]);

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
    drawPlayersWithJump(ctx, currentMousePos); 
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
  }, [getLocalPlayer, drawWorldBackground, drawPlayersWithJump, isMinimapOpen, players, localPlayerId, isMouseOverMinimap, canvasSize.width, canvasSize.height]); // Use drawPlayersWithJump

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