import { gameConfig } from '../config/gameConfig';

/**
 * Renders the tiled world background onto the canvas.
 * @param ctx - The CanvasRenderingContext2D to draw on.
 * @param grassImageRef - Ref to the loaded grass texture image.
 */
export function renderWorldBackground(
    ctx: CanvasRenderingContext2D,
    grassImageRef: React.RefObject<HTMLImageElement | null>
): void {
    const grassImg = grassImageRef.current;
    if (!grassImg || !grassImg.complete || grassImg.naturalHeight === 0) {
        // Draw fallback color if image not loaded or invalid
        ctx.fillStyle = '#8FBC8F'; // Medium Aquamarine fallback
        ctx.fillRect(0, 0, gameConfig.worldWidth * gameConfig.tileSize, gameConfig.worldHeight * gameConfig.tileSize);
        console.warn("[renderWorldBackground] Grass image not ready, drawing fallback.");
        return;
    }

    const drawGridLines = false; // Keep grid lines off
    const overlap = 1; // Overlap tiles slightly to prevent gaps

    // --- Draw tiles individually using drawImage --- 
    for (let y = 0; y < gameConfig.worldHeight; y++) {
        for (let x = 0; x < gameConfig.worldWidth; x++) {
            // Draw the tile image at each grid position
            ctx.drawImage(
                grassImg, // Source image
                x * gameConfig.tileSize, // Destination x
                y * gameConfig.tileSize, // Destination y
                gameConfig.tileSize + overlap, // Destination width + overlap
                gameConfig.tileSize + overlap  // Destination height + overlap
            );
        }
    }
    // --- End individual tile drawing ---

    // Optional: Draw grid lines on top if needed
    if (drawGridLines) {
        ctx.strokeStyle = 'rgba(221, 221, 221, 0.5)';
        ctx.lineWidth = 1;
        for (let y = 0; y < gameConfig.worldHeight; y++) {
            for (let x = 0; x < gameConfig.worldWidth; x++) {
                ctx.strokeRect(x * gameConfig.tileSize, y * gameConfig.tileSize, gameConfig.tileSize, gameConfig.tileSize);
            }
        }
    }
} 