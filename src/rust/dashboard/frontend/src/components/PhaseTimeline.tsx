interface Phase {
  phaseId: string;
  status: string;
}

export default function PhaseTimeline({ phases, activePhaseId }: { phases: Phase[]; activePhaseId: string }) {
  return (
    <div className="flex items-center gap-1 overflow-x-auto py-4">
      {phases.map((phase, idx) => {
        const isActive = phase.phaseId === activePhaseId;
        const isComplete = phase.status === 'completed';
        const dotColor = isComplete ? 'bg-green-500' : isActive ? 'bg-blue-500' : 'bg-gray-300';
        return (
          <div key={phase.phaseId} className="flex items-center">
            <div className="flex flex-col items-center">
              <div className={`h-3 w-3 rounded-full ${dotColor}`} />
              <span className="mt-1 text-xs text-gray-600">{phase.phaseId}</span>
            </div>
            {idx < phases.length - 1 && (
              <div className={`h-0.5 w-8 ${isComplete ? 'bg-green-400' : 'bg-gray-200'}`} />
            )}
          </div>
        );
      })}
    </div>
  );
}
