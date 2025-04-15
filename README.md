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

## 🚀 Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) v22+ 
- [Rust](https://www.rust-lang.org/tools/install) latest stable
- [SpacetimeDB CLI](https://spacetimedb.com/docs/getting-started/installation)

### One-Command Setup

```bash
# Clone and setup the project
git clone https://github.com/your-username/vibe-coding-starter-pack-2d-survival.git
cd vibe-coding-starter-pack-2d-survival
npm install
npm run dev  # Server will start on port 3008
```

## 📋 Detailed Setup Instructions

### 1. Environment Setup

```bash
# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Add WebAssembly target
rustup target add wasm32-unknown-unknown

# Install Node.js via nvm (if not installed)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.2/install.sh | bash
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
nvm install 22
```

### 2. SpacetimeDB Setup

```bash
# Install SpacetimeDB CLI
curl -sSf https://install.spacetimedb.com | sh

# Add to PATH if needed
export PATH="$HOME/.local/bin:$PATH"
```

### 3. Running the Project

```bash
# Terminal 1: Build and run the server
cd server
spacetime build
spacetime dev

# Terminal 2: Run the client
cd client
npm install
npm run dev
```

The client will be available at http://localhost:3008

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

---

_Built with ❤️ by [Your Name/Team]_
