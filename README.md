### EleX Proxy ( English | [中文文档](README-ZH.md) )

![GitHub release (with filter)](https://img.shields.io/github/v/release/AstroxNetwork/elex-proxy)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**EleX Proxy** is a lightweight Rust implementation designed to proxy communication with [Atomicals ElectrumX](https://github.com/atomicals/atomicals-electrumx) servers. The project aims to provide a simple and efficient solution for handling ElectrumX requests.

#### Installation

1. Open the [GitHub Releases](https://github.com/AstroxNetwork/elex-proxy/releases) page to download the latest version.
2. Download the compressed package for your platform.
3. Extract and run the executable file.

#### Configuration

Create a file named `.env` in the project's root directory with the following content:

```dotenv
# Host and port the proxy server listens on
PROXY_HOST=0.0.0.0:12321
# Default wss://electrumx.atomicals.xyz:50012, comma-separated for multiple servers
ELECTRUMX_WSS=wss://electrumx.atomicals.xyz:50012
# Default 1, add 1 allowed access every xx seconds
# IP_LIMIT_PER_SECOND=1
# Default 10, add 1 allowed access every xx milliseconds
IP_LIMIT_PER_MILLS=1
# Default 10, if this value is used up, new access will be limited.
IP_LIMIT_BURST_SIZE=10
# Default 1, concurrently running ws instances, can improve throughput, set as needed
ELECTRUMX_WS_INSTANCE=5
# Default 500, maximum concurrent connections
CONCURRENCY_LIMIT=500
# Default 10s, timeout for receiving WebSocket messages
RESPONSE_TIMEOUT=10

# Default 10000, max cache entry
MAX_CACHE_ENTRIES=10000
# Default 480s, cache max live time
CACHE_TIME_TO_LIVE=480
# Default 60s, cache idle time, if no access, cache will be removed
CACHE_TIME_TO_IDLE=60

RUST_LOG=info
```

Adjust these values as needed. Here's a brief explanation of the configuration parameters:

- `PROXY_HOST`: Host and port the proxy server listens on.
- `ELECTRUMX_WSS`: ElectrumX servers to connect to. Comma-separated for multiple servers.
- `IP_LIMIT_PER_SECOND`: Add 1 allowed access every xx seconds.
- `IP_LIMIT_PER_MILLS`: Add 1 allowed access every xx milliseconds.
- `IP_LIMIT_BURST_SIZE`: If this value is used up, new access will be limited.
- `ELECTRUMX_WS_INSTANCE`: Concurrently running ws instances, can improve throughput, set as needed.
- `CONCURRENCY_LIMIT`: Maximum allowed concurrent connections.
- `RESPONSE_TIMEOUT`: Timeout for receiving WebSocket messages.
- `MAX_CACHE_ENTRIES`: Maximum cache entry.
- `CACHE_TIME_TO_LIVE`: Cache max live time.
- `CACHE_TIME_TO_IDLE`: Cache idle time, if no access, cache will be removed.
- `RUST_LOG`: Log level for Rust logging framework. Options include `trace`, `debug`, `info`, `warn`, and `error`.

#### Usage

Once the proxy server is running, it will forward ElectrumX requests to the specified server. If multiple servers are configured, it will switch to the next server after one server disconnects. Clients can connect to the configured `PROXY_HOST`.

### License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

**Note:** It is recommended to review and customize the configuration according to your specific requirements.