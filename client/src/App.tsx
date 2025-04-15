import React, { useState, useEffect } from 'react';
import './App.css';
import GameCanvas from './components/GameCanvas';

// Import generated bindings (assuming they are in './generated')
import * as SpacetimeDB from './generated';
// Import base types from the SDK
import { Identity as SpacetimeDBIdentity } from '@clockworklabs/spacetimedb-sdk'; 
// Use generated Player type and generated DbConnection
const { Player, DbConnection } = SpacetimeDB; 

// SpacetimeDB connection parameters
const SPACETIME_DB_ADDRESS = 'ws://localhost:3000';
const SPACETIME_DB_NAME = 'vibe-survival-game'; // Adjust if your module name is different

function App() {
  // Use the generated Player type
  const [players, setPlayers] = useState<Map<string, SpacetimeDB.Player>>(new Map());
  // State holds the generated connection type after successful connection
  const [connection, setConnection] = useState<SpacetimeDB.DbConnection | null>(null);
  const [username, setUsername] = useState<string>('');
  const [isConnected, setIsConnected] = useState<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  
  useEffect(() => {
    // Use generated DbConnection type
    let connectionInstance: SpacetimeDB.DbConnection | null = null; 

    const connectToSpacetimeDB = async () => {
      try {
        console.log(`Attempting to connect to SpacetimeDB at ${SPACETIME_DB_ADDRESS}, module: ${SPACETIME_DB_NAME}...`);
        
        // Use builder with correct methods from SDK definition
        connectionInstance = DbConnection.builder()
          .withUri(SPACETIME_DB_ADDRESS)       // Correct method for address
          .withModuleName(SPACETIME_DB_NAME)     // Correct method for db name
          .onConnect((conn: SpacetimeDB.DbConnection, identity: SpacetimeDBIdentity, token: string) => { // Correct method
            console.log('Connected!', { identity: identity.toHexString(), token });
            setConnection(conn);
            setError(null);
            // DO NOT set isConnected here; wait for player registration
            // setIsConnected(true); 
          })
          .onDisconnect((context: any, error?: Error) => { // Correct method
            console.log('Disconnected.', error?.message);
            setConnection(null);
            setIsConnected(false);
            setError(`Disconnected${error ? ': ' + error.message : ''}. Please refresh.`);
          })
          // No general onError, use onConnectError for initial connection issues
          .onConnectError((context: any, error: Error) => { // Correct method for initial errors
            console.error('Initial Connection Error:', error);
            setConnection(null);
            setIsConnected(false);
            setError(`Connection failed: ${error.message || error}`);
          })
          .build(); // build() takes no args
          
      } catch (err: any) {
        // Catch errors during the builder.build() call itself (e.g., invalid URI format)
        console.error('Failed to build SpacetimeDB connection:', err);
        setError(`Failed to build connection: ${err.message || err}`);
        setConnection(null);
        setIsConnected(false);
      }
    };

    connectToSpacetimeDB();
    
    return () => {
      // Cleanup: Use disconnect() suggested by linter
      if (connectionInstance) {
        console.log('Closing SpacetimeDB connection.');
        connectionInstance.disconnect(); // Use disconnect()
      }
    };
  }, []);
  
  // Effect for Subscribing to Player data and handling updates
  useEffect(() => {
    // Ensure we have a connection before subscribing
    if (!connection) return;

    console.log('Setting up Player table subscriptions...');

    // --- Player Table Callbacks --- 
    const handlePlayerInsert = (ctx: any, player: SpacetimeDB.Player) => {
      console.log('handlePlayerInsert called for:', player.identity.toHexString()); // Log when callback fires
      console.log('Current connection identity:', connection?.identity?.toHexString()); // Log current identity
      
      console.log('Player Inserted:', player.username, player.identity.toHexString());
      setPlayers(prev => new Map(prev.set(player.identity.toHexString(), player)));
      
      // Check if the inserted player is the local player (and connection/identity exist)
      if (connection && connection.identity && player.identity.isEqual(connection.identity)) {
        console.log('Local player registered, switching to game view.');
        setIsConnected(true); // Now switch to game view
      }
    };

    const handlePlayerUpdate = (ctx: any, oldPlayer: SpacetimeDB.Player, newPlayer: SpacetimeDB.Player) => {
      console.log('Player Updated:', newPlayer.username, newPlayer.identity.toHexString());
      // Update the map with the new player data
      setPlayers(prev => new Map(prev.set(newPlayer.identity.toHexString(), newPlayer)));
    };

    const handlePlayerDelete = (ctx: any, player: SpacetimeDB.Player) => {
      console.log('Player Deleted:', player.username, player.identity.toHexString());
      setPlayers(prev => {
        const newMap = new Map(prev);
        newMap.delete(player.identity.toHexString());
        return newMap;
      });
    };

    // Register the callbacks
    connection.db.player.onInsert(handlePlayerInsert);
    connection.db.player.onUpdate(handlePlayerUpdate);
    connection.db.player.onDelete(handlePlayerDelete);

    // --- Subscription --- 
    console.log('Subscribing to Player table...');
    const subscription = connection.subscriptionBuilder()
      .onApplied(() => { 
        console.log('Subscription to Player table APPLIED.');
      })
      // Correct onError signature for SubscriptionBuilder
      .onError((ctx: SpacetimeDB.ErrorContext) => { 
        // Error details might be within the context or need separate handling
        // For now, log a generic message and check console for context details
        console.error('Subscription to Player table FAILED. Context:', ctx); 
        setError(`Subscription failed. Check console.`);
      })
      .subscribe('SELECT * FROM player');

    // Cleanup function for this effect
    return () => {
      console.log('Cleaning up Player table subscriptions...');
      // Remove listeners when connection changes or component unmounts
      connection.db.player.removeOnInsert(handlePlayerInsert);
      connection.db.player.removeOnUpdate(handlePlayerUpdate);
      connection.db.player.removeOnDelete(handlePlayerDelete);
      
      // Unsubscribe from the query
      subscription.unsubscribe();
    };
    
  }, [connection]); // Re-run this effect if the connection object changes
  
  // Handle player registration
  const handleRegisterPlayer = () => {
    if (connection && username.trim()) {
      setIsLoading(true);
      console.log(`Calling registerPlayer reducer with username: ${username}`); // Add log here
      try {
        // Call the actual SpacetimeDB reducer using camelCase
        connection.reducers.registerPlayer(username);
        // Let the connection and subscriptions handle the connected state
        // setIsConnected(true); // Remove this mock call 
      } catch (err) {
        console.error('Failed to register player:', err);
        setError('Failed to register player. Please try again.');
      } finally {
        setIsLoading(false);
      }
    }
  };
  
  // Update player position handler
  const updatePlayerPosition = (x: number, y: number) => {
    if (connection) {
      try {
        // Call the actual SpacetimeDB reducer using camelCase
        connection.reducers.updatePlayerPosition(x, y);
      } catch (err) {
        console.error('Failed to update player position:', err);
      }
    }
  };
  
  // --- Jump Reducer Call ---
  const callJumpReducer = () => {
    if (connection) {
      try {
        // Call the actual SpacetimeDB reducer using camelCase
        connection.reducers.jump();
      } catch (err) {
        console.error('Failed to call jump reducer:', err);
        // Optionally set an error state here
      }
    }
  };
  
  return (
    <div className="App">
      {error && <div className="error-message">{error}</div>}
      
      {!isConnected ? (
        <div className="login-container">
          <input
            type="text"
            placeholder="Enter your username"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            disabled={isLoading}
          />
          <button 
            onClick={handleRegisterPlayer}
            disabled={isLoading || !username.trim()}
          >
            {isLoading ? 'Joining...' : 'Join Game'}
          </button>
        </div>
      ) : (
        <div className="game-container">
          <GameCanvas
            players={players}
            localPlayerId={connection?.identity?.toHexString()}
            updatePlayerPosition={updatePlayerPosition}
            callJumpReducer={callJumpReducer}
          />
        </div>
      )}
    </div>
  );
}

export default App;
