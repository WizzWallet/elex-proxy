## Build

1. Create .cargo/config and add the instructions below:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "x86_64-unknown-linux-gnu-gcc"
```

2. Build the project:

```shell
sh build_linux.sh
```