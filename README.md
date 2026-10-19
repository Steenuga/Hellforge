<div align="center">
  <img src="assets/hellforge-icon.png" alt="Hellforge emblem" width="135" />
  <br />
  <img src="assets/hellforge-wordmark.png" alt="HELLFORGE" width="500" />
  <p><strong>Your DOOM universe. One launcher.</strong></p>
  <p>A customizable Windows launcher for classic DOOM games, mods, maps, and custom adventures.</p>
  <p><strong>English</strong> · <a href="README.uk.md">Українська</a></p>
  <p><img alt="Platform: Windows" src="https://img.shields.io/badge/platform-Windows-2563eb" /> <img alt="Version: 1.0" src="https://img.shields.io/badge/version-1.0-c9342d" /> <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-555" /> <img alt="Languages: 23" src="https://img.shields.io/badge/languages-23-8844cc" /></p>
</div>

---

## About

**Hellforge** brings your classic DOOM collection together in one modern desktop launcher. Organize IWADs, mods, maps, and custom profiles, then launch your adventures through your installed source port.

## Features

- **Game library:** add your own IWAD files and organize supported classic games.
- **Mods & maps:** import WAD, PK3, PK7, and ZIP files into separate libraries; rename library entries without renaming files on disk.
- **Custom profiles:** combine a game, a source port, maps, and mods; control the load order and customize profile covers.
- **Source ports:** configure your installed GZDoom or select a custom executable.
- **OBSIDIAN integration:** generate random DOOM maps using a separately installed OBSIDIAN v20 executable.
- **Launcher statistics:** track launches and playtime, view launch history, and mark completed profiles manually. No in-game tracking required.
- **Personalization:** seven interface themes, profile groups, and a multilingual interface with 23 language options.
- **Local library:** SQLite storage, with metadata/settings backup and restore tools.

## Getting started

> **Windows release:** the downloadable installer will be linked here after the release build has been compiled and tested.

1. Install a supported DOOM source port, such as [GZDoom](https://zdoom.org/downloads).
2. Prepare your own legally obtained game IWAD (for example, `DOOM2.WAD`).
3. In Hellforge, add the source-port executable and import your IWAD.
4. Optionally import mods and maps.
5. Create a profile, set the load order, and select **Play**.

Hellforge does **not** include commercial DOOM game files.

## Build from source

Requirements: Windows, Node.js LTS, Rust with the MSVC toolchain, Microsoft C++ Build Tools, and the WebView2 runtime.

```powershell
npm install
npm run tauri dev
```

To produce a Windows release build:

```powershell
npm run tauri build
```

You can also use `Build_Hellforge.bat` in the project root. Release binaries should be published under [GitHub Releases](https://github.com/Steenuga/Hellforge/releases) after testing.

## Languages

Ukrainian, English, Polish, German, French, Spanish, Japanese, Simplified Chinese, Czech, Slovak, Bulgarian, Croatian, Serbian, Belarusian, Romanian, Lithuanian, Latvian, Estonian, Georgian, Kazakh, Uzbek, Azerbaijani, and Armenian.

Translations are community-correctable. Feel free to open an issue or submit a pull request with improvements.

## Technology

- [Tauri 2](https://tauri.app/) and **Rust** for the desktop backend
- **React** and **TypeScript** for the interface
- **SQLite** for library data

## Feedback & contributions

Found a bug, a translation issue, or a usability problem? Please use [Issues](https://github.com/Steenuga/Hellforge/issues). Contributions and pull requests are welcome.

## Legal

Hellforge is an independent community project and is not affiliated with id Software, Bethesda, or ZeniMax. DOOM and related trademarks belong to their respective owners. You must supply your own game files and any external tools.

## License

Licensed under the [MIT License](LICENSE).
