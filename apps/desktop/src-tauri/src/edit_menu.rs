use serde::Deserialize;

pub const EDIT_UNDO: &str = "edit-undo";
pub const EDIT_REDO: &str = "edit-redo";
pub const EDIT_CUT: &str = "edit-cut";
pub const EDIT_COPY: &str = "edit-copy";
pub const EDIT_PASTE: &str = "edit-paste";
pub const EDIT_SELECT_ALL: &str = "edit-select-all";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EditMenuSurface {
    #[default]
    None,
    EditableText,
    ReadonlySelection,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditMenuContext {
    pub surface: EditMenuSurface,
    pub can_undo: bool,
    pub can_redo: bool,
    pub has_selection: bool,
    pub has_content: bool,
    pub can_paste: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditMenuEnabled {
    pub undo: bool,
    pub redo: bool,
    pub cut: bool,
    pub copy: bool,
    pub paste: bool,
    pub select_all: bool,
}

pub fn edit_menu_enabled(context: EditMenuContext) -> EditMenuEnabled {
    match context.surface {
        EditMenuSurface::None => EditMenuEnabled {
            undo: false,
            redo: false,
            cut: false,
            copy: false,
            paste: false,
            select_all: false,
        },
        EditMenuSurface::EditableText => EditMenuEnabled {
            undo: context.can_undo,
            redo: context.can_redo,
            cut: context.has_selection,
            copy: context.has_selection,
            paste: context.can_paste,
            select_all: context.has_content,
        },
        EditMenuSurface::ReadonlySelection => EditMenuEnabled {
            undo: false,
            redo: false,
            cut: false,
            copy: context.has_selection,
            paste: false,
            select_all: false,
        },
        EditMenuSurface::Terminal => EditMenuEnabled {
            undo: false,
            redo: false,
            cut: false,
            copy: context.has_selection,
            paste: context.can_paste,
            select_all: context.has_content,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(surface: EditMenuSurface) -> EditMenuContext {
        EditMenuContext {
            surface,
            can_undo: true,
            can_redo: true,
            has_selection: true,
            has_content: true,
            can_paste: true,
        }
    }

    #[test]
    fn none_or_ordinary_ui_disables_every_edit_item() {
        assert_eq!(
            edit_menu_enabled(ctx(EditMenuSurface::None)),
            EditMenuEnabled {
                undo: false,
                redo: false,
                cut: false,
                copy: false,
                paste: false,
                select_all: false,
            }
        );
    }

    #[test]
    fn editable_text_follows_history_selection_and_content() {
        let mut context = ctx(EditMenuSurface::EditableText);
        assert_eq!(
            edit_menu_enabled(context),
            EditMenuEnabled {
                undo: true,
                redo: true,
                cut: true,
                copy: true,
                paste: true,
                select_all: true,
            }
        );

        context.can_undo = false;
        context.can_redo = false;
        context.has_selection = false;
        context.has_content = false;
        assert_eq!(
            edit_menu_enabled(context),
            EditMenuEnabled {
                undo: false,
                redo: false,
                cut: false,
                copy: false,
                paste: true,
                select_all: false,
            }
        );
    }

    #[test]
    fn readonly_selection_only_enables_copy() {
        let mut context = ctx(EditMenuSurface::ReadonlySelection);
        assert_eq!(
            edit_menu_enabled(context),
            EditMenuEnabled {
                undo: false,
                redo: false,
                cut: false,
                copy: true,
                paste: false,
                select_all: false,
            }
        );

        context.has_selection = false;
        assert!(!edit_menu_enabled(context).copy);
    }

    #[test]
    fn terminal_disables_undo_redo_cut_and_keeps_copy_paste_select_all() {
        let mut context = ctx(EditMenuSurface::Terminal);
        assert_eq!(
            edit_menu_enabled(context),
            EditMenuEnabled {
                undo: false,
                redo: false,
                cut: false,
                copy: true,
                paste: true,
                select_all: true,
            }
        );

        context.has_selection = false;
        context.has_content = false;
        let enabled = edit_menu_enabled(context);
        assert!(!enabled.copy);
        assert!(enabled.paste);
        assert!(!enabled.select_all);
        assert!(!enabled.undo);
        assert!(!enabled.redo);
        assert!(!enabled.cut);
    }
}
