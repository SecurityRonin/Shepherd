use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WizardPhase {
    NorthStar,
    NameGen,
    LogoGen,
    SuperpowersSetup,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PhaseStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseState {
    pub phase: WizardPhase,
    pub status: PhaseStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardState {
    pub project_id: String,
    pub current_phase: WizardPhase,
    pub phases: Vec<PhaseState>,
}

impl WizardState {
    /// Journey order: North Star -> Name/Domain -> Logo -> Superpowers
    pub fn new(project_id: &str) -> Self {
        Self {
            project_id: project_id.to_string(),
            current_phase: WizardPhase::NorthStar,
            phases: vec![
                PhaseState {
                    phase: WizardPhase::NorthStar,
                    status: PhaseStatus::Pending,
                    result: None,
                },
                PhaseState {
                    phase: WizardPhase::NameGen,
                    status: PhaseStatus::Pending,
                    result: None,
                },
                PhaseState {
                    phase: WizardPhase::LogoGen,
                    status: PhaseStatus::Pending,
                    result: None,
                },
                PhaseState {
                    phase: WizardPhase::SuperpowersSetup,
                    status: PhaseStatus::Pending,
                    result: None,
                },
            ],
        }
    }

    pub fn skip_current(&mut self) {
        if let Some(phase) = self
            .phases
            .iter_mut()
            .find(|p| p.phase == self.current_phase)
        {
            phase.status = PhaseStatus::Skipped;
        }
        self.advance();
    }

    pub fn complete_current(&mut self, result: String) {
        if let Some(phase) = self
            .phases
            .iter_mut()
            .find(|p| p.phase == self.current_phase)
        {
            phase.status = PhaseStatus::Completed;
            phase.result = Some(result);
        }
        self.advance();
    }

    pub fn jump_to(&mut self, target: WizardPhase) {
        self.current_phase = target;
    }

    pub fn is_complete(&self) -> bool {
        self.phases
            .iter()
            .all(|p| matches!(p.status, PhaseStatus::Completed | PhaseStatus::Skipped))
    }

    pub fn get_result(&self, phase: WizardPhase) -> Option<String> {
        self.phases
            .iter()
            .find(|p| p.phase == phase)
            .and_then(|p| p.result.clone())
    }

    fn advance(&mut self) {
        let order = [
            WizardPhase::NorthStar,
            WizardPhase::NameGen,
            WizardPhase::LogoGen,
            WizardPhase::SuperpowersSetup,
        ];
        let current_idx = order
            .iter()
            .position(|p| *p == self.current_phase)
            .unwrap_or(0);
        if current_idx + 1 < order.len() {
            self.current_phase = order[current_idx + 1].clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_wizard_starts_at_north_star() {
        let state = WizardState::new("my-project");
        assert_eq!(state.current_phase, WizardPhase::NorthStar);
        assert!(!state.is_complete());
    }

    #[test]
    fn test_skip_advances_phase() {
        let mut state = WizardState::new("my-project");
        state.skip_current();
        assert_eq!(state.current_phase, WizardPhase::NameGen);
        assert_eq!(state.phases[0].status, PhaseStatus::Skipped);
    }

    #[test]
    fn test_complete_advances_phase() {
        let mut state = WizardState::new("my-project");
        state.complete_current("north-star-done".into());
        assert_eq!(state.current_phase, WizardPhase::NameGen);
        assert_eq!(state.phases[0].status, PhaseStatus::Completed);
        assert_eq!(state.phases[0].result.as_deref(), Some("north-star-done"));
    }

    #[test]
    fn test_jump_to_phase() {
        let mut state = WizardState::new("my-project");
        state.jump_to(WizardPhase::LogoGen);
        assert_eq!(state.current_phase, WizardPhase::LogoGen);
    }

    #[test]
    fn test_all_complete_or_skipped_marks_done() {
        let mut state = WizardState::new("my-project");
        state.complete_current("north-star-done".into()); // north star -> name gen
        state.complete_current("acme-tools".into()); // name gen -> logo gen
        state.skip_current(); // logo -> superpowers
        state.skip_current(); // superpowers (last)
        assert!(state.is_complete());
    }

    #[test]
    fn test_results_flow_forward() {
        let mut state = WizardState::new("my-project");
        state.complete_current("pmf-analysis".into());
        assert_eq!(
            state.get_result(WizardPhase::NorthStar).as_deref(),
            Some("pmf-analysis")
        );
    }
}
