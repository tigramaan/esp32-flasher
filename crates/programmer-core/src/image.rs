use crate::{ChipFamily, ErrorCode, OperationError, Result};

const ESP_IMAGE_MAGIC: u8 = 0xE9;
const ESP_IMAGE_HEADER_SIZE: usize = 24;

pub fn validate_esp_image(bytes: &[u8], allowed_chips: &[ChipFamily]) -> Result<ChipFamily> {
    if bytes.len() < ESP_IMAGE_HEADER_SIZE || bytes[0] != ESP_IMAGE_MAGIC {
        return Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "Application BIN не содержит корректный ESP image header",
        ));
    }
    let chip_id = u16::from_le_bytes([bytes[12], bytes[13]]);
    let chip = chip_from_image_id(chip_id).ok_or_else(|| {
        OperationError::new(
            ErrorCode::PackageUnsupported,
            "Application BIN предназначен для неизвестного ESP-чипа",
        )
        .with_detail(chip_id.to_string())
    })?;
    if !allowed_chips.contains(&chip) {
        return Err(OperationError::new(
            ErrorCode::ChipMismatch,
            "Чип application BIN отсутствует в target_chips",
        )
        .with_detail(chip.to_string()));
    }
    Ok(chip)
}

fn chip_from_image_id(value: u16) -> Option<ChipFamily> {
    match value {
        0 => Some(ChipFamily::Esp32),
        2 => Some(ChipFamily::Esp32s2),
        5 => Some(ChipFamily::Esp32c3),
        9 => Some(ChipFamily::Esp32s3),
        12 => Some(ChipFamily::Esp32c2),
        13 => Some(ChipFamily::Esp32c6),
        16 => Some(ChipFamily::Esp32h2),
        18 => Some(ChipFamily::Esp32p4),
        23 => Some(ChipFamily::Esp32c5),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::validate_esp_image;
    use crate::{ChipFamily, ErrorCode};

    #[test]
    fn validates_chip_id() {
        let mut image = vec![0; 24];
        image[0] = 0xE9;
        image[12..14].copy_from_slice(&9_u16.to_le_bytes());
        assert_eq!(
            validate_esp_image(&image, &[ChipFamily::Esp32s3]).unwrap(),
            ChipFamily::Esp32s3
        );
    }

    #[test]
    fn rejects_target_mismatch() {
        let mut image = vec![0; 24];
        image[0] = 0xE9;
        let error = validate_esp_image(&image, &[ChipFamily::Esp32s3]).unwrap_err();
        assert_eq!(error.code, ErrorCode::ChipMismatch);
    }
}
