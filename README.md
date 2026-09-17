# jenkins-stream

Streams Jenkins build logs to your terminal.

## Install

```bash
cargo install --git <this git url>
```

## Build from source

```bash
cargo build --release
```

## Usage

```
jenkins-stream [OPTIONS]
```

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--domain` | `-d` | `$JENKINS_URL` | Jenkins base URL |
| `--builder` | `-b` | _(interactive)_ | Job name |
| `--number` | `-n` | _(latest)_ | Build number |
| `--wait` | `-w` | `false` | Wait for the build to appear |
| `--min-query` | | `100` ms | Minimum polling interval |
| `--max-query` | | `5000` ms | Maximum polling interval |

### Examples

```bash
# Stream the latest build (interactive job picker)
jenkins-stream

# Stream a specific job and build
jenkins-stream --builder my-pipeline --number 42

# Wait for the next build to start, then stream it
jenkins-stream --builder my-pipeline --wait
```
