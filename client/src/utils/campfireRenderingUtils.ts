import campfireSprite from '../assets/doodads/campfire.png';

// --- Constants ---
export const CAMPFIRE_WIDTH = 64;
export const CAMPFIRE_HEIGHT = 64;

// --- Image Preloading ---
let campfireImage: HTMLImageElement | null = null;
let isCampfireImageLoaded = false;

export function preloadCampfireImage() {
    if (!campfireImage) {
        campfireImage = new Image();
        campfireImage.onload = () => {
            isCampfireImageLoaded = true;
            console.log('Campfire image loaded successfully.');
        };
        campfireImage.onerror = () => {
            console.error('Failed to load campfire image.');
            campfireImage = null; // Reset on error
        };
        campfireImage.src = campfireSprite;
    }
}

// --- Rendering Function ---
// Change signature back to screen coordinates
export function renderCampfire(ctx: CanvasRenderingContext2D, screenX: number, screenY: number) {
    // console.log(`[renderCampfire] Drawing at screen (${screenX.toFixed(0)}, ${screenY.toFixed(0)}), Image Loaded: ${isCampfireImageLoaded}`);

    if (!isCampfireImageLoaded || !campfireImage) {
        // Draw fallback at screen coordinates
        ctx.fillStyle = '#FFFF00';
        ctx.beginPath();
        ctx.arc(screenX, screenY, 15, 0, Math.PI * 2); // Draw centered at screenX, screenY
        ctx.fill();
        console.warn(`[renderCampfire] Fallback drawn at screen (${screenX.toFixed(0)}, ${screenY.toFixed(0)})`);
        return;
    }

    // Draw the campfire sprite, centered at the given screen coordinates
    const drawX = screenX - CAMPFIRE_WIDTH / 2;
    const drawY = screenY - CAMPFIRE_HEIGHT / 2; // Center vertically

    ctx.drawImage(
        campfireImage!,
        drawX,
        drawY,
        CAMPFIRE_WIDTH,
        CAMPFIRE_HEIGHT
    );
} 