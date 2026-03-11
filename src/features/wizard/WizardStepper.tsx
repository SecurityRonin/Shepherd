interface PhaseState {
  phase: string;
  label: string;
  status: 'pending' | 'in_progress' | 'completed' | 'skipped';
}

interface WizardStepperProps {
  phases: PhaseState[];
  currentIndex: number;
  onJump: (index: number) => void;
}

export function WizardStepper({ phases, currentIndex, onJump }: WizardStepperProps) {
  return (
    <div className="flex items-center justify-between">
      {phases.map((phase, index) => (
        <div key={phase.phase} className="flex items-center flex-1">
          <button
            onClick={() => onJump(index)}
            className={`
              flex items-center justify-center w-8 h-8 rounded-full text-xs font-bold
              transition-colors cursor-pointer
              ${phase.status === 'completed'
                ? 'bg-green-500 text-white'
                : phase.status === 'skipped'
                ? 'bg-gray-300 text-gray-500'
                : index === currentIndex
                ? 'bg-blue-600 text-white'
                : 'bg-gray-200 text-gray-500'
              }
            `}
            title={phase.label}
          >
            {phase.status === 'completed' ? '\u2713' : phase.status === 'skipped' ? '\u2014' : index + 1}
          </button>
          <span className={`ml-2 text-xs font-medium ${
            index === currentIndex ? 'text-gray-900' : 'text-gray-500'
          }`}>
            {phase.label}
          </span>
          {index < phases.length - 1 && (
            <div className={`flex-1 h-px mx-3 ${
              phase.status === 'completed' ? 'bg-green-400' : 'bg-gray-200'
            }`} />
          )}
        </div>
      ))}
    </div>
  );
}
