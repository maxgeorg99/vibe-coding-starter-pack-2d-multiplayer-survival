import React, { useState, useEffect } from 'react';

interface DeathScreenProps {
  respawnAt: number; // Respawn timestamp in milliseconds since epoch
  onRespawn: () => void; // Function to call when the respawn button is clicked
}

const DeathScreen: React.FC<DeathScreenProps> = ({ respawnAt, onRespawn }) => {
  const [remainingTime, setRemainingTime] = useState<number>(0);
  const [isRespawnReady, setIsRespawnReady] = useState<boolean>(false);

  useEffect(() => {
    const calculateRemainingTime = () => {
      const now = Date.now();
      const diff = respawnAt - now;
      const secondsLeft = Math.max(0, Math.ceil(diff / 1000));
      setRemainingTime(secondsLeft);
      setIsRespawnReady(diff <= 0);
    };

    calculateRemainingTime(); // Initial calculation

    const intervalId = setInterval(calculateRemainingTime, 500); // Update every 500ms

    return () => clearInterval(intervalId); // Cleanup interval on unmount
  }, [respawnAt]);

  return (
    <div style={styles.overlay}>
      <div style={styles.container}>
        <h1 style={styles.title}>You Died</h1>
        {remainingTime > 0 && (
          <p style={styles.timerText}>Respawn available in: {remainingTime}s</p>
        )}
        <button
          onClick={onRespawn}
          disabled={!isRespawnReady}
          style={isRespawnReady ? styles.buttonEnabled : styles.buttonDisabled}
        >
          Respawn
        </button>
      </div>
    </div>
  );
};

// Basic styling - can be moved to CSS/modules later
const styles: { [key: string]: React.CSSProperties } = {
  overlay: {
    position: 'absolute',
    top: 0,
    left: 0,
    width: '100%',
    height: '100%',
    backgroundColor: 'rgba(0, 0, 0, 0.7)',
    display: 'flex',
    justifyContent: 'center',
    alignItems: 'center',
    zIndex: 1000, // Ensure it's above the canvas
    fontFamily: '"Press Start 2P", cursive', // Match game font
    color: 'white',
  },
  container: {
    textAlign: 'center',
    padding: '40px',
    backgroundColor: 'rgba(50, 50, 50, 0.8)',
    borderRadius: '10px',
  },
  title: {
    color: '#DC143C', // Crimson Red
    fontSize: '2.5em',
    marginBottom: '20px',
    textShadow: '2px 2px 4px #000000',
  },
  timerText: {
      fontSize: '1.2em',
      marginBottom: '30px',
  },
  buttonEnabled: {
    padding: '15px 30px',
    fontSize: '1.2em',
    fontFamily: '"Press Start 2P", cursive',
    backgroundColor: '#4CAF50', // Green
    color: 'white',
    border: 'none',
    borderRadius: '5px',
    cursor: 'pointer',
    transition: 'background-color 0.3s',
  },
   buttonDisabled: {
    padding: '15px 30px',
    fontSize: '1.2em',
    fontFamily: '"Press Start 2P", cursive',
    backgroundColor: '#777', // Grey
    color: '#ccc',
    border: 'none',
    borderRadius: '5px',
    cursor: 'not-allowed',
  }
};

export default DeathScreen; 