use std::collections::HashSet;
use std::env;
use std::sync::LazyLock;

pub static IP_LIMIT_PER_MILLS: LazyLock<u64> = LazyLock::new(|| {
    let per_second: u64 = env::var("IP_LIMIT_PER_SECOND")
        .unwrap_or("0".to_string())
        .parse()
        .unwrap();
    if per_second > 0 {
        per_second * 1000
    } else {
        env::var("IP_LIMIT_PER_MILLS")
            .unwrap_or("10".to_string())
            .parse()
            .unwrap()
    }
});

pub static IP_LIMIT_BURST_SIZE: LazyLock<u32> = LazyLock::new(|| {
    env::var("IP_LIMIT_BURST_SIZE")
        .unwrap_or("10".to_string())
        .parse()
        .unwrap()
});

pub static CONCURRENCY_LIMIT: LazyLock<usize> = LazyLock::new(|| {
    env::var("CONCURRENCY_LIMIT")
        .unwrap_or("500".to_string())
        .parse()
        .unwrap()
});

pub static ELECTRUMX_WSS: LazyLock<String> = LazyLock::new(|| {
    env::var("ELECTRUMX_WSS").unwrap_or("wss://electrumx.atomicals.xyz:50012".to_string())
});

pub static ELECTRUMX_WS_INSTANCE: LazyLock<u32> = LazyLock::new(|| {
    env::var("ELECTRUMX_WS_INSTANCE")
        .unwrap_or("1".to_string())
        .parse()
        .unwrap()
});

pub static PROXY_HOST: LazyLock<String> =
    LazyLock::new(|| env::var("PROXY_HOST").unwrap_or("0.0.0.0:12321".into()));

pub static RESPONSE_TIMEOUT: LazyLock<u64> = LazyLock::new(|| {
    env::var("RESPONSE_TIMEOUT")
        .unwrap_or("10".to_string())
        .parse()
        .unwrap()
});

pub static MAX_CACHE_ENTRIES: LazyLock<u64> = LazyLock::new(|| {
    env::var("MAX_CACHE_ENTRIES")
        .unwrap_or("10000".to_string())
        .parse()
        .unwrap()
});

pub static CACHE_TIME_TO_LIVE: LazyLock<u64> = LazyLock::new(|| {
    env::var("CACHE_TIME_TO_LIVE")
        .unwrap_or("600".to_string())
        .parse()
        .unwrap()
});

pub static CACHE_TIME_TO_IDLE: LazyLock<u64> = LazyLock::new(|| {
    env::var("CACHE_TIME_TO_IDLE")
        .unwrap_or("180".to_string())
        .parse()
        .unwrap()
});

pub static NO_CACHE_METHODS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    env::var("NO_CACHE_METHODS")
        .unwrap_or("blockchain.atomicals.get_global,blockchain.estimatefee,blockchain.scripthash.subscribe,blockchain.transaction.broadcast,server.peers.subscribe,server.ping,mempool.get_fee_histogram,blockchain.atomicals.dump,blockchain.scripthash.unsubscribe,blockchain.relayfee".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .collect()
});
