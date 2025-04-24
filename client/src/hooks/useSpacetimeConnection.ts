import { useState, useEffect, useCallback } from 'react';
import { Identity as SpacetimeDBIdentity } from '@clockworklabs/spacetimedb-sdk';
import { DbConnection } from '../generated';

// SpacetimeDB connection parameters (Keep these configurable or move to a config file later)
const SPACETIME_DB_ADDRESS = 'ws://localhost:3000';
const SPACETIME_DB_NAME = 'vibe-survival-game';

// Define the hook's return type
interface SpacetimeConnectionState {
    connection: DbConnection | null;
    isConnected: boolean;
    isLoading: boolean;
    error: string | null;
    registerPlayer: (username: string) => void;
    updatePlayerPosition: (moveX: number, moveY: number) => void;
    callSetSprintingReducer: (isSprinting: boolean) => void;
    callJumpReducer: () => void;
    callUpdateViewportReducer: (minX: number, minY: number, maxX: number, maxY: number) => void;
}

export const useSpacetimeConnection = (): SpacetimeConnectionState => {
    const [connection, setConnection] = useState<DbConnection | null>(null);
    const [isConnected, setIsConnected] = useState<boolean>(false); // Track actual game-ready state
    const [isLoading, setIsLoading] = useState<boolean>(false); // Track connection/registration attempt
    const [error, setError] = useState<string | null>(null);

    // --- Connection Logic Effect ---
    useEffect(() => {
        let connectionInstance: DbConnection | null = null;

        const connectToSpacetimeDB = async () => {
             // Start in loading state until connection or error
            setIsLoading(true);
            setError(null);
            try {
                console.log(`[useSpacetimeConnection] Attempting to connect to SpacetimeDB at ${SPACETIME_DB_ADDRESS}, module: ${SPACETIME_DB_NAME}...`);

                connectionInstance = DbConnection.builder()
                    .withUri(SPACETIME_DB_ADDRESS)
                    .withModuleName(SPACETIME_DB_NAME)
                    .onConnect((conn: DbConnection, identity: SpacetimeDBIdentity, token: string) => {
                        console.log('[useSpacetimeConnection] Connected!', { identity: identity.toHexString(), token });
                        setConnection(conn);
                        setError(null);
                        setIsLoading(false); // <-- Set loading false once connected
                    })
                    .onDisconnect((context: any, error?: Error) => {
                        console.log('[useSpacetimeConnection] Disconnected.', error?.message);
                        setConnection(null);
                        setIsConnected(false);
                        setIsLoading(false);
                        setError(`Disconnected${error ? ': ' + error.message : ''}. Please refresh.`);
                    })
                    .onConnectError((context: any, error: Error) => {
                        console.error('[useSpacetimeConnection] Initial Connection Error:', error);
                        setConnection(null);
                        setIsConnected(false);
                        setIsLoading(false);
                        setError(`Connection failed: ${error.message || error}`);
                    })
                    .build();

            } catch (err: any) {
                console.error('[useSpacetimeConnection] Failed to build SpacetimeDB connection:', err);
                setError(`Failed to build connection: ${err.message || err}`);
                setConnection(null);
                setIsConnected(false);
                setIsLoading(false);
            }
        };

        connectToSpacetimeDB();

        // Cleanup function
        return () => {
            if (connectionInstance) {
                console.log('[useSpacetimeConnection] Closing SpacetimeDB connection.');
                connectionInstance.disconnect();
                setConnection(null);
                setIsConnected(false);
                setIsLoading(false);
            }
        };
    }, []); // Empty dependency array ensures this runs only once on mount

    // --- Signal game readiness (called from App.tsx after player registration) ---
    // We need a way for App.tsx to tell this hook "the player is registered, we are truly connected now"
    // We can achieve this by checking the connection's identity in App.tsx's subscription callback
    // and then calling a function returned by this hook, OR by simply setting isConnected=true in App.tsx
    // For simplicity now, we'll let App.tsx manage the final `isConnected` state based on player registration.
    // This hook provides the base `connection` object.

    // --- Reducer Call Wrappers ---
    const registerPlayer = useCallback((username: string) => {
        if (connection && username.trim()) {
            setIsLoading(true); // Set loading during registration attempt
            setError(null);
            console.log(`[useSpacetimeConnection] Calling registerPlayer reducer with username: ${username}`);
            try {
                connection.reducers.registerPlayer(username);
                // setIsLoading will be set to false in App.tsx when the player entity appears OR if register fails
                // setIsLoading(false); // Remove this - App.tsx will manage registration loading state
            } catch (err: any) {
                console.error('[useSpacetimeConnection] Failed to register player:', err);
                setError(`Failed to register player: ${err.message || err}. Please try again.`);
                // If registration fails immediately, stop loading
                // Let App.tsx handle resetting its own isRegistering state
            }
        } else {
            console.warn("[useSpacetimeConnection] Cannot register player: No connection or empty username.");
            setError("Cannot register: Not connected or username is empty.");
        }
    }, [connection]);

    // --- Update Player Position --- (Modified Signature)
    const updatePlayerPosition = useCallback((moveX: number, moveY: number) => {
        if (!connection?.reducers) {
            console.warn("Connection not ready for updatePlayerPosition");
            return;
        }
        try {
            connection.reducers.updatePlayerPosition(moveX, moveY);
        } catch (err) {
            console.error("Error calling updatePlayerPosition reducer:", err);
            // Consider setting an error state
        }
    }, [connection]);

    const callSetSprintingReducer = useCallback((isSprinting: boolean) => {
        if (connection?.reducers) {
            try {
                connection.reducers.setSprinting(isSprinting);
            } catch (err: any) {
                console.error('[useSpacetimeConnection] Failed to call setSprinting reducer:', err);
                // setError(`Sprint toggle failed: ${err.message || err}`);
            }
        }
    }, [connection]);

    const callJumpReducer = useCallback(() => {
        if (connection?.reducers) {
            try {
                connection.reducers.jump();
            } catch (err: any) {
                console.error('[useSpacetimeConnection] Failed to call jump reducer:', err);
                // setError(`Jump failed: ${err.message || err}`);
            }
        }
    }, [connection]);

    const callUpdateViewportReducer = useCallback((minX: number, minY: number, maxX: number, maxY: number) => {
        if (connection?.reducers) {
            try {
                connection.reducers.updateViewport(minX, minY, maxX, maxY);
            } catch (err: any) {
                console.error('[useSpacetimeConnection] Failed to call updateViewport reducer:', err);
                // setError(`Viewport update failed: ${err.message || err}`);
            }
        }
    }, [connection]);

    return {
        connection,
        isConnected, // App.tsx will manage the final value of this based on registration
        isLoading,
        error,
        registerPlayer,
        updatePlayerPosition,
        callSetSprintingReducer,
        callJumpReducer,
        callUpdateViewportReducer,
    };
}; 