pub(crate) fn python_version_from_magic(bytes: &[u8; 4]) -> Option<(u16, u16)> {
    let number = u16::from_le_bytes([bytes[0], bytes[1]]);
    python_version_from_magic_number(number)
}

// based on CPython: Lib/importlib/_bootstrap_external.py
fn python_version_from_magic_number(number: u16) -> Option<(u16, u16)> {
    match number {
        3360..=3361 => Some((3, 6)),
        3370..=3379 => Some((3, 6)),
        3390..=3394 => Some((3, 7)),
        3400..=3401 => Some((3, 8)),
        3410..=3413 => Some((3, 8)),
        3420..=3425 => Some((3, 9)),
        3430..=3439 => Some((3, 10)),
        3450..=3495 => Some((3, 11)),
        3500..=3531 => Some((3, 12)),
        // preliminary ranges for pre-releases
        3550..=3599 => Some((3, 13)),
        3600..=3649 => Some((3, 14)),
        _ => None,
    }
}

pub(crate) fn pyc_header_length(version: (u16, u16)) -> usize {
    if version >= (3, 7) {
        16
    } else if version >= (3, 3) {
        12
    } else {
        8
    }
}
