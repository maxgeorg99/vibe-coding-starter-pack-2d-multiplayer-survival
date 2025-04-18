import { Tree } from '../generated'; // Import generated types
import treeOakImage from '../assets/doodads/tree.png'; // Adjust path if needed
// import treeStumpImage from '../assets/doodads/tree_stump.png'; // REMOVED
import { drawShadow } from './shadowUtils'; // Import shadow utility

// Define image sources map (could be moved to config)
// Use string literals for keys based on the expected enum tags/values
export const treeImageSources: { [key: string]: string } = {
  Oak: treeOakImage,  // Use string 'Oak'
  // Stump: treeStumpImage, // REMOVED
};

// Simple cache for loaded images
const imageCache: { [key: string]: HTMLImageElement } = {};

// Preload images (optional but good practice)
export function preloadTreeImages() {
  Object.values(treeImageSources).forEach((src) => {
    if (!imageCache[src]) {
      const img = new Image();
      img.src = src;
      imageCache[src] = img;
      img.onload = () => console.log(`Loaded tree image: ${src}`);
      img.onerror = () => console.error(`Failed to load tree image: ${src}`);
    }
  });
}

// Function to get the correct image for a tree
function getTreeImage(tree: Tree): HTMLImageElement | null {
  let src: string;
  // REMOVED Stump check
  // if (tree.state.tag === 'Stump') { // Access the tag property for state comparison
  //   src = treeImageSources.Stump;
  // } else {

  // Access the tag property of the treeType enum for indexing
  const typeKey = tree.treeType.tag; // Assuming .tag holds 'Oak' or 'Pine'
  src = treeImageSources[typeKey] || treeImageSources.Oak; // Use the extracted key

  // REMOVED closing brace for else block
  // }

  if (!src) {
    console.error('Could not determine image source for tree:', tree);
    return null;
  }

  // Return cached image if available, otherwise start loading (though preload is preferred)
  if (!imageCache[src]) {
    console.warn(`Tree image not preloaded: ${src}. Attempting load.`);
    const img = new Image();
    img.src = src;
    imageCache[src] = img; // Add to cache immediately
  }

  return imageCache[src];
}

// Function to draw a single tree
const TARGET_TREE_WIDTH_PX = 160; // Target width on screen, slightly larger than player (was 64)
const SHAKE_DURATION_MS = 150; // How long the shake effect lasts
const SHAKE_INTENSITY_PX = 10; // Maximum pixel offset for the shake

export function renderTree(ctx: CanvasRenderingContext2D, tree: Tree, now_ms: number) {
  const img = getTreeImage(tree);
  if (!img || !img.complete || img.naturalWidth === 0) {
    // Image not loaded yet or failed to load, draw placeholder (optional)
    return;
  }

  // Calculate scaling factor based on target width
  const scaleFactor = TARGET_TREE_WIDTH_PX / img.naturalWidth;
  const drawWidth = TARGET_TREE_WIDTH_PX; // Set width to target
  const drawHeight = img.naturalHeight * scaleFactor; // Scale height proportionally

  const centerX = tree.posX;
  const baseY = tree.posY; // Shadow sits at the base Y coordinate
  const drawX = centerX - drawWidth / 2; // Top-left corner for image drawing
  const drawY = baseY - drawHeight; // Draw image upwards from base Y

  // Calculate shake offset
  let shakeOffsetX = 0;
  let shakeOffsetY = 0;
  if (tree.lastHitTime) {
    const lastHitTimeMs = Number(tree.lastHitTime.microsSinceUnixEpoch / 1000n);
    const elapsedSinceHit = now_ms - lastHitTimeMs;

    if (elapsedSinceHit >= 0 && elapsedSinceHit < SHAKE_DURATION_MS) {
      const shakeFactor = 1.0 - (elapsedSinceHit / SHAKE_DURATION_MS); // Fade out shake
      const currentShakeIntensity = SHAKE_INTENSITY_PX * shakeFactor;
      shakeOffsetX = (Math.random() - 0.5) * 2 * currentShakeIntensity;
      shakeOffsetY = (Math.random() - 0.5) * 2 * currentShakeIntensity;
    }
  }

  const shakenDrawX = drawX + shakeOffsetX;
  const shakenDrawY = drawY + shakeOffsetY;

  // Draw shadow first
  const shadowRadiusX = drawWidth * 0.4;
  const shadowRadiusY = shadowRadiusX * 0.5;
  // Draw shadow relative to unshaken base position
  const shadowOffsetY = -drawHeight * 0.05; // Push shadow up slightly (consistent with stone maybe? Adjust if needed)
  drawShadow(ctx, centerX, baseY + shadowOffsetY, shadowRadiusX, shadowRadiusY);

  // Draw the tree image with shake
  ctx.drawImage(img, shakenDrawX, shakenDrawY, drawWidth, drawHeight);

  // Optional: Draw health bar or other info for debugging
  // ctx.fillStyle = 'red';
  // ctx.fillRect(drawX, drawY - 10, drawWidth, 5);
  // ctx.fillStyle = 'green';
  // ctx.fillRect(drawX, drawY - 10, drawWidth * (tree.health / 100), 5);
}
