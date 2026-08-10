const statusColors: Record<string, string> = {
  completed: 'bg-green-100 text-green-800',
  executing: 'bg-blue-100 text-blue-800',
  reviewing: 'bg-yellow-100 text-yellow-800',
  repairing: 'bg-orange-100 text-orange-800',
  planning: 'bg-purple-100 text-purple-800',
  blocked: 'bg-red-100 text-red-800',
};

export default function StatusBadge({ status }: { status: string }) {
  const colorClass = statusColors[status.toLowerCase()] || 'bg-gray-100 text-gray-800';
  return (
    <span className={`inline-block rounded px-2 py-0.5 text-xs font-medium ${colorClass}`}>
      {status}
    </span>
  );
}
