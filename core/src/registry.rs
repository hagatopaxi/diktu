//! Embedded catalogue of downloadable models.

use serde::{Deserialize, Serialize};

const REGISTRY: &str = include_str!("../../data/models.toml");

#[derive(Debug, Deserialize)]
struct Registry {
    model: Vec<Model>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub langs: Vec<String>,
    pub engine: String,
    pub license: String,
    pub repo: String,
    pub revision: String,
    pub files: Vec<ModelFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModelFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

impl Model {
    pub fn total_size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}

/// All registered models, in registry order.
pub fn models() -> Vec<Model> {
    toml::from_str::<Registry>(REGISTRY)
        .expect("embedded registry is valid")
        .model
}

/// The first registered model for `lang`.
pub fn default_for(lang: &str) -> Option<Model> {
    models()
        .into_iter()
        .find(|m| m.langs.iter().any(|l| l == lang))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_pinned_and_consistent() {
        let models = models();
        assert!(!models.is_empty());
        let is_hex = |s: &str, len| s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit());
        for m in &models {
            assert!(
                is_hex(&m.revision, 40),
                "{}: revision must be a commit hash",
                m.id
            );
            assert_eq!(m.engine, "sherpa-onnx-transducer", "{}", m.id);
            assert!(!m.langs.is_empty() && !m.license.is_empty(), "{}", m.id);
            for f in &m.files {
                assert!(is_hex(&f.sha256, 64), "{}: {}", m.id, f.path);
                assert!(
                    !f.path.contains("..") && !f.path.starts_with('/'),
                    "{}",
                    f.path
                );
            }
            for role in ["encoder", "decoder", "joiner", "tokens"] {
                assert!(
                    m.files.iter().any(|f| f.path.starts_with(role)),
                    "{}: {role}",
                    m.id
                );
            }
        }
        let mut ids: Vec<_> = models.iter().map(|m| &m.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), models.len());
        assert!(default_for("fr").is_some());
    }
}
