import campfireSprite from '../assets/doodads/campfire.png';
import campfireOffSprite from '../assets/doodads/campfire_off.png'; // Import the off state sprite

// --- Constants ---
export const CAMPFIRE_WIDTH = 64;
export const CAMPFIRE_HEIGHT = 64;

// --- Image Preloading ---
let campfireImage: HTMLImageElement | null = null;
let campfireOffImage: HTMLImageElement | null = null; // Add variable for off image
let isCampfireImageLoaded = false;
let isCampfireOffImageLoaded = false; // Add loaded flag for off image

export function preloadCampfireImage() {
    // Preload Campfire ON image
    if (!campfireImage) {
        campfireImage = new Image();
        campfireImage.onload = () => {
            isCampfireImageLoaded = true;
            console.log('Campfire ON image loaded successfully.');
        };
        campfireImage.onerror = () => {
            console.error('Failed to load campfire ON image.');
            campfireImage = null; // Reset on error
        };
        campfireImage.src = campfireSprite;
    }
    // Preload Campfire OFF image
    if (!campfireOffImage) {
        campfireOffImage = new Image();
        campfireOffImage.onload = () => {
            isCampfireOffImageLoaded = true;
            console.log('Campfire OFF image loaded successfully.');
        };
        campfireOffImage.onerror = () => {
            console.error('Failed to load campfire OFF image.');
            campfireOffImage = null; // Reset on error
        };
        campfireOffImage.src = campfireOffSprite;
    }
}

// --- Rendering Function ---
// Change signature to include isBurning state
export function renderCampfire(ctx: CanvasRenderingContext2D, screenX: number, screenY: number, isBurning: boolean) {
    // console.log(`[renderCampfire] Drawing at screen (${screenX.toFixed(0)}, ${screenY.toFixed(0)}), Image Loaded: ${isCampfireImageLoaded}`);

    const imageToDraw = isBurning ? campfireImage : campfireOffImage;
    const isImageReady = isBurning ? isCampfireImageLoaded : isCampfireOffImageLoaded;

    if (!isImageReady || !imageToDraw) {
        // Draw fallback at screen coordinates (Yellow for burning, Gray for off)
        ctx.fillStyle = isBurning ? '#FFFF00' : '#808080';
        ctx.beginPath();
        ctx.arc(screenX, screenY, 15, 0, Math.PI * 2); // Draw centered at screenX, screenY
        ctx.fill();
        // console.warn(`[renderCampfire] Fallback drawn at screen (${screenX.toFixed(0)}, ${screenY.toFixed(0)}) - Burning: ${isBurning}`);
        return;
    }

    // Draw the correct campfire sprite, centered at the given screen coordinates
    const drawX = screenX - CAMPFIRE_WIDTH / 2;
    const drawY = screenY - CAMPFIRE_HEIGHT / 2; // Center vertically

    ctx.drawImage(
        imageToDraw!,
        drawX,
        drawY,
        CAMPFIRE_WIDTH,
        CAMPFIRE_HEIGHT
    );
} 