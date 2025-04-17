import React, { useState } from 'react';
import styles from './InventoryUI.module.css';
// Import Custom Components
import DraggableItem from './DraggableItem';
import DroppableSlot from './DroppableSlot';

// Import from shared location
import { DragSourceSlotInfo, DraggedItemInfo } from '../types/dragDropTypes'; // Import both from shared

// Import SpacetimeDB types needed for props and logic
import {
    ItemDefinition,
    InventoryItem,
    DbConnection,
    EquipmentSlot as BackendEquipmentSlot, // Keep alias if used
    ActiveEquipment,
    Campfire as SpacetimeDBCampfire // Import Campfire type
} from '../generated';
import { Identity } from '@clockworklabs/spacetimedb-sdk';
import { itemIcons, getItemIcon } from '../utils/itemIconUtils';

// --- Type Definitions ---
// Define props for InventoryUI component
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
    // Add new props for interaction context
    interactionTarget: { type: string; id: number | bigint } | null;
    campfires: Map<string, SpacetimeDBCampfire>;
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
    interactionTarget,
    campfires,
}) => {
    const NUM_FUEL_SLOTS = 5; // Define constant for UI

    // --- Determine Campfire Interaction Data --- 
    let currentCampfire: SpacetimeDBCampfire | undefined = undefined;
    // Use an array to store populated items for each slot
    const fuelItems: (PopulatedItem | null)[] = Array(NUM_FUEL_SLOTS).fill(null);
    let campfireIdNum: number | null = null;
    const isCampfireInteraction = interactionTarget?.type === 'campfire'; // Flag for conditional render

    if (isCampfireInteraction) { // Fetch data only if interacting with campfire
        campfireIdNum = Number(interactionTarget.id);
        currentCampfire = campfires.get(campfireIdNum.toString());

        if (currentCampfire) {
            // Get instance IDs from individual fields
            const instanceIds = [
                currentCampfire.fuelInstanceId0,
                currentCampfire.fuelInstanceId1,
                currentCampfire.fuelInstanceId2,
                currentCampfire.fuelInstanceId3,
                currentCampfire.fuelInstanceId4,
            ];

            // Populate fuelItems array
            instanceIds.forEach((instanceIdOpt, index) => {
                if (instanceIdOpt) {
                    const instanceIdStr = instanceIdOpt.toString();
                    // console.log(`[InventoryUI Populate] Checking slot ${index}, looking for instance ID: ${instanceIdStr}`); // Log search
                    const foundInvItem = Array.from(inventoryItems.values()).find(invItem =>
                        invItem.instanceId.toString() === instanceIdStr
                    );
                    // Log if found
                    // console.log(`[InventoryUI Populate] Found item in map for ${instanceIdStr}?`, foundInvItem ? `Yes (Slots: I=${foundInvItem.inventorySlot}, H=${foundInvItem.hotbarSlot})` : 'No'); 
                    if (foundInvItem) {
                        const definition = itemDefinitions.get(foundInvItem.itemDefId.toString());
                        if (definition) {
                            fuelItems[index] = { instance: foundInvItem, definition };
                        } else {
                            console.warn(`[InventoryUI] Definition NOT FOUND for itemDefId: ${foundInvItem.itemDefId.toString()} in slot ${index}`);
                        }
                    } else {
                         // This might happen briefly if item is moved out before state syncs
                         // console.warn(`[InventoryUI] InventoryItem instance NOT FOUND for ID: ${instanceIdStr} in slot ${index}`);
                    }
                }
            });
        }
        // Log error if interactionTarget exists but campfire data doesn't
        if (!currentCampfire) {
             console.warn(`Campfire data not found for ID: ${interactionTarget.id}`);
             // Optional: Set a flag to show an error message in render?
        }
    }

    // --- Right Click Handler (Inventory) ---
    const handleInventoryItemContextMenu = (event: React.MouseEvent<HTMLDivElement>, itemInfo: PopulatedItem) => {
        console.log("[InventoryUI] Context menu on:", itemInfo?.definition?.name);
        if (!connection?.reducers || !itemInfo) return;

        const itemInstanceId = BigInt(itemInfo.instance.instanceId);

        // --- NEW: Check for Campfire interaction + Wood --- 
        if (isCampfireInteraction && campfireIdNum !== null && itemInfo.definition.name === 'Wood') {
             console.log(`[InventoryUI ContextMenu] Wood right-clicked while Campfire ${campfireIdNum} open. Calling add_wood_to_first_available_campfire_slot...`);
            try {
                connection.reducers.addWoodToFirstAvailableCampfireSlot(campfireIdNum, itemInstanceId);
                // No further action needed here, backend handles moving the item
            } catch (error: any) {
                console.error("[InventoryUI ContextMenu] Failed to call add_wood_to_first_available_campfire_slot reducer:", error);
                // TODO: Show user feedback? (e.g., "Campfire full")
            }
            return; // Stop processing this context menu event
        }
        // --- END NEW Check --- 

        // --- Existing Armor/Hotbar Logic --- 
        if (itemInfo.definition.category.tag === 'Armor') {
            console.log(`[InventoryUI ContextMenu] Item is Armor. Calling equip_armor for item ${itemInstanceId}`);
            try {
                connection.reducers.equipArmor(itemInstanceId);
            } catch (error: any) {
                console.error("[InventoryUI ContextMenu] Failed to call equipArmor reducer:", error);
            }
        } else {
            console.log(`[InventoryUI ContextMenu] Item is not Armor/Wood-in-Campfire-Context. Calling move_to_first_available_hotbar_slot for item ${itemInstanceId}`);
            try {
                connection.reducers.moveToFirstAvailableHotbarSlot(itemInstanceId);
            } catch (error: any) {
                console.error("[InventoryUI ContextMenu] Failed to call moveToFirstAvailableHotbarSlot reducer:", error);
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

    // --- Handlers --- 
    // Handler for removing fuel item (e.g., via context menu on the fuel item)
    const handleRemoveFuel = (event: React.MouseEvent<HTMLDivElement>, slotIndex: number) => {
        event.preventDefault(); // Prevent default browser menu
        if (!connection?.reducers || campfireIdNum === null) return;
        console.log(`Attempting to remove fuel from campfire ${campfireIdNum}, slot ${slotIndex}`);
        try {
            // Call reducer with campfireId and slotIndex
            connection.reducers.removeFuelFromCampfire(campfireIdNum, slotIndex);
        } catch (error) {
            console.error("Error calling removeFuelFromCampfire reducer:", error);
        }
    };

    // Handler for the Light/Extinguish button
    const handleToggleBurn = () => {
        if (!connection?.reducers || campfireIdNum === null) return;
            console.log(`Attempting to toggle burn state for campfire ${campfireIdNum}`);
        try {
            connection.reducers.toggleCampfireBurning(campfireIdNum);
        } catch (error) {
            console.error("Error calling toggleCampfireBurning reducer:", error);
        }
    };

    // --- NEW: Calculate disabled state for the toggle button ---
    const getIsToggleButtonDisabled = () => {
        if (!currentCampfire) return true; // Cannot toggle if no campfire data

        if (currentCampfire.isBurning) {
            return false; // Always allow extinguishing
        } else {
            // Check if ANY slot has valid fuel
            const hasValidFuel = fuelItems.some(item =>
                item && item.definition.name === 'Wood' && item.instance.quantity > 0
            );
            return !hasValidFuel; // Disable if NO slot has valid fuel
        }
    };
    const isToggleButtonDisabled = getIsToggleButtonDisabled(); // Call the function

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
                 {/* Wrap inventory grid and conditional campfire section in a fragment */}
                 <>
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
                     </div> {/* End inventory grid */}

                    {/* --- Conditional Campfire Interaction Section --- */}
                    {isCampfireInteraction && currentCampfire && campfireIdNum !== null && (
                        <div className={styles.externalInventorySection}>
                            <h3 className={styles.sectionTitle} style={{ marginTop: '20px' }}>CAMPFIRE</h3>
                            <div className={styles.multiSlotContainer} style={{ display: 'flex', flexDirection: 'row', gap: '5px'}}>
                                {Array.from({ length: NUM_FUEL_SLOTS }).map((_, index) => {
                                    const itemInSlot = fuelItems[index];
                                    // --- Pass campfireIdNum as parentId ---
                                    const currentCampfireSlotInfo: DragSourceSlotInfo = { type: 'campfire_fuel', index: index, parentId: campfireIdNum }; 
                                    return (
                                        <DroppableSlot
                                            key={`campfire-fuel-${campfireIdNum}-${index}`}
                                            slotInfo={currentCampfireSlotInfo} // Pass updated info
                                            onItemDrop={onItemDrop}
                                            className={styles.slot}
                                            isDraggingOver={false}
                                        >
                                            {itemInSlot && (
                                                <DraggableItem
                                                    item={itemInSlot}
                                                    sourceSlot={currentCampfireSlotInfo} // Pass updated info
                                                    onItemDragStart={onItemDragStart}
                                                    onItemDrop={onItemDrop}
                                                    onContextMenu={(event) => handleRemoveFuel(event, index)}
                                                />
                                            )}
                                        </DroppableSlot>
                                    );
                                })}
                            </div>
                            <button 
                                onClick={handleToggleBurn} 
                                disabled={isToggleButtonDisabled}
                                className={styles.interactionButton}
                                title={
                                    isToggleButtonDisabled && !currentCampfire.isBurning
                                    ? "Requires Wood with quantity > 0 in a slot to light"
                                    : ""
                                }
                            >
                                {currentCampfire.isBurning ? "Extinguish Fire" : "Light Fire"}
                            </button>
                        </div>
                    )}
                    {/* Handle case where interaction is campfire but data wasn't found */}
                    {isCampfireInteraction && !currentCampfire && (
                        <div className={styles.externalInventorySection}>
                            <h3 className={styles.sectionTitle}>CAMPFIRE</h3>
                            <div>Error: Campfire data missing.</div>
                        </div>
                    )}
                    {/* --- End Campfire Section --- */}
                 </> {/* End fragment wrapper */}
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