### EleX Proxy ( English | [中文文档](README-ZH.md) )

![GitHub release (with filter)](https://img.shields.io/github/v/release/AstroxNetwork/elex-proxy)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/AstroxNetwork/elex-proxy/blob/main/LICENSE)

**EleX Proxy** stands as a sophisticated Rust implementation, meticulously crafted to serve as a proxy for seamless communication with [ElectrumX](https://github.com/atomicals/atomicals-electrumx) servers. This software project has been developed with precision to offer an advanced and efficient solution for managing ElectrumX requests.

#### Installation

1. Visit the [GitHub Releases](https://github.com/AstroxNetwork/elex-proxy/releases) page to acquire the latest release.
2. Download the compressed archive tailored for your platform.
3. Extract the archive and execute the provided binary.

#### Configuration

Create a `.env` file in the project's root directory, populating it with the following configuration:

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
RUST_LOG=info
```

Adjust the values to align with your requirements. Below is a concise explanation of the configuration parameters:

- `PROXY_HOST`: The host and port on which the proxy server will listen.
- `ELECTRUMX_WSS`: ElectrumX server(s) to connect to. Use commas to separate multiple servers.
- `IP_LIMIT_PER_SECOND`: The interval in seconds to add new capacity.
- `IP_LIMIT_BURST_SIZE`: The burst size; if bursts exceed this value within the time set in `IP_LIMIT_PER_SECOND`, it will be restricted.
- `CONCURRENCY_LIMIT`: The maximum number of concurrent connections allowed.
- `RUST_LOG`: Log level for Rust's logging framework. Options include `trace`, `debug`, `info`, `warn`, and `error`.

#### Usage

Once the proxy server is operational, it adeptly forwards ElectrumX requests to the specified server(s). If multiple servers are configured, it seamlessly switches to the next server after a disconnection. Clients can effortlessly connect to the configured `PROXY_HOST`.

### License

This project operates under the MIT License - refer to the [LICENSE](https://github.com/atomicals/electrumx-proxy/blob/main/LICENSE) file for comprehensive details.

**Note:** It is advised to scrutinize and tailor the configuration according to your specific demands.