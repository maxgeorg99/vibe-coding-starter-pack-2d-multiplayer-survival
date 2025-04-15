import React, { useState, useEffect } from 'react';
import { ItemDefinition, InventoryItem, DbConnection } from '../generated';
import { Identity } from '@clockworklabs/spacetimedb-sdk';
import { itemIcons } from '../utils/itemIconUtils';

// Style constants similar to PlayerUI
const UI_BG_COLOR = 'rgba(40, 40, 60, 0.85)';
const UI_BORDER_COLOR = '#a0a0c0';
const UI_SHADOW = '2px 2px 0px rgba(0,0,0,0.5)';
const UI_FONT_FAMILY = '"Press Start 2P", cursive';
const SLOT_SIZE = 60; // Size of each hotbar slot in pixels
const SLOT_MARGIN = 6;
const SELECTED_BORDER_COLOR = '#ffffff';

// Define the props the Hotbar component expects
interface HotbarProps {
  playerIdentity: Identity | null;
  itemDefinitions: Map<string, ItemDefinition>;
  inventoryItems: Map<string, InventoryItem>;
  startCampfirePlacement: () => void; // Add prop for starting placement
  cancelCampfirePlacement: () => void; // Add prop for cancelling placement
  connection: DbConnection | null; // Add connection prop
}

const Hotbar: React.FC<HotbarProps> = ({ 
  playerIdentity, 
  itemDefinitions, 
  inventoryItems, 
  startCampfirePlacement, // Receive the prop
  cancelCampfirePlacement, // Receive the prop
  connection // Receive the prop
}) => {
  // console.log("Hotbar Props:", { playerIdentity, itemDefinitions, inventoryItems }); // Log received props
  const [selectedSlot, setSelectedSlot] = useState<number>(0); // 0-indexed (0-5)
  const numSlots = 6;

  // Helper function to find the item for a specific hotbar slot
  const findItemForSlot = (slotIndex: number, identity: Identity | null, items: Map<string, InventoryItem>): InventoryItem | null => {
    if (!identity) return null;
    for (const item of items.values()) {
      if (item.playerIdentity.isEqual(identity) && item.hotbarSlot === slotIndex) {
        return item;
      }
    }
    return null;
  };

  // Find the currently selected item
  const selectedInventoryItem = findItemForSlot(selectedSlot, playerIdentity, inventoryItems);
  const selectedItemDef = selectedInventoryItem ? itemDefinitions.get(selectedInventoryItem.itemDefId.toString()) : null;

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const keyNum = parseInt(event.key);
      if (!isNaN(keyNum) && keyNum >= 1 && keyNum <= numSlots) {
        const newSlotIndex = keyNum - 1; // Adjust to 0-based index
        setSelectedSlot(newSlotIndex);

        const itemInNewSlot = findItemForSlot(newSlotIndex, playerIdentity, inventoryItems);
        const itemDefInNewSlot = itemInNewSlot ? itemDefinitions.get(itemInNewSlot.itemDefId.toString()) : null;

        if (connection?.reducers) {
          if (itemDefInNewSlot?.name === 'Camp Fire') {
            // Special case: Campfire - Start placement, unequip others
            console.log(`Hotbar Key ${keyNum}: Starting campfire placement.`);
            startCampfirePlacement(); 
            try {
              connection.reducers.unequipItem();
            } catch (err) {
              console.error("Error calling unequipItem for campfire selection:", err);
            }
          } else if (itemInNewSlot && itemDefInNewSlot?.isEquippable) {
            // Equippable item (not campfire) - Equip it, cancel placement
            console.log(`Hotbar Key ${keyNum}: Equipping item instance ${itemInNewSlot.instanceId} (${itemDefInNewSlot.name})`);
            cancelCampfirePlacement(); // Cancel placement if switching to a tool
            try {
              connection.reducers.equipItem(BigInt(itemInNewSlot.instanceId)); 
            } catch (err) {
              console.error("Error calling equipItem reducer:", err);
            }
          } else {
            // Empty slot or non-equippable item - Unequip, cancel placement
            console.log(`Hotbar Key ${keyNum}: Slot empty or non-equippable, unequipping.`);
            cancelCampfirePlacement();
            try {
              connection.reducers.unequipItem();
            } catch (err) {
              console.error("Error calling unequipItem reducer:", err);
            }
          }
        } else {
           console.warn("Cannot equip/unequip: Connection or reducers not available.");
        }
      }
    };

    const handleWheel = (event: WheelEvent) => {
      event.preventDefault();
      const delta = Math.sign(event.deltaY);
      let newSlotIndex = -1;

      setSelectedSlot(prev => {
        let nextSlot = prev + delta;
        if (nextSlot < 0) nextSlot = numSlots - 1;
        else if (nextSlot >= numSlots) nextSlot = 0;
        newSlotIndex = nextSlot;
        return nextSlot;
      });

      // Use setTimeout to ensure state update is processed before checking item
      setTimeout(() => {
        if (newSlotIndex !== -1 && connection?.reducers) {
          const itemInNewSlot = findItemForSlot(newSlotIndex, playerIdentity, inventoryItems);
          const itemDefInNewSlot = itemInNewSlot ? itemDefinitions.get(itemInNewSlot.itemDefId.toString()) : null;

          if (itemDefInNewSlot?.name === 'Camp Fire') {
            console.log(`Hotbar Wheel: Starting campfire placement (Slot ${newSlotIndex + 1}).`);
            startCampfirePlacement();
            try {
              connection.reducers.unequipItem();
            } catch (err) {
              console.error("Error calling unequipItem for campfire selection:", err);
            }
          } else if (itemInNewSlot && itemDefInNewSlot?.isEquippable) {
            console.log(`Hotbar Wheel: Equipping item instance ${itemInNewSlot.instanceId} (${itemDefInNewSlot.name}) in slot ${newSlotIndex + 1}`);
            cancelCampfirePlacement();
            try {
              connection.reducers.equipItem(BigInt(itemInNewSlot.instanceId)); 
            } catch (err) {
              console.error("Error calling equipItem reducer:", err);
            }
          } else {
            console.log(`Hotbar Wheel: Slot ${newSlotIndex + 1} empty or non-equippable, unequipping.`);
            cancelCampfirePlacement();
            try {
              connection.reducers.unequipItem();
            } catch (err) {
              console.error("Error calling unequipItem reducer:", err);
            }
          }
        } else if (newSlotIndex !== -1) {
           console.warn("Cannot equip/unequip via wheel: Connection or reducers not available.");
        }
      }, 0);
    };

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('wheel', handleWheel, { passive: false });

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('wheel', handleWheel);
    };
    // Add dependencies: Need to re-run if inventory/definitions change to correctly check the new item
  }, [numSlots, playerIdentity, inventoryItems, itemDefinitions, startCampfirePlacement, cancelCampfirePlacement]);

  // --- Click Handler for Slots --- 
  const handleSlotClick = (index: number) => {
      setSelectedSlot(index);
      const clickedItem = findItemForSlot(index, playerIdentity, inventoryItems);
      const clickedItemDef = clickedItem ? itemDefinitions.get(clickedItem.itemDefId.toString()) : null;

      if (connection?.reducers) {
        if (clickedItemDef?.name === 'Camp Fire') {
          console.log(`Hotbar Click: Starting campfire placement (Slot ${index + 1}).`);
          startCampfirePlacement();
          try {
            connection.reducers.unequipItem();
          } catch (err) {
            console.error("Error calling unequipItem for campfire selection:", err);
          }
        } else if (clickedItem && clickedItemDef?.isEquippable) {
          console.log(`Hotbar Click: Equipping item instance ${clickedItem.instanceId} (${clickedItemDef.name}) in slot ${index + 1}`);
          cancelCampfirePlacement();
          try {
            connection.reducers.equipItem(BigInt(clickedItem.instanceId)); 
          } catch (err) {
            console.error("Error calling equipItem reducer:", err);
          }
        } else {
          console.log(`Hotbar Click: Slot ${index + 1} empty or non-equippable, unequipping.`);
          cancelCampfirePlacement();
          try {
            connection.reducers.unequipItem();
          } catch (err) {
            console.error("Error calling unequipItem reducer:", err);
          }
        }
      } else {
         console.warn("Cannot equip/unequip: Connection or reducers not available.");
      }
  };

  return (
    <div style={{
      position: 'fixed',
      bottom: '15px',
      left: '50%',
      transform: 'translateX(-50%)',
      display: 'flex',
      backgroundColor: UI_BG_COLOR,
      padding: `${SLOT_MARGIN}px`,
      borderRadius: '4px',
      border: `1px solid ${UI_BORDER_COLOR}`,
      boxShadow: UI_SHADOW,
      fontFamily: UI_FONT_FAMILY,
    }}>
      {Array.from({ length: numSlots }).map((_, index) => {
        // console.log(`Rendering slot ${index}`); // Optional: Log each slot rendering
        // Find the inventory item for this slot
        const inventoryItem = findItemForSlot(index, playerIdentity, inventoryItems);
        // Find the corresponding item definition if an item exists
        const itemDef = inventoryItem ? itemDefinitions.get(inventoryItem.itemDefId.toString()) : null;

        // console.log(`Slot ${index}: InvItem=`, inventoryItem, "ItemDef=", itemDef); // Log lookup results

        return (
          <div
            key={index}
            onClick={() => handleSlotClick(index)} // Add onClick handler
            style={{
              width: `${SLOT_SIZE}px`,
              height: `${SLOT_SIZE}px`,
              border: `2px solid ${index === selectedSlot ? SELECTED_BORDER_COLOR : UI_BORDER_COLOR}`,
              backgroundColor: 'rgba(0, 0, 0, 0.3)', // Inner slot background
              borderRadius: '3px',
              marginLeft: index > 0 ? `${SLOT_MARGIN}px` : '0px',
              position: 'relative', // Needed for absolute positioning of the number
              display: 'flex',
              justifyContent: 'center',
              alignItems: 'center',
              transition: 'border-color 0.1s ease-in-out', // Smooth selection transition
              boxSizing: 'border-box', // Include border in size calculation
              cursor: 'pointer', // Add pointer cursor
            }}
          >
            {/* Slot Number */}
            <span style={{
              position: 'absolute',
              bottom: '2px',
              right: '4px',
              fontSize: '10px',
              color: 'rgba(255, 255, 255, 0.7)',
              userSelect: 'none', // Prevent text selection
            }}>
              {index + 1}
            </span>
            {/* Item content will go here later */}
            {itemDef && (
              <>
                <img 
                  src={itemIcons[itemDef.iconAssetName] || ''}
                  alt={itemDef.name}
                  title={`${itemDef.name}${itemDef.description ? ' - ' + itemDef.description : ''}`}
                  style={{ 
                    width: '75%',  // Adjust size as needed
                    height: '75%',
                    objectFit: 'contain', // Prevent stretching
                    imageRendering: 'pixelated', // Keep pixel art sharp
                  }} 
                />
                {/* Display quantity if stackable and > 1 */} 
                {itemDef.isStackable && inventoryItem && inventoryItem.quantity > 1 && (
                    <span style={{
                         position: 'absolute',
                         bottom: '2px',
                         left: '4px',
                         fontSize: '10px',
                         color: 'rgba(255, 255, 255, 0.9)',
                         backgroundColor: 'rgba(0, 0, 0, 0.5)', // slight background for readability
                         padding: '0 2px',
                         borderRadius: '2px',
                         userSelect: 'none',
                    }}>
                        {inventoryItem.quantity}
                    </span>
                )}
              </>
            )}
          </div>
        );
      })}
    </div>
  );
};

export default Hotbar; 