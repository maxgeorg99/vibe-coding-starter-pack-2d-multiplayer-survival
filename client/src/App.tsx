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
import { useDebouncedCallback } from 'use-debounce';
import StoryTextbox from "./components/StoryTextbox.tsx"; // Import debounce helper

// Viewport constants
const VIEWPORT_WIDTH = 1200; // Example: Base viewport width
const VIEWPORT_HEIGHT = 800; // Example: Base viewport height
const VIEWPORT_BUFFER = 1200; // Increased buffer (was 600) to create larger "chunks" of visible area
const VIEWPORT_UPDATE_THRESHOLD_SQ = (VIEWPORT_WIDTH / 2) ** 2; // Increased threshold (was WIDTH/4), so updates happen less frequently
const VIEWPORT_UPDATE_DEBOUNCE_MS = 750; // Increased debounce time (was 250ms) to reduce update frequency

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
        callUpdateViewportReducer, // Get the new reducer function
    } = useSpacetimeConnection();

    const [placementState, placementActions] = usePlacementManager(connection);
    const { placementInfo, placementError } = placementState; // Destructure state
    const { cancelPlacement, startPlacement } = placementActions; // Destructure actions

    const { interactingWith, handleSetInteractingWith } = useInteractionManager();

    const { draggedItemInfo, dropError, handleItemDragStart, handleItemDrop } = useDragDropManager({ connection, interactingWith });

    // Add state for story completion
    const [storyCompleted, setStoryCompleted] = useState(false);

    // --- App-Level State --- 
    const [appIsConnected, setAppIsConnected] = useState<boolean>(false); // Tracks if player is registered & game ready
    const [username, setUsername] = useState<string>('');
    const [isRegistering, setIsRegistering] = useState<boolean>(false); // Tracks registration attempt
    const [uiError, setUiError] = useState<string | null>(null); // For general UI errors not handled by hooks
    const [isMinimapOpen, setIsMinimapOpen] = useState<boolean>(false);
    const [isChatting, setIsChatting] = useState<boolean>(false); // <<< Add isChatting state

    // --- Viewport State & Refs ---
    const [currentViewport, setCurrentViewport] = useState<{ minX: number, minY: number, maxX: number, maxY: number } | null>(null);
    const lastSentViewportCenterRef = useRef<{ x: number, y: number } | null>(null);
    const localPlayerRef = useRef<any>(null); // Ref to hold local player data

    // --- Pass viewport state to useSpacetimeTables ---
    const { 
      players, trees, stones, campfires, mushrooms, itemDefinitions, 
      inventoryItems, worldState, activeEquipments, droppedItems, 
      woodenStorageBoxes, recipes, craftingQueueItems, localPlayerRegistered,
      messages,
      playerPins, // Destructure playerPins
    } = useSpacetimeTables({ 
        connection, 
        cancelPlacement, 
        viewport: currentViewport // Make sure viewport is passed correctly
    });

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

    // --- Debounced Viewport Update ---
    const debouncedUpdateViewport = useDebouncedCallback(
        (vp: { minX: number, minY: number, maxX: number, maxY: number }) => {
            // console.log(`[App] Calling debounced server viewport update: ${JSON.stringify(vp)}`);
            callUpdateViewportReducer(vp.minX, vp.minY, vp.maxX, vp.maxY);
            lastSentViewportCenterRef.current = { x: (vp.minX + vp.maxX) / 2, y: (vp.minY + vp.maxY) / 2 };
        },
        VIEWPORT_UPDATE_DEBOUNCE_MS
    );

    // --- Effect to Update Viewport Based on Player Position ---
    useEffect(() => {
        const localPlayer = connection?.identity ? players.get(connection.identity.toHexString()) : undefined;
        localPlayerRef.current = localPlayer; // Update ref whenever local player changes

        // If player is gone, dead, or not fully connected yet, clear viewport
        if (!localPlayer || localPlayer.isDead) {
             if (currentViewport) setCurrentViewport(null);
             // Consider if we need to tell the server the viewport is invalid?
             // Server might time out old viewports anyway.
             return;
        }

        const playerCenterX = localPlayer.positionX;
        const playerCenterY = localPlayer.positionY;

        // Check if viewport center moved significantly enough
        const lastSentCenter = lastSentViewportCenterRef.current;
        const shouldUpdate = !lastSentCenter ||
            (playerCenterX - lastSentCenter.x)**2 + (playerCenterY - lastSentCenter.y)**2 > VIEWPORT_UPDATE_THRESHOLD_SQ;

        if (shouldUpdate) {
            const newMinX = playerCenterX - (VIEWPORT_WIDTH / 2) - VIEWPORT_BUFFER;
            const newMaxX = playerCenterX + (VIEWPORT_WIDTH / 2) + VIEWPORT_BUFFER;
            const newMinY = playerCenterY - (VIEWPORT_HEIGHT / 2) - VIEWPORT_BUFFER;
            const newMaxY = playerCenterY + (VIEWPORT_HEIGHT / 2) + VIEWPORT_BUFFER;
            const newViewport = { minX: newMinX, minY: newMinY, maxX: newMaxX, maxY: newMaxY };

            // console.log(`[App] Viewport needs update. Triggering debounced call.`);
            setCurrentViewport(newViewport); // Update local state immediately for useSpacetimeTables
            debouncedUpdateViewport(newViewport); // Call debounced server update
        }
    // Depend on the players map (specifically the local player's position), connection identity, and app connected status.
    }, [players, connection?.identity, debouncedUpdateViewport]); // Removed currentViewport dependency to avoid loops

    // --- Effect to Sync App Connection State with Table Hook Registration State ---
    useEffect(() => {
        // console.log(`[App Sync Revert] Running. localPlayerRegistered: ${localPlayerRegistered}, appIsConnected: ${appIsConnected}, isRegistering: ${isRegistering}, hookIsLoading: ${hookIsLoading}`);

        // Reverted Logic: Check registration status first
        if (localPlayerRegistered) {
            // If registered and not connected, connect.
            if (!appIsConnected) {
                // console.log("[App Sync Revert] Player registered, setting appIsConnected = true");
                setAppIsConnected(true);
            }
            // If registered and currently in the registering process, mark registering as done.
            if (isRegistering) {
                // console.log("[App Sync Revert] Player registered, setting isRegistering = false");
                setIsRegistering(false);
            }
        } else {
            // Player is not registered according to the tables hook.
            // If we *were* connected, this means a disconnect or server cleanup.
            // We should transition back to the login screen.
            // *** However, this is where the original bug was - a dead player might also cause `localPlayerRegistered` to become false. ***
            // For now, let's comment out the disconnection logic here and rely on the GameScreen to handle the 'dead' state.
            // A more robust solution would involve checking the actual connection status from useSpacetimeConnection.
            /*
            if (appIsConnected) { // Only change if previously connected
                console.log("[App Sync Revert] Local player unregistered, setting app disconnected. (COMMENTED OUT)");
                // setAppIsConnected(false);
                // if (isRegistering) setIsRegistering(false);
                // setCurrentViewport(null);
                // lastSentViewportCenterRef.current = null;
            }
            */
        }

    // Keep dependencies minimal: only trigger when registration status changes or registering state changes.
    // hookIsLoading might not be necessary if the core connection logic handles its own loading state internally.
    // Let's remove hookIsLoading for now to simplify.
    }, [localPlayerRegistered, isRegistering]); // Removed appIsConnected and hookIsLoading

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

    const handleStoryComplete = () => {
        setStoryCompleted(true);
    };

    // --- Effect to handle global key presses that aren't directly game actions ---
    useEffect(() => {
        const handleGlobalKeyDown = (event: KeyboardEvent) => {
            // If chat is active, let the Chat component handle Enter/Escape
            if (isChatting) return;

            // Prevent global context menu unless placing item (moved from other effect)
            if (event.key === 'ContextMenu' && !placementInfoRef.current) {
                event.preventDefault();
            }

            // Other global keybinds could go here if needed
        };

        // Prevent global context menu unless placing item (separate listener for clarity)
        const handleGlobalContextMenu = (event: MouseEvent) => {
            if (!placementInfoRef.current) { // Use ref to check current placement status
                event.preventDefault();
            }
        };

        window.addEventListener('keydown', handleGlobalKeyDown);
        window.addEventListener('contextmenu', handleGlobalContextMenu);

        return () => {
            window.removeEventListener('keydown', handleGlobalKeyDown);
            window.removeEventListener('contextmenu', handleGlobalContextMenu);
        };
    }, [isChatting]); // <<< Add isChatting dependency

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
            ) : !storyCompleted ? (
                // --- Render Story Prolog ---
                <StoryTextbox onComplete={handleStoryComplete} />
                ) : (
                // --- Render Game Screen Component --- 
                // Only render GameScreen if viewport is calculated (REMOVING THIS CONDITION)
                // currentViewport && (
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
                      playerPins={playerPins} // Pass playerPins down
                      draggedItemInfo={draggedItemInfo}
                      onItemDragStart={handleItemDragStart}
                      onItemDrop={handleItemDrop}
                      updatePlayerPosition={updatePlayerPosition}
                      callJumpReducer={callJumpReducer}
                      callSetSprintingReducer={callSetSprintingReducer}
                      isMinimapOpen={isMinimapOpen}
                      setIsMinimapOpen={setIsMinimapOpen}
                      isChatting={isChatting} // Pass isChatting state
                      setIsChatting={setIsChatting} // Pass isChatting setter
                      messages={messages} // Pass messages map
                  />
                // )
            )}
        </div>
    );
}

export default App;
