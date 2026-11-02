# Rustcord

<p align="center">
  <img src="packaging/linux/hicolor/128x128/apps/rustcord.png" alt="Rustcord" width="128" height="128" />
</p>

<p align="center">
  <strong>Ultra-lightweight, high-performance native Discord desktop client written in pure Rust.</strong>
</p>

<p align="center">
  <a href="#downloads--releases"><strong>📦 Downloads</strong></a> &nbsp;•&nbsp;
  <a href="#features"><strong>⚡ Features</strong></a> &nbsp;•&nbsp;
  <a href="#performance"><strong>📊 Performance</strong></a> &nbsp;•&nbsp;
  <a href="#building-from-source"><strong>🛠️ Build</strong></a> &nbsp;•&nbsp;
  <a href="#architecture"><strong>🏗️ Architecture</strong></a>
</p>

<p align="center">
  <a href="https://github.com/surf1k/rustcord/releases"><img src="https://img.shields.io/github/v/release/surf1k/rustcord?label=release&color=5865F2" alt="GitHub Release" /></a>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/rust-1.98.1_pinned-blue.svg?logo=rust" alt="Rust 1.98.1 Pinned" /></a>
  <a href="crates/ui"><img src="https://img.shields.io/badge/ui-egui%20%2F%20wgpu-orange.svg" alt="UI egui/wgpu" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green.svg" alt="License: MIT or Apache-2.0" /></a>
</p>

---

## Overview

**Rustcord** is a native, standalone Discord desktop client engineered from the ground up in Rust using `egui` and `wgpu`. Unlike the standard Electron-based client which consumes gigabytes of memory and multiple CPU cores, Rustcord delivers blazing-fast rendering, negligible resource consumption, and seamless day-to-day usability.

---

## Downloads & Releases

Pre-compiled release packages are published on GitHub [Releases](https://github.com/surf1k/rustcord/releases).

| Platform | Format | Architecture | Download |
|---|---|---|---|
| **Windows** | `.zip` (Portable `rustcord.exe`) | `x86_64` | [`rustcord-windows-x86_64.zip`](https://github.com/surf1k/rustcord/releases/latest) |
| **Linux** | `.rpm` (Fedora / RHEL) | `x86_64` | [`rustcord-0.1.0-1.fc44.x86_64.rpm`](https://github.com/surf1k/rustcord/releases/latest) |
| **Linux** | `.tar.gz` (Generic portable) | `x86_64` | [`rustcord-v0.1.0-linux-x86_64.tar.gz`](https://github.com/surf1k/rustcord/releases/latest) |
| **macOS** | `.zip` (`Rustcord.app`) | Apple Silicon (`arm64`) | [`rustcord-macos-arm64.zip`](https://github.com/surf1k/rustcord/releases/latest) |

---

## Features

### ⚡ Extreme Performance & Low Memory Footprint
- **Microsoft `mimalloc` Global Allocator**: Eliminates glibc thread-arena memory fragmentation across 60+ threads.
- **Strict Resident Memory**: Runs at ~100–180 MiB RAM even in active voice calls with video streams (up to 10× lighter than official Discord).
- **GPU-Accelerated Immediate Mode UI**: Instant 60/120+ FPS rendering powered by `wgpu` without WebViews or Chromium overhead.

### 🎮 Automated Discord Quests
- **Port of Orion Automation Engine**: Background game spoofing, video task progress, and activity heartbeat loops.
- **Auto-Enroll & Auto-Claim**: Automatically discovers available quests, signs up, runs them in the background, and claims gift codes.
- **"⚡ Auto Complete All"**: One-click completion of all eligible desktop quests in sequence.

### 🔒 Persistent Sessions
- **Safe Session Restoration**: Automatically remembers your authenticated Discord token in a private `0600` local credential store.
- **Seamless Launch**: Restores your account, servers, and channels immediately upon launch without repeated logins.

### 🖥️ Native Screen Sharing & Voice
- **Wayland & Hyprland Support**: Monotonic presentation timestamps (PTS) for PipeWire portal buffers, eliminating frozen screen streams.
- **Hardware Acceleration**: Automatic NVENC (`nvh264enc`) hardware encoding with repeated sequence headers (SPS/PPS) for instant client decode.
- **Adjustable Presets**: Balanced (720p30), Sharp (1080p30), and Smooth (1080p60) screen capture presets.
- **Speaking Indicator**: Glowing green aura around your own avatar in voice channels whenever your microphone transmits.

### 📬 Global "Mark All as Read"
- **One-Click Clear**: Dedicated global checkmark button in the navigation rail instantly acknowledges all servers, channels, and mentions.
- **Immediate Local Clear**: Badges and pill indicators clear instantaneously without waiting for network round-trips.

### 📊 Accurate Resource Usage Monitor
- **Real-Time CPU %**: Normalized system load percentage calculated across all available CPU cores.
- **Live Resident RAM**: Exact resident set size (RSS) displayed in the bottom-right corner.

---

## Building from Source

### Prerequisites
- **Rust Toolchain**: `rustup default 1.98.1` (or latest stable)
- **Linux Dependencies**:
  ```sh
  # Fedora / RHEL
  sudo dnf install webkitgtk6.0-devel gtk4-devel gstreamer1-devel gstreamer1-plugins-base-devel libxkbcommon-devel pipewire-devel alsa-lib-devel pulseaudio-libs-devel

  # Ubuntu / Debian
  sudo apt-get install libgtk-4-dev libwebkitgtk-6.0-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libxkbcommon-dev libwayland-dev libx11-dev libasound2-dev libpulse-dev
  ```

### Build Binary
```sh
cargo build --release --locked -p serein --bin rustcord
```
The optimized binary will be created at `target/release/rustcord` (or `target/release/rustcord.exe` on Windows).

---

## Architecture

```
rustcord/
├── apps/
│   └── desktop/          # Main application entry point, window lifecycle, background workers
├── crates/
│   ├── client-core/      # Client state machine, cache, local unread tracking, permissions
│   ├── discord-api/      # Discord REST HTTP API, quests automation, rate limits
│   ├── discord-gateway/  # Real-time WebSocket Gateway client (v9)
│   ├── discord-voice/    # WebRTC voice, opus audio engine, video/screen capture (GStreamer)
│   ├── discord-protocol/ # Wire protocol serialization and deserialization
│   ├── local-store/      # SQLite local database and secure credential storage
│   ├── model/            # Strongly-typed Discord domain models (Guild, Channel, Message, etc.)
│   ├── platform/         # OS integration (notifications, secret storage, portal screencast)
│   └── ui/               # Complete GUI layout, custom widgets, styling, and navigation
```

---

## License

Dual-licensed under either:
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
