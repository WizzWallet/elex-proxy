### EleX Proxy ( [English](README.md) | 中文文档 )

![GitHub release (with filter)](https://img.shields.io/github/v/release/AstroxNetwork/elex-proxy)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**EleX Proxy** 代理是一个轻量级的 Rust 实现，用于代理与 [Atomicals ElectrumX](https://github.com/atomicals/atomicals-electrumx) 服务器的通信。该项目旨在为处理 ElectrumX 请求提供简单而高效的解决方案。

#### 安装

1. 打开 [GitHub Releases](https://github.com/AstroxNetwork/elex-proxy/releases) 页面下载最新版本。
2. 下载适用于您平台的压缩包。
3. 解压并且运行可执行文件。

#### 配置

在项目根目录创建一个名为 `.env` 的文件，内容如下：

```dotenv
# 代理服务器监听的主机和端口
PROXY_HOST=0.0.0.0:12321
# 默认 wss://electrumx.atomicals.xyz:50012，使用逗号分隔多个服务器
ELECTRUMX_WSS=wss://electrumx.atomicals.xyz:50012
# 默认 1, 每 xx 秒添加1个允许访问数
# IP_LIMIT_PER_SECOND=1
# 默认 10, 每 xx 毫秒添加1个允许访问数
IP_LIMIT_PER_MILLS=1
# 默认 10，如果这个值被用完，新的访问将会被限制。
IP_LIMIT_BURST_SIZE=10
# 默认 1, 同时运行的 ws 实例，可以提高吞吐量，按需设置
ELECTRUMX_WS_INSTANCE=5
# 默认 500，最大并发连接数
CONCURRENCY_LIMIT=500
# 默认 10，接收 WebSocket 消息的超时时间
RESPONSE_TIMEOUT=10
RUST_LOG=info
```

根据需要调整这些值。以下是对配置参数的简要解释：

- `PROXY_HOST`：代理服务器监听的主机和端口。
- `ELECTRUMX_WSS`：要连接的 ElectrumX 服务器。使用逗号分隔多个服务器。
- `IP_LIMIT_PER_SECOND`：xx秒添加1个允许访问数。
- `IP_LIMIT_PER_MILLS`：xx毫秒添加1个允许访问数。
- `IP_LIMIT_BURST_SIZE`：如果这个值被用完，新的访问将会被限制。
- `ELECTRUMX_WS_INSTANCE`：同时运行的 ws 实例，可以提高吞吐量，按需设置。
- `CONCURRENCY_LIMIT`：允许的最大并发连接数。
- `RESPONSE_TIMEOUT`：接收 WebSocket 消息的超时时间。
- `RUST_LOG`：Rust 日志框架的日志级别。选项包括 `trace`、`debug`、`info`、`warn` 和 `error`。

#### 使用

一旦代理服务器运行，它将转发 ElectrumX 请求到指定的服务器，如果配置了多个服务器，将在一个服务器断开连接之后，切换到下一个服务器。客户端可以连接到配置的 `PROXY_HOST`。

### 许可

本项目采用 MIT 许可证 - 有关详细信息，请参阅 [LICENSE](LICENSE) 文件。

**注意：** 建议审查并根据您的特定需求定制配置。