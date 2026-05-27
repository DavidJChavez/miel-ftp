use suppaftp::AsyncNativeTlsConnector;
use suppaftp::async_native_tls::TlsConnector;

use crate::error::AppResult;

pub fn build_tls_connector(accept_invalid_certs: bool) -> AppResult<AsyncNativeTlsConnector> {
    let mut builder = TlsConnector::new();
    if accept_invalid_certs {
        builder = builder.danger_accept_invalid_certs(true);
    }
    Ok(AsyncNativeTlsConnector::from(builder))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_connector() {
        assert!(build_tls_connector(false).is_ok());
        assert!(build_tls_connector(true).is_ok());
    }
}
