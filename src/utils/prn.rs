use uuid::Uuid;

/// A parsed PRN structure that provides easy access to components
#[derive(Debug, Clone, PartialEq)]
pub struct PRN {
    pub version: String,
    pub organization_id: String,
    pub resource_type: String,
    pub resource_id: String,
}

impl PRN {
    /// Parse a PRN string into its components
    /// Format: prn:1:organization_id:resource_type:resource_id
    pub fn parse(prn: &str) -> Result<Self, PRNError> {
        let parts: Vec<&str> = prn.split(':').collect();

        if parts.len() != 5 {
            return Err(PRNError::InvalidFormat(format!(
                "PRN must have 5 parts separated by colons, got {} parts",
                parts.len()
            )));
        }

        if parts[0] != "prn" {
            return Err(PRNError::InvalidPrefix(parts[0].to_string()));
        }

        if parts[1] != "1" {
            return Err(PRNError::UnsupportedVersion(parts[1].to_string()));
        }

        // Validate organization_id is a valid UUID
        if Uuid::try_parse(parts[2]).is_err() {
            return Err(PRNError::InvalidOrganizationId(parts[2].to_string()));
        }

        // Validate resource_id is a valid UUID
        if Uuid::try_parse(parts[4]).is_err() {
            return Err(PRNError::InvalidResourceId(parts[4].to_string()));
        }

        Ok(PRN {
            version: parts[1].to_string(),
            organization_id: parts[2].to_string(),
            resource_type: parts[3].to_string(),
            resource_id: parts[4].to_string(),
        })
    }

    /// Parse an organization PRN to extract just the organization ID
    /// Format: prn:1:organization_id
    pub fn parse_organization_id(prn: &str) -> Result<String, PRNError> {
        let parts: Vec<&str> = prn.split(':').collect();

        if parts.len() != 3 {
            return Err(PRNError::InvalidFormat(format!(
                "Organization PRN must have 3 parts separated by colons, got {} parts",
                parts.len()
            )));
        }

        if parts[0] != "prn" {
            return Err(PRNError::InvalidPrefix(parts[0].to_string()));
        }

        if parts[1] != "1" {
            return Err(PRNError::UnsupportedVersion(parts[1].to_string()));
        }

        // Validate organization_id is a valid UUID
        if Uuid::try_parse(parts[2]).is_err() {
            return Err(PRNError::InvalidOrganizationId(parts[2].to_string()));
        }

        Ok(parts[2].to_string())
    }
}

/// Builder for constructing PRNs
#[derive(Debug, Clone)]
pub struct PRNBuilder {
    organization_id: String,
}

impl PRNBuilder {
    /// Create a new PRN builder with the given organization ID
    pub fn new(organization_id: String) -> Self {
        Self { organization_id }
    }

    /// Create a PRN builder from an existing PRN (extracts organization_id)
    /// Supports both organization PRNs (3-part) and resource PRNs (5-part)
    pub fn from_prn(prn: &str) -> Result<Self, PRNError> {
        let parts: Vec<&str> = prn.split(':').collect();

        match parts.len() {
            3 => {
                // Organization PRN: prn:1:organization_id
                let org_id = PRN::parse_organization_id(prn)?;
                Ok(Self::new(org_id))
            }
            5 => {
                // Resource PRN: prn:1:organization_id:resource_type:resource_id
                let parsed = PRN::parse(prn)?;
                Ok(Self::new(parsed.organization_id))
            }
            _ => Err(PRNError::InvalidFormat(format!(
                "PRN must have 3 or 5 parts separated by colons, got {} parts",
                parts.len()
            ))),
        }
    }

    /// Build a binary PRN
    pub fn binary(&self, binary_id: &str) -> Result<String, PRNError> {
        self.build_prn("binary", binary_id)
    }

    /// Build an artifact PRN
    pub fn artifact(&self, artifact_id: &str) -> Result<String, PRNError> {
        self.build_prn("artifact", artifact_id)
    }

    /// Build an artifact version PRN
    pub fn artifact_version(&self, version_id: &str) -> Result<String, PRNError> {
        self.build_prn("artifact_version", version_id)
    }

    /// Build a generic PRN with the specified resource type and ID
    pub fn build_prn(&self, resource_type: &str, resource_id: &str) -> Result<String, PRNError> {
        // Validate organization_id is a valid UUID
        if Uuid::try_parse(&self.organization_id).is_err() {
            return Err(PRNError::InvalidOrganizationId(
                self.organization_id.clone(),
            ));
        }

        // Validate resource_id is a valid UUID
        if Uuid::try_parse(resource_id).is_err() {
            return Err(PRNError::InvalidResourceId(resource_id.to_string()));
        }

        Ok(format!(
            "prn:1:{}:{}:{}",
            self.organization_id, resource_type, resource_id
        ))
    }
}

/// Errors that can occur when parsing or building PRNs
#[derive(Debug, Clone, PartialEq)]
pub enum PRNError {
    InvalidFormat(String),
    InvalidPrefix(String),
    UnsupportedVersion(String),
    InvalidOrganizationId(String),
    InvalidResourceId(String),
}

impl std::fmt::Display for PRNError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PRNError::InvalidFormat(msg) => write!(f, "Invalid PRN format: {}", msg),
            PRNError::InvalidPrefix(prefix) => write!(f, "Invalid PRN prefix: {}", prefix),
            PRNError::UnsupportedVersion(version) => {
                write!(f, "Unsupported PRN version: {}", version)
            }
            PRNError::InvalidOrganizationId(id) => write!(f, "Invalid organization ID: {}", id),
            PRNError::InvalidResourceId(id) => write!(f, "Invalid resource ID: {}", id),
        }
    }
}

impl std::error::Error for PRNError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prn_parse_valid() {
        let prn_str = "prn:1:550e8400-e29b-41d4-a716-446655440000:binary:550e8400-e29b-41d4-a716-446655440001";
        let prn = PRN::parse(prn_str).unwrap();

        assert_eq!(prn.version, "1");
        assert_eq!(prn.organization_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(prn.resource_type, "binary");
        assert_eq!(prn.resource_id, "550e8400-e29b-41d4-a716-446655440001");
    }

    #[test]
    fn test_prn_parse_invalid_format() {
        let result = PRN::parse("invalid:prn");
        assert!(matches!(result, Err(PRNError::InvalidFormat(_))));
    }

    #[test]
    fn test_prn_parse_invalid_prefix() {
        let result = PRN::parse("invalid:1:org:binary:id");
        assert!(matches!(result, Err(PRNError::InvalidPrefix(_))));
    }

    #[test]
    fn test_prn_parse_invalid_version() {
        let result = PRN::parse("prn:2:org:binary:id");
        assert!(matches!(result, Err(PRNError::UnsupportedVersion(_))));
    }

    #[test]
    fn test_prn_parse_invalid_org_id() {
        let result = PRN::parse("prn:1:invalid-uuid:binary:550e8400-e29b-41d4-a716-446655440001");
        assert!(matches!(result, Err(PRNError::InvalidOrganizationId(_))));
    }

    #[test]
    fn test_prn_builder_from_prn() {
        let artifact_version_prn =
            "prn:1:550e8400-e29b-41d4-a716-446655440000:artifact_version:550e8400-e29b-41d4-a716-446655440002";
        let builder = PRNBuilder::from_prn(artifact_version_prn).unwrap();

        let binary_prn = builder
            .binary("550e8400-e29b-41d4-a716-446655440003")
            .unwrap();
        assert_eq!(
            binary_prn,
            "prn:1:550e8400-e29b-41d4-a716-446655440000:binary:550e8400-e29b-41d4-a716-446655440003"
        );
    }

    #[test]
    fn test_prn_builder_various_resources() {
        let org_id = "550e8400-e29b-41d4-a716-446655440000";
        let builder = PRNBuilder::new(org_id.to_string());

        assert_eq!(
            builder.binary("550e8400-e29b-41d4-a716-446655440001").unwrap(),
            "prn:1:550e8400-e29b-41d4-a716-446655440000:binary:550e8400-e29b-41d4-a716-446655440001"
        );

        assert_eq!(
            builder.artifact("550e8400-e29b-41d4-a716-446655440002").unwrap(),
            "prn:1:550e8400-e29b-41d4-a716-446655440000:artifact:550e8400-e29b-41d4-a716-446655440002"
        );

        assert_eq!(
            builder.bundle("550e8400-e29b-41d4-a716-446655440003").unwrap(),
            "prn:1:550e8400-e29b-41d4-a716-446655440000:bundle:550e8400-e29b-41d4-a716-446655440003"
        );

        assert_eq!(
            builder.artifact_version("550e8400-e29b-41d4-a716-446655440004").unwrap(),
            "prn:1:550e8400-e29b-41d4-a716-446655440000:artifact_version:550e8400-e29b-41d4-a716-446655440004"
        );
    }

    #[test]
    fn test_prn_builder_invalid_org_id() {
        let builder = PRNBuilder::new("invalid-uuid".to_string());
        let result = builder.binary("550e8400-e29b-41d4-a716-446655440001");
        assert!(matches!(result, Err(PRNError::InvalidOrganizationId(_))));
    }

    #[test]
    fn test_prn_builder_invalid_resource_id() {
        let org_id = "550e8400-e29b-41d4-a716-446655440000";
        let builder = PRNBuilder::new(org_id.to_string());
        let result = builder.binary("invalid-resource-id");
        assert!(matches!(result, Err(PRNError::InvalidResourceId(_))));
    }

    #[test]
    fn test_prn_builder_from_organization_prn() {
        let organization_prn = "prn:1:550e8400-e29b-41d4-a716-446655440000";
        let builder = PRNBuilder::from_prn(organization_prn).unwrap();

        let binary_prn = builder
            .binary("550e8400-e29b-41d4-a716-446655440001")
            .unwrap();
        assert_eq!(
            binary_prn,
            "prn:1:550e8400-e29b-41d4-a716-446655440000:binary:550e8400-e29b-41d4-a716-446655440001"
        );
    }

    #[test]
    fn test_parse_organization_id() {
        let org_prn = "prn:1:550e8400-e29b-41d4-a716-446655440000";
        let org_id = PRN::parse_organization_id(org_prn).unwrap();
        assert_eq!(org_id, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_parse_organization_id_invalid() {
        let result = PRN::parse_organization_id("prn:1:invalid-uuid");
        assert!(matches!(result, Err(PRNError::InvalidOrganizationId(_))));

        let result = PRN::parse_organization_id("invalid:format");
        assert!(matches!(result, Err(PRNError::InvalidFormat(_))));
    }
}
