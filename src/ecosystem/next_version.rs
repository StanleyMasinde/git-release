use crate::ecosystem::types::ReleaseKind;

pub fn get_next_version(current_version: &str, kind: &ReleaseKind) -> String {
    let mut parts = current_version
        .split(".")
        .filter_map(|s| s.parse::<u32>().ok());
    let (mut major, mut minor, mut patch) = (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    );

    match kind {
        ReleaseKind::Major => {
            major += 1;
            minor = 0;
            patch = 0
        }
        ReleaseKind::Minor => {
            minor += 1;
            patch = 0
        }
        ReleaseKind::Patch => patch += 1,
    }

    format!("{major}.{minor}.{patch}")
}

#[cfg(test)]
mod test {
    use crate::ecosystem::{next_version::get_next_version, types::ReleaseKind};

    #[test]
    fn test_get_next_version() {
        let current_version = "2.4.0";
        let major_version = get_next_version(current_version, &ReleaseKind::Major);
        assert_eq!(major_version, "3.0.0");

        let minor_version = get_next_version(current_version, &ReleaseKind::Minor);
        assert_eq!(minor_version, "2.5.0");

        let patch_version = get_next_version(current_version, &ReleaseKind::Patch);
        assert_eq!(patch_version, "2.4.1");
    }
}
