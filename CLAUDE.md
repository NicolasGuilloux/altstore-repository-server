# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a personal AltStore source repository for distributing iOS applications via AltStore. The repository manages IPA files and their metadata for sideloading to iOS devices.

## Development Environment

The project uses **Nix Flakes with devenv** for reproducible development environments.

### Setup Commands

```bash
# Enter the development environment (automatically via direnv)
direnv allow

# Or manually activate
nix develop

# Update dependencies
devenv update
```

### Configured Tools
- **Git**: Version control
- **Rust**: Axum-based web server for dynamic repository generation
- **Nix**: Package management via cachix/devenv-nixpkgs/rolling

## Repository Architecture

### Directory Structure

```
/
├── apps/                # App storage directory
│   └── YourApp/         # App-specific subdirectory
│       └── *.ipa       # iOS app packages (auto-discovered)
├── config.json          # Base configuration (app metadata without versions)
├── src/                 # Rust web server source code
│   ├── main.rs         # Server entry point
│   ├── models.rs       # Data structures for config/repository
│   ├── generator.rs    # Dynamic repository.json generator
│   ├── discovery.rs    # IPA file discovery
│   ├── state.rs        # Application state
│   └── routes/         # HTTP endpoints
├── Cargo.toml          # Rust dependencies
├── devenv.yaml         # Development environment config
├── devenv.nix          # Nix package definition
└── .env.example        # Environment variables template
```

### Dynamic Repository Generation

The server dynamically generates `repository.json` by combining:

1. **config.json** - Base app metadata (static):
   - Repository metadata (name, identifier, website, etc.)
   - App configurations (bundle ID, permissions, screenshots, etc.)
   - News items for updates

2. **Filesystem Discovery** - Version information (dynamic):
   - Automatically scans `apps/` directory for app subdirectories
   - Discovers IPA files in each app subdirectory
   - Extracts version info from filenames (e.g., `YourApp_v1.2.3.ipa`)
   - Generates download URLs pointing to the server
   - Includes file sizes and current date

The `/repository.json` endpoint serves this dynamically generated manifest.

### App Management Workflow

**Adding IPAs (Automatic Version Detection):**
1. Place IPA file in the app's directory under `apps/` (e.g., `apps/YourApp/YourApp_v1.2.3.ipa`)
2. Server automatically discovers and generates version entry
3. Filename format: `AppName_appVersion.ipa` or `AppName_version.ipa`
4. No manual JSON updates needed for versions!

**Configuring Apps (One-Time Setup):**
1. Create a subdirectory in `apps/` for the app (e.g., `apps/AppName/`)
2. Add app metadata to `config.json` in the `apps` array
3. Include: bundle ID, developer name, description, permissions, screenshots
4. The `versions` field can be left empty or include manual entries (will be merged with auto-discovered versions)
5. Optionally add news items to announce updates

**Server Deployment:**
1. Set environment variables (see `.env.example`)
2. Run `cargo build --release`
3. Start server: `./target/release/altstore-repository-server`
4. Server serves `/repository.json` dynamically and IPAs at `/apps/:app/:file`

## Key Files

- **config.json**: Base app metadata (without versions - those are auto-generated)
- **src/generator.rs**: Logic for parsing IPA filenames and generating versions
- **src/models.rs**: Rust data structures for config and repository JSON schemas
- **.env.example**: Environment variable configuration template
  - `LISTEN_URL`: Server bind address (default: 0.0.0.0)
  - `LISTEN_PORT`: Server port (default: 8080)
  - `EXTERNAL_BASE_URL`: Public URL for download links
  - `AUTH_TOKEN`: Optional authentication token for API access
  - `DOWNLOAD_SECRET`: Optional secret for obfuscated URLs (when set, disables direct downloads)
- **devenv.nix**: Development environment packages and language configurations
- **.envrc**: Direnv integration for automatic environment loading

## Common Tasks

### Adding a New App

1. Create a subdirectory in `apps/` for the app (e.g., `apps/AppName/`)
2. Add app metadata to `config.json` `apps` array:
   ```json
   {
     "name": "AppName",
     "bundleIdentifier": "com.example.app",
     "developerName": "Developer",
     "localizedDescription": "App description",
     "iconURL": "https://...",
     "tintColor": "ff0000",
     "category": "utilities",
     "screenshotURLs": [...],
     "appPermissions": {...},
     "versions": []
   }
   ```
3. Place IPA file(s) in the app subdirectory (e.g., `apps/AppName/app_1.0.0.ipa`)
4. Server automatically detects and generates versions

### Updating an App Version

1. Place new IPA file in the app's subdirectory with proper naming:
   - Location: `apps/AppName/filename.ipa`
   - Format: `AppName_appVersion.ipa` or `AppName_version.ipa`
   - Example: `apps/YourApp/YourApp_v1.2.3.ipa`
2. Server automatically adds the new version to `/repository.json`
3. Optionally update `news` array in `config.json` to notify users

### Modifying Repository Metadata

Edit the top-level fields in `config.json`:
- `name`, `subtitle`, `description`: Display information
- `tintColor`: Hex color code (without #)
- `iconURL`: Repository icon URL
- `website`, `sourceURL`: Links for users

### Running the Server Locally

```bash
# Set environment variables
export LISTEN_PORT=8080
export EXTERNAL_BASE_URL="http://localhost:8080"

# Build and run
cargo run

# Test the repository endpoint
curl http://localhost:8080/repository.json
```

## Distribution Notes

- IPAs can be stored locally or on external cloud storage
- When using the Rust server:
  - IPAs are served via two possible endpoints (see Download Security below)
  - Download URLs are dynamically generated based on `EXTERNAL_BASE_URL`
  - Large files are streamed efficiently using Tokio
- The AltStore client fetches `/repository.json` from the server

### Download Security

The server supports two download modes based on the `DOWNLOAD_SECRET` environment variable:

**Without DOWNLOAD_SECRET (Standard Mode):**
- Direct downloads: `/apps/:appName/:filename` endpoint is available
- Repository JSON contains standard URLs like `/apps/AppName/app.ipa`
- URLs are predictable but simpler to configure

**With DOWNLOAD_SECRET (Secure Mode):**
- Direct downloads: `/apps/:appName/:filename` returns **403 Forbidden**
- Obfuscated downloads: Only `/download/:token` endpoint works
- Repository JSON contains obfuscated URLs like `/download/abc123xyz`
- Tokens are deterministic SHA256-based hashes (app + filename + secret)
- Prevents unauthorized access by URL guessing
- **Recommended for production deployments**

## Development Guidelines

- This is a personal repository not meant for public sharing
- The Rust server handles dynamic repository generation
- IPA files are pre-built and obtained from upstream sources
- Follow filename conventions for automatic version detection
- Metadata in `config.json` should be accurate and complete
- Test the server locally before deploying changes

## Testing Guidelines

This project follows **Test-Driven Development (TDD)** methodology. Tests must be written before implementation code to ensure comprehensive coverage and better design.

### TDD Workflow (Red-Green-Refactor)

1. **Red**: Write a failing test that describes the expected behavior
2. **Green**: Write the minimum implementation code to make the test pass
3. **Refactor**: Improve the code while keeping tests passing

### Test Coverage Expectations

Every new feature or bug fix should include tests covering:

- **Happy paths**: Normal, expected use cases
- **Edge cases**: Boundary conditions, empty inputs, minimum/maximum values
- **Error conditions**: Invalid inputs, missing files, malformed data

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output displayed
cargo test -- --nocapture

# Run tests for a specific module
cargo test generator::tests

# Check that tests compile without running them
cargo test --no-run
```

### Test Organization

Tests are organized as inline modules at the bottom of each source file using Rust's convention:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example() {
        // Test implementation
    }
}
```

**Source files with tests:**
- `src/generator.rs` - Repository generation logic tests
- `src/discovery.rs` - IPA file discovery tests
- `src/token.rs` - Download token generation tests
- `src/ipa_info.rs` - IPA metadata extraction tests

### Writing New Tests

When adding new functionality:

1. **First**, write tests that describe the expected behavior
2. **Run tests** to confirm they fail (red phase)
3. **Implement** the minimum code to pass the tests (green phase)
4. **Refactor** for clarity and maintainability
5. **Verify** all tests still pass

## IPA Filename Conventions

The server parses IPA filenames to extract version information:

**Three-part format** (recommended for tweaked apps):
- Pattern: `AppName_appVersion.ipa`
- Example: `YourApp_v1.2.3.ipa`
- Generates: version "20.26.7", description includes "tweak version: 5.2b1"

**Two-part format** (standard apps):
- Pattern: `AppName_version.ipa`
- Example: `MyApp_1.2.3.ipa`
- Generates: version "1.2.3"

**Version Merging:**
- Manual versions can be defined in `config.json` with custom descriptions and dates
- When an IPA file matches a manual version (by version string), they are merged:
  - Manual description and date are preserved
  - Download URL and file size are updated from the IPA file
- New IPA files not in `config.json` are auto-generated with default descriptions

**Important:** The app subdirectory name in `apps/` must match the `name` field in `config.json`.
