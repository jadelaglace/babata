#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyClass {
    Private,
    Restricted,
    Shareable,
}

#[derive(Debug, Clone)]
pub struct PrivacyPolicy {
    pub default_class: PrivacyClass,
    pub require_confirmation_for_external_processing: bool,
}
