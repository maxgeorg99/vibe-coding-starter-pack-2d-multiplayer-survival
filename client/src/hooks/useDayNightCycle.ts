import { useState, useEffect, useRef, useMemo } from 'react';
import { WorldState as SpacetimeDBWorldState, Campfire as SpacetimeDBCampfire } from '../generated';
import { interpolateRgba, getDynamicKeyframes } from '../utils/colorUtils';
import {
    FULL_MOON_CYCLE_INTERVAL,
    CAMPFIRE_LIGHT_RADIUS_BASE,
} from '../config/gameConfig';

interface UseDayNightCycleProps {
    worldState: SpacetimeDBWorldState | null;
    campfires: Map<string, SpacetimeDBCampfire>;
    cameraOffsetX: number;
    cameraOffsetY: number;
    canvasSize: { width: number; height: number };
}

interface UseDayNightCycleResult {
    overlayRgba: string;
    maskCanvasRef: React.RefObject<HTMLCanvasElement | null>;
}

/**
 * Manages the day/night cycle overlay color and the light mask canvas.
 */
export function useDayNightCycle({
    worldState,
    campfires,
    cameraOffsetX,
    cameraOffsetY,
    canvasSize,
}: UseDayNightCycleProps): UseDayNightCycleResult {
    const maskCanvasRef = useRef<HTMLCanvasElement | null>(null);

    // Calculate the current dynamic keyframes based on moon cycle
    const currentKeyframes = useMemo(() => {
        const currentProgress = worldState?.cycleProgress ?? 0.25;
        const currentCycleCount = worldState?.cycleCount ?? 0;
        const anticipationThreshold = 0.75;

        let effectiveIsFullMoon = worldState?.isFullMoon ?? false;
        if (currentProgress > anticipationThreshold) {
            const nextCycleIsFullMoon = ((currentCycleCount + 1) % FULL_MOON_CYCLE_INTERVAL) === 0;
            effectiveIsFullMoon = nextCycleIsFullMoon;
        }
        return getDynamicKeyframes(effectiveIsFullMoon);
    }, [worldState?.cycleProgress, worldState?.cycleCount, worldState?.isFullMoon]);

    // Calculate the final overlay color
    const overlayRgba = useMemo(() => {
        if (!worldState) return 'transparent';
        return interpolateRgba(worldState.cycleProgress, currentKeyframes);
    }, [worldState?.cycleProgress, currentKeyframes]);

    // Effect to initialize and draw the mask canvas
    useEffect(() => {
        // Initialize mask canvas if it doesn't exist
        if (!maskCanvasRef.current) {
            maskCanvasRef.current = document.createElement('canvas');
            console.log('Off-screen mask canvas created by hook.');
        }
        const maskCanvas = maskCanvasRef.current;
        const maskCtx = maskCanvas.getContext('2d');

        if (!maskCtx) {
            console.error("Failed to get mask canvas context");
            return;
        }

        // Resize mask canvas if necessary
        if (maskCanvas.width !== canvasSize.width || maskCanvas.height !== canvasSize.height) {
            maskCanvas.width = canvasSize.width;
            maskCanvas.height = canvasSize.height;
            // console.log('Off-screen mask canvas resized by hook.'); // Reduce console noise
        }

        // --- Prepare Mask --- 
        maskCtx.clearRect(0, 0, maskCanvas.width, maskCanvas.height);

        // Only draw mask if overlay is not fully transparent
        if (overlayRgba !== 'transparent' && overlayRgba !== 'rgba(0,0,0,0.00)') {
            // 1. Fill with overlay color
            maskCtx.fillStyle = overlayRgba;
            maskCtx.fillRect(0, 0, maskCanvas.width, maskCanvas.height);

            // 2. Cut holes for light sources (campfires)
            maskCtx.globalCompositeOperation = 'destination-out';
            campfires.forEach(fire => {
                if (fire.isBurning) {
                    const screenX = fire.posX + cameraOffsetX;
                    const screenY = fire.posY + cameraOffsetY;
                    const radius = CAMPFIRE_LIGHT_RADIUS_BASE; // Use base radius for mask shape

                    // Create gradient for soft edges
                    const maskGradient = maskCtx.createRadialGradient(screenX, screenY, radius * 0.5, screenX, screenY, radius);
                    maskGradient.addColorStop(0, 'rgba(255, 255, 255, 1)');
                    maskGradient.addColorStop(1, 'rgba(255, 255, 255, 0)');

                    maskCtx.fillStyle = maskGradient;
                    maskCtx.beginPath();
                    maskCtx.arc(screenX, screenY, radius, 0, Math.PI * 2);
                    maskCtx.fill();
                }
            });
            // Reset composite operation
            maskCtx.globalCompositeOperation = 'source-over';
        }

    // Dependencies: Re-run when these change
    }, [overlayRgba, campfires, cameraOffsetX, cameraOffsetY, canvasSize.width, canvasSize.height]); // Depend on specific size props

    return {
        overlayRgba,
        maskCanvasRef,
    };
} 