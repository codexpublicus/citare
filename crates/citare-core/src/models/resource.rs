use sha2::{Digest, Sha256};

use super::citation::Citation;

/// A resolved resource that one or more citations point to.
#[derive(Debug, Clone)]
pub struct Resource {
    /// The canonical (typically full) citation that identifies this resource.
    pub citation: Citation,
}

impl std::hash::Hash for Resource {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher as _;

        let mut inner = DefaultHasher::new();
        self.citation.hash(&mut inner);

        let mut hasher = Sha256::new();
        hasher.update(format!("citation:{}", inner.finish()));
        hasher.update("class:Resource");
        hasher.finalize().as_slice().hash(state);
    }
}

impl PartialEq for Resource {
    fn eq(&self, other: &Self) -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        self.hash(&mut h1);
        other.hash(&mut h2);
        h1.finish() == h2.finish()
    }
}

impl Eq for Resource {}
