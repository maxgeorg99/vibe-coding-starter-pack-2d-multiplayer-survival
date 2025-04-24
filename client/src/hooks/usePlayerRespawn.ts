import { useCallback, useMemo } from 'react';
import { Player as SpacetimeDBPlayer } from '../generated';
import { DbConnection } from '../generated';

interface UsePlayerRespawnResult {
  respawnTimestampMs: number;
  isPlayerDead: boolean;
  shouldShowDeathScreen: boolean;
  handleRespawnRequest: () => void;
}

/**
 * Hook to manage player death state and respawn functionality
 */
export function usePlayerRespawn(
  localPlayer: SpacetimeDBPlayer | undefined | null,
  connection: DbConnection | null
): UsePlayerRespawnResult {
  // Calculate respawn timestamp
  const respawnTimestampMs = useMemo(() => {
    if (localPlayer?.isDead && localPlayer.respawnAt) {
      return Number(localPlayer.respawnAt.microsSinceUnixEpoch / 1000n);
    }
    return 0;
  }, [localPlayer?.isDead, localPlayer?.respawnAt]);

  // Handle respawn request
  const handleRespawnRequest = useCallback(() => {
    if (!connection?.reducers) {
      console.error("Connection or reducers not available for respawn request.");
      return;
    }
    
    try {
      connection.reducers.requestRespawn();
    } catch (err) {
      console.error("Error calling requestRespawn reducer:", err);
    }
  }, [connection]);

  // Determine if player is dead
  const isPlayerDead = !!localPlayer?.isDead;
  
  // Should the death screen be displayed?
  const shouldShowDeathScreen = !!(localPlayer?.isDead && respawnTimestampMs > 0 && connection);

  return {
    respawnTimestampMs,
    isPlayerDead,
    shouldShowDeathScreen,
    handleRespawnRequest
  };
} 