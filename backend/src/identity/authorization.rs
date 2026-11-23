//! Hybrid RBAC + ABAC authorization policy and immutable permission catalog.
//!
//! Resource attributes must be loaded by trusted backend repositories. Never build
//! an AuthorizationResource from request JSON, query parameters, or client claims.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationRole {
    BusinessOwner,
    Investor,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    ProfileRead,
    ProfileUpdate,
    BusinessProfileRead,
    BusinessProfileUpdate,
    OpportunityCreate,
    OpportunityRead,
    OpportunityUpdate,
    InvestorDiscoveryRead,
    ConnectionCreate,
    ConnectionRead,
    DocumentRead,
    DocumentShare,
    ModerationReportCreate,
    AdminDashboardRead,
    AdminRecoveryReview,
    AdminModerationManage,
    AdminAccountRestrict,
    AdminAuditRead,
    AdminSecurityInvestigate,
}

impl Permission {
    pub const fn code(self) -> &'static str {
        match self {
            Self::ProfileRead => "profile.read",
            Self::ProfileUpdate => "profile.update",
            Self::BusinessProfileRead => "business_profile.read",
            Self::BusinessProfileUpdate => "business_profile.update",
            Self::OpportunityCreate => "opportunity.create",
            Self::OpportunityRead => "opportunity.read",
            Self::OpportunityUpdate => "opportunity.update",
            Self::InvestorDiscoveryRead => "investor.discovery.read",
            Self::ConnectionCreate => "connection.create",
            Self::ConnectionRead => "connection.read",
            Self::DocumentRead => "document.read",
            Self::DocumentShare => "document.share",
            Self::ModerationReportCreate => "moderation.report.create",
            Self::AdminDashboardRead => "admin.dashboard.read",
            Self::AdminRecoveryReview => "admin.recovery.review",
            Self::AdminModerationManage => "admin.moderation.manage",
            Self::AdminAccountRestrict => "admin.account.restrict",
            Self::AdminAuditRead => "admin.audit.read",
            Self::AdminSecurityInvestigate => "admin.security.investigate",
        }
    }
    pub const fn expected_resource_type(self) -> Option<&'static str> {
        match self {
            Self::ProfileRead | Self::ProfileUpdate => Some("user_profile"),
            Self::BusinessProfileRead | Self::BusinessProfileUpdate => Some("business_profile"),
            Self::OpportunityRead | Self::OpportunityUpdate => Some("opportunity"),
            Self::ConnectionRead => Some("connection"),
            Self::DocumentRead | Self::DocumentShare => Some("document"),
            Self::AdminRecoveryReview => Some("recovery_case"),
            Self::AdminModerationManage => Some("moderation_case"),
            Self::AdminAccountRestrict => Some("user_account"),
            Self::AdminAuditRead => Some("audit_scope"),
            Self::AdminSecurityInvestigate => Some("security_case"),
            Self::OpportunityCreate
            | Self::InvestorDiscoveryRead
            | Self::ConnectionCreate
            | Self::ModerationReportCreate
            | Self::AdminDashboardRead => None,
        }
    }

    pub const fn requires_resource(self) {
        !matches!(
            self,
            Self::OpportunityCreate
                | Self::InvestorDiscoveryRead
                | Self::ConnectionCreate
                | Self::ModerationReportCreate
                | Self::AdminDashboardRead
        )
    }
    const fn allowed_for(self, role: AuthorizationRole) -> bool {
        use AuthorizationRole::{Admin, BusinessOwner, Investor};
        match self {
            Self::ProfileRead
            | Self::ProfileUpdate
            | Self::ConnectionCreate
            | Self::ConnectionRead
            | Self::DocumentRead
            | Self::DocumentShare
            | Self::ModerationReportCreate => matches!(role, BusinessOwner | Investor),
            Self::BusinessProfileRead => matches!(role, BusinessOwner | Investor),
            Self::BusinessProfileUpdate | Self::OpportunityCreate | Self::OpportunityUpdate => {
                matches!(role, BusinessOwner)
            },
            Self::OpportunityRead | Self::InvestorDiscoveryRead => {
                matches!(role, BusinessOwner | Investor)
            }
            Self::AdminDashboardRead => matches!(role, Admin),
            Self::AdminRecoveryReview
            | Self::AdminModerationManage
            | Self::AdminAccountRestrict
            | Self::AdminAuditRead
            | Self::AdminSecurityInvestigate => matches!(role, Admin),
        }
    }
    const fn is_admin_operation(self) -> bool {
        matches!(
            self,
            Self::AdminRecoveryReview
                | Self::AdminModerationManage
                | Self::AdminAccountRestrict
                | Self::AdminAuditRead
                | Self::AdminSecurityInvestigate
        )
    }
}

/// Immutable permission catalog. No role mutation or permanent account deletion grant exists.
pub const PERMISSION_CATALOG: &[Permission] = &[
    Permission::ProfileRead,
    Permission::ProfileUpdate,
    Permission::BusinessProfileRead,
    Permission::BusinessProfileUpdate,
    Permission::OpportunityCreate,
    Permission::OpportunityRead,
    Permission::OpportunityUpdate,
    Permission::InvestorDiscoveryRead,
    Permission::ConnectionCreate,
    Permission::ConnectionRead,
    Permission::DocumentRead,
    Permission::DocumentShare,
    Permission::ModerationReportCreate,
    Permission::AdminDashboardRead,
    Permission::AdminRecoveryReview,
    Permission::AdminModerationManage,
    Permission::AdminAccountRestrict,
    Permission::AdminAuditRead,
    Permission::AdminSecurityInvestigate,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataClassification {
    Public,
    Internal,
    Confidential,
    HighlyConfidential,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceVisibility {
    Public,
    Private,
    Unlisted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPurpose {
    SelfService,
    BusinessOperation,
    SupportCase,
    UserReport,
    SecurityInvestigation,
    RecoveryReview,
}
impl AccessPurpose {
    pub const fn code(self) -> &'static str {
        match self {
            Self::SelfService => "self_service",
            Self::BusinessOperation => "business_operation",
            Self::SupportCase => "support_case",
            Self::UserReport => "user_report",
            Self::SecurityInvestigation => "security_investigation",
            Self::RecoveryReview => "recovery_review",
        }
    }
    const fn permits_admin_operation(self, permission: Permission) -> bool {
        match permission {
            Permission::AdminRecoveryReview => matches!(self, Self::RecoveryReview),
            Permission::AdminModerationManage | Permission::AdminAccountRestrict => {
                matches!(self, Self::UserReport | Self::SupportCase)
            }
            Permission::AdminAuditRead | Permission::AdminSecurityInvestigate => {
                matches!(self, Self::SecurityInvestigation)
            }
            _ => false,
        }
    }
}

/// Construct only from trusted database-backed resource loaders, never from client fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationResource {
    pub resource_type: String,
    pub resource_id: String,
    pub owner_user_id: Option<String>,
    pub classification: DataClassification,
    pub visibility: ResourceVisibility,
    pub connected: bool,
    pub consent_granted: bool,
    /// True only when a backend repository confirmed the Admin's case/report scope.
    pub admin_scope_granted: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenialReason {
    RoleNotAllowed,
    ResourceContextRequired,
    ResourceTypeMismatch,
    ResourceInactive,
    NotOwnerOrPublic,
    ConnectionRequired,
    ConsentRequired,
    ClassificationRestricted,
    AdminPurposeRequired,
    AdminScopeRequired,
}
impl fmt::Display for DenialReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::RoleNotAllowed => "role_not_allowed",
            Self::ResourceContextRequired => "resource_context_required",
            Self::ResourceTypeMismatch => "resource_type_mismatch",
            Self::ResourceInactive => "resource_inactive",
            Self::NotOwnerOrPublic => "not_owner_or_public",
            Self::ConnectionRequired => "connection_required",
            Self::ConsentRequired => "consent_required",
            Self::ClassificationRestricted => "classification_restricted",
            Self::AdminPurposeRequired => "admin_purpose_required",
            Self::AdminScopeRequired => "admin_scope_required",
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDecision {
    Allow,
    Deny(DenialReason),
}
pub struct AuthorizationContext<'a> {
    pub actor_id: &'a str,
    pub role: AuthorizationRole,
    pub permission: Permission,
    pub purpose: AccessPurpose,
    pub resource: Option<&'a AuthorizationResource>,
}

/// Apply immutable RBAC first, then ABAC. Every missing/unknown condition is denied.
pub fn evaluate(context: &AuthorizationContext<'_>) -> AccessDecision {
    let permission = context.permission;
    if !permission.allowed_for(context.role) {
        return AccessDecision::Deny(DenialReason::RoleNotAllowed);
    }
    if permission.is_admin_operation() {
        if !context.purpose.permits_admin_operation(permission) {
            return AccessDecision::Deny(DenialReason::AdminPurposeRequired);
        }
        let Some(resource) = context.resource else {
            return AccessDecision::Deny(DenialReason::ResourceContextRequired);
        };
        if permission.expected_resource_type() != Some(resource.resource_type.as_str()) {
            return AccessDecision::Deny(DenialReason::ResourceTypeMismatch);
        }
        if !resource.admin_scope_granted {
            return AccessDecision::Deny(DenialReason::AdminScopeRequired);
        }
        if permission.expected_resource_type() != Some(resource.resource_type.as_str()) {
        return AccessDecision::Deny(DenialReason::ResourceTypeMismatch);
    }
    if !resource.active {
            return AccessDecision::Deny(DenialReason::ResourceInactive);
        }
        if resource.classification == DataClassification::HighlyConfidential {
            return AccessDecision::Deny(DenialReason::ClassificationRestricted);
        }
        return AccessDecision::Allow;
    }
    if !permission.requires_resource() {
        return AccessDecision::Allow;
    }
    let Some(resource) = context.resource else {
        return AccessDecision::Deny(DenialReason::ResourceContextRequired);
    };
    if !resource.active {
        return AccessDecision::Deny(DenialReason::ResourceInactive);
    }
    if resource.classification == DataClassification::HighlyConfidential {
        return AccessDecision::Deny(DenialReason::ClassificationRestricted);
    }
    let is_owner = resource.owner_user_id.as_deref() == Some(context.actor_id);
    if matches!(
        permission,
        Permission::ProfileUpdate
            | Permission::BusinessProfileUpdate
            | Permission::OpportunityUpdate
            | Permission::DocumentShare
    ) {
        return if is_owner {
            AccessDecision::Allow
        } else {
            AccessDecision::Deny(DenialReason::NotOwnerOrPublic)
        };
    }
    if resource.classification == DataClassification::Confidential && !is_owner {
        if !resource.connected {
            return AccessDecision::Deny(DenialReason::ConnectionRequired);
        }
        if !resource.consent_granted {
            return AccessDecision::Deny(DenialReason::ConsentRequired);
        }
    }
    if is_owner || resource.visibility == ResourceVisibility::Public {
        return AccessDecision::Allow;
    }
    if resource.visibility == ResourceVisibility::Private {
        if !resource.connected {
            return AccessDecision::Deny(DenialReason::ConnectionRequired);
        }
        if !resource.consent_granted {
            return AccessDecision::Deny(DenialReason::ConsentRequired);
        }
        return AccessDecision::Allow;
    }
    AccessDecision::Deny(DenialReason::NotOwnerOrPublic)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn resource(owner: &str, visibility: ResourceVisibility) -> AuthorizationResource {
        AuthorizationResource {
            resource_type: "document".into(),
            resource_id: "resource-1".into(),
            owner_user_id: Some(owner.into()),
            classification: DataClassification::Confidential,
            visibility,
            connected: false,
            consent_granted: false,
            admin_scope_granted: false,
            active: true,
        }
    }
    fn ctx<'a>(
        actor: &'a str,
        role: AuthorizationRole,
        permission: Permission,
        purpose: AccessPurpose,
        resource: Option<&'a AuthorizationResource>,
    ) -> AuthorizationContext<'a> {
        AuthorizationContext {
            actor_id: actor,
            role,
            permission,
            purpose,
            resource,
        }
    }
    #[test]
    fn permission_catalog_is_unique_and_excludes_privilege_escalation_capabilities() {
        let mut codes = PERMISSION_CATALOG
            .iter()
            .map(|p| p.code())
            .collect::<Vec<_>>();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), PERMISSION_CATALOG.len());
        assert!(!PERMISSION_CATALOG
            .iter()
            .any(|p| matches!(p.code(), "admin.role.change" | "account.permanent_delete")));
    }
    #[test]
    fn admin_dashboard_permission_is_limited_to_the_admin_role() {
        assert_eq!(
            evaluate(&ctx(
                "admin-1",
                AuthorizationRole::Admin,
                Permission::AdminDashboardRead,
                AccessPurpose::SelfService,
                None
            )),
            AccessDecision::Allow
        );
        assert_eq!(
            evaluate(&ctx(
                "investor-1",
                AuthorizationRole::Investor,
                Permission::AdminDashboardRead,
                AccessPurpose::SelfService,
                None
            )),
            AccessDecision::Deny(DenialReason::RoleNotAllowed)
        );
    }

    #[test]
    fn role_function_mismatch_is_denied_bfla() {
        assert_eq!(
            evaluate(&ctx(
                "investor-1",
                AuthorizationRole::Investor,
                Permission::OpportunityUpdate,
                AccessPurpose::BusinessOperation,
                None
            )),
            AccessDecision::Deny(DenialReason::RoleNotAllowed)
        );
        assert_eq!(
            evaluate(&ctx(
                "admin-1",
                AuthorizationRole::Admin,
                Permission::OpportunityUpdate,
                AccessPurpose::BusinessOperation,
                None
            )),
            AccessDecision::Deny(DenialReason::RoleNotAllowed)
        );
    }
    #[test]
    fn cross_owner_object_mutation_is_denied_bola_idor() {
        let item = resource("owner-1", ResourceVisibility::Private);
        assert_eq!(
            evaluate(&ctx(
                "owner-1",
                AuthorizationRole::BusinessOwner,
                Permission::DocumentShare,
                AccessPurpose::BusinessOperation,
                Some(&item)
            )),
            AccessDecision::Allow
        );
        assert_eq!(
            evaluate(&ctx(
                "attacker-2",
                AuthorizationRole::BusinessOwner,
                Permission::DocumentShare,
                AccessPurpose::BusinessOperation,
                Some(&item)
            )),
            AccessDecision::Deny(DenialReason::NotOwnerOrPublic)
        );
    }
    #[test]
    fn private_document_requires_connection_and_explicit_consent() {
        let mut item = resource("owner-1", ResourceVisibility::Private);
        item.connected = true;
        assert_eq!(
            evaluate(&ctx(
                "investor-2",
                AuthorizationRole::Investor,
                Permission::DocumentRead,
                AccessPurpose::SelfService,
                Some(&item)
            )),
            AccessDecision::Deny(DenialReason::ConsentRequired)
        );
        item.consent_granted = true;
        assert_eq!(
            evaluate(&ctx(
                "investor-2",
                AuthorizationRole::Investor,
                Permission::DocumentRead,
                AccessPurpose::SelfService,
                Some(&item)
            )),
            AccessDecision::Allow
        );
    }
    #[test]
    fn mismatched_resource_type_fails_closed() {
        let mut item = resource("owner-1", ResourceVisibility::Public);
        item.resource_type = "opportunity".into();
        assert_eq!(
            evaluate(&ctx(
                "owner-1",
                AuthorizationRole::BusinessOwner,
                Permission::DocumentRead,
                AccessPurpose::SelfService,
                Some(&item)
            )),
            AccessDecision::Deny(DenialReason::ResourceTypeMismatch)
        );
    }

    #[test]
    fn highly_confidential_data_is_never_exposed_to_application_roles() {
        let mut item = resource("owner-1", ResourceVisibility::Public);
        item.classification = DataClassification::HighlyConfidential;
        assert_eq!(
            evaluate(&ctx(
                "owner-1",
                AuthorizationRole::BusinessOwner,
                Permission::DocumentRead,
                AccessPurpose::SelfService,
                Some(&item)
            )),
            AccessDecision::Deny(DenialReason::ClassificationRestricted)
        );
    }
    #[test]
    fn admin_access_requires_matching_purpose_and_backend_confirmed_scope() {
        let mut item = resource("owner-1", ResourceVisibility::Private);
        item.admin_scope_granted = true;
        assert_eq!(
            evaluate(&ctx(
                "admin-1",
                AuthorizationRole::Admin,
                Permission::AdminRecoveryReview,
                AccessPurpose::SecurityInvestigation,
                Some(&item)
            )),
            AccessDecision::Deny(DenialReason::AdminPurposeRequired)
        );
        assert_eq!(
            evaluate(&ctx(
                "admin-1",
                AuthorizationRole::Admin,
                Permission::AdminRecoveryReview,
                AccessPurpose::RecoveryReview,
                Some(&item)
            )),
            AccessDecision::Allow
        );
        item.admin_scope_granted = false;
        assert_eq!(
            evaluate(&ctx(
                "admin-1",
                AuthorizationRole::Admin,
                Permission::AdminRecoveryReview,
                AccessPurpose::RecoveryReview,
                Some(&item)
            )),
            AccessDecision::Deny(DenialReason::AdminScopeRequired)
        );
    }
    #[test]
    fn missing_or_inactive_resource_context_fails_closed() {
        assert_eq!(
            evaluate(&ctx(
                "owner-1",
                AuthorizationRole::BusinessOwner,
                Permission::DocumentRead,
                AccessPurpose::SelfService,
                None
            )),
            AccessDecision::Deny(DenialReason::ResourceContextRequired)
        );
        let mut item = resource("owner-1", ResourceVisibility::Public);
        item.active = false;
        assert_eq!(
            evaluate(&ctx(
                "owner-1",
                AuthorizationRole::BusinessOwner,
                Permission::DocumentRead,
                AccessPurpose::SelfService,
                Some(&item)
            )),
            AccessDecision::Deny(DenialReason::ResourceInactive)
        );
    }
}
