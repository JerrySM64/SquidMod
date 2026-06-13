# SquidMod

SquidMod is a cross-platform modding tool for Splatoon. It works with Cemu, Xapfish, and real Wii U consoles. It provides a graphical interface for memory editing, real-time match information, and plugin management.

## Features

- **Memory Editing**: Read and write process memory for real-time game modifications.
- **Wii U Support**: Connect to a real Wii U console via TCPGecko for memory editing.
- **Player Information**: Display player details including name, PID, and gear information.
- **Match Information**: Display current match details including player and team data.
- **Scene Information**: Show current game scene state.
- **Memory Viewer**: Inspect memory at specific addresses.
- **Plugin System**: Extend functionality with plugins.
- **Network Support**: Compatible with Pretendo and Spacebar.
- **Cross-Platform**: Native support for Windows, Linux, and macOS.

## Supported Platforms

| Platform | Status |
|----------|--------|
| Windows | Supported |
| Linux (glibc) | Supported |
| macOS | Supported |

## Building from Source

### Prerequisites

- **Rust** (latest stable toolchain)
- **GTK 4** and **libadwaita**
- **pkg-config**

### Platform-Specific Requirements

**Windows**: MSYS2 environment with the MINGW64 toolchain.

**Linux**: GTK4 and libadwaita development packages for your distribution.

**macOS**: Homebrew with `gtk4`, `libadwaita`, and `pkg-config`. For release builds, `dylibbundler` is also required.

### Build

```bash
cargo build --release
```

For platform-specific release builds, use the provided Makefile:

```bash
# Linux AppImage (glibc)
make linux-release

# Windows ZIP
make windows-release

# macOS .app bundle
make macos-release
```

## Development

A Nix flake is provided for reproducible development environments:

```bash
nix develop
```

## Acknowledgments

SquidMod is based on the original [PNIDGrab](https://github.com/JerrySM64/PNIDGrab) project. Special thanks to the following people for their help:

- [c8ff](https://github.com/c8ff)
- [javiig8](https://github.com/javiig8)
- [Tombuntu](https://github.com/ReXiSp)
- [CrafterPika](https://github.com/CrafterPika)
- [RusticMaple](https://github.com/RusticMaple)
- [vyrval](https://github.com/tvyrval)
- [oomi\_the\_octo](https://github.com/oomi-the-octo)

## License

This project is licensed under the MIT License.
