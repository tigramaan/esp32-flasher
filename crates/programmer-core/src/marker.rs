use crate::error::{ErrorCode, OperationError, Result};

#[derive(Debug, Clone)]
pub struct MarkerDetector {
    marker: Vec<u8>,
    tail: Vec<u8>,
    matched: bool,
}

impl MarkerDetector {
    pub fn new(marker: impl AsRef<[u8]>) -> Result<Self> {
        let marker = marker.as_ref();
        if marker.is_empty() || marker.len() > 256 {
            return Err(OperationError::new(
                ErrorCode::PackageInvalid,
                "UART-маркер должен занимать от 1 до 256 байт",
            ));
        }
        Ok(Self {
            marker: marker.to_vec(),
            tail: Vec::with_capacity(marker.len().saturating_sub(1)),
            matched: false,
        })
    }

    pub fn feed(&mut self, chunk: &[u8]) -> bool {
        if self.matched {
            return true;
        }
        let mut window = Vec::with_capacity(self.tail.len() + chunk.len());
        window.extend_from_slice(&self.tail);
        window.extend_from_slice(chunk);

        self.matched = window
            .windows(self.marker.len())
            .any(|candidate| candidate == self.marker);

        let keep = self.marker.len().saturating_sub(1).min(window.len());
        self.tail.clear();
        self.tail
            .extend_from_slice(&window[window.len().saturating_sub(keep)..]);
        self.matched
    }

    pub const fn matched(&self) -> bool {
        self.matched
    }
}
