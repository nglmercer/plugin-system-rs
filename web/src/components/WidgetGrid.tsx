import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { WidgetConfig, WidgetType, DashboardLayout, SystemStats } from '../lib/types';
import { fetchDashboard, saveDashboard, executeAction } from '../lib/api';

const WIDGET_CATALOG: { type: WidgetType; label: string; icon: string; description: string; defaultColSpan: number; defaultRowSpan: number }[] = [
  { type: 'system-monitor', label: 'System Monitor', icon: '%', description: 'CPU, Memory, Load, Uptime', defaultColSpan: 1, defaultRowSpan: 1 },
  { type: 'clock', label: 'Clock', icon: 'T', description: 'Current time & date', defaultColSpan: 1, defaultRowSpan: 1 },
  { type: 'quick-actions', label: 'Quick Actions', icon: '*', description: 'All actions list', defaultColSpan: 1, defaultRowSpan: 1 },
  { type: 'send-hotkey', label: 'Send Hotkey', icon: 'H', description: 'Trigger keyboard hotkey', defaultColSpan: 1, defaultRowSpan: 1 },
  { type: 'open-url', label: 'Open URL', icon: 'U', description: 'Open a URL in browser', defaultColSpan: 1, defaultRowSpan: 1 },
  { type: 'type-text', label: 'Type Text', icon: 'A', description: 'Type text string', defaultColSpan: 1, defaultRowSpan: 1 },
];

function generateId(): string {
  return Math.random().toString(36).substring(2, 10);
}

function getDefaultSettings(type: WidgetType): Record<string, any> {
  switch (type) {
    case 'system-monitor': return { refreshInterval: 2000 };
    case 'clock': return { format: 'digital' };
    case 'quick-actions': return {};
    case 'send-hotkey': return { keys: 'ctrl+c', label: 'Copy' };
    case 'open-url': return { url: 'https://example.com', label: 'Example' };
    case 'type-text': return { text: 'Hello!', label: 'Greeting' };
    default: return {};
  }
}

export function WidgetGrid() {
  const [layout, setLayout] = useState<DashboardLayout>({ widgets: [], columns: 3 });
  const [loading, setLoading] = useState(true);
  const [showLibrary, setShowLibrary] = useState(false);
  const [editingWidget, setEditingWidget] = useState<string | null>(null);

  useEffect(() => {
    fetchDashboard().then(data => {
      setLayout(data);
      setLoading(false);
    }).catch(() => setLoading(false));
  }, []);

  function persist(next: DashboardLayout) {
    setLayout(next);
    saveDashboard(next);
  }

  function handleAddWidget(type: WidgetType) {
    const catalog = WIDGET_CATALOG.find(w => w.type === type);
    if (!catalog) return;
    const settings = getDefaultSettings(type);
    const widget: WidgetConfig = {
      id: generateId(),
      type,
      title: settings.label || catalog.label,
      colSpan: catalog.defaultColSpan,
      rowSpan: catalog.defaultRowSpan,
      settings,
    };
    persist({ ...layout, widgets: [...layout.widgets, widget] });
    setShowLibrary(false);
  }

  function handleRemoveWidget(id: string) {
    persist({ ...layout, widgets: layout.widgets.filter(w => w.id !== id) });
    setEditingWidget(null);
  }

  function handleUpdateWidget(id: string, updates: Partial<WidgetConfig>) {
    persist({
      ...layout,
      widgets: layout.widgets.map(w => w.id === id ? { ...w, ...updates } : w),
    });
  }

  function handleUpdateSetting(id: string, key: string, value: string) {
    persist({
      ...layout,
      widgets: layout.widgets.map(w => w.id === id ? { ...w, settings: { ...w.settings, [key]: value } } : w),
    });
  }

  if (loading) {
    return h('div', { class: 'dashboard-loading' }, 'Loading dashboard...');
  }

  return h('div', { class: 'dashboard-root' },
    h('div', { class: 'dashboard-header' },
      h('h2', null, 'Dashboard'),
      h('button', { class: 'add-widget-btn', onClick: () => setShowLibrary(true) }, '+ Add Widget')
    ),

    layout.widgets.length === 0
      ? h('div', { class: 'dashboard-empty' },
          h('div', { class: 'empty-icon' }, '+'),
          h('div', { class: 'empty-text' }, 'No widgets added yet'),
          h('div', { class: 'empty-sub' }, 'Click "Add Widget" to get started'),
        )
      : h('div', {
          class: 'dashboard-grid',
          style: { gridTemplateColumns: `repeat(${layout.columns}, 1fr)` },
        },
          layout.widgets.map(widget =>
            h('div', {
              key: widget.id,
              class: `dashboard-widget ${editingWidget === widget.id ? 'editing' : ''}`,
              style: {
                gridColumn: `span ${widget.colSpan}`,
                gridRow: `span ${widget.rowSpan}`,
              },
            },
              h('div', { class: 'widget-header' },
                h('span', { class: 'widget-title' }, widget.title),
                h('div', { class: 'widget-controls' },
                  h('button', {
                    class: 'widget-control-btn edit',
                    onClick: () => setEditingWidget(editingWidget === widget.id ? null : widget.id),
                  }, 'E'),
                  h('button', {
                    class: 'widget-control-btn remove',
                    onClick: () => handleRemoveWidget(widget.id),
                  }, 'X'),
                ),
              ),
              h('div', { class: 'widget-content' },
                h(WidgetContent, { widget, onExecute: () => {} })
              ),
              editingWidget === widget.id && h(WidgetEditor, {
                widget,
                layoutColumns: layout.columns,
                onUpdateTitle: (t) => handleUpdateWidget(widget.id, { title: t }),
                onUpdateColSpan: (c) => handleUpdateWidget(widget.id, { colSpan: c }),
                onUpdateSetting: (k, v) => handleUpdateSetting(widget.id, k, v),
                onRemove: () => handleRemoveWidget(widget.id),
              })
            )
          )
        ),

    showLibrary && h(WidgetLibrary, {
      onAdd: handleAddWidget,
      onClose: () => setShowLibrary(false),
    })
  );
}

function WidgetEditor({ widget, layoutColumns, onUpdateTitle, onUpdateColSpan, onUpdateSetting, onRemove }: {
  widget: WidgetConfig;
  layoutColumns: number;
  onUpdateTitle: (t: string) => void;
  onUpdateColSpan: (c: number) => void;
  onUpdateSetting: (k: string, v: string) => void;
  onRemove: () => void;
}) {
  return h('div', { class: 'widget-editor' },
    h('div', { class: 'editor-field' },
      h('label', null, 'Title'),
      h('input', {
        type: 'text',
        value: widget.title,
        onInput: (e: Event) => onUpdateTitle((e.target as HTMLInputElement).value),
      }),
    ),
    h('div', { class: 'editor-field' },
      h('label', null, 'Columns'),
      h('input', {
        type: 'number',
        min: '1',
        max: String(layoutColumns),
        value: String(widget.colSpan),
        onInput: (e: Event) => onUpdateColSpan(parseInt((e.target as HTMLInputElement).value) || 1),
      }),
    ),
    widget.type === 'send-hotkey' && h('div', { class: 'editor-field' },
      h('label', null, 'Hotkey'),
      h('input', {
        type: 'text',
        value: widget.settings.keys || '',
        placeholder: 'ctrl+c',
        onInput: (e: Event) => onUpdateSetting('keys', (e.target as HTMLInputElement).value),
      }),
    ),
    widget.type === 'open-url' && h('div', { class: 'editor-field' },
      h('label', null, 'URL'),
      h('input', {
        type: 'text',
        value: widget.settings.url || '',
        placeholder: 'https://...',
        onInput: (e: Event) => onUpdateSetting('url', (e.target as HTMLInputElement).value),
      }),
    ),
    widget.type === 'type-text' && h('div', { class: 'editor-field' },
      h('label', null, 'Text'),
      h('input', {
        type: 'text',
        value: widget.settings.text || '',
        placeholder: 'Text to type...',
        onInput: (e: Event) => onUpdateSetting('text', (e.target as HTMLInputElement).value),
      }),
    ),
    h('button', { class: 'edit-remove-btn', onClick: onRemove }, 'Remove Widget'),
  );
}

function WidgetLibrary({ onAdd, onClose }: { onAdd: (t: WidgetType) => void; onClose: () => void }) {
  return h('div', { class: 'widget-library-overlay', onClick: onClose },
    h('div', { class: 'widget-library-modal', onClick: (e: Event) => e.stopPropagation() },
      h('div', { class: 'library-header' },
        h('span', null, 'Widget Library'),
        h('button', { class: 'picker-close', onClick: onClose }, 'X'),
      ),
      h('div', { class: 'library-grid' },
        WIDGET_CATALOG.map(item =>
          h('button', {
            class: 'library-item',
            key: item.type,
            onClick: () => onAdd(item.type),
          },
            h('div', { class: 'library-icon' }, item.icon),
            h('div', { class: 'library-label' }, item.label),
            h('div', { class: 'library-desc' }, item.description),
          )
        )
      )
    )
  );
}

function WidgetContent({ widget }: { widget: WidgetConfig; onExecute: () => void }) {
  switch (widget.type) {
    case 'system-monitor': return h(SystemMonitorWidget, { settings: widget.settings });
    case 'clock': return h(ClockWidget, null);
    case 'quick-actions': return h(QuickActionsWidget, null);
    case 'send-hotkey': return h(SendHotkeyWidget, { settings: widget.settings });
    case 'open-url': return h(OpenUrlWidget, { settings: widget.settings });
    case 'type-text': return h(TypeTextWidget, { settings: widget.settings });
    default: return h('div', { class: 'widget-unknown' }, 'Unknown widget');
  }
}

function SystemMonitorWidget({ settings }: { settings: Record<string, any> }) {
  const [stats, setStats] = useState<SystemStats | null>(null);

  useEffect(() => {
    let active = true;
    const load = async () => {
      try {
        const res = await fetch('/api/system-stats');
        const data = await res.json();
        if (active && data.data) setStats(data.data);
      } catch (e) {}
    };
    load();
    const interval = setInterval(load, settings.refreshInterval || 2000);
    return () => { active = false; clearInterval(interval); };
  }, []);

  if (!stats) return h('div', { class: 'sysmon-loading' }, 'Loading...');

  function fmtBytes(b: number): string {
    if (b >= 1073741824) return (b / 1073741824).toFixed(1) + ' GB';
    if (b >= 1048576) return (b / 1048576).toFixed(0) + ' MB';
    return (b / 1024).toFixed(0) + ' KB';
  }

  function fmtUptime(s: number): string {
    const d = Math.floor(s / 86400);
    const h = Math.floor((s % 86400) / 3600);
    const m = Math.floor((s % 3600) / 60);
    if (d > 0) return `${d}d ${h}h`;
    if (h > 0) return `${h}h ${m}m`;
    return `${m}m`;
  }

  const cpuColor = stats.cpu_usage < 50 ? '#4caf50' : stats.cpu_usage < 80 ? '#ff9800' : '#f44336';
  const memColor = stats.memory_usage < 60 ? '#2196f3' : stats.memory_usage < 85 ? '#ff9800' : '#f44336';
  const swapPct = stats.swap_total > 0 ? (stats.swap_used / stats.swap_total * 100) : 0;
  const swapColor = swapPct < 50 ? '#9c27b0' : swapPct < 80 ? '#ff9800' : '#f44336';

  return h('div', { class: 'sysmon-detail' },
    h('div', { class: 'sysmon-cpu-model' }, stats.cpu_model),
    h('div', { class: 'sysmon-bars' },
      h('div', { class: 'sysmon-bar-group' },
        h('div', { class: 'sysmon-bar-header' },
          h('span', { class: 'sysmon-bar-label' }, 'CPU'),
          h('span', { class: 'sysmon-bar-value', style: { color: cpuColor } }, `${stats.cpu_usage.toFixed(1)}%`),
        ),
        h('div', { class: 'sysmon-bar-track' },
          h('div', { class: 'sysmon-bar-fill', style: { width: `${stats.cpu_usage}%`, background: cpuColor } })
        ),
      ),
      h('div', { class: 'sysmon-bar-group' },
        h('div', { class: 'sysmon-bar-header' },
          h('span', { class: 'sysmon-bar-label' }, 'Memory'),
          h('span', { class: 'sysmon-bar-value', style: { color: memColor } }, `${stats.memory_usage.toFixed(1)}%`),
        ),
        h('div', { class: 'sysmon-bar-track' },
          h('div', { class: 'sysmon-bar-fill', style: { width: `${stats.memory_usage}%`, background: memColor } })
        ),
        h('div', { class: 'sysmon-bar-detail' }, `${fmtBytes(stats.memory_used)} / ${fmtBytes(stats.memory_total)}`),
      ),
      stats.swap_total > 0 && h('div', { class: 'sysmon-bar-group' },
        h('div', { class: 'sysmon-bar-header' },
          h('span', { class: 'sysmon-bar-label' }, 'Swap'),
          h('span', { class: 'sysmon-bar-value', style: { color: swapColor } }, `${swapPct.toFixed(1)}%`),
        ),
        h('div', { class: 'sysmon-bar-track' },
          h('div', { class: 'sysmon-bar-fill', style: { width: `${swapPct}%`, background: swapColor } })
        ),
        h('div', { class: 'sysmon-bar-detail' }, `${fmtBytes(stats.swap_used)} / ${fmtBytes(stats.swap_total)}`),
      ),
    ),
    h('div', { class: 'sysmon-info-grid' },
      h('div', { class: 'sysmon-info-item' },
        h('div', { class: 'sysmon-info-label' }, 'Cores'),
        h('div', { class: 'sysmon-info-value' }, String(stats.cpu_cores)),
      ),
      h('div', { class: 'sysmon-info-item' },
        h('div', { class: 'sysmon-info-label' }, 'Load'),
        h('div', { class: 'sysmon-info-value' }, `${stats.load_avg[0].toFixed(2)} / ${stats.load_avg[1].toFixed(2)} / ${stats.load_avg[2].toFixed(2)}`),
      ),
      h('div', { class: 'sysmon-info-item' },
        h('div', { class: 'sysmon-info-label' }, 'Uptime'),
        h('div', { class: 'sysmon-info-value' }, fmtUptime(stats.uptime)),
      ),
      h('div', { class: 'sysmon-info-item' },
        h('div', { class: 'sysmon-info-label' }, 'Processes'),
        h('div', { class: 'sysmon-info-value' }, `${stats.process_count} / ${stats.thread_count}`),
      ),
    ),
  );
}

function ClockWidget() {
  const [time, setTime] = useState(() => new Date());

  useEffect(() => {
    const interval = setInterval(() => setTime(new Date()), 1000);
    return () => clearInterval(interval);
  }, []);

  const hh = time.getHours().toString().padStart(2, '0');
  const mm = time.getMinutes().toString().padStart(2, '0');
  const ss = time.getSeconds().toString().padStart(2, '0');

  return h('div', { class: 'clock-slot' },
    h('div', { class: 'clock-time' }, `${hh}:${mm}`),
    h('div', { class: 'clock-seconds' }, ss),
    h('div', { class: 'clock-date' },
      time.toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric', year: 'numeric' })
    )
  );
}

function QuickActionsWidget() {
  const [actions, setActions] = useState<string[]>([]);
  const [executing, setExecuting] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);

  useEffect(() => {
    fetch('/api/actions')
      .then(r => r.json())
      .then(d => setActions(d.data || []))
      .catch(() => {});
  }, []);

  async function handleExecute(actionName: string) {
    setExecuting(actionName);
    setResult(null);
    try {
      const res = await executeAction(actionName);
      setResult(res);
    } catch (e) {
      setResult('Error executing action');
    }
    setExecuting(null);
    setTimeout(() => setResult(null), 3000);
  }

  return h('div', { class: 'quick-actions-widget' },
    result && h('div', { class: 'action-result' }, result),
    actions.map((a, i) => {
      const name = a.split(' (')[0];
      const cat = a.match(/\((.+)\)/)?.[1] || '';
      return h('button', {
        class: `action-btn ${executing === a ? 'executing' : ''}`,
        key: i,
        onClick: () => handleExecute(name),
        disabled: executing !== null,
      },
        h('span', { class: 'action-btn-name' }, name),
        h('span', { class: 'action-btn-cat' }, cat),
      );
    }),
  );
}

function SendHotkeyWidget({ settings }: { settings: Record<string, any> }) {
  const [executing, setExecuting] = useState(false);
  const [result, setResult] = useState<string | null>(null);

  async function handleExecute() {
    setExecuting(true);
    setResult(null);
    try {
      const res = await executeAction('Send Hotkey');
      setResult(res);
    } catch (e) {
      setResult('Error');
    }
    setExecuting(false);
    setTimeout(() => setResult(null), 3000);
  }

  return h('div', { class: 'action-single-widget' },
    h('div', { class: 'action-single-keys' }, settings.keys || 'No key set'),
    result && h('div', { class: 'action-result' }, result),
    h('button', {
      class: `action-single-btn ${executing ? 'executing' : ''}`,
      onClick: handleExecute,
      disabled: executing,
    }, executing ? 'Sending...' : 'Send'),
  );
}

function OpenUrlWidget({ settings }: { settings: Record<string, any> }) {
  const [executing, setExecuting] = useState(false);
  const [result, setResult] = useState<string | null>(null);

  async function handleExecute() {
    setExecuting(true);
    setResult(null);
    try {
      const res = await executeAction('Open URL');
      setResult(res);
    } catch (e) {
      setResult('Error');
    }
    setExecuting(false);
    setTimeout(() => setResult(null), 3000);
  }

  return h('div', { class: 'action-single-widget' },
    h('div', { class: 'action-single-url' }, settings.url || 'No URL set'),
    result && h('div', { class: 'action-result' }, result),
    h('button', {
      class: `action-single-btn ${executing ? 'executing' : ''}`,
      onClick: handleExecute,
      disabled: executing,
    }, executing ? 'Opening...' : 'Open'),
  );
}

function TypeTextWidget({ settings }: { settings: Record<string, any> }) {
  const [executing, setExecuting] = useState(false);
  const [result, setResult] = useState<string | null>(null);

  async function handleExecute() {
    setExecuting(true);
    setResult(null);
    try {
      const res = await executeAction('Type Text');
      setResult(res);
    } catch (e) {
      setResult('Error');
    }
    setExecuting(false);
    setTimeout(() => setResult(null), 3000);
  }

  return h('div', { class: 'action-single-widget' },
    h('div', { class: 'action-single-text' }, settings.text || 'No text set'),
    result && h('div', { class: 'action-result' }, result),
    h('button', {
      class: `action-single-btn ${executing ? 'executing' : ''}`,
      onClick: handleExecute,
      disabled: executing,
    }, executing ? 'Typing...' : 'Type'),
  );
}
