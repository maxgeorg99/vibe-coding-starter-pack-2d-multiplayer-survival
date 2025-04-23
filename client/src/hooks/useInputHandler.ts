import { useEffect, useRef, useState, useCallback, RefObject } from 'react';
import * as SpacetimeDB from '../generated';
import { DbConnection } from '../generated';
import { PlacementItemInfo, PlacementActions } from './usePlacementManager'; // Assuming usePlacementManager exports these

// --- Constants (Copied from GameCanvas) ---
const HOLD_INTERACTION_DURATION_MS = 250;
const SWING_COOLDOWN_MS = 500;

// --- Hook Props Interface ---
interface UseInputHandlerProps {
    canvasRef: RefObject<HTMLCanvasElement | null>;
    connection: DbConnection | null;
    isInputDisabled: boolean; // Whether input should be processed (e.g., player dead)
    localPlayerId?: string;
    localPlayer?: SpacetimeDB.Player | null; // Pass the local player data
    activeEquipments?: Map<string, SpacetimeDB.ActiveEquipment>; // Pass active equipment map
    placementInfo: PlacementItemInfo | null;
    placementActions: PlacementActions;
    worldMousePos: { x: number | null; y: number | null }; // Pass world mouse position
    // Closest interactables (passed in for now)
    closestInteractableMushroomId: bigint | null;
    closestInteractableCampfireId: number | null;
    closestInteractableDroppedItemId: bigint | null;
    closestInteractableBoxId: number | null;
    isClosestInteractableBoxEmpty: boolean;
    // Callbacks for actions
    onSetInteractingWith: (target: { type: string; id: number | bigint } | null) => void;
    updatePlayerPosition: (dx: number, dy: number, intendedDirection?: 'up' | 'down' | 'left' | 'right' | null) => void;
    callJumpReducer: () => void;
    callSetSprintingReducer: (isSprinting: boolean) => void;
    // Note: attemptSwing logic will be internal to the hook
}

// --- Hook Return Value Interface ---
interface InputHandlerState {
    // State needed for rendering or other components
    interactionProgress: { targetId: number | bigint | null; startTime: number } | null;
    isSprinting: boolean; // Expose current sprint state if needed elsewhere
    // Function to be called each frame by the game loop
    processInputsAndActions: () => void;
}

export const useInputHandler = ({
    canvasRef,
    connection,
    isInputDisabled,
    localPlayerId,
    localPlayer,
    activeEquipments,
    placementInfo,
    placementActions,
    worldMousePos,
    closestInteractableMushroomId,
    closestInteractableCampfireId,
    closestInteractableDroppedItemId,
    closestInteractableBoxId,
    isClosestInteractableBoxEmpty,
    onSetInteractingWith,
    updatePlayerPosition,
    callJumpReducer,
    callSetSprintingReducer,
}: UseInputHandlerProps): InputHandlerState => {
    // --- Internal State and Refs ---
    const keysPressed = useRef<Set<string>>(new Set());
    const isSprintingRef = useRef<boolean>(false);
    const isEHeldDownRef = useRef<boolean>(false);
    const isMouseDownRef = useRef<boolean>(false);
    const lastClientSwingAttemptRef = useRef<number>(0);
    const eKeyDownTimestampRef = useRef<number>(0);
    const eKeyHoldTimerRef = useRef<number | null>(null); // Use number for browser timeout ID
    const [interactionProgress, setInteractionProgress] = useState<{ targetId: number | bigint | null; startTime: number } | null>(null);

    // Refs for dependencies to avoid re-running effect too often
    const placementActionsRef = useRef(placementActions);
    const connectionRef = useRef(connection);
    const localPlayerRef = useRef(localPlayer);
    const activeEquipmentsRef = useRef(activeEquipments);
    const closestIdsRef = useRef({
        mushroom: closestInteractableMushroomId,
        campfire: closestInteractableCampfireId,
        droppedItem: closestInteractableDroppedItemId,
        box: closestInteractableBoxId,
        boxEmpty: isClosestInteractableBoxEmpty,
    });
    const onSetInteractingWithRef = useRef(onSetInteractingWith);
    const worldMousePosRefInternal = useRef(worldMousePos); // Shadow prop name

    // Update refs when props change
    useEffect(() => { placementActionsRef.current = placementActions; }, [placementActions]);
    useEffect(() => { connectionRef.current = connection; }, [connection]);
    useEffect(() => { localPlayerRef.current = localPlayer; }, [localPlayer]);
    useEffect(() => { activeEquipmentsRef.current = activeEquipments; }, [activeEquipments]);
    useEffect(() => {
        closestIdsRef.current = {
            mushroom: closestInteractableMushroomId,
            campfire: closestInteractableCampfireId,
            droppedItem: closestInteractableDroppedItemId,
            box: closestInteractableBoxId,
            boxEmpty: isClosestInteractableBoxEmpty,
        };
    }, [closestInteractableMushroomId, closestInteractableCampfireId, closestInteractableDroppedItemId, closestInteractableBoxId, isClosestInteractableBoxEmpty]);
    useEffect(() => { onSetInteractingWithRef.current = onSetInteractingWith; }, [onSetInteractingWith]);
    useEffect(() => { worldMousePosRefInternal.current = worldMousePos; }, [worldMousePos]);

    // --- Swing Logic --- 
    const attemptSwing = useCallback(() => {
        const currentConnection = connectionRef.current;
        if (!currentConnection?.reducers || !localPlayerId) return;

        const currentEquipments = activeEquipmentsRef.current;
        const localEquipment = currentEquipments?.get(localPlayerId);
        if (!localEquipment || localEquipment.equippedItemDefId === null) {
            return;
        }

        const now = Date.now();

        // Client-side cooldown
        if (now - lastClientSwingAttemptRef.current < SWING_COOLDOWN_MS) {
            return;
        }

        // Server-side cooldown check (using equipment state)
        if (now - Number(localEquipment.swingStartTimeMs) < SWING_COOLDOWN_MS) {
            return;
        }

        // Attempt the swing
        try {
            currentConnection.reducers.useEquippedItem();
            lastClientSwingAttemptRef.current = now;
        } catch (err) { // Use unknown type for error
            console.error("[AttemptSwing] Error calling useEquippedItem reducer:", err);
        }
    }, [localPlayerId]); // Only depends on localPlayerId (refs handle the rest)

    // --- Input Handling useEffect (Listeners only) ---
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (isInputDisabled && e.key.toLowerCase() !== 'escape') return;
            const key = e.key.toLowerCase();

            // Placement cancellation
            if (key === 'escape' && placementInfo) {
                placementActionsRef.current?.cancelPlacement();
                return;
            }
            if (isInputDisabled) return; // Block other input if disabled

            // Sprinting start
            if (key === 'shift' && !isSprintingRef.current && !e.repeat) {
                isSprintingRef.current = true;
                callSetSprintingReducer(true);
                return; // Don't add shift to keysPressed
            }

            // Avoid adding modifier keys
            if (key === 'shift' || key === 'control' || key === 'alt' || key === 'meta') {
                return;
            }

            keysPressed.current.add(key);

            // Jump
            if (key === ' ' && !e.repeat) {
                if (localPlayerRef.current) { // Check player exists via ref
                    callJumpReducer();
                }
            }

            // Interaction key ('e')
            if (key === 'e' && !e.repeat && !isEHeldDownRef.current) {
                const currentConnection = connectionRef.current;
                if (!currentConnection?.reducers) return; // Need connection for interactions

                const closest = closestIdsRef.current; // Use ref value
                const { mushroom, campfire, droppedItem, box, boxEmpty } = closest;

                // Priority: DroppedItem > Empty Box > Mushroom > Open Box > Campfire
                if (droppedItem !== null) {
                    try {
                        currentConnection.reducers.pickupDroppedItem(droppedItem);
                    } catch (err) {
                        console.error("Error calling pickupDroppedItem reducer:", err);
                    }
                    return;
                } else if (box !== null) {
                    console.log(`[InputHandler KeyDown E] Starting hold check for Box ID: ${box}. Empty: ${boxEmpty}`);
                    isEHeldDownRef.current = true;
                    eKeyDownTimestampRef.current = Date.now();
                    if (boxEmpty) {
                        setInteractionProgress({ targetId: box, startTime: Date.now() });
                    }
                    if (eKeyHoldTimerRef.current) clearTimeout(eKeyHoldTimerRef.current);
                    eKeyHoldTimerRef.current = setTimeout(() => {
                        if (isEHeldDownRef.current) {
                            const stillClosest = closestIdsRef.current; // Re-check closest box via ref
                            if (stillClosest.box === box && stillClosest.boxEmpty) {
                                console.log(`[InputHandler Hold Timer] Executing pickup for EMPTY Box ID: ${box}`);
                                try {
                                    connectionRef.current?.reducers.pickupStorageBox(box);
                                    isEHeldDownRef.current = false;
                                    setInteractionProgress(null);
                                    if (eKeyHoldTimerRef.current) clearTimeout(eKeyHoldTimerRef.current);
                                    eKeyHoldTimerRef.current = null;
                                } catch (err) {
                                    console.error("[InputHandler Hold Timer] Error calling pickupStorageBox reducer:", err);
                                    isEHeldDownRef.current = false;
                                    setInteractionProgress(null);
                                    if (eKeyHoldTimerRef.current) clearTimeout(eKeyHoldTimerRef.current);
                                    eKeyHoldTimerRef.current = null;
                                }
                            } else {
                                console.log(`[InputHandler Hold Timer] Hold expired, but box ${box} is not empty or no longer closest. No pickup.`);
                                setInteractionProgress(null);
                                if (eKeyHoldTimerRef.current) clearTimeout(eKeyHoldTimerRef.current);
                                eKeyHoldTimerRef.current = null;
                            }
                        }
                    }, HOLD_INTERACTION_DURATION_MS);
                    return;
                } else if (mushroom !== null) {
                    try {
                        currentConnection.reducers.interactWithMushroom(mushroom);
                    } catch (err) {
                        console.error("Error calling interactWithMushroom reducer:", err);
                    }
                    return;
                } else if (campfire !== null) {
                    console.log(`[InputHandler KeyDown E] Starting hold check for Campfire ID: ${campfire}`);
                    isEHeldDownRef.current = true;
                    eKeyDownTimestampRef.current = Date.now();
                    setInteractionProgress({ targetId: campfire, startTime: Date.now() });
                    if (eKeyHoldTimerRef.current) clearTimeout(eKeyHoldTimerRef.current);
                    eKeyHoldTimerRef.current = setTimeout(() => {
                        if (isEHeldDownRef.current) {
                            const stillClosest = closestIdsRef.current; // Re-check via ref
                            if (stillClosest.campfire === campfire) {
                                console.log(`[InputHandler Hold Timer - Campfire] Executing toggle for Campfire ID: ${campfire}`);
                                try {
                                    connectionRef.current?.reducers.toggleCampfireBurning(campfire);
                                } catch (err) { console.error("[InputHandler Hold Timer - Campfire] Error toggling campfire:", err); }
                            }
                            isEHeldDownRef.current = false;
                            setInteractionProgress(null);
                            if (eKeyHoldTimerRef.current) clearTimeout(eKeyHoldTimerRef.current);
                            eKeyHoldTimerRef.current = null;
                        }
                    }, HOLD_INTERACTION_DURATION_MS);
                    return;
                }
            }
        };

        const handleKeyUp = (e: KeyboardEvent) => {
            const key = e.key.toLowerCase();
            // Sprinting end
            if (key === 'shift') {
                if (isSprintingRef.current) {
                    isSprintingRef.current = false;
                    if (!isInputDisabled) {
                        callSetSprintingReducer(false);
                    }
                }
            }
            keysPressed.current.delete(key);

            // Interaction key ('e') up
            if (key === 'e') {
                if (isEHeldDownRef.current) {
                    const closestBeforeClear = { ...closestIdsRef.current }; // Capture state before clearing
                    const holdDuration = Date.now() - eKeyDownTimestampRef.current;

                    isEHeldDownRef.current = false;
                    if (eKeyHoldTimerRef.current) {
                        clearTimeout(eKeyHoldTimerRef.current);
                        eKeyHoldTimerRef.current = null;
                    }
                    setInteractionProgress(null);
                    eKeyDownTimestampRef.current = 0;

                    if (holdDuration < HOLD_INTERACTION_DURATION_MS) {
                        const currentConnection = connectionRef.current;
                        if (!currentConnection?.reducers) return;

                        // Prioritize Box if it was the target. Remove check for emptiness here.
                        if (closestBeforeClear.box !== null) {
                             console.log(`[KeyUp E - Box] Short press detected for Box ID: ${closestBeforeClear.box}. Opening UI.`);
                             try {
                                currentConnection.reducers.interactWithStorageBox(closestBeforeClear.box);
                                onSetInteractingWithRef.current({ type: 'wooden_storage_box', id: closestBeforeClear.box });
                             } catch (err) { console.error("[KeyUp E - Box] Error interacting:", err); }
                        } else if (closestBeforeClear.campfire !== null) {
                             console.log(`[KeyUp E - Campfire] Short press detected for Campfire ID: ${closestBeforeClear.campfire}. Opening UI.`);
                            try {
                                currentConnection.reducers.interactWithCampfire(closestBeforeClear.campfire);
                                onSetInteractingWithRef.current({ type: 'campfire', id: closestBeforeClear.campfire });
                            } catch (err) {
                                console.error("[KeyUp E - Campfire] Error interacting:", err);
                            }
                        }
                    }
                }
            }
        };

        // --- Mouse Handlers ---
        const handleMouseDown = (event: MouseEvent) => {
            if (isInputDisabled || event.button !== 0 || placementInfo) return;
            isMouseDownRef.current = true;
            attemptSwing(); // Call internal swing logic
        };

        const handleMouseUp = (event: MouseEvent) => {
            if (event.button === 0) {
                isMouseDownRef.current = false;
            }
        };

        // --- Canvas Click for Placement ---
        // Note: This needs the *canvas element* to add the listener.
        // It might be better to keep this specific listener in GameCanvas
        // OR pass the canvasRef into this hook.
        // For now, let's assume GameCanvas handles adding this specific listener.
        const handleCanvasClick = (event: MouseEvent) => {
            // This listener is now attached directly to the canvas if ref exists
            if (isInputDisabled || event.button !== 0) return;
            const currentWorldMouse = worldMousePosRefInternal.current;
            if (placementInfo && currentWorldMouse.x !== null && currentWorldMouse.y !== null) {
                 placementActionsRef.current?.attemptPlacement(currentWorldMouse.x, currentWorldMouse.y);
                 return;
            }
            // If not placing, maybe handle other clicks later?
        };

        // --- Context Menu for Placement Cancellation ---
        const handleContextMenu = (event: MouseEvent) => {
            if (placementInfo) {
                event.preventDefault();
                placementActionsRef.current?.cancelPlacement();
            }
            // Prevent default context menu unless placing
            else {
                 event.preventDefault();
            }
        };

        // --- Wheel for Placement Cancellation (optional) ---
        const handleWheel = (event: WheelEvent) => {
            if (placementInfo) {
                placementActionsRef.current?.cancelPlacement();
            }
        };

        // --- Blur Handler ---
        const handleBlur = () => {
            if (isSprintingRef.current) {
                isSprintingRef.current = false;
                if (!isInputDisabled) {
                    callSetSprintingReducer(false);
                }
            }
            keysPressed.current.clear();
            isMouseDownRef.current = false; // Ensure mouse down state is cleared on blur
            isEHeldDownRef.current = false; // Ensure E hold state is cleared
            if(eKeyHoldTimerRef.current) clearTimeout(eKeyHoldTimerRef.current);
            eKeyHoldTimerRef.current = null;
            setInteractionProgress(null);
        };

        // Add global listeners
        window.addEventListener('keydown', handleKeyDown);
        window.addEventListener('keyup', handleKeyUp);
        window.addEventListener('mousedown', handleMouseDown);
        window.addEventListener('mouseup', handleMouseUp);
        window.addEventListener('wheel', handleWheel, { passive: true });
        window.addEventListener('contextmenu', handleContextMenu);
        window.addEventListener('blur', handleBlur);

        // Add listener for canvas click (if canvas ref is passed in)
        const canvas = canvasRef?.current; // Get canvas element from ref
        if (canvas) {
           // Attach the locally defined handler
           canvas.addEventListener('click', handleCanvasClick);
           console.log("[useInputHandler] Added canvas click listener.");
        } else {
            console.warn("[useInputHandler] Canvas ref not available on mount to add click listener.");
        }

        // Cleanup
        return () => {
            window.removeEventListener('keydown', handleKeyDown);
            window.removeEventListener('keyup', handleKeyUp);
            window.removeEventListener('mousedown', handleMouseDown);
            window.removeEventListener('mouseup', handleMouseUp);
            window.removeEventListener('wheel', handleWheel);
            window.removeEventListener('contextmenu', handleContextMenu);
            window.removeEventListener('blur', handleBlur);
            // Remove canvas listener on cleanup
            if (canvas) {
               canvas.removeEventListener('click', handleCanvasClick);
               console.log("[useInputHandler] Removed canvas click listener.");
            }
            // Clear any active timers on cleanup
            if (eKeyHoldTimerRef.current) {
                clearTimeout(eKeyHoldTimerRef.current);
            }
        };
        // Dependencies: Include direct props that influence the logic inside handlers
        // Add canvasRef to dependencies so listener is re-attached if ref changes (unlikely but safe)
    }, [canvasRef, isInputDisabled, placementInfo, callSetSprintingReducer, callJumpReducer, attemptSwing]);

    // --- Function to process inputs and call actions (called by game loop) ---
    const processInputsAndActions = useCallback(() => {
        if (isInputDisabled) return; // Skip if dead

        const localPlayerExists = !!localPlayerRef.current;
        if (!localPlayerExists) return;

        let dx = 0;
        let dy = 0;
        const speed = 5; // Base speed, server calculates actual distance
        let intendedDirection: 'up' | 'down' | 'left' | 'right' | null = null;

        const pressed = keysPressed.current;
        if (pressed.has('w') || pressed.has('arrowup')) { dy = -speed; intendedDirection = 'up'; }
        if (pressed.has('s') || pressed.has('arrowdown')) { dy = speed; intendedDirection = 'down'; }
        if (pressed.has('a') || pressed.has('arrowleft')) { dx = -speed; intendedDirection = 'left'; }
        if (pressed.has('d') || pressed.has('arrowright')) { dx = speed; intendedDirection = 'right'; }

        // Simple override: last direction pressed wins if multiple are held
        if (dx !== 0 && dy !== 0) {
            const magnitude = Math.sqrt(dx * dx + dy * dy);
            dx = (dx / magnitude) * speed;
            dy = (dy / magnitude) * speed;
        }

        // Call updatePlayerPosition (passed as prop)
        updatePlayerPosition(dx, dy, intendedDirection);

        // Handle continuous swing check
        if (isMouseDownRef.current && !placementInfo) { // Only swing if not placing
            attemptSwing(); // Call internal attemptSwing function
        }
    // Include dependencies that are read inside this function
    }, [isInputDisabled, updatePlayerPosition, attemptSwing, placementInfo]);

    // --- Return State & Actions ---
    return {
        interactionProgress,
        isSprinting: isSprintingRef.current, // Return current value of the ref
        processInputsAndActions, // Return the processing function
    };
}; 