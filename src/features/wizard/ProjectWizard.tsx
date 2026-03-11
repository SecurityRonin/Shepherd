import { useState, useCallback } from 'react';
import { WizardStepper } from './WizardStepper';

type WizardPhase = 'north_star' | 'name_gen' | 'logo_gen' | 'superpowers_setup';

interface PhaseState {
  phase: WizardPhase;
  label: string;
  description: string;
  status: 'pending' | 'in_progress' | 'completed' | 'skipped';
  result?: string;
}

const INITIAL_PHASES: PhaseState[] = [
  {
    phase: 'north_star',
    label: 'North Star',
    description: 'Define your product strategy, target audience, and success metrics',
    status: 'pending',
  },
  {
    phase: 'name_gen',
    label: 'Brand Name',
    description: 'Brainstorm and validate product names with domain availability',
    status: 'pending',
  },
  {
    phase: 'logo_gen',
    label: 'Logo & Identity',
    description: 'Generate app icons and visual identity assets',
    status: 'pending',
  },
  {
    phase: 'superpowers_setup',
    label: 'Superpowers',
    description: 'Install Obra Superpowers for enhanced agent capabilities',
    status: 'pending',
  },
];

interface ProjectWizardProps {
  projectId: string;
  onNavigate: (route: string) => void;
  onComplete: () => void;
  onDismiss: () => void;
}

export function ProjectWizard({ projectId: _projectId, onNavigate, onComplete, onDismiss }: ProjectWizardProps) {
  const [phases, setPhases] = useState<PhaseState[]>(INITIAL_PHASES);
  const [currentIndex, setCurrentIndex] = useState(0);

  const currentPhase = phases[currentIndex];

  const updatePhase = useCallback((index: number, update: Partial<PhaseState>) => {
    setPhases(prev => prev.map((p, i) => i === index ? { ...p, ...update } : p));
  }, []);

  const handleStart = useCallback(() => {
    updatePhase(currentIndex, { status: 'in_progress' });
    const routes: Record<WizardPhase, string> = {
      north_star: '/tools/northstar',
      name_gen: '/tools/namegen',
      logo_gen: '/tools/logogen',
      superpowers_setup: '/settings/superpowers',
    };
    onNavigate(routes[currentPhase.phase]);
  }, [currentIndex, currentPhase, updatePhase, onNavigate]);

  const handleComplete = useCallback((_result?: string) => {
    updatePhase(currentIndex, { status: 'completed', result: _result });
    const nextIndex = currentIndex + 1;
    if (nextIndex < phases.length) {
      setCurrentIndex(nextIndex);
    } else {
      onComplete();
    }
  }, [currentIndex, phases.length, updatePhase, onComplete]);

  const handleSkip = useCallback(() => {
    updatePhase(currentIndex, { status: 'skipped' });
    const nextIndex = currentIndex + 1;
    if (nextIndex < phases.length) {
      setCurrentIndex(nextIndex);
    } else {
      onComplete();
    }
  }, [currentIndex, phases.length, updatePhase, onComplete]);

  const handleJump = useCallback((index: number) => {
    setCurrentIndex(index);
  }, []);

  const isComplete = phases.every(p => p.status === 'completed' || p.status === 'skipped');

  // Expose handleComplete for external use
  void handleComplete;

  return (
    <div className="max-w-2xl mx-auto py-8 px-4">
      <div className="mb-8">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-xl font-semibold text-gray-900">New Project Setup</h2>
            <p className="text-sm text-gray-500 mt-1">
              Optional guided journey — skip any step or dismiss entirely
            </p>
          </div>
          <button
            onClick={onDismiss}
            className="text-sm text-gray-400 hover:text-gray-600"
          >
            Dismiss wizard
          </button>
        </div>
      </div>

      <WizardStepper
        phases={phases}
        currentIndex={currentIndex}
        onJump={handleJump}
      />

      {!isComplete && currentPhase && (
        <div className="mt-8 bg-white rounded-xl border border-gray-200 p-6">
          <h3 className="text-lg font-medium text-gray-900">{currentPhase.label}</h3>
          <p className="text-sm text-gray-600 mt-2">{currentPhase.description}</p>

          <div className="flex items-center gap-3 mt-6">
            <button
              onClick={handleStart}
              className="px-4 py-2 rounded-lg bg-blue-600 text-white text-sm font-medium hover:bg-blue-700"
            >
              {currentPhase.status === 'in_progress' ? 'Continue' : 'Start'}
            </button>
            <button
              onClick={handleSkip}
              className="px-4 py-2 rounded-lg bg-gray-100 text-gray-600 text-sm font-medium hover:bg-gray-200"
            >
              Skip
            </button>
          </div>
        </div>
      )}

      {isComplete && (
        <div className="mt-8 bg-green-50 rounded-xl border border-green-200 p-6 text-center">
          <p className="text-green-800 font-medium">Project setup complete!</p>
          <p className="text-green-600 text-sm mt-1">
            You can always revisit these tools from the sidebar.
          </p>
        </div>
      )}
    </div>
  );
}
