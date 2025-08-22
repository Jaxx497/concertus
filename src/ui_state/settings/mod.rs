mod root_mgmt;

#[derive(Default, PartialEq, Clone)]
pub enum SettingsMode {
    #[default]
    ViewRoots,
    AddRoot,
    RemoveRoot,
}
