import React, { useState, useEffect } from 'react';
import { Player } from '../generated'; // DbConnection is not needed here anymore
import { Identity } from '@clockworklabs/spacetimedb-sdk';

// Define the StatusBar component inline for simplicity
interface StatusBarProps {
  label: string;
  icon: string; // Placeholder for icon, e.g., emoji or text
  value: number;
  maxValue: number;
  barColor: string;
}

const StatusBar: React.FC<StatusBarProps> = ({ label, icon, value, maxValue, barColor }) => {
  const percentage = Math.max(0, Math.min(100, (value / maxValue) * 100));

  return (
    <div style={{ marginBottom: '4px', display: 'flex', alignItems: 'center' }}>
      <span style={{ marginRight: '5px', minWidth: '18px', textAlign: 'center', fontSize: '14px' }}>{icon}</span>
      <div style={{ flexGrow: 1 }}>
        <div style={{
          height: '8px',
          backgroundColor: '#555',
          borderRadius: '2px',
          overflow: 'hidden',
          border: '1px solid #333',
        }}>
          <div style={{
            height: '100%',
            width: `${percentage}%`,
            backgroundColor: barColor,
          }}></div>
        </div>
      </div>
      <span style={{ marginLeft: '5px', fontSize: '10px', minWidth: '30px', textAlign: 'right' }}>
        {value.toFixed(0)}
      </span>
    </div>
  );
};


interface PlayerUIProps {
  identity: Identity | null;
  players: Map<string, Player>;
}

const PlayerUI: React.FC<PlayerUIProps> = ({ identity, players }) => {
    const [localPlayer, setLocalPlayer] = useState<Player | null>(null);

    useEffect(() => {
        if (!identity) {
            setLocalPlayer(null);
            return;
        }
        const player = players.get(identity.toHexString());
        setLocalPlayer(player || null);
    }, [identity, players]);

    if (!localPlayer) {
        return null;
    }

    // Retro SNES RPG Style - Compact
    return (
        <div style={{
            position: 'fixed',
            bottom: '15px',
            right: '15px',
            backgroundColor: 'rgba(40, 40, 60, 0.85)',
            color: 'white',
            padding: '10px',
            borderRadius: '4px',
            border: '1px solid #a0a0c0',
            fontFamily: '"Press Start 2P", cursive',
            minWidth: '200px',
            boxShadow: '2px 2px 0px rgba(0,0,0,0.5)',
        }}>
            <StatusBar
                label="HP"
                icon="❤️"
                value={localPlayer.health}
                maxValue={100}
                barColor="#ff4040"
            />
            <StatusBar
                label="SP"
                icon="⚡"
                value={localPlayer.stamina}
                maxValue={100}
                barColor="#40ff40"
            />
            <StatusBar
                label="Thirst"
                icon="💧"
                value={localPlayer.thirst}
                maxValue={100}
                barColor="#40a0ff"
            />
            <StatusBar
                label="Hunger"
                icon="🍖"
                value={localPlayer.hunger}
                maxValue={100}
                barColor="#ffa040"
            />
            <StatusBar
                label="Warmth"
                icon="🔥"
                value={localPlayer.warmth}
                maxValue={100}
                barColor="#ffcc00"
            />
        </div>
    );
};

export default PlayerUI;
