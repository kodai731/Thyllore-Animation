#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiPointerOwnerId {
    Timeline,
    CurveEditor,
    PanelSplitter,
}

/// Which raw-io UI handler is dragging the pointer; held from the press that started the drag until every
/// mouse button is up, so no other handler starts a drag underneath it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiPointerOwner {
    owner: Option<UiPointerOwnerId>,
}

impl UiPointerOwner {
    pub fn owner(&self) -> Option<UiPointerOwnerId> {
        self.owner
    }

    pub fn is_held_by(&self, id: UiPointerOwnerId) -> bool {
        self.owner == Some(id)
    }

    pub fn is_free_for(&self, id: UiPointerOwnerId) -> bool {
        self.owner.map_or(true, |owner| owner == id)
    }

    pub fn try_claim(&mut self, id: UiPointerOwnerId) -> bool {
        if !self.is_free_for(id) {
            return false;
        }
        self.owner = Some(id);
        true
    }

    pub fn release_when_buttons_up(&mut self, any_button_down: bool) {
        if !any_button_down {
            self.owner = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_succeeds_only_while_unowned_or_already_held() {
        let mut pointer = UiPointerOwner::default();

        assert!(pointer.try_claim(UiPointerOwnerId::Timeline));
        assert!(pointer.try_claim(UiPointerOwnerId::Timeline));
        assert!(!pointer.try_claim(UiPointerOwnerId::CurveEditor));
        assert_eq!(pointer.owner(), Some(UiPointerOwnerId::Timeline));
    }

    #[test]
    fn ownership_survives_while_a_button_is_down_and_ends_when_all_are_up() {
        let mut pointer = UiPointerOwner::default();
        pointer.try_claim(UiPointerOwnerId::PanelSplitter);

        pointer.release_when_buttons_up(true);
        assert!(pointer.is_held_by(UiPointerOwnerId::PanelSplitter));

        pointer.release_when_buttons_up(false);
        assert_eq!(pointer.owner(), None);
        pointer.release_when_buttons_up(false);
        assert_eq!(pointer.owner(), None);
        assert!(pointer.try_claim(UiPointerOwnerId::CurveEditor));
    }
}
