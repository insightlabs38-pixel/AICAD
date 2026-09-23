//! [`ExternalAssetId`] — stable, content-provenance-derived identity for
//! an imported/external asset (`D26`'s "external-asset identity" domain,
//! `project/DECISION_LOG.md#DL-28`: "filesystem path does not define
//! imported-asset identity").
//!
//! There is deliberately no `from_path` constructor anywhere in this
//! module — the only way to build an `ExternalAssetId` is from
//! [`AssetProvenance`]'s own normalized `format` and `content` fields, so
//! a caller cannot accidentally mint identity from wherever a file
//! happened to live on disk. Full import provenance capture (checksums of
//! decoded geometry, source tool metadata, packaging) is `AICAD-153`;
//! this task only fixes the identity *shape* — deterministic,
//! content-derived, and structurally distinct from every other `D26`
//! domain.

use crate::hash::StableHasher;
use cad_diagnostics::json::Json;

/// The caller-supplied normalized description an [`ExternalAssetId`] is
/// derived from. `content` is whatever normalized byte representation the
/// eventual importer settles on (`AICAD-153`) — this task only requires
/// that it be a function of the asset's own data, not of where that data
/// was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetProvenance<'a> {
    pub format: &'a str,
    pub content: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExternalAssetId(u64);

impl ExternalAssetId {
    pub fn from_provenance(provenance: &AssetProvenance<'_>) -> ExternalAssetId {
        let mut hasher = StableHasher::new();
        hasher.write_bytes(provenance.format.as_bytes());
        hasher.write_u8(0);
        hasher.write_bytes(provenance.content);
        ExternalAssetId(hasher.finish())
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("external_asset")),
            ("digest".to_string(), Json::str(format!("{:016x}", self.0))),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_content_and_format_is_the_same_asset_identity() {
        let a = ExternalAssetId::from_provenance(&AssetProvenance {
            format: "step",
            content: b"geometry-bytes",
        });
        let b = ExternalAssetId::from_provenance(&AssetProvenance {
            format: "step",
            content: b"geometry-bytes",
        });
        assert_eq!(a, b);
    }

    #[test]
    fn different_content_is_a_distinct_asset_identity() {
        let a = ExternalAssetId::from_provenance(&AssetProvenance {
            format: "step",
            content: b"geometry-bytes-v1",
        });
        let b = ExternalAssetId::from_provenance(&AssetProvenance {
            format: "step",
            content: b"geometry-bytes-v2",
        });
        assert_ne!(a, b);
    }

    #[test]
    fn different_format_with_identical_content_is_a_distinct_asset_identity() {
        let a = ExternalAssetId::from_provenance(&AssetProvenance {
            format: "step",
            content: b"same-bytes",
        });
        let b = ExternalAssetId::from_provenance(&AssetProvenance {
            format: "stl",
            content: b"same-bytes",
        });
        assert_ne!(a, b);
    }

    #[test]
    fn two_assets_re_imported_from_different_paths_with_identical_content_collapse_to_one_identity()
    {
        // The whole point of content-derived identity: two callers who
        // happened to read the same bytes from two different filesystem
        // locations must still agree this is the same external asset.
        let from_path_a = AssetProvenance {
            format: "step",
            content: b"shared-bytes",
        };
        let from_path_b = AssetProvenance {
            format: "step",
            content: b"shared-bytes",
        };
        assert_eq!(
            ExternalAssetId::from_provenance(&from_path_a),
            ExternalAssetId::from_provenance(&from_path_b)
        );
    }

    #[test]
    fn serialization_is_deterministic() {
        let id = ExternalAssetId::from_provenance(&AssetProvenance {
            format: "step",
            content: b"geometry-bytes",
        });
        assert_eq!(
            id.to_json().to_canonical_string(),
            id.to_json().to_canonical_string()
        );
    }
}
