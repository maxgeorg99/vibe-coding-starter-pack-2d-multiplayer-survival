import { Stone } from '../generated'; // Import generated Stone type
import stoneImage from '../assets/doodads/stone.png'; // Ensure this path is correct

// Define image source
const stoneImageSource: string = stoneImage;

// Simple cache for loaded images
const imageCache: { [key: string]: HTMLImageElement } = {};

// Preload stone image
export function preloadStoneImage() {
  if (!imageCache[stoneImageSource]) {
    const img = new Image();
    img.src = stoneImageSource;
    imageCache[stoneImageSource] = img;
    img.onload = () => console.log(`Loaded stone image: ${stoneImageSource}`);
    img.onerror = () => console.error(`Failed to load stone image: ${stoneImageSource}`);
  }
}

// Function to get the stone image (simple since there's only one type)
function getStoneImage(): HTMLImageElement | null {
  if (!stoneImageSource) {
    console.error('Stone image source not defined.');
    return null;
  }

  // Return cached image if available, otherwise start loading (though preload is preferred)
  if (!imageCache[stoneImageSource]) {
    console.warn(`Stone image not preloaded: ${stoneImageSource}. Attempting load.`);
    const img = new Image();
    img.src = stoneImageSource;
    imageCache[stoneImageSource] = img; // Add to cache immediately
  }

  return imageCache[stoneImageSource];
}

// Function to draw a single stone
const TARGET_STONE_WIDTH_PX = 120; // Target width on screen (adjust as needed)
const SHAKE_DURATION_MS = 150; // How long the shake effect lasts
const SHAKE_INTENSITY_PX = 10; // Max pixel offset (less than tree?)

export function renderStone(ctx: CanvasRenderingContext2D, stone: Stone, now_ms: number) {
  const img = getStoneImage();
  if (!img || !img.complete || img.naturalWidth === 0) {
    // Image not loaded yet or failed to load, can draw a placeholder
    // ctx.fillStyle = 'gray';
    // ctx.beginPath();
    // ctx.arc(stone.posX, stone.posY - 10, 10, 0, Math.PI * 2);
    // ctx.fill();
    return;
  }

  // Calculate scaling factor based on target width
  const scaleFactor = TARGET_STONE_WIDTH_PX / img.naturalWidth;
  const drawWidth = TARGET_STONE_WIDTH_PX; // Set width to target
  const drawHeight = img.naturalHeight * scaleFactor; // Scale height proportionally

  let drawX = stone.posX - drawWidth / 2; // Center horizontally
  let drawY = stone.posY - drawHeight; // Draw upwards from position y using scaled height

  // --- Shake Logic (Copied from Tree, adjust intensity) ---
  if (stone.lastHitTime) { 
    const lastHitTimeMs = Number(stone.lastHitTime.microsSinceUnixEpoch / 1000n);
    const elapsedSinceHit = now_ms - lastHitTimeMs;

    if (elapsedSinceHit >= 0 && elapsedSinceHit < SHAKE_DURATION_MS) {
      const shakeFactor = 1.0 - (elapsedSinceHit / SHAKE_DURATION_MS); 
      const currentShakeIntensity = SHAKE_INTENSITY_PX * shakeFactor;
      const shakeX = (Math.random() - 0.5) * 2 * currentShakeIntensity;
      const shakeY = (Math.random() - 0.5) * 2 * currentShakeIntensity;
      drawX += shakeX;
      drawY += shakeY;
    }
  }
  // --- End Shake Logic ---

  ctx.drawImage(img, drawX, drawY, drawWidth, drawHeight);

  // Optional: Draw health bar or other info for debugging (similar to trees)
  // ctx.fillStyle = 'darkred';
  // ctx.fillRect(drawX, drawY - 10, drawWidth, 5);
  // ctx.fillStyle = 'darkgray'; // Use a different color for stone health?
  // ctx.fillRect(drawX, drawY - 10, drawWidth * (stone.health / 100), 5);
} 