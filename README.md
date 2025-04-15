# Vibe Coding Starter Pack: 2D Survival Multiplayer

A lightweight 2D multiplayer survival game starter kit using React, Vite, and SpacetimeDB (or WebSockets).  
Build interactive survival experiences with persistent player state, real-time synchronization, and modular game logic.

## Features

- Real-time multiplayer with player movement sync
- Infinite scrolling tile-based terrain
- Inventory and resource collection system
- Client-server architecture with clear separation
- Built with performance, extensibility, and good vibes in mind

## Tech Stack

| Layer       | Stack                    |
|-------------|--------------------------|
| Frontend    | React + Vite + TypeScript|
| Multiplayer | SpacetimeDB             |
| Backend     | Node.js or Rust         |

## Project Structure

```
vibe-coding-starter-pack-2d-survival/
├── client/       # Frontend game logic and rendering
├── server/       # WebSocket or SpacetimeDB server logic
├── public/       # Static files (tilemaps, sprites)
├── .cursor/rules/# Optional Cursor IDE rules
├── README.md
├── LICENSE
└── setup.sh
```

## Getting Started

1. Clone the repo:
```bash
git clone https://github.com/your-username/vibe-coding-starter-pack-2d-survival.git
```

2. Install dependencies:
```bash
cd vibe-coding-starter-pack-2d-survival
npm install
```

3. Start the development servers:
```bash
npm run dev
```

The server will start on port 3008. Make sure SpacetimeDB (or your WebSocket server) is running before launching the client.

## Detailed Setup Instructions

### Prerequisites

#### 1. Install Rust (if not already installed)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

#### 2. Add WASM target for Rust
```bash
rustup target add wasm32-unknown-unknown
```

#### 3. Install Node.js via nvm (if not already installed)
```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.2/install.sh | bash
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
nvm install 22
nvm use 22
```

### SpacetimeDB Setup

#### 4. Install SpacetimeDB CLI
```bash
curl -sSf https://install.spacetimedb.com | sh
```

Note: SpacetimeDB installs to `~/.local/bin` - add it to your PATH if needed:
```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Running Client and Server Separately

#### 5. Install client dependencies
```bash
cd client
npm install
```

#### 6. Build server code
```bash
cd ../server
spacetime build
```

#### 7. Generate TypeScript bindings
```bash
spacetime generate --lang typescript --out-dir ../client/src/generated
```

#### 8. Run the server
```bash
# From the server directory
spacetime dev
```

#### 9. Run the client
```bash
# From the client directory, in a separate terminal
npm run dev
```

## Roadmap

- Day/night cycle and weather
- Basic enemy AI
- Base building
- Farming and cooking
- PvP mechanics
- Fog of war and minimap

## License

MIT License
