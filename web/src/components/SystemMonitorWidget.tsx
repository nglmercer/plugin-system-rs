import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { fetchSystemStats } from '../lib/api';
import { SystemStats } from '../lib/types';

interface SystemMonitorProps {
  settings: Record<string, any>;
}

export function SystemMonitorWidget({ settings }: SystemMonitorProps) {
  const [stats, setStats] = useState<SystemStats | null>(null);
  const refreshInterval = settings.refreshInterval || 2000;

  useEffect(() => {
    loadStats();
    const interval = setInterval(loadStats, refreshInterval);
    return () => clearInterval(interval);
  }, []);

  async function loadStats() {
    try {
      const data = await fetchSystemStats();
      if (data) setStats(data);
    } catch (e) {
      console.error('Failed to load system stats:', e);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function formatUptime(seconds: number): string {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours}h ${mins}m`;
    return `${mins}m`;
  }

  function getCpuColor(usage: number): string {
    if (usage < 50) return '#4caf50';
    if (usage < 80) return '#ff9800';
    return '#f44336';
  }

  function getMemoryColor(usage: number): string {
    if (usage < 60) return '#2196f3';
    if (usage < 85) return '#ff9800';
    return '#f44336';
  }

  if (!stats) {
    return h('div', { class: 'widget-system-monitor loading' },
      h('div', { class: 'loading-text' }, 'Loading...')
    );
  }

  return h('div', { class: 'widget-system-monitor' },
    h('div', { class: 'stats-grid' },
      h('div', { class: 'stat-card' },
        h('div', { class: 'stat-label' }, 'CPU'),
        h('div', { class: 'stat-value', style: { color: getCpuColor(stats.cpu_usage) } },
          `${stats.cpu_usage.toFixed(1)}%`
        ),
        h('div', { class: 'stat-bar' },
          h('div', {
            class: 'stat-bar-fill cpu',
            style: { width: `${stats.cpu_usage}%`, background: getCpuColor(stats.cpu_usage) }
          })
        )
      ),
      h('div', { class: 'stat-card' },
        h('div', { class: 'stat-label' }, 'Memory'),
        h('div', { class: 'stat-value', style: { color: getMemoryColor(stats.memory_usage) } },
          `${stats.memory_usage.toFixed(1)}%`
        ),
        h('div', { class: 'stat-detail' },
          `${formatBytes(stats.memory_used)} / ${formatBytes(stats.memory_total)}`
        ),
        h('div', { class: 'stat-bar' },
          h('div', {
            class: 'stat-bar-fill memory',
            style: { width: `${stats.memory_usage}%`, background: getMemoryColor(stats.memory_usage) }
          })
        )
      ),
      h('div', { class: 'stat-card' },
        h('div', { class: 'stat-label' }, 'Load Avg'),
        h('div', { class: 'stat-value load' },
          `${stats.load_avg[0].toFixed(2)} / ${stats.load_avg[1].toFixed(2)} / ${stats.load_avg[2].toFixed(2)}`
        )
      ),
      h('div', { class: 'stat-card' },
        h('div', { class: 'stat-label' }, 'Uptime'),
        h('div', { class: 'stat-value' }, formatUptime(stats.uptime))
      ),
      h('div', { class: 'stat-card' },
        h('div', { class: 'stat-label' }, 'Processes'),
        h('div', { class: 'stat-value' }, `${stats.process_count}`)
      ),
      stats.swap_total > 0 && h('div', { class: 'stat-card' },
        h('div', { class: 'stat-label' }, 'Swap'),
        h('div', { class: 'stat-value' },
          `${formatBytes(stats.swap_used)} / ${formatBytes(stats.swap_total)}`
        )
      )
    )
  );
}
