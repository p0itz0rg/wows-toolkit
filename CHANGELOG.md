# Changelog

All notable changes to this project will be documented in this file.

## [0.1.25] - 2024-11-17

### 🚀 Features

- *(player_tracker)* Only consider ranked / random battles
- Add twitch integration to detect stream snipers
- *(player_tracker)* Ignore players in division
- *(player_tracker)* Add more time ranges for time filter
- *(player_tracker)* Add players from current match with stream sniper detection
- *(settings)* Allow customizing which twitch channel to watch for player tracker

### 🐛 Bug Fixes

- Bug with loading game data when no locale is set

## [0.1.24] - 2024-11-15

### 🚀 Features

- *(player_tracker)* Add editable player notes

### 🐛 Bug Fixes

- *(player_tracker)* Fix bug with sorting encounters in time range
- *(player_tracker)* Colors stopped for high numbers
- Dark mode did not work for system-wide light mode users

### Player_tracker

- Change default sort to be times encountered within the tim range

## [0.1.23] - 2024-11-15

### 🚀 Features

- *(replays)* Add checkbox to auto-load most recent replay
- *(replays)* Colorize base XP and damage
- Add new player tracker tab
- *(replays)* Add hover text to break down damage by damage type

### 🐛 Bug Fixes

- *(replays)* Fix operation replays failing to load

### ⚙️ Miscellaneous Tasks

- Update gui

### Replays

- Adjust some table column sizes
- Enable auto loading of latest replay by default

## [0.1.22] - 2024-11-13

### 🚀 Features

- *(replays)* Add base xp

### 🐛 Bug Fixes

- *(replays)* Fixed total damage numbers reflecting incorrect teams

### ⚙️ Miscellaneous Tasks

- Update changelog

## [0.1.21] - 2024-11-12

### 🚀 Features

- *(replays)* Show which division a player was in (div letters probably don't match in-game)
- Default wows dir was previously broken, now should work

### 🐛 Bug Fixes

- Resolved application hang when first using the application

## [0.1.20] - 2024-11-11

### 🚀 Features

- *(replays)* Add total damage dealt in a match between the teams
- *(replays)* Selected replay will be highlighted in sidebar
- *(replays)* Add indicator for if a player disconnected from match
- *(replays)* Add action button to see raw player metadata

### 🐛 Bug Fixes

- Log file rotates hourly to reduce total log file size
- *(replays)* Airstrike and plane potential damage are the same

### ⚙️ Miscellaneous Tasks

- Update replay screenshot
- Use better screenshot
- Add github discord workflow
- Bump version to v0.1.20

## [0.1.19] - 2024-11-10

### 🚀 Features

- Show actual damage numbers
- Add button for showing raw battle results
- Add potential and spotting damage + fixed some labels

### ⚙️ Miscellaneous Tasks

- Add upgrade path for re-generating game params in v0.1.19
- Bump version to v0.1.19

## [0.1.18] - 2024-09-14

### 🚀 Features

- *(replays)* Add more statuses to indicate some action was done

### 🐛 Bug Fixes

- *(replays)* Fix bug where app would crash if it was focused at the end of a match
- *(settings)* Setting WoWs directory didn't work so well
- *(replays)* Chat is visually more appealing, easier to read (fixes #3)
- *(app)* Only show update window if there's a build to download

## [0.1.17] - 2024-09-05

### 🐛 Bug Fixes

- *(replays)* Watch replays directory only

## [0.1.16] - 2024-09-05

### 🚀 Features

- *(file_unpacker)* Add support for serializing as JSON/CBOR, including for WoWs Toolkit's internal representation
- Game version updates are auto-detected and new files will be auto-loaded
- *(replays)* Add support for ranked and sending ranked builds back to ShipBuild
- *(replays)* Consolidate the manual replay loading into a single button

## [0.1.15] - 2024-09-03

### 🚀 Features

- *(replays)* Add button for exporting game chat
- *(replays)* Add support for sending replays that were created when app was closed

### 🐛 Bug Fixes

- Log files were not cleared
- *(replays)* Fix ci compilation error

## [0.1.14] - 2024-08-30

### 🐛 Bug Fixes

- *(settings)* Sending replay data was not enabled by default

## [0.1.13] - 2024-08-30

### 🐛 Bug Fixes

- *(replays)* Replays would not show any data when parsing

### ⚙️ Miscellaneous Tasks

- Update changelog

## [0.1.12] - 2024-08-30

### 🚀 Features

- *(resource_unpacker)* Add button for dumping GameParams.json
- Automatically send builds to ShipBuilds.com

### 🚜 Refactor

- Use crates.io versions of wowsunpack and wows_replays

### ⚙️ Miscellaneous Tasks

- Cargo fix
- Cargo fmt

## [0.1.11] - 2024-06-12

### ⚙️ Miscellaneous Tasks

- Update changelog

## [0.1.10] - 2024-04-02

### 🐛 Bug Fixes

- *(replays)* Fix incompatability with 13.2.0

### ⚙️ Miscellaneous Tasks

- Oops updated changelog before tagging
- Bump version

## [0.1.9] - 2024-03-11

### 🐛 Bug Fixes

- *(replays)* Replays in build-specific dirs should now work

### ⚙️ Miscellaneous Tasks

- Add changelog
- Bump version
- Update changelog

## [0.1.8] - 2024-03-10

### 🚀 Features

- Add support for tomato.gg

### 🐛 Bug Fixes

- *(replays)* Double processing of replays
- Ensure replays dir is correctly reset if wows dir changes
- Improve perf for file listing filter + regression from egui update
- Ensure the found replays dir is used for loading replay files

### 🚜 Refactor

- Tab_name -> title

### ⚙️ Miscellaneous Tasks

- Update egui deps
- Cargo fix
- Bump version

<!-- generated by git-cliff -->
