import { useQuery } from '@tanstack/react-query';
import { api } from '../api/client';
import StatusBadge from '../components/StatusBadge';

export default function Overview() {
  const { data, isLoading } = useQuery({
    queryKey: ['projectStatus'],
    queryFn: api.projectStatus,
  });

  if (isLoading) return <p>加载中...</p>;
  if (!data) return <p>无法加载项目状态</p>;

  return (
    <div className="space-y-6">
      <h2 className="text-xl font-semibold">项目状态</h2>
      {!data.initialized ? (
        <div className="rounded border border-yellow-300 bg-yellow-50 p-4 text-yellow-800">
          当前项目未初始化 Loom。在 agent 中运行 <code>/loom</code> 开始。
        </div>
      ) : (
        <>
          <div className="rounded-lg border bg-white p-4">
            <div className="flex items-center justify-between">
              <span className="text-sm text-gray-500">活跃交付</span>
              {data.activeDeliveryId && <StatusBadge status="executing" />}
            </div>
            {data.activeDeliveryId ? (
              <p className="mt-2 font-mono text-lg">{data.activeDeliveryId}</p>
            ) : (
              <p className="mt-2 text-gray-400">无活跃交付</p>
            )}
          </div>
          <div>
            <h3 className="mb-2 text-sm font-medium text-gray-700">所有交付</h3>
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b text-left text-gray-500">
                  <th className="py-2">Delivery</th>
                  <th>状态</th>
                  <th>更新时间</th>
                </tr>
              </thead>
              <tbody>
                {data.deliveries.map((d) => (
                  <tr key={d.deliveryId} className="border-b hover:bg-gray-50">
                    <td className="py-2 font-mono text-xs">
                      <a href={`/deliveries/${d.deliveryId}`} className="text-blue-600 hover:underline">
                        {d.deliveryId}
                      </a>
                    </td>
                    <td><StatusBadge status={d.status} /></td>
                    <td className="text-gray-500">{d.updatedAt}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  );
}
