import { useEffect, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';

export function useSSE() {
  const queryClient = useQueryClient();
  const [connected, setConnected] = useState(false);
  const [statusText, setStatusText] = useState('连接中...');
  const reconnectTimer = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => {
    let es: EventSource | null = null;
    let retryDelay = 1000;

    function connect() {
      es = new EventSource('/api/events');

      es.onopen = () => {
        setConnected(true);
        setStatusText('实时');
        retryDelay = 1000;
      };

      const handlers: Record<string, () => void> = {
        status: () => queryClient.invalidateQueries({ queryKey: ['projectStatus'] }),
        delivery: () => {
          queryClient.invalidateQueries({ queryKey: ['deliveries'] });
        },
        tasks: () => queryClient.invalidateQueries({ queryKey: ['tasks'] }),
        reviews: () => queryClient.invalidateQueries({ queryKey: ['reviews'] }),
        deploy: () => queryClient.invalidateQueries({ queryKey: ['deployStatus'] }),
        knowledge: () => queryClient.invalidateQueries({ queryKey: ['knowledgeSources'] }),
      };

      Object.entries(handlers).forEach(([event, handler]) => {
        es?.addEventListener(event, handler);
      });

      es.onerror = () => {
        setConnected(false);
        setStatusText(`连接中断 (${retryDelay / 1000}s 后重连)`);
        es?.close();
        reconnectTimer.current = setTimeout(() => {
          retryDelay = Math.min(retryDelay * 2, 30000);
          connect();
        }, retryDelay);
      };
    }

    connect();

    return () => {
      es?.close();
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
    };
  }, [queryClient]);

  return { connected, statusText };
}
