# AltStore Repository Server

A dynamic AltStore repository server built with Rust and Axum that automatically generates `repository.json` endpoint from filesystem-discovered IPA files.

## Quick Start

### Option 1: Using Docker (Recommended)

1. **Build the Docker image:**
   ```bash
   docker build -f docker/Dockerfile -t altstore-repository-server .
   ```

2. **Run the container:**
   ```bash
   docker run -d \
     --name altstore-repository-server \
     -p 8080:8080 \
     -v $(pwd)/apps:/apps:ro \
     -v $(pwd)/config.json:/app/config.json:ro \
     -e EXTERNAL_BASE_URL=https://your-domain.com \
     altstore-repository-server
   ```

3. **Access your repository:**
   ```
   http://localhost:8080/repository.json
   ```

### Option 2: Native Binary

#### Prerequisites

- Nix with flakes support (for development environment)
- Rust toolchain (provided by devenv)
- Create your own config.json from [AltStudio](https://altstudio.app/)

#### Setup

1. **Enter the development environment:**
   ```bash
   direnv allow  # or: nix develop
   ```

2. **Configure environment variables (optional):**
   ```bash
   cp .env.example .env
   # Edit .env with your settings
   ```

3. **Build and run:**
   ```bash
   cargo build --release
   ./target/release/altstore-repository-server --help
   ./target/release/altstore-repository-server
   ```

## Configuration

The server can be configured using **CLI arguments** or **environment variables**. CLI arguments take precedence over environment variables.

### CLI Arguments

```bash
altstore-repository-server --help
```

| Argument | Environment Variable | Description | Default |
|----------|---------------------|-------------|---------|
| `--listen-url` | `LISTEN_URL` | Server bind address | `0.0.0.0` |
| `--listen-port` | `LISTEN_PORT` | Server port | `8080` |
| `--external-base-url` | `EXTERNAL_BASE_URL` | Public URL for download links | `http://<listen-url>:<listen-port>` |
| `--apps-dir` | `APPS_DIR` | Directory containing IPA files | `apps` |

### Examples

**Using CLI arguments:**
```bash
./altstore-repository-server \
  --listen-port 9123 \
  --external-base-url https://altstore.example.com \
  --apps-dir /path/to/apps
```

**Using environment variables:**
```bash
export LISTEN_PORT=9123
export EXTERNAL_BASE_URL=https://altstore.example.com
export APPS_DIR=/path/to/apps
./altstore-repository-server
```

**Using .env file:**
```bash
cp .env.example .env
# Edit .env with your settings
./altstore-repository-server
```

### config.json Structure

The `config.json` file contains repository and app metadata (without versions):

```json
{
  "name": "My Repository",
  "identifier": "com.example.repo",
  "sourceURL": "https://example.com/repository.json",
  "apps": [
    {
      "name": "AppName",
      "bundleIdentifier": "com.example.app",
      "developerName": "Developer",
      "localizedDescription": "App description",
      "iconURL": "https://example.com/icon.png",
      "tintColor": "ff0000",
      "category": "utilities",
      "screenshotURLs": ["..."],
      "appPermissions": {
        "entitlements": [...],
        "privacy": {...}
      }
    }
  ]
}
```

**Note:** The `versions` array is automatically populated from discovered IPA files.

## IPA File Naming Convention

The server extracts version information from IPA filenames:

### Format 1: Tweaked Apps (3 parts)
```
AppName_appVersion.ipa
```
Example: `YourApp_v1.2.3.ipa`
- Version: `20.26.7`
- Description: "Version 20.26.7 (tweak version: 5.2b1)"

### Format 2: Standard Apps (2 parts)
```
AppName_version.ipa
```
Example: `MyApp_1.2.3.ipa`
- Version: `1.2.3`
- Description: "Version 1.2.3"

**Important:** The app directory name must match the `name` field in `config.json`.

## Directory Structure

```
/
├── config.json           # Repository and app metadata
├── apps/                 # Apps directory (configurable with --apps-dir)
│   ├── AppName1/        # App directory (matches config.json name)
│   │   ├── App_1.0.0.ipa
│   │   └── App_1.1.0.ipa
│   └── AppName2/
│       └── App_2.0.0.ipa
└── src/                 # Server source code
```

## API Endpoints

### GET /repository.json
Returns the dynamically generated AltStore repository manifest.

**Response:**
```json
{
  "name": "...",
  "apps": [
    {
      "name": "...",
      "versions": [
        {
          "version": "1.0.0",
          "date": "2026-01-13",
          "downloadURL": "http://localhost:8080/apps/AppName/App_1.0.0.ipa",
          "size": 12345678
        }
      ]
    }
  ]
}
```

### GET /apps/:appName/:filename
Downloads the specified IPA file with streaming support.

## Workflow

### Adding a New App

1. Create app directory: `mkdir apps/AppName`
2. Add app metadata to `config.json`
3. Place IPA file: `apps/AppName/App_1.0.0.ipa`
4. Server automatically detects it on next `/repository.json` request (no restart needed)

### Updating an App Version

1. Place new IPA in app directory: `apps/AppName/App_1.1.0.ipa`
2. Server automatically detects it on next `/repository.json` request

### Announcing Updates

Add a news item to `config.json`:
```json
{
  "news": [
    {
      "appID": "com.example.app",
      "title": "App Updated",
      "caption": "Version 1.1.0 is now available!",
      "date": "2026-01-13T00:00:00Z",
      "notify": true,
      "tintColor": "ff0000",
      "identifier": "release-1.1.0"
    }
  ]
}
```

## Development

### Project Structure

- `src/main.rs` - Server entry point and configuration
- `src/models.rs` - Data structures for config/repository schemas
- `src/generator.rs` - Dynamic repository generation logic
- `src/discovery.rs` - IPA file discovery and indexing
- `src/state.rs` - Shared application state
- `src/routes/` - HTTP endpoint handlers
  - `repository.rs` - Repository manifest endpoint
  - `apps.rs` - IPA file download endpoint

### Building

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Run tests
cargo test
```

### Logging

Set the `RUST_LOG` environment variable to control log levels:
```bash
RUST_LOG=debug cargo run
```

## Deployment

### Option 1: Docker Deployment (Recommended)

1. **Build the Docker image:**
   ```bash
   docker build -f docker/Dockerfile -t altstore-repository-server:latest .
   ```

2. **Run the container:**
   ```bash
   docker run -d \
     --name altstore-repository-server \
     --restart unless-stopped \
     -p 8080:8080 \
     -v /path/to/apps:/apps:ro \
     -v /path/to/config.json:/app/config.json:ro \
     -e LISTEN_PORT=8080 \
     -e EXTERNAL_BASE_URL=https://altstore.example.com \
     -e RUST_LOG=altstore_server=info,tower_http=info \
     altstore-repository-server:latest
   ```

3. **Configure reverse proxy** (nginx/caddy) to forward to the server:

   **Nginx example:**
   ```nginx
   server {
       listen 443 ssl http2;
       server_name altstore.example.com;

       ssl_certificate /path/to/cert.pem;
       ssl_certificate_key /path/to/key.pem;

       location / {
           proxy_pass http://localhost:8080;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
           proxy_set_header X-Forwarded-Proto $scheme;
       }
   }
   ```

4. **Point AltStore clients to:** `https://altstore.example.com/repository.json`

### Option 2: Native Binary Deployment

1. **Build the release binary:**
   ```bash
   cargo build --release
   ```

2. **Copy binary and config to server:**
   ```bash
   scp target/release/altstore-repository-server server:/opt/altstore/
   scp config.json server:/opt/altstore/
   scp -r apps server:/opt/altstore/
   ```

3. **Create a systemd service** (optional):
   ```ini
   [Unit]
   Description=AltStore Repository Server
   After=network.target

   [Service]
   Type=simple
   User=altstore
   WorkingDirectory=/opt/altstore
   ExecStart=/opt/altstore/altstore-repository-server \
     --listen-port 8080 \
     --external-base-url https://altstore.example.com \
     --apps-dir /opt/altstore/apps
   Restart=always
   RestartSec=10

   [Install]
   WantedBy=multi-user.target
   ```

4. **Start the service:**
   ```bash
   sudo systemctl enable altstore-repository-server
   sudo systemctl start altstore-repository-server
   ```

5. **Configure reverse proxy** (nginx/caddy) to forward to the server

6. **Point AltStore clients to:** `https://altstore.example.com/repository.json`

### Pre-built Binaries

Pre-built binaries for Linux (amd64, arm64) and macOS (amd64, arm64) are automatically generated via GitHub Actions. Check the [Releases](../../releases) page for the latest builds.

## License

This is a personal project. See repository owner for licensing information.
