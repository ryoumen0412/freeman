# Freeman TUI

A terminal-based API testing tool written in Rust.

## Requirements

- Rust 1.70 or later
- Cargo

## Building

```bash
# Clone the repository
git clone https://github.com/your-username/freeman.git
cd freeman

# Build release version
cargo build --release

# The binary will be at target/release/freeman-tui
```

## Running

```bash
# Run directly with cargo
cargo run

# Or run the built binary
./target/release/freeman-tui
```

## Usage

The application has three tabs:

- **1:HTTP** - Standard HTTP request testing
- **2:WebSocket** - WebSocket connection testing
- **3:GraphQL** - GraphQL query execution

### HTTP Tab

| Key | Action |
|-----|--------|
| Tab | Switch between panels (URL, Body, Headers, Auth, Response, Workspace) |
| m | Cycle HTTP method (GET, POST, PUT, PATCH, DELETE) |
| e | Edit current field |
| s | Send request |
| Esc | Stop editing |
| q | Quit |

### WebSocket Tab

| Key | Action |
|-----|--------|
| c | Connect to WebSocket server |
| d | Disconnect |
| u | Edit URL |
| e | Edit message |
| s | Send message |

### GraphQL Tab

| Key | Action |
|-----|--------|
| u | Edit endpoint URL |
| e | Edit query |
| v | Edit variables |
| s | Execute query |
| Tab | Cycle between fields |

### General

| Key | Action |
|-----|--------|
| 1 | Switch to HTTP tab |
| 2 | Switch to WebSocket tab |
| 3 | Switch to GraphQL tab |
| ? | Show help |
| Ctrl+C | Quit |

## Workspace Discovery

Freeman can auto-detect API endpoints from project source code:

- Press `o` to open a project directory
- Supported frameworks: OpenAPI, FastAPI, Flask, Django, Express.js, NestJS, Spring Boot, Laravel

## cURL Import/Export

- Press `i` in the URL panel to import a cURL command
- Press `c` to export the current request as cURL

## License

MIT

## Disclaimer(?)

Do whatever you want with this code, I don't care. I built this using my poor, lacking knowledge of rust and tui and Opus 4.5, and because I hate Postman and curl is not nearly enough.
