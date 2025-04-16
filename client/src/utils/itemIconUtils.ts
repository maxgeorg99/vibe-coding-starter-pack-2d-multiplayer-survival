// client/src/utils/itemIconUtils.ts

// Import all potential item icons
import woodIcon from '../assets/items/wood.png';
import stoneIcon from '../assets/items/stone.png';
import woodHatchetIcon from '../assets/items/wood_hatchet.png';
import pickAxeIcon from '../assets/items/pick_axe.png';
import campFireIcon from '../assets/items/campfire.png';
import rockItemIcon from '../assets/items/rock_item.png';
// Import other icons as needed

// Create a mapping from the asset name (stored in DB) to the imported module path
export const itemIcons: { [key: string]: string } = {
  'wood.png': woodIcon,
  'stone.png': stoneIcon,
  'wood_hatchet.png': woodHatchetIcon,
  'pick_axe.png': pickAxeIcon,
  'campfire.png': campFireIcon,
  'rock_item.png': rockItemIcon,
  // Add other mappings here
};

// Function to get the icon path (optional, can also access map directly)
export function getItemIconPath(assetName: string): string | undefined {
  return itemIcons[assetName];
} 