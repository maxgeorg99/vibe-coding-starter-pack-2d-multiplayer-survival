import { DroppedItem as SpacetimeDBDroppedItem, ItemDefinition as SpacetimeDBItemDefinition } from '../generated';
import { drawShadow } from './shadowUtils'; // Import shadow utility
import { itemIcons } from './itemIconUtils'; // For fallback loading

interface RenderDroppedItemParams {
    ctx: CanvasRenderingContext2D;
    item: SpacetimeDBDroppedItem;
    itemDef: SpacetimeDBItemDefinition | undefined;
    itemImagesRef: React.RefObject<Map<string, HTMLImageElement>>;
}

const DRAW_WIDTH = 64;
const DRAW_HEIGHT = 64;

export function renderDroppedItem({
    ctx,
    item,
    itemDef,
    itemImagesRef,
}: RenderDroppedItemParams): void {
    let itemImg: HTMLImageElement | null = null;
    let iconAssetName: string | null = null;

    if (itemDef) {
        iconAssetName = itemDef.iconAssetName;
        itemImg = itemImagesRef.current?.get(iconAssetName) ?? null;
    } else {
        console.warn(`[Render DroppedItem Util] Definition not found for ID: ${item.itemDefId}`);
    }

    const canRenderIcon = itemDef && iconAssetName && itemImg && itemImg.complete && itemImg.naturalHeight !== 0;

    if (canRenderIcon && iconAssetName) {
        // Draw shadow first
        const centerX = item.posX;
        const baseY = item.posY;
        const shadowRadiusX = DRAW_WIDTH * 0.3;
        const shadowRadiusY = shadowRadiusX * 0.4;
        const shadowOffsetY = -DRAW_HEIGHT * -0.2; // Push shadow UP slightly
        drawShadow(ctx, centerX, baseY + shadowOffsetY, shadowRadiusX, shadowRadiusY);

        // Draw the item image
        ctx.drawImage(itemImg!, item.posX - DRAW_WIDTH / 2, item.posY - DRAW_HEIGHT / 2, DRAW_WIDTH, DRAW_HEIGHT);

        // Eagerly load image if not already cached (Consider removing if preloading effect is reliable)
        if (!itemImagesRef.current?.has(iconAssetName)) {
            const iconSrc = itemIcons[iconAssetName] || '';
            if (iconSrc) {
                // console.log(`[Render DroppedItem Util] Eagerly loading ${iconAssetName}`);
                const img = new Image();
                img.src = iconSrc;
                img.onload = () => itemImagesRef.current?.set(iconAssetName!, img);
                itemImagesRef.current?.set(iconAssetName, img); // Add placeholder immediately
            }
        }
    } else {
        // Fallback rendering if icon isn't ready or definition missing
        ctx.fillStyle = '#CCCCCC'; // Grey square fallback
        ctx.fillRect(item.posX - DRAW_WIDTH / 2, item.posY - DRAW_HEIGHT / 2, DRAW_WIDTH, DRAW_HEIGHT);
        // Optionally, try to trigger loading again if image ref exists but isn't loaded
        if (itemDef && iconAssetName && !itemImagesRef.current?.has(iconAssetName)) {
             const iconSrc = itemIcons[iconAssetName] || '';
            if (iconSrc) {
                const img = new Image();
                img.src = iconSrc;
                img.onload = () => itemImagesRef.current?.set(iconAssetName!, img);
                itemImagesRef.current?.set(iconAssetName, img); // Add placeholder
            }
        }
    }
} 