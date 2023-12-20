### EleX Proxy ( English | [中文文档](README-ZH.md) )

![GitHub release (with filter)](https://img.shields.io/github/v/release/AstroxNetwork/elex-proxy)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**EleX Proxy** is a lightweight Rust implementation designed to proxy communication with the [Atomicals ElectrumX](https://github.com/atomicals/atomicals-electrumx) server. This project aims to provide a simple and efficient solution for handling ElectrumX requests.

#### Installation

1. Open the [GitHub Releases](https://github.com/AstroxNetwork/elex-proxy/releases) page to download the latest version.
2. Download the compressed archive for your platform.
3. Extract and run the executable file.

#### Configuration

Create a file named `.env` in the project's root directory with the following content:

```dotenv
# Host and port on which the proxy server will listen
PROXY_HOST=0.0.0.0:12321
# Default wss://electrumx.atomicals.xyz:50012, use commas to separate multiple servers
ELECTRUMX_WSS=wss://electrumx.atomicals.xyz:50012
# Default 1, interval in seconds to add new capacity
IP_LIMIT_PER_SECOND=1
# Default 10, burst size; if bursts exceed this value within the time set in IP_LIMIT_PER_SECOND, it will be restricted
IP_LIMIT_BURST_SIZE=10
# Default 500, maximum concurrent connections
CONCURRENCY_LIMIT=500
# Default 10, timeout for receiving WebSocket messages
RESPONSE_TIMEOUT=10
RUST_LOG=info
```

Adjust these values as needed. Here's a brief explanation of the configuration parameters:

- `PROXY_HOST`: The host and port on which the proxy server will listen.
- `ELECTRUMX_WSS`: ElectrumX server(s) to connect to. Use commas to separate multiple servers.
- `IP_LIMIT_PER_SECOND`: Interval in seconds to add new capacity.
- `IP_LIMIT_BURST_SIZE`: Burst size; if bursts exceed this value within the time set in `IP_LIMIT_PER_SECOND`, it will be restricted.
- `CONCURRENCY_LIMIT`: The maximum number of concurrent connections allowed.
- `RESPONSE_TIMEOUT`: Timeout for receiving WebSocket messages.
- `RUST_LOG`: Log level for Rust's logging framework. Options include `trace`, `debug`, `info`, `warn`, and `error`.

#### Usage

Once the proxy server is running, it adeptly forwards ElectrumX requests to the specified server(s). If multiple servers are configured, it seamlessly switches to the next server after a disconnection. Clients can effortlessly connect to the configured `PROXY_HOST`.

### License

This project is released under the MIT License - for detailed information, please refer to the [LICENSE](LICENSE) file.

**Note:** It is advised to review and customize the configuration according to your specific requirements.