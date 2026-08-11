# Freeman TUI

[![Crates.io](https://img.shields.io/crates/v/freeman.svg)](https://crates.io/crates/freeman)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

**Freeman** is a blazing-fast, terminal-based API testing tool written in Rust.

Built for developers who find Postman too bloated and cURL too barebones.

It provides a seamless, keyboard-driven interface to explore, test, and manage REST, GraphQL, and WebSocket APIs directly from your terminal.

## Features

- **Blazing Fast TUI:** Built with `ratatui` and `crossterm` for a responsive, zero-lag experience.
- **Multi-Protocol Support:**
  - **HTTP/REST:** Full control over methods, headers, auth (Bearer/Basic), and body.
  - **WebSockets:** Real-time bi-directional messaging with live connection logs.
  - **GraphQL:** Native support for executing queries and managing variables.
- **Auto-Discovery (Workspaces):** Automatically detects and parses API endpoints directly from your source code or OpenAPI specs.
  - *Supported: OpenAPI, FastAPI, Flask, Django, Express.js, NestJS, Spring Boot, Laravel.*
- **cURL Integration:** Seamlessly import cURL commands to instantly populate requests, or export your current request to a cURL string.
- **Request History:** Never lose an endpoint. Navigate through your previously executed requests.

---

## Installation

### Option 1: Using Cargo (Recommended)

If you have Rust installed, you can simply pull the crate from crates.io:

```bash
cargo install freeman
```

### Option 2: Build from Source

```bash
git clone https://github.com/ryoumen0412/freeman.git
cd freeman
cargo build --release
# The binary will be available at target/release/freeman-tui
```

---

## Usage

Run the application:

```bash
freeman
```

### Navigation & Core Controls

Freeman is fully keyboard-driven. The interface is split into three main tabs:

| Key | Global Action |
|-----|---------------|
| `1` | Switch to **HTTP** Tab |
| `2` | Switch to **WebSocket** Tab |
| `3` | Switch to **GraphQL** Tab |
| `Tab` / `Shift+Tab` | Cycle through panels within the current tab |
| `?` | Toggle Help Menu |
| `Ctrl+C` | Quit Application |

### HTTP Tab

| Key | Action |
|-----|--------|
| `m` | Cycle HTTP Methods (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`) |
| `e` | Edit the currently focused field |
| `s` | Send the request |
| `o` | Open a Workspace (Auto-discover endpoints from a local directory) |
| `i` | Import request from a cURL command |
| `c` | Export current request as a cURL command |
| `Esc` | Stop editing |

### WebSocket Tab

| Key | Action |
|-----|--------|
| `u` | Edit WebSocket URL |
| `c` | Connect to the server |
| `e` | Edit message payload |
| `s` | Send message |
| `d` | Disconnect |

### GraphQL Tab

| Key | Action |
|-----|--------|
| `u` | Edit GraphQL Endpoint |
| `e` | Edit Query |
| `v` | Edit Variables (JSON) |
| `s` | Execute Query |

---

## Workspace Auto-Discovery

Freeman can scan your local project directories and automatically extract available API endpoints.

1. Navigate to the **HTTP Tab**.
2. Press `o` to open the Workspace input.
3. Type the path to your project (e.g., `~/projects/my-api`). Press `Tab` for autocompletion.
4. Hit `Enter`. Freeman will detect the framework and load the endpoints into the Workspace panel.

---

## License

This project is licensed under the **GNU General Public License v3.0 (GPL-3.0)**. See the [LICENSE](LICENSE) file for more details.

## Philosophy

*Do whatever you want with this code (except earning money). I built this using my poor, lacking knowledge of Rust and TUI, and because I hate Postman... and cURL is not nearly enough. God bless Opus 4.6.*

Contributions, issues, and feature requests are **almost** always welcome!
