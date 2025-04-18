import boxSprite from '../assets/doodads/wooden_storage_box.png';
import { drawShadow } from './shadowUtils';

export const BOX_WIDTH = 64; // Adjust as needed
export const BOX_HEIGHT = 64; // Adjust as needed

let boxImage: HTMLImageElement | null = null;
let isBoxImageLoaded = false;

export function preloadWoodenStorageBoxImage() {
  if (boxImage) return; // Already loading or loaded

  console.log("Preloading Wooden Storage Box image...");
  boxImage = new Image();
  boxImage.onload = () => {
    isBoxImageLoaded = true;
    console.log("Wooden Storage Box image loaded successfully.");
  };
  boxImage.onerror = () => {
    console.error("Failed to load Wooden Storage Box image.");
    boxImage = null; // Reset on error
  };
  boxImage.src = boxSprite;
}

export function renderWoodenStorageBox(ctx: CanvasRenderingContext2D, x: number, y: number) {
  if (!isBoxImageLoaded || !boxImage) {
    // Draw fallback placeholder if image not loaded
    ctx.fillStyle = '#8B4513'; // SaddleBrown
    ctx.fillRect(x - BOX_WIDTH / 2, y - BOX_HEIGHT / 2, BOX_WIDTH, BOX_HEIGHT);
    ctx.strokeStyle = '#000';
    ctx.strokeRect(x - BOX_WIDTH / 2, y - BOX_HEIGHT / 2, BOX_WIDTH, BOX_HEIGHT);
    return;
  }

  const drawX = x - BOX_WIDTH / 2;
  const drawY = y - BOX_HEIGHT / 2;

  // Draw shadow first
  const shadowRadiusX = BOX_WIDTH * 0.45;
  const shadowRadiusY = shadowRadiusX * 0.45;
  const shadowOffsetY = BOX_HEIGHT * 0.25; // Slight vertical offset for shadow
  drawShadow(ctx, x, y + shadowOffsetY, shadowRadiusX, shadowRadiusY);

  // Draw the box image
  ctx.drawImage(boxImage, drawX, drawY, BOX_WIDTH, BOX_HEIGHT);
} 