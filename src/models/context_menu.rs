use crate::models::panel::PanelKind;

#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub panel: PanelKind,
    pub target: String,
}
