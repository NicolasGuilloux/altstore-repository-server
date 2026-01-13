{ pkgs, lib, config, inputs, ... }:

{
  imports = [ ./devenv.ai.nix ];

  # https://devenv.sh/packages/
  packages = with pkgs; [
    git
    cargo-watch  # For auto-reload during development
  ];

  # https://devenv.sh/languages/
  languages.rust.enable = true;

  # https://devenv.sh/processes/
  processes.cargo-watch.exec = "cargo-watch";

  # https://devenv.sh/scripts/
  scripts.serve.exec = ''
    echo "Starting AltStore server in development mode..."
    cargo run
  '';

  scripts.serve-release.exec = ''
    echo "Building and starting AltStore server in release mode..."
    cargo build --release
    ./target/release/altstore-repository-server
  '';

  scripts.build.exec = ''
    echo "Building AltStore server (release)..."
    cargo build --release
    echo "✓ Binary created at: target/release/altstore-repository-server"
  '';

  scripts.build-dev.exec = ''
    echo "Building AltStore server (debug)..."
    cargo build
  '';

  scripts.watch.exec = ''
    echo "Starting AltStore server with auto-reload on file changes..."
    cargo watch -x run
  '';

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  # https://devenv.sh/tests/
  enterTest = ''
  '';

  # https://devenv.sh/git-hooks/
  pre-commit.hooks = {
    # Format Rust code
    rustfmt = {
      enable = true;
      description = "Format Rust code with rustfmt";
    };

    # Lint Rust code
    clippy = {
      enable = true;
      description = "Lint Rust code with clippy";
      entry = "cargo clippy --all-features --all-targets -- -D warnings";
    };
  };

  # See full reference at https://devenv.sh/reference/options/
  dotenv.disableHint = true;
}
