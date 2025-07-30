use crate::security::error::SecurityError;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::path::Path;

pub struct TlsStore;

impl TlsStore {
    pub async fn try_load<'a>(
        certificate_path: &Path,
        key_path: &Path,
    ) -> Result<(CertificateDer<'a>, PrivateKeyDer<'a>), SecurityError> {
        if !certificate_path.exists() {
            return Err(SecurityError::CertificateNotFound(
                certificate_path.to_string_lossy().to_string(),
            ));
        }

        if !key_path.exists() {
            return Err(SecurityError::KeyNotFound(
                key_path.to_string_lossy().to_string(),
            ));
        }

        let certificate_data = tokio::fs::read(certificate_path).await?;
        let key_data = tokio::fs::read(key_path).await?;

        let certificate = CertificateDer::from(certificate_data);
        let key = match PrivateKeyDer::try_from(key_data) {
            Ok(key) => key,
            Err(error) => {
                return Err(SecurityError::InvalidKey(error.to_string()));
            }
        };

        Ok((certificate, key))
    }
}
