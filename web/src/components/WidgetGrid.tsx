import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { WidgetConfig, WidgetType, DashboardLayout } from '../lib/types';
import { fetchDashboard, saveDashboard } from '../lib/api';

const WIDGET_CATALOG: { type: WidgetType; label: string; icon: string; description: string; defaultColSpan: number; defaultRowSpan: number }[] = [
  { type: 'system-monitor', label: 'System Monitor', icon: '%', description: 'CPU, Memory usage', defaultColSpan: 1, defaultRowSpan: 1 },
  { type: 'clock', label: 'Clock', icon: 'T', description: 'Current time & date', defaultColSpan: 1, defaultRowSpan: 1 },
  { type: 'actions', label: 'Quick Actions', icon: '*', description: 'Trigger actions', defaultColSpan: 1, defaultRowSpan: 1 },
];

function generateId(): string {
  return Math.random().toString(36).substring(2, 10);
}

function getDefaultSettings(type: WidgetType): Record<string, any> {
  switch (type) {
    case 'system-monitor': return { refreshInterval: 2000 };
    case 'clock': return { format: 'digital' };
    case 'actions': return {};
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
    const widget: WidgetConfig = {
      id: generateId(),
      type,
      title: catalog.label,
      colSpan: catalog.defaultColSpan,
      rowSpan: catalog.defaultRowSpan,
      settings: getDefaultSettings(type),
    };
    persist({ ...layout, widgets: [...layout.widgets, widget] });
    setShowLibrary(false);
  }

  function handleRemoveWidget(id: string) {
    persist({ ...layout, widgets: layout.widgets.filter(w => w.id !== id) });
    setEditingWidget(null);
  }

  function handleUpdateTitle(id: string, title: string) {
    persist({
      ...layout,
      widgets: layout.widgets.map(w => w.id === id ? { ...w, title } : w),
    });
  }

  function handleUpdateColSpan(id: string, colSpan: number) {
    persist({
      ...layout,
      widgets: layout.widgets.map(w => w.id === id ? { ...w, colSpan } : w),
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
                h(WidgetContent, { widget })
              ),
              editingWidget === widget.id && h('div', { class: 'widget-editor' },
                h('div', { class: 'editor-field' },
                  h('label', null, 'Title'),
                  h('input', {
                    type: 'text',
                    value: widget.title,
                    onInput: (e: Event) => handleUpdateTitle(widget.id, (e.target as HTMLInputElement).value),
                  }),
                ),
                h('div', { class: 'editor-field' },
                  h('label', null, 'Columns'),
                  h('input', {
                    type: 'number',
                    min: '1',
                    max: String(layout.columns),
                    value: String(widget.colSpan),
                    onInput: (e: Event) => handleUpdateColSpan(widget.id, parseInt((e.target as HTMLInputElement).value) || 1),
                  }),
                ),
              )
            )
          )
        ),

    showLibrary && h('div', { class: 'widget-library-overlay', onClick: () => setShowLibrary(false) },
      h('div', { class: 'widget-library-modal', onClick: (e: Event) => e.stopPropagation() },
        h('div', { class: 'library-header' },
          h('span', null, 'Widget Library'),
          h('button', { class: 'picker-close', onClick: () => setShowLibrary(false) }, 'X'),
        ),
        h('div', { class: 'library-grid' },
          WIDGET_CATALOG.map(item =>
            h('button', {
              class: 'library-item',
              key: item.type,
              onClick: () => handleAddWidget(item.type),
            },
              h('div', { class: 'library-icon' }, item.icon),
              h('div', { class: 'library-label' }, item.label),
              h('div', { class: 'library-desc' }, item.description),
            )
          )
        )
      )
    )
  );
}

function WidgetContent({ widget }: { widget: WidgetConfig }) {
  switch (widget.type) {
    case 'system-monitor': return h(SystemMonitorWidget, { settings: widget.settings });
    case 'clock': return h(ClockWidget, null);
    case 'actions': return h(ActionsWidget, null);
    default: return h('div', { class: 'widget-unknown' }, 'Unknown widget');
  }
}

function SystemMonitorWidget({ settings }: { settings: Record<string, any> }) {
  const [cpu, setCpu] = useState(0);
  const [mem, setMem] = useState(0);

  useEffect(() => {
    let active = true;
    const load = async () => {
      try {
        const res = await fetch('/api/system-stats');
        const data = await res.json();
        if (active && data.data) {
          setCpu(data.data.cpu_usage);
          setMem(data.data.memory_usage);
        }
      } catch (e) {}
    };
    load();
    const interval = setInterval(load, settings.refreshInterval || 2000);
    return () => { active = false; clearInterval(interval); };
  }, []);

  return h('div', { class: 'sysmon-slot' },
    h('div', { class: 'sysmon-row' },
      h('div', { class: 'sysmon-item' },
        h('div', { class: 'sysmon-val', style: { color: cpu < 50 ? '#4caf50' : cpu < 80 ? '#ff9800' : '#f44336' } },
          `${cpu.toFixed(0)}%`
        ),
        h('div', { class: 'sysmon-bar' },
          h('div', { class: 'sysmon-bar-fill', style: { width: `${cpu}%`, background: cpu < 50 ? '#4caf50' : cpu < 80 ? '#ff9800' : '#f44336' } })
        ),
        h('div', { class: 'sysmon-lbl' }, 'CPU')
      ),
      h('div', { class: 'sysmon-item' },
        h('div', { class: 'sysmon-val', style: { color: mem < 60 ? '#2196f3' : mem < 85 ? '#ff9800' : '#f44336' } },
          `${mem.toFixed(0)}%`
        ),
        h('div', { class: 'sysmon-bar' },
          h('div', { class: 'sysmon-bar-fill', style: { width: `${mem}%`, background: mem < 60 ? '#2196f3' : mem < 85 ? '#ff9800' : '#f44336' } })
        ),
        h('div', { class: 'sysmon-lbl' }, 'RAM')
      )
    )
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
      time.toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' })
    )
  );
}

function ActionsWidget() {
  const [actions, setActions] = useState<string[]>([]);

  useEffect(() => {
    fetch('/api/actions')
      .then(r => r.json())
      .then(d => setActions(d.data || []))
      .catch(() => {});
  }, []);

  return h('div', { class: 'actions-slot' },
    actions.length === 0
      ? h('div', { class: 'actions-empty' }, 'No actions available')
      : actions.slice(0, 5).map((a, i) =>
          h('button', { class: 'actions-slot-btn', key: i, onClick: (e: Event) => e.stopPropagation() },
            a.split(' (')[0]
          )
        )
  );
}
