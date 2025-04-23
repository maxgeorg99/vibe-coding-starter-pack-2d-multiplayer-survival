/**
 * App.tsx
 * 
 * Main application component.
 * Handles:
 *  - Initializing all core application hooks (connection, tables, placement, drag/drop, interaction).
 *  - Managing top-level application state (connection status, registration status).
 *  - Conditionally rendering either the `LoginScreen` or the main `GameScreen`.
 *  - Displaying global errors (connection, UI, etc.).
 *  - Passing down necessary state and action callbacks to the active screen (`LoginScreen` or `GameScreen`).
 */

import { useState, useEffect, useRef, useCallback } from 'react';

// Components
import LoginScreen from './components/LoginScreen';
import GameScreen from './components/GameScreen';

// Hooks
import { useSpacetimeConnection } from './hooks/useSpacetimeConnection';
import { useSpacetimeTables } from './hooks/useSpacetimeTables';
import { usePlacementManager } from './hooks/usePlacementManager';
import { useDragDropManager } from './hooks/useDragDropManager';
import { useInteractionManager } from './hooks/useInteractionManager';

// Assets & Styles
import './App.css';

function App() {
    // --- Core Hooks --- 
    const {
        connection,
        isLoading: hookIsLoading, // Renamed to avoid clash with app-level loading/registering
        error: connectionError,
        registerPlayer,
        updatePlayerPosition,
        callSetSprintingReducer,
        callJumpReducer,
    } = useSpacetimeConnection();

    const [placementState, placementActions] = usePlacementManager(connection);
    const { placementInfo, placementError } = placementState; // Destructure state
    const { cancelPlacement, startPlacement } = placementActions; // Destructure actions

    const { interactingWith, handleSetInteractingWith } = useInteractionManager();

    const { draggedItemInfo, dropError, handleItemDragStart, handleItemDrop } = useDragDropManager({ connection, interactingWith });

    const { 
      players, trees, stones, campfires, mushrooms, itemDefinitions, 
      inventoryItems, worldState, activeEquipments, droppedItems, 
      woodenStorageBoxes, recipes, craftingQueueItems, localPlayerRegistered 
    } = useSpacetimeTables({ connection, cancelPlacement }); // Pass cancelPlacement needed by table callbacks

    // --- App-Level State --- 
    const [appIsConnected, setAppIsConnected] = useState<boolean>(false); // Tracks if player is registered & game ready
    const [username, setUsername] = useState<string>('');
    const [isRegistering, setIsRegistering] = useState<boolean>(false); // Tracks registration attempt
    const [uiError, setUiError] = useState<string | null>(null); // For general UI errors not handled by hooks

    // --- Refs for Cross-Hook/Component Communication --- 
    // Ref for Placement cancellation needed by useSpacetimeTables callbacks
    const cancelPlacementActionRef = useRef(cancelPlacement);
    useEffect(() => {
        cancelPlacementActionRef.current = cancelPlacement;
    }, [cancelPlacement]);
    // Ref for placementInfo needed for global context menu effect
    const placementInfoRef = useRef(placementInfo);
    useEffect(() => {
        placementInfoRef.current = placementInfo;
    }, [placementInfo]);

    // --- Effect to Sync App Connection State with Table Hook Registration State ---
    useEffect(() => {
        if (localPlayerRegistered) {
            // console.log("[App Sync Effect] Local player registered, setting app connected.");
            if (!appIsConnected) setAppIsConnected(true); // Set connected only if not already
            if (isRegistering) setIsRegistering(false); // Stop registering if registration confirmed
        } else {
            if (appIsConnected) { // Only change if previously connected
                // console.log("[App Sync Effect] Local player unregistered, setting app disconnected.");
                setAppIsConnected(false);
                // Player is gone, ensure registering state is also false
                if (isRegistering) setIsRegistering(false);
            }
        }
        // Depend only on the flag from the hook and the app's own states
    }, [localPlayerRegistered, appIsConnected, isRegistering]);

    // --- Action Handlers --- 
    const handleAttemptRegisterPlayer = useCallback(() => {
        setUiError(null);
        setIsRegistering(true);
        registerPlayer(username);
    }, [registerPlayer, username]);

    // --- Global Window Effects --- 
    useEffect(() => {
        // Prevent global context menu unless placing item
        const handleGlobalContextMenu = (event: MouseEvent) => {
            if (!placementInfoRef.current) { // Use ref to check current placement status
                event.preventDefault();
            }
        };
        window.addEventListener('contextmenu', handleGlobalContextMenu);
        return () => {
            window.removeEventListener('contextmenu', handleGlobalContextMenu);
        };
    }, []); // Empty dependency array: run only once on mount

    // --- Error Display Logic --- 
    // Combine potential errors from different sources for a single display point
    const displayError = connectionError || uiError || placementError || dropError;

    // --- Render Logic --- 
    return (
        <div className="App" style={{ backgroundColor: '#111' }}>
            {/* Display combined errors */} 
            {displayError && <div className="error-message">{displayError}</div>}

            {/* Conditional Rendering: Login vs Game */} 
            {!appIsConnected ? (
                // --- Render Login Screen Component --- 
                <LoginScreen
                    username={username}
                    setUsername={setUsername}
                    handleLogin={handleAttemptRegisterPlayer} // Pass the registration handler
                    isLoading={hookIsLoading || isRegistering} // Combine loading states
                    error={connectionError} // Pass only connection errors here
                />
            ) : (
                // --- Render Game Screen Component --- 
                <GameScreen 
                    // Pass all necessary state and actions down as props
                    players={players}
                    trees={trees}
                    stones={stones}
                    campfires={campfires}
                    mushrooms={mushrooms}
                    droppedItems={droppedItems}
                    woodenStorageBoxes={woodenStorageBoxes}
                    inventoryItems={inventoryItems}
                    itemDefinitions={itemDefinitions}
                    worldState={worldState}
                    activeEquipments={activeEquipments}
                    recipes={recipes}
                    craftingQueueItems={craftingQueueItems}
                    localPlayerId={connection?.identity?.toHexString() ?? undefined}
                    playerIdentity={connection?.identity || null}
                    connection={connection}
                    placementInfo={placementInfo}
                    placementActions={placementActions}
                    placementError={placementError}
                    startPlacement={startPlacement}
                    cancelPlacement={cancelPlacement}
                    interactingWith={interactingWith}
                    handleSetInteractingWith={handleSetInteractingWith}
                    draggedItemInfo={draggedItemInfo}
                    onItemDragStart={handleItemDragStart}
                    onItemDrop={handleItemDrop}
                    updatePlayerPosition={updatePlayerPosition}
                    callJumpReducer={callJumpReducer}
                    callSetSprintingReducer={callSetSprintingReducer}
                />
            )}
        </div>
    );
}

export default App;
