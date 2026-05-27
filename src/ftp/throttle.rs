use std::time::Duration;

/// Espera el tiempo proporcional a `bytes` según el límite en KB/s.
pub async fn sleep_for_bytes(bytes: u64, limit_kbps: Option<u32>) {
    let Some(kbps) = limit_kbps.filter(|k| *k > 0) else {
        return;
    };

    let limit_bps = u64::from(kbps) * 1024;
    let nanos = bytes.saturating_mul(1_000_000_000) / limit_bps;
    if nanos > 0 {
        tokio::time::sleep(Duration::from_nanos(nanos)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn unlimited_is_noop() {
        let start = Instant::now();
        sleep_for_bytes(1_048_576, None).await;
        assert!(start.elapsed() < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn zero_limit_is_noop() {
        let start = Instant::now();
        sleep_for_bytes(1_048_576, Some(0)).await;
        assert!(start.elapsed() < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn throttles_large_chunk() {
        let start = Instant::now();
        // 64 KiB at 64 KB/s ≈ 1 s
        sleep_for_bytes(65_536, Some(64)).await;
        assert!(start.elapsed() >= Duration::from_millis(900));
    }
}
