# Hellforge 1.0 development — Achievements

New Achievements tab with 25 launcher-level achievements derived from SQLite play_history, imported library entries and manually marked completed profiles. Completed flags stored in SQLite completed_profiles.

**Important:** achievements for enemies defeated, items collected, secrets found, or maps visited are NOT shown because generic GZDoom/DSDA/LZDoom/Zandronum processes do not expose standardized telemetry. A real integration would require an engine-side mod/bridge. Do not treat a successful process exit as proof a game was completed.

Build using Build_Hellforge.bat. Windows Tauri compilation was not available for verification in this environment.
