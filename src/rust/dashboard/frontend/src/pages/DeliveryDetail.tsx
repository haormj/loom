import { useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';
import PhaseTimeline from '../components/PhaseTimeline';
import TaskTable from '../components/TaskTable';

export default function DeliveryDetail() {
  const { deliveryId } = useParams<{ deliveryId: string }>();
  const { data: delivery } = useQuery({
    queryKey: ['deliveries', deliveryId],
    queryFn: () => api.delivery(deliveryId!),
    enabled: !!deliveryId,
  });

  if (!delivery) return <p className="text-gray-400">交付不存在</p>;

  const activePhase = delivery.phases.find((p) => p.phaseId === delivery.activePhaseId);

  const { data: taskRun } = useQuery({
    queryKey: ['tasks', deliveryId, activePhase?.phaseId],
    queryFn: () => api.tasks(deliveryId!, activePhase!.phaseId),
    enabled: !!activePhase,
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-3">
        <h2 className="text-xl font-semibold font-mono">{delivery.deliveryId}</h2>
        <span className="rounded bg-blue-100 px-2 py-0.5 text-xs text-blue-800">{delivery.status}</span>
      </div>
      <div>
        <h3 className="mb-2 text-sm font-medium text-gray-700">阶段时间线</h3>
        <PhaseTimeline phases={delivery.phases} activePhaseId={delivery.activePhaseId} />
      </div>
      {activePhase && (
        <div>
          <h3 className="mb-2 text-sm font-medium text-gray-700">
            任务列表 ({activePhase.phaseId})
          </h3>
          <TaskTable run={taskRun as any} />
        </div>
      )}
    </div>
  );
}
