import React, { useState, useEffect } from 'react';
import { Player, DbConnection } from '../generated'; // Get DbConnection from generated
import { Identity } from '@clockworklabs/spacetimedb-sdk'; // Import Identity from base SDK

interface PlayerUIProps {
  identity: Identity | null;
  players: Map<string, Player>; // Get the players map from App.tsx
}

const PlayerUI: React.FC<PlayerUIProps> = ({ identity, players }) => {
    const [localPlayer, setLocalPlayer] = useState<Player | null>(null);

    useEffect(() => {
        if (!identity) {
            setLocalPlayer(null); // Clear if no identity
            return;
        }

        // Find the player in the map passed via props
        const player = players.get(identity.toHexString());
        setLocalPlayer(player || null);

        // No need for direct DB listeners here, App.tsx manages the players map

    }, [identity, players]); // Rerun effect if identity or the players map changes

    if (!localPlayer) {
        return null; // Don't render anything if local player data isn't available
    }

    // Simple display for now
    return (
        <div style={{
            position: 'fixed',
            bottom: '20px',
            right: '20px',
            backgroundColor: 'rgba(0, 0, 0, 0.7)',
            color: 'white',
            padding: '15px',
            borderRadius: '8px',
            fontFamily: 'Arial, sans-serif',
            minWidth: '200px', // Ensure minimum width
        }}>
            <h4>{localPlayer.username}</h4>
            <div>Health: {localPlayer.health.toFixed(0)}/100</div>
            <div>Stamina: {localPlayer.stamina.toFixed(0)}/100</div>
            <div>Thirst: {localPlayer.thirst.toFixed(0)}/100</div>
            <div>Hunger: {localPlayer.hunger.toFixed(0)}/100</div>
            <div>Warmth: {localPlayer.warmth.toFixed(0)}/100</div>
        </div>
    );
};

export default PlayerUI;
