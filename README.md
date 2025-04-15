# Vibe Coding Starter Pack: 2D Survival Multiplayer

![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)
![React](https://img.shields.io/badge/React-19-blue.svg)
![Vite](https://img.shields.io/badge/Vite-6-purple.svg)
![SpacetimeDB](https://img.shields.io/badge/SpacetimeDB-latest-orange.svg)

A lightweight 2D multiplayer survival game starter kit built with modern web technologies. Create interactive survival experiences with persistent player state, real-time multiplayer synchronization, and modular game logic.

## ✨ Features

- **Real-time Multiplayer**: Seamless player movement synchronization
- **Infinite World**: Procedurally generated, tile-based terrain with efficient rendering
- **Resource System**: Comprehensive inventory and collection mechanics
- **Modern Architecture**: Clean separation between client and server components
- **Performance Focused**: Built with optimization, extensibility, and good vibes in mind

## 🛠️ Tech Stack

| Layer       | Technologies                |
|-------------|----------------------------|
| Frontend    | React 19, Vite 6, TypeScript |
| Multiplayer | SpacetimeDB                |
| Backend     | Rust (WebAssembly)         |
| Development | Node.js 22+                |

## 📁 Project Structure

```
vibe-coding-starter-pack-2d-survival/
├── client/         # React frontend game logic and rendering
│   ├── src/        # Source code
│   ├── public/     # Static assets
│   └── package.json
├── server/         # SpacetimeDB server logic (Rust)
│   ├── src/        # Server code
│   └── Cargo.toml
├── public/         # Shared static assets (tilemaps, sprites)
├── README.md
└── LICENSE
```

## 🚀 Running the Project Locally

This guide assumes you have installed the prerequisites: Node.js v22+, Rust, and the SpacetimeDB CLI.

1.  **Clone the Repository:**
    ```bash
    git clone https://github.com/SeloSlav/vibe-coding-starter-pack-2d-survival.git
    cd vibe-coding-starter-pack-2d-survival
    ```

2.  **Install Client Dependencies:**
    ```bash
    # From the project root directory
    npm install
    ```

3.  **Start Local SpacetimeDB Server:**
    Open a **separate terminal** window and run:
    ```bash
    spacetime start
    ```
    Keep this terminal running in the background. It hosts your local game database.

4.  **Build, Publish Server Module & Generate Client Bindings:**
    Open **another terminal** window, navigate to the `server` directory, and run these commands:
    ```bash
    cd server
    spacetime publish vibe-survival-game
    spacetime generate --lang typescript --out-dir ../client/src/generated
    ```
    *   **Note:** You need to re-run these two commands *every time* you change the server schema (e.g., modify tables or reducers in `server/src/lib.rs`).

5.  **Run the Client:**
    In the **same terminal** as step 4 (or a new one, just make sure you are in the project root directory `vibe-coding-starter-pack-2d-survival`), run:
    ```bash
    npm run dev
    ```

6.  **Access the Game:**
    Open your browser and navigate to the local address provided by Vite (usually `http://localhost:5173` or similar).

## 🔧 Troubleshooting Local Setup

*   **`Cannot find module './generated'` error in client:**
    *   Ensure you ran `spacetime generate --lang typescript --out-dir ../client/src/generated` from the `server` directory *after* the last `spacetime publish`.
    *   Make sure the `client/src/generated` folder was actually created and contains `.ts` files.
    *   Restart the Vite dev server (`npm run dev`).
*   **Client connects but game doesn't load / players don't appear:**
    *   Check the browser console for errors (e.g., subscription failures).
    *   Check the terminal running `spacetime start` for server-side errors (e.g., reducer panics).
*   **Old players still appearing after disconnect/refresh:**
    *   The disconnect logic might not be removing them correctly. The most reliable way to ensure a clean state is to delete and recreate the local database:
        ```bash
        # Stop spacetime start (Ctrl+C)
        spacetime delete vibe-survival-game # Run from any directory
        spacetime start # Restart the server
        # Then re-publish and re-generate (Step 4 above)
        ```
*   **`spacetime publish` tries to publish to Maincloud instead of local:**
    *   Ensure you are logged out: `spacetime logout`.
    *   Ensure the `spacetime start` server is running *before* you publish.
    *   Check your SpacetimeDB config file (`%LOCALAPPDATA%/SpacetimeDB/config/cli.toml` on Windows, `~/.local/share/spacetime/config/cli.toml` on Linux/macOS) and make sure `default_server` is set to `local` (or comment it out).

## 🔄 Development Workflow

1. **Server Development**:
   - Modify Rust code in the `server/src` directory
   - Run `spacetime build` to compile changes
   - Generate updated TypeScript bindings: `spacetime generate --lang typescript --out-dir ../client/src/generated`

2. **Client Development**:
   - Edit React components in `client/src`
   - The dev server supports hot module replacement

## 🗺️ Roadmap

- **Environment Systems**: Day/night cycle with dynamic weather effects
- **Advanced AI**: Basic enemy behaviors and interaction patterns
- **Construction**: Base building with defensive structures
- **Survival Mechanics**: Farming, cooking, and resource management
- **Competitive Elements**: PvP mechanics and team-based gameplay
- **World Discovery**: Fog of war, minimap, and exploration rewards

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📜 License

This project is licensed under the MIT License - see the LICENSE file for details.

## ⚙️ Client Configuration (`client/src/config/gameConfig.ts`)

This file centralizes various client-side game parameters. Modify these values to tweak game behavior:

*   `tileSize`: The width and height of each world tile in pixels.
*   `worldWidth`, `worldHeight`: The dimensions of the game world, measured in tiles.
*   `playerRadius`: The radius used for simple collision detection and mouse hover checks, typically half the sprite width.
*   `playerSpeed`: How many pixels the player moves per frame when input is detected.
*   `spriteWidth`, `spriteHeight`: The dimensions of a single frame within the player's spritesheet.
*   `fps`: The target frames per second; primarily informational as the game loop uses `requestAnimationFrame`.

---

Created by SeloSlav
