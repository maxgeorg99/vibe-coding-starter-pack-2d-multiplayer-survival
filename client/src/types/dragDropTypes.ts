import { PopulatedItem } from '../components/InventoryUI'; // Assuming PopulatedItem stays in InventoryUI for now

// Type definition for the source/target of a drag/drop operation
export interface DragSourceSlotInfo {
    type: 'inventory' | 'hotbar' | 'equipment' | 'campfire_fuel'; // Added campfire_fuel
    index: number | string; // number for inv/hotbar/campfire_id, string for equip name
}

// Type definition for the item being dragged
export interface DraggedItemInfo {
    item: PopulatedItem;
    sourceSlot: DragSourceSlotInfo;
    splitQuantity?: number;
    // Add split info later if needed
} 