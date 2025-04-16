// client/src/utils/mushroomRenderingUtils.ts
import { Mushroom } from '../generated'; // Import generated Mushroom type
import mushroomImage from '../assets/doodads/mushroom.png'; // Adjust path if needed

// Define image sources map
export const mushroomImageSources: { [key: string]: string } = {
  Default: mushroomImage, // Only one type for now
};

// Simple cache for loaded images
const imageCache: { [key: string]: HTMLImageElement } = {};

// Preload images
export function preloadMushroomImages() {
  Object.values(mushroomImageSources).forEach((src) => {
    if (!imageCache[src]) {
      const img = new Image();
      img.src = src;
      imageCache[src] = img;
      img.onload = () => console.log(`Loaded mushroom image: ${src}`);
      img.onerror = () => console.error(`Failed to load mushroom image: ${src}`);
    }
  });
}

// Function to get the image for a mushroom
function getMushroomImage(mushroom: Mushroom): HTMLImageElement | null {
  // Only one type for now
  const src = mushroomImageSources.Default;

  if (!src) {
    console.error('Could not determine image source for mushroom:', mushroom);
    return null;
  }

  if (!imageCache[src]) {
    console.warn(`Mushroom image not preloaded: ${src}. Attempting load.`);
    const img = new Image();
    img.src = src;
    imageCache[src] = img;
  }

  return imageCache[src];
}

// Function to draw a single mushroom
const TARGET_MUSHROOM_WIDTH_PX = 48; // Target width on screen (adjust as needed)

export function renderMushroom(ctx: CanvasRenderingContext2D, mushroom: Mushroom, now_ms: number) {
  const img = getMushroomImage(mushroom);
  if (!img || !img.complete || img.naturalWidth === 0) {
    return; // Image not loaded yet or failed
  }

  // Calculate scaling factor based on target width
  const scaleFactor = TARGET_MUSHROOM_WIDTH_PX / img.naturalWidth;
  const drawWidth = TARGET_MUSHROOM_WIDTH_PX;
  const drawHeight = img.naturalHeight * scaleFactor;

  const drawX = mushroom.posX - drawWidth / 2; // Center horizontally
  const drawY = mushroom.posY - drawHeight; // Draw upwards from position y using scaled height

  ctx.drawImage(img, drawX, drawY, drawWidth, drawHeight);

  // No health bar or shake needed for mushrooms currently
} 