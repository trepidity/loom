use crate::action::{ActiveLayout, FocusTarget};

/// Manages which panel is currently focused.
pub struct FocusManager {
    current: FocusTarget,
    panels: Vec<FocusTarget>,
}

impl FocusManager {
    pub fn new() -> Self {
        let panels = vec![
            FocusTarget::TreePanel,
            FocusTarget::DetailPanel,
            FocusTarget::CommandPanel,
        ];
        Self {
            current: FocusTarget::TreePanel,
            panels,
        }
    }

    pub fn current(&self) -> FocusTarget {
        self.current
    }

    pub fn set(&mut self, target: FocusTarget) {
        self.current = target;
    }

    pub fn is_focused(&self, target: FocusTarget) -> bool {
        self.current == target
    }

    /// Move focus to the next panel.
    pub fn next(&mut self) {
        let idx = self
            .panels
            .iter()
            .position(|p| *p == self.current)
            .unwrap_or(0);
        self.current = self.panels[(idx + 1) % self.panels.len()];
    }

    /// Switch panel lists based on the active layout.
    pub fn set_layout(&mut self, layout: ActiveLayout) {
        self.panels = match layout {
            ActiveLayout::Browser => vec![
                FocusTarget::TreePanel,
                FocusTarget::DetailPanel,
                FocusTarget::CommandPanel,
            ],
            ActiveLayout::Connections => {
                vec![FocusTarget::ConnectionsTree, FocusTarget::ConnectionForm]
            }
        };
        if !self.panels.contains(&self.current) {
            self.current = self.panels[0];
        }
    }

    /// Move focus to the previous panel.
    pub fn prev(&mut self) {
        let idx = self
            .panels
            .iter()
            .position(|p| *p == self.current)
            .unwrap_or(0);
        self.current = self.panels[(idx + self.panels.len() - 1) % self.panels.len()];
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_focus() {
        let fm = FocusManager::new();
        assert_eq!(fm.current(), FocusTarget::TreePanel);
    }

    #[test]
    fn test_focus_next_cycles() {
        let mut fm = FocusManager::new();
        assert_eq!(fm.current(), FocusTarget::TreePanel);

        fm.next();
        assert_eq!(fm.current(), FocusTarget::DetailPanel);

        fm.next();
        assert_eq!(fm.current(), FocusTarget::CommandPanel);

        fm.next();
        assert_eq!(fm.current(), FocusTarget::TreePanel); // wraps
    }

    #[test]
    fn test_focus_prev_cycles() {
        let mut fm = FocusManager::new();
        fm.prev();
        assert_eq!(fm.current(), FocusTarget::CommandPanel); // wraps back

        fm.prev();
        assert_eq!(fm.current(), FocusTarget::DetailPanel);

        fm.prev();
        assert_eq!(fm.current(), FocusTarget::TreePanel);
    }

    #[test]
    fn test_focus_set() {
        let mut fm = FocusManager::new();
        fm.set(FocusTarget::CommandPanel);
        assert_eq!(fm.current(), FocusTarget::CommandPanel);
        assert!(fm.is_focused(FocusTarget::CommandPanel));
        assert!(!fm.is_focused(FocusTarget::TreePanel));
    }
}
