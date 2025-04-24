import campfireSprite from '../assets/doodads/campfire.png';
import campfireOffSprite from '../assets/doodads/campfire_off.png'; // Import the off state sprite

import { drawShadow } from './shadowUtils'; // Import shadow utility

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
            // console.log('Campfire ON image loaded successfully.');
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
            // console.log('Campfire OFF image loaded successfully.');
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
export function renderCampfire(ctx: CanvasRenderingContext2D, worldX: number, worldY: number, isBurning: boolean) {
    const img = getImage(isBurning);
    if (!img) return; // Image not loaded

    const drawWidth = CAMPFIRE_WIDTH;
    const drawHeight = CAMPFIRE_HEIGHT;
    const centerX = worldX;
    const baseY = worldY; // Shadow sits at the base Y coordinate
    const drawX = centerX - drawWidth / 2; // Center horizontally
    const drawY = baseY - drawHeight; // Draw upwards from base Y

    // Draw shadow first
    const shadowRadiusX = drawWidth * 0.4;
    const shadowRadiusY = shadowRadiusX * 0.5;
    const shadowOffsetY = -drawHeight * 0.25; // Push shadow up slightly (10% of campfire height)
    drawShadow(ctx, centerX, baseY + shadowOffsetY, shadowRadiusX, shadowRadiusY);

    // Draw the campfire image
    ctx.drawImage(img, drawX, drawY, drawWidth, drawHeight);
}

function getImage(isBurning: boolean) {
    if (isBurning) {
        return campfireImage;
    } else {
        return campfireOffImage;
    }
} 