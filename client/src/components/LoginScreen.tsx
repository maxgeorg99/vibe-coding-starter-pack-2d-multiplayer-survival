/**
 * LoginScreen.tsx
 * 
 * Displays the initial welcome/login screen.
 * Handles:
 *  - Displaying game title and logo.
 *  - Input field for the player's username.
 *  - "Join Game" button to trigger the login/registration process.
 *  - Displaying loading states (Connecting/Joining).
 *  - Displaying connection errors.
 * Receives username state, loading/error status, and login action handler as props.
 */

import React, { useRef, useEffect } from 'react';
import githubLogo from '../../public/github.png'; // Adjust path as needed

// Style Constants (Consider moving to a shared file)
const UI_BG_COLOR = 'rgba(40, 40, 60, 0.85)';
const UI_BORDER_COLOR = '#a0a0c0';
const UI_SHADOW = '2px 2px 0px rgba(0,0,0,0.5)';
const UI_FONT_FAMILY = '"Press Start 2P", cursive';

interface LoginScreenProps {
    username: string;
    setUsername: (value: string) => void;
    handleLogin: () => void;
    isLoading: boolean; // Combined loading state (connection + registration)
    error: string | null;
}

const LoginScreen: React.FC<LoginScreenProps> = ({
    username,
    setUsername,
    handleLogin,
    isLoading,
    error,
}) => {
    const usernameInputRef = useRef<HTMLInputElement>(null);

    // Autofocus on initial render
    useEffect(() => {
        usernameInputRef.current?.focus();
    }, []);

    const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
        if (event.key === 'Enter' && !isLoading && username.trim()) {
            handleLogin();
        }
    };

    return (
        <div style={{ /* Centering styles */
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            minHeight: '100vh',
            width: '100%',
            fontFamily: UI_FONT_FAMILY,
        }}>
            <div style={{ /* Login Box Styles */
                backgroundColor: UI_BG_COLOR,
                color: 'white',
                padding: '40px',
                borderRadius: '4px',
                border: `1px solid ${UI_BORDER_COLOR}`,
                boxShadow: UI_SHADOW,
                textAlign: 'center',
                minWidth: '350px',
            }}>
                <img
                    src={githubLogo}
                    alt="Vibe Coding Logo"
                    style={{
                        width: '240px',
                        height: 'auto',
                        marginBottom: '25px',
                    }}
                />
                <h2 style={{ marginBottom: '20px', fontWeight: 'normal' }}>2D Survival Multiplayer</h2>
                <input
                    ref={usernameInputRef}
                    type="text"
                    placeholder="Enter Username"
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    onKeyDown={handleKeyDown}
                    disabled={isLoading} // Use combined isLoading prop
                    style={{ /* Input Styles */
                        padding: '10px',
                        marginBottom: '15px',
                        border: `1px solid ${UI_BORDER_COLOR}`,
                        backgroundColor: '#333',
                        color: 'white',
                        fontFamily: UI_FONT_FAMILY,
                        fontSize: '14px',
                        display: 'block',
                        width: 'calc(100% - 22px)',
                        textAlign: 'center',
                    }}
                />
                <button
                    onClick={handleLogin}
                    disabled={isLoading || !username.trim()} // Use combined isLoading
                    style={{ /* Button Styles */
                        padding: '10px 20px',
                        border: `1px solid ${UI_BORDER_COLOR}`,
                        backgroundColor: isLoading ? '#555' : '#777',
                        color: isLoading ? '#aaa' : 'white',
                        fontFamily: UI_FONT_FAMILY,
                        fontSize: '14px',
                        cursor: (isLoading || !username.trim()) ? 'not-allowed' : 'pointer',
                        boxShadow: UI_SHADOW,
                    }}
                >
                    {/* Show text based on loading state */} 
                    {isLoading ? 'Connecting...' : 'Join Game'} {/* Simplified text based on combined loading */} 
                </button>
                {error && <p style={{ color: 'red', marginTop: '15px' }}>{error}</p>}
            </div>
        </div>
    );
};

export default LoginScreen; 