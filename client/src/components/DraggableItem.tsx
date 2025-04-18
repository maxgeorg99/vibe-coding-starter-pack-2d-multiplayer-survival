import React, { useRef, useState, useEffect, useCallback } from 'react';
import { PopulatedItem } from './InventoryUI'; // Assuming type is exported from InventoryUI
import { DragSourceSlotInfo, DraggedItemInfo } from '../types/dragDropTypes'; // Correct import path
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
  const didDragRef = useRef(false);

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
    const thresholdSq = 2*2; // Compare squared distances. Lowered from 3*3 for more sensitivity.

    // Update ghost position IF it exists
    if (ghostRef.current) {
        ghostRef.current.style.left = `${e.clientX + 10}px`;
        ghostRef.current.style.top = `${e.clientY + 10}px`;
    }
    // Create ghost only if threshold is met AND ghost doesn't exist yet
    else if (distSq >= thresholdSq) {
        didDragRef.current = true;
        console.log(`[DraggableItem] Drag threshold met, didDrag = true.`);
        createGhostElement(e, currentSplitQuantity.current);
    }
  }, [createGhostElement]);

  const handleMouseUp = useCallback((e: MouseEvent) => {
    // Capture drag state BEFORE removing listeners / resetting state
    const wasDragging = didDragRef.current;
    console.log(`[DraggableItem MouseUp] Button: ${e.button}, wasDragging: ${wasDragging}`);

    if (!isDraggingRef.current) {
        // Safety check - mouseup when not dragging shouldn't happen often here
        // but ensure listeners are cleaned up if it does.
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
        return; 
    }

    // --- Remove Listeners FIRST --- 
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleMouseUp);

    // --- Determine Drop Target --- 
    let targetSlotInfo: DragSourceSlotInfo | null = null;
    let dropHandledInternal = false; 
    if (ghostRef.current) {
      ghostRef.current.style.display = 'none'; 
      const dropTargetElement = document.elementFromPoint(e.clientX, e.clientY);
      if (dropTargetElement) {
          const droppableSlot = dropTargetElement.closest('[data-slot-type]');
          if (droppableSlot) {
              const targetType = droppableSlot.getAttribute('data-slot-type') as DragSourceSlotInfo['type'];
              const targetIndexAttr = droppableSlot.getAttribute('data-slot-index');
              if (targetType && targetIndexAttr !== null) {
                   const targetIndex: number | string = (targetType === 'inventory' || targetType === 'hotbar' || targetType === 'campfire_fuel') 
                                                      ? parseInt(targetIndexAttr, 10) 
                                                      : targetIndexAttr; 
                  if (!isNaN(targetIndex as number) || typeof targetIndex === 'string') { 
                      targetSlotInfo = { type: targetType, index: targetIndex };
                      if (!(sourceSlot.type === targetSlotInfo.type && sourceSlot.index === targetSlotInfo.index)) { 
                           dropHandledInternal = true;
                       } else {
                           console.log("[DraggableItem] Drop on source slot ignored (no action needed).");
                           dropHandledInternal = true; 
                           targetSlotInfo = null; 
                       }
                  }
              }
          } 
      }
       if (ghostRef.current && document.body.contains(ghostRef.current)) { 
        document.body.removeChild(ghostRef.current);
      }
      ghostRef.current = null;
    } else {
        console.log("[DraggableItem] MouseUp without significant drag/ghost.");
    }
    // --- End Drop Target Determination ---

    // --- NEW Decision Logic --- 
    if (e.button === 2) { // Right Button Release
        if (wasDragging) {
            // Right-DRAG: Perform the drop action (split/merge)
            console.log("[DraggableItem MouseUp] Right-DRAG detected. Calling onItemDrop.");
             if (dropHandledInternal) {
                onItemDrop(targetSlotInfo); 
            } else {
                onItemDrop(null); // Dropped outside
            }
        } else {
            // Right-CLICK: Perform the context menu action
            console.log("[DraggableItem MouseUp] Right-CLICK detected. Calling onContextMenu prop.");
            if (onContextMenu) {
                // We might need to pass a simulated event if the handler expects it,
                // but for now, let's pass null or a minimal object. 
                // Pass the original mouse event `e` for position info if needed.
                onContextMenu(e as any, item); // Call the prop function
            }
        }
    } else { // Left or Middle Button Release
        console.log("[DraggableItem MouseUp] Left/Middle drag release. Calling onItemDrop.");
        if (dropHandledInternal) {
            onItemDrop(targetSlotInfo); 
        } else {
            onItemDrop(null);
        }
    }
    // --- End Decision Logic ---

    // Common Cleanup (Visuals, Dragging State)
    isDraggingRef.current = false;
    setIsDraggingState(false);
    document.body.classList.remove('item-dragging');
    if (itemRef.current) {
         itemRef.current.style.opacity = '1';
    }

  }, [handleMouseMove, item, sourceSlot, onItemDrop, onContextMenu]);

  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    // --- RESTORE Resetting didDrag flag --- 
    didDragRef.current = false;
    // --- END RESTORE ---

    // --- NEW: Prevent default for right-click --- 
    if (e.button === 2) {
        console.log('[DraggableItem MouseDown] Right button pressed, preventing default.');
        e.preventDefault(); // Attempt to suppress native context menu
    }
    // --- END NEW ---

    // Check stackability and quantity for splitting possibility
    const canSplit = item.definition.isStackable && item.instance.quantity > 1;
    let splitQuantity: number | null = null;
    if (canSplit) {
        if (e.button === 1) { // Middle mouse button
            e.preventDefault(); 
            if (e.shiftKey) {
                splitQuantity = Math.max(1, Math.floor(item.instance.quantity / 3));
            } else {
                splitQuantity = Math.max(1, Math.floor(item.instance.quantity / 2));
            }
        } else if (e.button === 2) { // Right mouse button
            e.preventDefault(); 
            splitQuantity = 1;
        }
    }
    if (e.button === 0 || !splitQuantity) {
         currentSplitQuantity.current = null; 
         if (e.button === 0) e.preventDefault(); 
    } else {
         currentSplitQuantity.current = splitQuantity; 
    }
    e.stopPropagation();
    isDraggingRef.current = true;
    setIsDraggingState(true); 
    dragStartPos.current = { x: e.clientX, y: e.clientY };
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    const dragInfo: DraggedItemInfo = { item, sourceSlot, splitQuantity: splitQuantity ?? undefined };
    onItemDragStart(dragInfo);

  }, [item, sourceSlot, onItemDragStart, handleMouseMove, handleMouseUp, createGhostElement]);

  // Basic rendering of the item
  return (
    <div 
      ref={itemRef}
      className={`${styles.draggableItem} ${isDraggingState ? styles.isDraggingFeedback : ''}`}
      onMouseDown={handleMouseDown}
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