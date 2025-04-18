import { Stone } from '../generated'; // Import generated Stone type
import stoneImage from '../assets/doodads/stone.png'; // Ensure this path is correct
import { drawShadow } from './shadowUtils'; // Import shadow utility

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
  // --- Check if Stone is depleted ---
  if (stone.health <= 0) {
    return; // Don't render depleted stones
  }
  // --- End Depleted Check ---

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

  const centerX = stone.posX;
  const baseY = stone.posY; // Shadow sits at the base Y coordinate
  let drawX = centerX - drawWidth / 2; // Top-left corner for image drawing
  let drawY = baseY - drawHeight; // Draw image upwards from base Y

  // --- Shake Logic (Copied from Tree, adjust intensity) ---
  let shakeOffsetX = 0;
  let shakeOffsetY = 0;
  if (stone.lastHitTime) { 
    const lastHitTimeMs = Number(stone.lastHitTime.microsSinceUnixEpoch / 1000n);
    const elapsedSinceHit = now_ms - lastHitTimeMs;

    if (elapsedSinceHit >= 0 && elapsedSinceHit < SHAKE_DURATION_MS) {
      const shakeFactor = 1.0 - (elapsedSinceHit / SHAKE_DURATION_MS); 
      const currentShakeIntensity = SHAKE_INTENSITY_PX * shakeFactor;
      shakeOffsetX = (Math.random() - 0.5) * 2 * currentShakeIntensity;
      shakeOffsetY = (Math.random() - 0.5) * 2 * currentShakeIntensity;
      // Apply shake later, after shadow is drawn relative to unshaken position
    }
  }
  // --- End Shake Logic ---

  // Draw shadow first (relative to unshaken position)
  const shadowRadiusX = drawWidth * 0.4;
  const shadowRadiusY = shadowRadiusX * 0.5;
  const shadowOffsetY = -drawHeight * 0.225; // Push shadow up slightly less (10% of stone height)
  drawShadow(ctx, centerX, baseY + shadowOffsetY, shadowRadiusX, shadowRadiusY);

  // Apply shake offset for drawing the image
  const shakenDrawX = drawX + shakeOffsetX;
  const shakenDrawY = drawY + shakeOffsetY;

  ctx.drawImage(img, shakenDrawX, shakenDrawY, drawWidth, drawHeight);

  // Optional: Draw health bar or other info for debugging (similar to trees)
  // ctx.fillStyle = 'darkred';
  // ctx.fillRect(drawX, drawY - 10, drawWidth, 5);
  // ctx.fillStyle = 'darkgray'; // Use a different color for stone health?
  // ctx.fillRect(drawX, drawY - 10, drawWidth * (stone.health / 100), 5);
} 