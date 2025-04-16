import React, { useState } from 'react';
import styles from './InventoryUI.module.css';
import { ItemDefinition, InventoryItem, DbConnection, EquipmentSlot as BackendEquipmentSlot, ActiveEquipment } from '../generated'; // Assuming DbConnection is needed for reducers
import { Identity } from '@clockworklabs/spacetimedb-sdk';
import { itemIcons, getItemIcon } from '../utils/itemIconUtils'; // Assuming you have this utility

// Import Custom Components
import DraggableItem from './DraggableItem';
import DroppableSlot from './DroppableSlot';

// Import Types (adjust path if moved)
import { DragSourceSlotInfo, DraggedItemInfo } from './PlayerUI';

// --- Type Definitions ---
interface InventoryUIProps {
  playerIdentity: Identity | null;
  onClose: () => void;
  inventoryItems: Map<string, InventoryItem>;
  itemDefinitions: Map<string, ItemDefinition>;
  connection: DbConnection | null;
  activeEquipments: Map<string, ActiveEquipment>;
  onItemDragStart: (info: DraggedItemInfo) => void;
  onItemDrop: (targetSlotInfo: DragSourceSlotInfo | null) => void;
  draggedItemInfo: DraggedItemInfo | null;
}

// Represents an item instance with its definition for rendering
export interface PopulatedItem {
    instance: InventoryItem;
    definition: ItemDefinition;
}

// --- Placeholder Data (Restore) ---
// Placeholder type for items - replace with actual type later
interface PlaceholderItem {
    id: number;
    name: string;
    icon: string; // Path to icon asset
}

// Placeholder data - replace with actual data later
const placeholderCraftableItems: PlaceholderItem[] = Array.from({ length: 15 }).map((_, i) => ({
    id: i,
    name: `Craftable ${i + 1}`,
    icon: './assets/icons/placeholder.png', // Example path
}));

const placeholderCraftingQueue: PlaceholderItem[] = Array.from({ length: 8 }).map((_, i) => ({
    id: 100 + i,
    name: `Crafting ${i + 1}`,
    icon: './assets/icons/placeholder.png', // Example path
}));

// --- Main Component ---

const InventoryUI: React.FC<InventoryUIProps> = ({
    playerIdentity,
    onClose,
    inventoryItems,
    itemDefinitions,
    connection,
    activeEquipments,
    onItemDragStart,
    onItemDrop,
    draggedItemInfo,
}) => {
    // --- Right Click Handler ---
    const handleInventoryItemContextMenu = (event: React.MouseEvent<HTMLDivElement>, itemInfo: PopulatedItem) => {
        console.log("[InventoryUI] Context menu handler called for:", itemInfo?.definition?.name);
        if (!connection?.reducers || !itemInfo) return;

        const itemInstanceId = BigInt(itemInfo.instance.instanceId);

        // Check if the item category is Armor
        if (itemInfo.definition.category.tag === 'Armor') {
            console.log(`[InventoryUI] Item is Armor. Calling equip_armor for item ${itemInstanceId}`);
            try {
                // Call the new reducer for armor
                connection.reducers.equipArmor(itemInstanceId);
            } catch (error: any) {
                console.error("[InventoryUI] Failed to call equipArmor reducer:", error);
                // Optionally show user message using error.message
            }
        }
        // Otherwise (Tool, Material, Placeable, etc.) try equipping to hotbar 
        else {
            // We removed the isEquippable check here, let the backend reducer handle it
            console.log(`[InventoryUI] Item is not Armor. Calling equip_to_hotbar for item ${itemInstanceId}`);
            try {
                // Call reducer, passing undefined for target slot to find first available
                connection.reducers.equipToHotbar(itemInstanceId, undefined);
            } catch (error: any) { 
                console.error("[InventoryUI] Failed to call equipToHotbar reducer:", error);
                // Optionally show user message using error.message
            }
        }
    };

    // Ensure props are defined
    const currentInventoryItems = inventoryItems || new Map<string, InventoryItem>();
    const currentItemDefinitions = itemDefinitions || new Map<string, ItemDefinition>();

    const inventoryRows = 4;
    const inventoryCols = 6;
    const totalInventorySlots = inventoryRows * inventoryCols;
    const hotbarSlots = 6;

    // --- Data Mapping ---
    const itemsByInvSlot = new Map<number, PopulatedItem>();
    // Key items by the LOGICAL slot name (Head, Chest, etc.)
    const itemsByEquipSlot = new Map<string, PopulatedItem>();
    const currentActiveEquipments = activeEquipments || new Map<string, ActiveEquipment>();

    // Map inventory items first
    Array.from(currentInventoryItems.values())
        .filter(item => item.playerIdentity && playerIdentity && item.playerIdentity.isEqual(playerIdentity))
        .forEach(itemInstance => {
            const definition = currentItemDefinitions.get(itemInstance.itemDefId.toString());
            if (!definition) return;
            const populatedItem = { instance: itemInstance, definition };

            if (itemInstance.inventorySlot !== null && itemInstance.inventorySlot !== undefined) {
                itemsByInvSlot.set(itemInstance.inventorySlot, populatedItem);
            }
        });

    // Map equipped items from ActiveEquipment
    const playerEquipment = playerIdentity ? currentActiveEquipments.get(playerIdentity.toHexString()) : null;

    if (playerEquipment) {
        // Map backend fields directly to logical slot names (matching EquipmentSlot enum variants)
        const equipMapping: { field: keyof ActiveEquipment; logicalSlot: string }[] = [
            { field: 'headItemInstanceId', logicalSlot: 'Head' },
            { field: 'chestItemInstanceId', logicalSlot: 'Chest' },
            { field: 'legsItemInstanceId', logicalSlot: 'Legs' },
            { field: 'feetItemInstanceId', logicalSlot: 'Feet' },
            { field: 'handsItemInstanceId', logicalSlot: 'Hands' },
            { field: 'backItemInstanceId', logicalSlot: 'Back' },
        ];

        equipMapping.forEach(({ field, logicalSlot }) => {
            const instanceId = playerEquipment[field];
            if (instanceId) {
                // Find InventoryItem and Definition based on instanceId (as before)
                let foundItem: InventoryItem | undefined = undefined;
                for (const item of currentInventoryItems.values()){
                    if(item.instanceId === instanceId){ foundItem = item; break; }
                }
                if (foundItem) {
                    const definition = currentItemDefinitions.get(foundItem.itemDefId.toString());
                    if (definition) {
                         // Use the logicalSlot name as the key
                         itemsByEquipSlot.set(logicalSlot, { instance: foundItem, definition });
                    } else { console.warn(`[MapEquip] Definition not found for equipped item instance ${instanceId}`); }
                } else { console.warn(`[MapEquip] InventoryItem not found for equipped instance ${instanceId}.`); }
            }
        });
    }

    // --- NEW Simplified Equipment Slot Layout ---
    const equipmentSlotLayout: { name: string, type: BackendEquipmentSlot | null }[] = [
        { name: 'Head', type: { tag: 'Head' } },
        { name: 'Chest', type: { tag: 'Chest' } },
        { name: 'Legs', type: { tag: 'Legs' } },
        { name: 'Feet', type: { tag: 'Feet' } },
        { name: 'Hands', type: { tag: 'Hands' } },
        { name: 'Back', type: { tag: 'Back' } },
    ];

    return (
        <div className={styles.inventoryPanel}>
            <button className={styles.closeButton} onClick={onClose}>X</button>

            {/* Left Pane */}
            <div className={styles.leftPane}>
                {/* REMOVE Player Name */}
                {/* <div className={styles.playerName}>Player Name</div> */}
                {/* ADD Equipment Title */}
                <h3 className={styles.sectionTitle}>EQUIPMENT</h3>

                {/* Equipment Grid Wrapper */}
                <div className={styles.equipmentGrid}>
                    {equipmentSlotLayout.map(slotInfo => {
                        const item = itemsByEquipSlot.get(slotInfo.name);
                        const currentSlotInfo: DragSourceSlotInfo = { type: 'equipment', index: slotInfo.name };
                        return (
                            <DroppableSlot
                                key={`equip-${slotInfo.name}`}
                                slotInfo={currentSlotInfo}
                                onItemDrop={onItemDrop}
                                className={styles.slot} 
                                isDraggingOver={false}
                            >
                                {item && (
                                    <DraggableItem
                                        item={item}
                                        sourceSlot={currentSlotInfo}
                                        onItemDragStart={onItemDragStart}
                                        onItemDrop={onItemDrop}
                                    />
                                )}
                            </DroppableSlot>
                        );
                    })}
                </div>
                 {/* REMOVE Stats Area */} 
                 {/* <div className={styles.playerStatsArea}> ... </div> */}
            </div>

            {/* Middle Pane - Use DroppableSlot and DraggableItem */}
             <div className={styles.middlePane}>
                 <h3 className={styles.sectionTitle}>INVENTORY</h3>
                 <div className={styles.inventoryGrid}>
                 {Array.from({ length: totalInventorySlots }).map((_, index) => {
                     const item = itemsByInvSlot.get(index); // Get item for this inv slot index
                     const currentSlotInfo: DragSourceSlotInfo = { type: 'inventory', index: index };
                     return (
                         // Use DroppableSlot and conditionally DraggableItem
                         <DroppableSlot
                            key={`inv-${index}`}
                            slotInfo={currentSlotInfo}
                            onItemDrop={onItemDrop}
                            className={styles.slot} // Base slot style from InventoryUI.module.css
                            isDraggingOver={false} // TODO: Add logic if visual feedback needed
                         >
                             {item && (
                                 <DraggableItem
                                    item={item}
                                    sourceSlot={currentSlotInfo}
                                    onItemDragStart={onItemDragStart}
                                    onItemDrop={onItemDrop}
                                    onContextMenu={(event) => handleInventoryItemContextMenu(event, item)}
                                 />
                             )}
                         </DroppableSlot>
                     );
                 })}
                 </div>
             </div>

             {/* Right Pane (Crafting - Not making draggable/droppable yet) */}
            <div className={styles.rightPane}>
                {/* --- Restore Crafting UI --- */}
                <div className={styles.craftingHeader}>
                    <h3 className={styles.sectionTitle}>CRAFTING</h3>
                    {/* Maybe add a button/link like in concept */}
                </div>

                <div className={styles.craftableItemsSection}>
                    <div className={styles.craftableItemsGrid}>
                        {placeholderCraftableItems.map((item) => (
                        <div key={`craftable-${item.id}`} className={styles.slot}>
                            <img src={getItemIcon(item.icon)} alt={item.name} style={{ width: '80%', height: '80%', objectFit: 'contain', imageRendering: 'pixelated' }} />
                        </div>
                        ))}
                    </div>
                </div>

                <div className={styles.craftingQueueSection}>
                    <h4 className={styles.queueTitle}>CRAFTING QUEUE</h4>
                    <div className={styles.craftingQueueList}>
                        {placeholderCraftingQueue.map((item) => (
                        <div key={`queue-${item.id}`} className={styles.queueItem}>
                            <div className={`${styles.slot} ${styles.queueItemSlot}`}>
                                <img src={getItemIcon(item.icon)} alt={item.name} style={{ width: '80%', height: '80%', objectFit: 'contain', imageRendering: 'pixelated' }} />
                            </div>
                            <span>{item.name} (??s)</span>
                        </div>
                        ))}
                        {placeholderCraftingQueue.length === 0 && <p className={styles.emptyQueueText}>Queue is empty</p>}
                    </div>
                </div>
                 {/* --- End Restore Crafting UI --- */}
            </div>
        </div>
    );
};

export default InventoryUI;