//! NSID (Namespaced Identifier) implementation for DeGov
//!
//! NSIDs follow the AT Protocol Lexicon format: `{authority}/{entity}[#{fragment}]`
//!
//! # Examples
//!
//! ```
//! use degov_core::Nsid;
//!
//! // Parse a basic NSID
//! let nsid: Nsid = "de.berlin/business".parse().unwrap();
//! assert_eq!(nsid.authority(), "de.berlin");
//! assert_eq!(nsid.entity(), "business");
//! assert_eq!(nsid.fragment(), None);
//!
//! // Parse an NSID with a fragment
//! let nsid: Nsid = "de.berlin/business-registration#workflow".parse().unwrap();
//! assert_eq!(nsid.authority(), "de.berlin");
//! assert_eq!(nsid.entity(), "business-registration");
//! assert_eq!(nsid.fragment(), Some("workflow"));
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Error type for NSID parsing and validation
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum NsidError {
    #[error("Invalid NSID format: {0}")]
    InvalidFormat(String),
    
    #[error("Invalid authority: {0}")]
    InvalidAuthority(String),
    
    #[error("Invalid entity name: {0}")]
    InvalidEntity(String),
    
    #[error("Invalid fragment: {0}")]
    InvalidFragment(String),
    
    #[error("NSID too long: {0} characters (max 256)")]
    TooLong(usize),
}

/// A Namespaced Identifier (NSID) following AT Protocol Lexicon format
///
/// Format: `{authority}/{entity}[#{fragment}]`
///
/// - Authority: Reverse DNS notation (e.g., `de.berlin`, `de.bund`)
/// - Entity: Kebab-case identifier (e.g., `business-registration`)
/// - Fragment: Optional type specifier (e.g., `workflow`, `permissions`)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Nsid {
    /// The full NSID string
    full: String,
    /// Byte position where entity starts (after '/')
    entity_start: usize,
    /// Byte position where fragment starts (after '#'), or None
    fragment_start: Option<usize>,
}

impl Nsid {
    /// Maximum length for an NSID
    pub const MAX_LENGTH: usize = 256;
    
    /// Create a new NSID from parts
    ///
    /// # Examples
    ///
    /// ```
    /// use degov_core::Nsid;
    ///
    /// let nsid = Nsid::new("de.berlin", "business", None).unwrap();
    /// assert_eq!(nsid.to_string(), "de.berlin/business");
    ///
    /// let nsid = Nsid::new("de.berlin", "business-registration", Some("workflow")).unwrap();
    /// assert_eq!(nsid.to_string(), "de.berlin/business-registration#workflow");
    /// ```
    pub fn new(authority: &str, entity: &str, fragment: Option<&str>) -> Result<Self, NsidError> {
        // Validate authority
        Self::validate_authority(authority)?;
        
        // Validate entity
        Self::validate_entity(entity)?;
        
        // Validate fragment if present
        if let Some(f) = fragment {
            Self::validate_fragment(f)?;
        }
        
        // Build the full NSID
        let mut full = String::with_capacity(authority.len() + entity.len() + 10);
        full.push_str(authority);
        full.push('/');
        let entity_start = full.len();
        full.push_str(entity);
        
        let fragment_start = if let Some(f) = fragment {
            full.push('#');
            let start = full.len();
            full.push_str(f);
            Some(start)
        } else {
            None
        };
        
        if full.len() > Self::MAX_LENGTH {
            return Err(NsidError::TooLong(full.len()));
        }
        
        Ok(Self {
            full,
            entity_start,
            fragment_start,
        })
    }
    
    /// Parse an NSID from a string
    pub fn parse(s: &str) -> Result<Self, NsidError> {
        s.parse()
    }
    
    /// Get the authority part (e.g., `de.berlin`)
    pub fn authority(&self) -> &str {
        &self.full[..self.entity_start - 1]
    }
    
    /// Get the entity part (e.g., `business-registration`)
    pub fn entity(&self) -> &str {
        match self.fragment_start {
            Some(pos) => &self.full[self.entity_start..pos - 1],
            None => &self.full[self.entity_start..],
        }
    }
    
    /// Get the fragment part if present (e.g., `workflow`)
    pub fn fragment(&self) -> Option<&str> {
        self.fragment_start.map(|pos| &self.full[pos..])
    }
    
    /// Get the NSID without the fragment (e.g., `de.berlin/business-registration`)
    pub fn without_fragment(&self) -> &str {
        match self.fragment_start {
            Some(pos) => &self.full[..pos - 1],
            None => &self.full,
        }
    }
    
    /// Check if this NSID has a fragment
    pub fn has_fragment(&self) -> bool {
        self.fragment_start.is_some()
    }
    
    /// Check if this is a federal (de.bund) NSID
    pub fn is_federal(&self) -> bool {
        self.authority().starts_with("de.bund")
    }
    
    /// Check if this is a state-level NSID (e.g., de.berlin, de.bayern)
    pub fn is_state(&self) -> bool {
        let auth = self.authority();
        auth.starts_with("de.") && !auth.starts_with("de.bund")
    }
    
    /// Get the NSID as a string slice
    pub fn as_str(&self) -> &str {
        &self.full
    }
    
    /// Convert into the inner string
    pub fn into_string(self) -> String {
        self.full
    }
    
    /// Create a new NSID with a different fragment
    pub fn with_fragment(&self, fragment: &str) -> Result<Self, NsidError> {
        Self::new(self.authority(), self.entity(), Some(fragment))
    }
    
    /// Create a new NSID without any fragment
    pub fn strip_fragment(&self) -> Result<Self, NsidError> {
        if self.fragment_start.is_none() {
            Ok(self.clone())
        } else {
            Self::new(self.authority(), self.entity(), None)
        }
    }
    
    // Validation functions
    
    fn validate_authority(authority: &str) -> Result<(), NsidError> {
        if authority.is_empty() {
            return Err(NsidError::InvalidAuthority("authority cannot be empty".to_string()));
        }
        
        // Authority must be reverse DNS notation (e.g., de.berlin, com.example)
        let parts: Vec<&str> = authority.split('.').collect();
        if parts.len() < 2 {
            return Err(NsidError::InvalidAuthority(
                format!("authority must have at least 2 parts: {}", authority)
            ));
        }
        
        for part in parts {
            if part.is_empty() {
                return Err(NsidError::InvalidAuthority(
                    "authority cannot have empty parts".to_string()
                ));
            }
            
            // Each part must be lowercase alphanumeric or hyphen
            if !part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return Err(NsidError::InvalidAuthority(
                    format!("authority part must be lowercase alphanumeric: {}", part)
                ));
            }
            
            // Cannot start or end with hyphen
            if part.starts_with('-') || part.ends_with('-') {
                return Err(NsidError::InvalidAuthority(
                    format!("authority part cannot start/end with hyphen: {}", part)
                ));
            }
        }
        
        Ok(())
    }
    
    fn validate_entity(entity: &str) -> Result<(), NsidError> {
        if entity.is_empty() {
            return Err(NsidError::InvalidEntity("entity cannot be empty".to_string()));
        }
        
        // Entity must be kebab-case (lowercase alphanumeric and hyphens)
        if !entity.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(NsidError::InvalidEntity(
                format!("entity must be kebab-case: {}", entity)
            ));
        }
        
        // Cannot start or end with hyphen
        if entity.starts_with('-') || entity.ends_with('-') {
            return Err(NsidError::InvalidEntity(
                format!("entity cannot start/end with hyphen: {}", entity)
            ));
        }
        
        Ok(())
    }
    
    fn validate_fragment(fragment: &str) -> Result<(), NsidError> {
        if fragment.is_empty() {
            return Err(NsidError::InvalidFragment("fragment cannot be empty".to_string()));
        }
        
        // Fragment must be kebab-case
        if !fragment.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(NsidError::InvalidFragment(
                format!("fragment must be kebab-case: {}", fragment)
            ));
        }
        
        // Cannot start or end with hyphen
        if fragment.starts_with('-') || fragment.ends_with('-') {
            return Err(NsidError::InvalidFragment(
                format!("fragment cannot start/end with hyphen: {}", fragment)
            ));
        }
        
        Ok(())
    }
}

impl FromStr for Nsid {
    type Err = NsidError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > Self::MAX_LENGTH {
            return Err(NsidError::TooLong(s.len()));
        }
        
        // Split by '#' to get fragment
        let (base, fragment) = match s.split_once('#') {
            Some((b, f)) => (b, Some(f)),
            None => (s, None),
        };
        
        // Split base by '/' to get authority and entity
        let (authority, entity) = base.split_once('/')
            .ok_or_else(|| NsidError::InvalidFormat(
                format!("NSID must contain '/': {}", s)
            ))?;
        
        Self::new(authority, entity, fragment)
    }
}

impl fmt::Display for Nsid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full)
    }
}

impl AsRef<str> for Nsid {
    fn as_ref(&self) -> &str {
        &self.full
    }
}

// Serde support
impl Serialize for Nsid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.full)
    }
}

impl<'de> Deserialize<'de> for Nsid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_basic_nsid() {
        let nsid: Nsid = "de.berlin/business".parse().unwrap();
        assert_eq!(nsid.authority(), "de.berlin");
        assert_eq!(nsid.entity(), "business");
        assert_eq!(nsid.fragment(), None);
        assert!(!nsid.has_fragment());
    }
    
    #[test]
    fn test_parse_nsid_with_fragment() {
        let nsid: Nsid = "de.berlin/business-registration#workflow".parse().unwrap();
        assert_eq!(nsid.authority(), "de.berlin");
        assert_eq!(nsid.entity(), "business-registration");
        assert_eq!(nsid.fragment(), Some("workflow"));
        assert!(nsid.has_fragment());
    }
    
    #[test]
    fn test_federal_detection() {
        let nsid: Nsid = "de.bund/person".parse().unwrap();
        assert!(nsid.is_federal());
        assert!(!nsid.is_state());
        
        let nsid: Nsid = "de.berlin/business".parse().unwrap();
        assert!(!nsid.is_federal());
        assert!(nsid.is_state());
    }
    
    #[test]
    fn test_without_fragment() {
        let nsid: Nsid = "de.berlin/business#workflow".parse().unwrap();
        assert_eq!(nsid.without_fragment(), "de.berlin/business");
    }
    
    #[test]
    fn test_with_fragment() {
        let nsid: Nsid = "de.berlin/business".parse().unwrap();
        let with_frag = nsid.with_fragment("workflow").unwrap();
        assert_eq!(with_frag.to_string(), "de.berlin/business#workflow");
    }
    
    #[test]
    fn test_strip_fragment() {
        let nsid: Nsid = "de.berlin/business#workflow".parse().unwrap();
        let stripped = nsid.strip_fragment().unwrap();
        assert_eq!(stripped.to_string(), "de.berlin/business");
        assert!(!stripped.has_fragment());
    }
    
    #[test]
    fn test_invalid_format() {
        assert!("invalid".parse::<Nsid>().is_err());
        assert!("no-slash".parse::<Nsid>().is_err());
        assert!("too/many/slashes".parse::<Nsid>().is_err());
    }
    
    #[test]
    fn test_invalid_authority() {
        assert!("single/entity".parse::<Nsid>().is_err());
        assert!("Invalid.Authority/entity".parse::<Nsid>().is_err());
        assert!("has space.here/entity".parse::<Nsid>().is_err());
        assert!("-starts-hyphen.bad/entity".parse::<Nsid>().is_err());
    }
    
    #[test]
    fn test_invalid_entity() {
        assert!("de.berlin/".parse::<Nsid>().is_err());
        assert!("de.berlin/Invalid_Entity".parse::<Nsid>().is_err());
        assert!("de.berlin/-starts-hyphen".parse::<Nsid>().is_err());
        assert!("de.berlin/ends-hyphen-".parse::<Nsid>().is_err());
    }
    
    #[test]
    fn test_serde_json() {
        let nsid: Nsid = "de.berlin/business#workflow".parse().unwrap();
        let json = serde_json::to_string(&nsid).unwrap();
        assert_eq!(json, r#""de.berlin/business#workflow""#);
        
        let deserialized: Nsid = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, nsid);
    }
    
    #[test]
    fn test_display() {
        let nsid: Nsid = "de.berlin/business#workflow".parse().unwrap();
        assert_eq!(format!("{}", nsid), "de.berlin/business#workflow");
    }
    
    #[test]
    fn test_common_fragments() {
        let fragments = ["workflow", "permissions", "credential", "plugin", "test"];
        for frag in fragments {
            let nsid_str = format!("de.berlin/service#{}", frag);
            let nsid: Nsid = nsid_str.parse().unwrap();
            assert_eq!(nsid.fragment(), Some(frag));
        }
    }
}

