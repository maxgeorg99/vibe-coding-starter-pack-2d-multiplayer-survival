import React, { useRef, useState, useEffect, useCallback } from 'react';
import { PopulatedItem } from './InventoryUI'; // Assuming type is exported from InventoryUI
import { DragSourceSlotInfo, DraggedItemInfo } from './PlayerUI'; // Import types from PlayerUI
import { itemIcons, getItemIcon } from '../utils/itemIconUtils';
import styles from './DraggableItem.module.css'; // We'll create this CSS file

interface DraggableItemProps {
  item: PopulatedItem;
  sourceSlot: DragSourceSlotInfo; // Where the item currently is
  onItemDragStart: (info: DraggedItemInfo) => void; // Callback to notify parent
  onItemDrop: (targetSlotInfo: DragSourceSlotInfo | null) => void; // Allow null
  onContextMenu?: (event: React.MouseEvent<HTMLDivElement>, itemInfo: PopulatedItem) => void;
}

const DraggableItem: React.FC<DraggableItemProps> = ({ 
  item, 
  sourceSlot,
  onItemDragStart,
  onItemDrop,
  onContextMenu
}) => {
  const itemRef = useRef<HTMLDivElement>(null);
  const ghostRef = useRef<HTMLDivElement | null>(null);
  const currentSplitQuantity = useRef<number | null>(null); // Ref to hold split qty for ghost
  const [isDraggingState, setIsDraggingState] = useState(false); // State for component re-render/styling
  const isDraggingRef = useRef(false); // Ref for up-to-date state in document listeners
  const dragStartPos = useRef({ x: 0, y: 0 });

  const createGhostElement = useCallback((e: MouseEvent | Touch, splitQuantity: number | null) => {
    console.log(`[DraggableItem] Creating ghost element... Split: ${splitQuantity}`);
    if (ghostRef.current && document.body.contains(ghostRef.current)) {
      document.body.removeChild(ghostRef.current);
    }

    const ghost = document.createElement('div');
    ghost.id = 'drag-ghost';
    ghost.className = styles.dragGhost; // Use CSS module class
    ghost.style.left = `${e.clientX + 10}px`;
    ghost.style.top = `${e.clientY + 10}px`;

    const imgEl = document.createElement('img');
    imgEl.src = getItemIcon(item.definition.iconAssetName) || '';
    imgEl.alt = item.definition.name;
    imgEl.style.width = '40px'; 
    imgEl.style.height = '40px';
    imgEl.style.objectFit = 'contain';
    imgEl.style.imageRendering = 'pixelated';
    ghost.appendChild(imgEl);

    // Display quantity: Either the split quantity or the original quantity
    const displayQuantity = splitQuantity ?? (item.definition.isStackable && item.instance.quantity > 1 ? item.instance.quantity : null);

    if (displayQuantity) {
        const quantityEl = document.createElement('div');
        quantityEl.textContent = displayQuantity.toString();
        quantityEl.className = styles.ghostQuantity; // Use CSS module class
        ghost.appendChild(quantityEl);
    }

    document.body.appendChild(ghost);
    ghostRef.current = ghost;
    console.log("[DraggableItem] Ghost element appended.");
  }, [item]); // Dependency: item (for definition and original quantity)

  const handleMouseMove = useCallback((e: MouseEvent) => {
    // Use the ref to check dragging status
    if (!isDraggingRef.current) return;

    // Basic movement threshold check
    const dx = e.clientX - dragStartPos.current.x;
    const dy = e.clientY - dragStartPos.current.y;
    const distSq = dx*dx + dy*dy;
    const thresholdSq = 3*3; // Compare squared distances

    // Update ghost position IF it exists
    if (ghostRef.current) {
        ghostRef.current.style.left = `${e.clientX + 10}px`;
        ghostRef.current.style.top = `${e.clientY + 10}px`;
    }
    // Create ghost only if threshold is met AND ghost doesn't exist yet
    else if (distSq >= thresholdSq) {
        console.log("[DraggableItem] Drag threshold met, creating ghost.");
        // Pass the current split quantity (might be null) to the ghost creation
        createGhostElement(e, currentSplitQuantity.current);
    }
  }, [createGhostElement]);

  const handleMouseUp = useCallback((e: MouseEvent) => {
    if (!isDraggingRef.current) return;
    console.log("[DraggableItem] handleMouseUp Fired.");

    // Cleanup listeners FIRST
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleMouseUp);

    // --- Begin Restore Drop Logic --- 
    let dropOccurred = false;
    if (ghostRef.current) {
      ghostRef.current.style.display = 'none'; // Hide ghost to check element underneath
      const dropTargetElement = document.elementFromPoint(e.clientX, e.clientY);
      
      if (dropTargetElement) {
          const droppableSlot = dropTargetElement.closest('[data-slot-type]'); // Find nearest slot element
          
          if (droppableSlot) {
              const targetType = droppableSlot.getAttribute('data-slot-type') as DragSourceSlotInfo['type'];
              const targetIndexAttr = droppableSlot.getAttribute('data-slot-index');
              
              if (targetType && targetIndexAttr !== null) {
                  const targetIndex: number | string = (targetType === 'inventory' || targetType === 'hotbar') ? parseInt(targetIndexAttr, 10) : targetIndexAttr;
                  const targetSlotInfo: DragSourceSlotInfo = { type: targetType, index: targetIndex };
                  
                  // Check if drop is not on the source slot itself
                  if (!(sourceSlot.type === targetSlotInfo.type && sourceSlot.index === targetSlotInfo.index)) { 
                        console.log(`[DraggableItem] Found drop target:`, targetSlotInfo);
                        // Call the MAIN drop handler passed from App
                        onItemDrop(targetSlotInfo); 
                        dropOccurred = true;
                   } else {
                       console.log("[DraggableItem] Drop on source slot ignored.");
                   }
              } else {
                   console.log("[DraggableItem] Drop target missing necessary data attributes.");
              }
          } else {
              console.log("[DraggableItem] Dropped outside a valid slot.");
          }
      } else {
           console.log("[DraggableItem] Could not find element at drop point.");
      }

      // Remove ghost from DOM after check
       if (ghostRef.current && document.body.contains(ghostRef.current)) { 
        document.body.removeChild(ghostRef.current);
      }
      ghostRef.current = null;
    }
    // --- End Restore Drop Logic --- 

    // Reset ref and state AFTER handling potential drop
    isDraggingRef.current = false;
    setIsDraggingState(false);
    document.body.classList.remove('item-dragging');
    if (itemRef.current) {
         itemRef.current.style.opacity = '1';
    }
    
    // If drop didn't occur on a valid target, PlayerUI's handler still needs to clear the draggedItemInfo state
    // We now rely on PlayerUI's handleItemDrop to clear the state, which it does.
    if (!dropOccurred) {
       // We need to tell PlayerUI the drop finished invalidly so it clears draggedItemInfo
       // Calling onItemDrop with null or a special flag might be one way.
       // Let's modify PlayerUI's handleItemDrop to clear state even if targetSlot is null.
       console.log("[DraggableItem] Drop invalid/outside. Relying on PlayerUI handler to clear drag state.");
       // Consider calling onItemDrop(null) here if PlayerUI doesn't clear state automatically.
    }

  }, [handleMouseMove, item, sourceSlot, onItemDrop]); // Add back dependencies

  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    // Only proceed if the item is stackable and quantity > 1 for splitting
    const canSplit = item.definition.isStackable && item.instance.quantity > 1;
    let splitQuantity: number | null = null;

    // Determine split quantity based on button and modifiers
    if (canSplit) {
        if (e.button === 1) { // Middle mouse button
            e.preventDefault(); // Prevent default middle-click behavior (like autoscroll)
            if (e.shiftKey) {
                // Shift + Middle: Split 1/3
                splitQuantity = Math.max(1, Math.floor(item.instance.quantity / 3));
                console.log('[DraggableItem] Middle + Shift Down: Splitting 1/3 ->', splitQuantity);
            } else {
                // Middle only: Split half
                splitQuantity = Math.max(1, Math.floor(item.instance.quantity / 2));
                console.log('[DraggableItem] Middle Down: Splitting half ->', splitQuantity);
            }
        } else if (e.button === 2) { // Right mouse button
             // Prevent context menu during drag sequence
            e.preventDefault(); 
            // Right only: Split 1
            splitQuantity = 1;
            console.log('[DraggableItem] Right Down: Splitting 1 ->', splitQuantity);
        }
    }

    // Handle normal left-click drag OR if splitting is not possible/intended
    if (e.button === 0 || !splitQuantity) {
         console.log('[DraggableItem] Left Mouse Down (or cannot split)');
         // Reset split quantity ref if it's a left click
         currentSplitQuantity.current = null; 
         // Prevent default only for left click to allow right-click context menu if needed (handled by onContextMenu prop)
         if (e.button === 0) e.preventDefault(); 
    } else {
         // We are splitting (Middle or Right click)
         currentSplitQuantity.current = splitQuantity; // Store for ghost creation
    }
    
    // Common drag setup logic
    e.stopPropagation();
    isDraggingRef.current = true;
    setIsDraggingState(true); 
    dragStartPos.current = { x: e.clientX, y: e.clientY };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    
    // Pass the calculated split quantity (can be null) to the drag start info
    const dragInfo: DraggedItemInfo = { item, sourceSlot, splitQuantity: splitQuantity ?? undefined };
    console.log('[DraggableItem] Calling onItemDragStart with:', dragInfo);
    onItemDragStart(dragInfo);

  }, [item, sourceSlot, onItemDragStart, handleMouseMove, handleMouseUp, createGhostElement]); // Added createGhostElement dependency

  const handleContextMenu = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    // Prevent context menu IF a drag wasn't initiated by right-click
    if (!isDraggingRef.current) {
        if (onContextMenu) {
            e.preventDefault(); // Prevent browser menu
            e.stopPropagation();
            console.log(`[DraggableItem] Right-click detected on item:`, item);
            onContextMenu(e, item); // Pass event and item info up
        } 
        // else { // Optional: Allow default if no handler provided and not dragging
           // console.log("[DraggableItem] No context menu handler, allowing default.");
        //} 
    } else {
        // If dragging started with right-click, prevent context menu on mouse up
         e.preventDefault(); 
         console.log("[DraggableItem] Preventing context menu because right-drag is active.");
    }
  }, [onContextMenu, item]); // Removed isDraggingRef dependency - it caused issues

  // Basic rendering of the item
  return (
    <div 
      ref={itemRef}
      className={`${styles.draggableItem} ${isDraggingState ? styles.isDraggingFeedback : ''}`}
      onMouseDown={handleMouseDown}
      onContextMenu={handleContextMenu}
      title={`${item.definition.name}${item.definition.description ? ' - ' + item.definition.description : ''}`}
    >
      <img
        src={getItemIcon(item.definition.iconAssetName)}
        alt={item.definition.name}
        className={styles.itemImage}
        draggable="false" // Prevent native image drag
      />
      {item.definition.isStackable && item.instance.quantity > 1 && (
        <div className={styles.itemQuantity}>{item.instance.quantity}</div>
      )}
    </div>
  );
};

export default DraggableItem; 