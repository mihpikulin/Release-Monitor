# Release Monitor

Release Monitor is a small Rust service that watches GitHub repositories for new releases and runs a configured shell command when a release tag changes. It stores the last observed tag in SQLite, can run interactively, and includes a systemd service plus installation scripts for Linux.

> [!WARNING]
> Release Monitor and its `on_release` commands run with root privileges when installed as a system service. Commands are passed directly to `sh -c`. Treat the configuration file as root-executable code, restrict who can edit it, and never insert untrusted values into `on_release`.

## Features

- Polls GitHub's `releases/latest` endpoint at a configurable interval
- Monitors multiple public GitHub repositories
- Executes a repository-specific shell command when a tag changes
- Persists the last processed tag in a local SQLite database
- Synchronizes database entries with the YAML configuration
- Runs in the foreground or as a systemd service
- Provides commands to start, stop, restart, inspect, add, and remove repositories

## How it works

On startup, Release Monitor creates the `repositories` table if necessary and synchronizes it with the configured repository list. New entries begin with the stored release `0.0.0`.

For every polling cycle, it:

1. Reads the tracked repositories from SQLite.
2. Requests `https://api.github.com/repos/<owner>/<name>/releases/latest`.
3. Compares the returned `tag_name` with the stored release.
4. Runs the repository's `on_release` command if the tags differ.
5. Updates SQLite only after the command succeeds.
6. Sleeps for `refresh_interval` seconds and repeats.

This means a newly configured repository normally triggers its command on the first successful check. If a command fails, the process exits without updating the stored tag; with systemd, the service is then restarted and will retry.

## Requirements

- Linux with systemd
- Rust 1.85 or newer with Cargo (for building from source)
- Network access to `api.github.com`
- SQLite development libraries when building `rusqlite` without its `bundled` feature
- Root privileges for installation and for running Release Monitor

The monitor currently uses unauthenticated GitHub API requests. GitHub rate limits therefore apply, so choose the polling interval with the number of repositories in mind.

## Configuration

The system service reads `/etc/release-monitor/config.yaml`:

```yaml
refresh_interval: 3600

repositories:
  - name: netbird
    owner: netbirdio
    on_release: |
      echo "A new NetBird release is available"
      /usr/local/sbin/update-netbird

  - name: caddy
    owner: caddyserver
    on_release: "systemctl restart my-dependent-service"
```

| Field | Type | Meaning |
| --- | --- | --- |
| `refresh_interval` | unsigned integer | Delay in seconds between complete polling cycles |
| `repositories` | list | Repositories that should be synchronized into SQLite and monitored |
| `repositories[].owner` | string | GitHub organization or account name |
| `repositories[].name` | string | GitHub repository name |
| `repositories[].on_release` | string | Shell program passed to `sh -c` when the latest tag changes |

The service and every configured `on_release` command run as root. Use absolute paths because the service has a minimal environment and uses `/var/lib/release-monitor` as its working directory.

`on_release` is currently executed verbatim. Placeholder strings such as `{{ current_version }}` and `{{ new_version }}` are **not** interpolated by the application.

Removing a repository from the YAML file removes its matching SQLite row at the next monitor startup. Changing `owner`, `name`, or `on_release` is treated as removing one entry and adding another, so its release state resets to `0.0.0`.

## Installation

Linux release archives are published on the [GitHub Releases page](https://github.com/mihpikulin/Release-Monitor/releases). Download the `.tar.gz` archive for the version and architecture you want to install.

Each archive contains:

```text
release-monitor
install.sh
uninstall.sh
```

Download and extract the archive, then run the installer as root. The installed `release-monitor` commands must also be run as root:

```bash
wget https://github.com/mihpikulin/Release-Monitor/releases/download/v0.2.0/release-monitor_v0.2.0.tar.gz
tar -xzf release-monitor_v0.2.0.tar.gz
cd release-monitor_v0.2.0
sudo ./install.sh
sudo release-monitor start
```

Replace `<version>` and `<architecture>` with the values used by the downloaded release. The exact archive and extracted-directory names may vary between releases.

The installer runs with root privileges and:

- installs the binary at `/usr/local/bin/release-monitor`;
- installs or preserves `/etc/release-monitor/config.yaml`;
- creates `/var/lib/release-monitor` for persistent state;
- installs and enables `release-monitor.service`.

Edit the configuration, then restart the service:

```bash
sudoedit /etc/release-monitor/config.yaml
sudo release-monitor restart
```

Follow logs with:

```bash
journalctl -u release-monitor -f
```

### Build from source

To build the application yourself instead of using a release archive:

```bash
git clone https://github.com/mihpikulin/Release-Monitor.git
cd Release-Monitor
cargo build --release
```

The binary is written to `target/release/release-monitor`. To create the same installation layout locally, place that binary alongside `install.sh` and `uninstall.sh`, then run `sudo ./install.sh` from that directory.

To inspect the CLI during development:

```bash
cargo run -- --help
cargo run -- add --help
cargo run -- remove --help
```

## CLI usage

```text
release-monitor [OPTIONS] <COMMAND>
```

| Command | Description |
| --- | --- |
| `run` | Run the polling loop in the current process |
| `start` | Start `release-monitor.service` through systemd |
| `stop` | Stop the systemd service |
| `restart` | Restart the systemd service |
| `status` | Show systemd state and the repository rows stored in SQLite |
| `add` | Append a repository to the system YAML configuration and synchronize SQLite |
| `remove` | Remove a repository from the system YAML configuration and synchronize SQLite |

Examples:

```bash
# Run with the system configuration
sudo release-monitor run

# Run with a different repository configuration
sudo release-monitor --config ./my-config.yaml run

# Add an entry to /etc/release-monitor/config.yaml
sudo release-monitor add \
  --owner netbirdio \
  --name netbird \
  --on-release '/usr/local/sbin/update-netbird'

# Remove an entry from the configuration and SQLite state
sudo release-monitor remove \
  --owner netbirdio \
  --name netbird

# Query service and repository state
sudo release-monitor status
```

Both `--name` and `--owner` are required by `remove`. Every configuration entry matching that owner/name pair is removed. The command reports an error without changing the configuration when no match exists.

The `--config` option affects the `run` command. Other commands, including `add` and `remove`, operate on the fixed system paths. Run all installed commands with root privileges because they initialize or check `/etc/release-monitor` and `/var/lib/release-monitor` before dispatch. A foreground `run` also executes its configured `on_release` commands as root.

## Files and directories

| Path | Purpose |
| --- | --- |
| `/usr/local/bin/release-monitor` | Installed executable |
| `/etc/release-monitor/config.yaml` | System YAML configuration |
| `/var/lib/release-monitor/repositories.db` | SQLite release state |
| `/etc/systemd/system/release-monitor.service` | Installed systemd unit |

The SQLite `repositories` table contains `id`, `name`, `owner`, `release`, and `on_release`. It is application-managed and should not normally be edited manually.

## Uninstall

Run the supplied interactive uninstaller from the directory extracted from the release archive. Keep the archive or extracted `uninstall.sh` if you may need it later:

```bash
cd release-monitor-<version>-<architecture>
sudo ./uninstall.sh
```

It stops and disables the service and removes the installed binary and unit. It asks separately before deleting configuration and application data, allowing `/etc/release-monitor` and `/var/lib/release-monitor` to be preserved.

## Current limitations

- Only public repositories accessible without GitHub authentication are supported.
- Only GitHub releases returned by `releases/latest` are considered; tags without a GitHub Release are ignored.
- There is no command timeout, templating, notification backend, or per-repository polling interval.
- A failing release command terminates the monitor loop.
- Updating the stored version matches by owner and name, so duplicate entries for the same repository should be avoided.
- The project currently contains no automated test cases.

## License

MIT License

Copyright (c) 2026 Mikhail Pikulin

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
