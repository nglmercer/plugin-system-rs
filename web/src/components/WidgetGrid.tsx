import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { WidgetConfig, WidgetType, WidgetVariant, DashboardLayout, SystemStats, WIDGET_VARIANTS, WIZARD_STEPS } from '../lib/types';
import { fetchDashboard, saveDashboard, executeAction, recordHotkey } from '../lib/api';

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

function getDefaultVariant(type: WidgetType): string {
  switch (type) {
    case 'system-monitor': return 'compact';
    case 'clock': return 'digital';
    case 'quick-actions': return 'compact';
    case 'send-hotkey': return 'compact';
    case 'open-url': return 'compact';
    case 'type-text': return 'compact';
    default: return 'compact';
  }
}

function getDefaultSettings(type: WidgetType): Record<string, any> {
  const settings: Record<string, any> = { variant: getDefaultVariant(type) };
  switch (type) {
    case 'system-monitor': settings.refreshInterval = 2000; break;
    case 'clock': break;
    case 'quick-actions': break;
    case 'send-hotkey': settings.keys = 'ctrl+c'; break;
    case 'open-url': settings.url = 'https://example.com'; break;
    case 'type-text': settings.text = 'Hello!'; break;
  }
  return settings;
}

export function WidgetGrid() {
  const [layout, setLayout] = useState<DashboardLayout>({ widgets: [], columns: 3 });
  const [loading, setLoading] = useState(true);
  const [showLibrary, setShowLibrary] = useState(false);
  const [wizardWidget, setWizardWidget] = useState<WidgetConfig | null>(null);

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
      title: catalog.label,
      colSpan: catalog.defaultColSpan,
      rowSpan: catalog.defaultRowSpan,
      settings,
    };
    persist({ ...layout, widgets: [...layout.widgets, widget] });
    setShowLibrary(false);
    setWizardWidget(widget);
  }

  function handleSaveWidget(id: string, updates: { title?: string; colSpan?: number; settings?: Record<string, any> }) {
    persist({
      ...layout,
      widgets: layout.widgets.map(w => w.id === id ? { ...w, ...updates } : w),
    });
    setWizardWidget(null);
  }

  function handleRemoveWidget(id: string) {
    persist({ ...layout, widgets: layout.widgets.filter(w => w.id !== id) });
    setWizardWidget(null);
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
              class: `dashboard-widget variant-${(widget.settings.variant || 'compact')}`,
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
                    onClick: () => setWizardWidget(widget),
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
            )
          )
        ),

    showLibrary && h(WidgetLibrary, {
      onAdd: handleAddWidget,
      onClose: () => setShowLibrary(false),
    }),

    wizardWidget && h(WidgetWizard, {
      widget: wizardWidget,
      columns: layout.columns,
      onSave: (id, updates) => handleSaveWidget(id, updates),
      onRemove: () => handleRemoveWidget(wizardWidget.id),
      onClose: () => setWizardWidget(null),
    })
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

function WidgetWizard({ widget, columns, onSave, onRemove, onClose }: {
  widget: WidgetConfig;
  columns: number;
  onSave: (id: string, updates: { title?: string; colSpan?: number; settings?: Record<string, any> }) => void;
  onRemove: () => void;
  onClose: () => void;
}) {
  const [step, setStep] = useState(0);
  const [title, setTitle] = useState(widget.title);
  const [colSpan, setColSpan] = useState(widget.colSpan);
  const [settings, setSettings] = useState({ ...widget.settings });
  const [variant, setVariant] = useState<string>(widget.settings.variant || getDefaultVariant(widget.type));

  const totalSteps = WIZARD_STEPS.length;
  const currentStep = WIZARD_STEPS[step];

  function handleNext() {
    if (step < totalSteps - 1) setStep(step + 1);
  }

  function handleBack() {
    if (step > 0) setStep(step - 1);
  }

  function handleApply() {
    onSave(widget.id, {
      title,
      colSpan,
      settings: { ...settings, variant },
    });
  }

  return h('div', { class: 'wizard-overlay', onClick: onClose },
    h('div', { class: 'wizard-modal', onClick: (e: Event) => e.stopPropagation() },
      h('div', { class: 'wizard-header' },
        h('div', { class: 'wizard-title' }, `Edit: ${WIDGET_CATALOG.find(w => w.type === widget.type)?.label || widget.type}`),
        h('button', { class: 'picker-close', onClick: onClose }, 'X'),
      ),

      h('div', { class: 'wizard-steps' },
        WIZARD_STEPS.map((s, i) =>
          h('div', {
            class: `wizard-step-indicator ${i === step ? 'active' : i < step ? 'done' : ''}`,
            key: s.id,
          },
            h('div', { class: 'wizard-step-circle' }, s.icon),
            h('div', { class: 'wizard-step-label' }, s.label),
          )
        )
      ),

      h('div', { class: 'wizard-body' },
        step === 0 && h(WizardGeneral, { title, colSpan, columns, onChangeTitle: setTitle, onChangeColSpan: setColSpan }),
        step === 1 && h(WizardConfig, { widget, settings, onChange: setSettings }),
        step === 2 && h(WizardStyle, { widget, variant, onChange: setVariant }),
        step === 3 && h(WizardConfirm, { widget, title, colSpan, settings, variant, onApply: handleApply, onRemove }),
      ),

      h('div', { class: 'wizard-footer' },
        step > 0 && h('button', { class: 'wizard-btn back', onClick: handleBack }, 'Back'),
        h('div', { class: 'wizard-footer-spacer' }),
        step < totalSteps - 1
          ? h('button', { class: 'wizard-btn next', onClick: handleNext }, 'Next')
          : h('button', { class: 'wizard-btn apply', onClick: handleApply }, 'Save & Close'),
      ),
    ),
  );
}

function WizardGeneral({ title, colSpan, columns, onChangeTitle, onChangeColSpan }: {
  title: string;
  colSpan: number;
  columns: number;
  onChangeTitle: (t: string) => void;
  onChangeColSpan: (c: number) => void;
}) {
  return h('div', { class: 'wizard-step-content' },
    h('h3', { class: 'wizard-step-heading' }, 'General Settings'),
    h('div', { class: 'wizard-field' },
      h('label', null, 'Widget Title'),
      h('input', {
        type: 'text',
        value: title,
        onInput: (e: Event) => onChangeTitle((e.target as HTMLInputElement).value),
        placeholder: 'Enter widget title...',
      }),
    ),
    h('div', { class: 'wizard-field' },
      h('label', null, 'Column Span'),
      h('input', {
        type: 'number',
        min: '1',
        max: String(columns),
        value: String(colSpan),
        onInput: (e: Event) => onChangeColSpan(parseInt((e.target as HTMLInputElement).value) || 1),
      }),
      h('span', { class: 'wizard-field-hint' }, `Grid has ${columns} columns`),
    ),
  );
}

function WizardConfig({ widget, settings, onChange }: {
  widget: WidgetConfig;
  settings: Record<string, any>;
  onChange: (s: Record<string, any>) => void;
}) {
  function set(key: string, value: any) {
    onChange({ ...settings, [key]: value });
  }

  return h('div', { class: 'wizard-step-content' },
    h('h3', { class: 'wizard-step-heading' }, 'Widget Configuration'),

    widget.type === 'send-hotkey' && h(HotkeyRecorder, {
      currentKeys: settings.keys || '',
      onChange: (keys) => set('keys', keys),
    }),

    widget.type === 'open-url' && h('div', { class: 'wizard-field' },
      h('label', null, 'URL'),
      h('input', {
        type: 'text',
        value: settings.url || '',
        placeholder: 'https://example.com',
        onInput: (e: Event) => set('url', (e.target as HTMLInputElement).value),
      }),
    ),

    widget.type === 'type-text' && h('div', { class: 'wizard-field' },
      h('label', null, 'Text'),
      h('textarea', {
        value: settings.text || '',
        placeholder: 'Text to type...',
        onInput: (e: Event) => set('text', (e.target as HTMLTextAreaElement).value),
      }),
    ),

    widget.type === 'system-monitor' && h('div', { class: 'wizard-field' },
      h('label', null, 'Refresh Interval (ms)'),
      h('input', {
        type: 'number',
        min: '500',
        step: '500',
        value: String(settings.refreshInterval || 2000),
        onInput: (e: Event) => set('refreshInterval', parseInt((e.target as HTMLInputElement).value) || 2000),
      }),
    ),
  );
}

function HotkeyRecorder({ currentKeys, onChange }: { currentKeys: string; onChange: (keys: string) => void }) {
  const [recording, setRecording] = useState(false);
  const [pending, setPending] = useState<string | null>(null);

  async function startRecording() {
    setRecording(true);
    setPending(null);
    try {
      const combo = await recordHotkey(5000);
      setPending(combo);
    } catch {
      setPending(null);
    }
    setRecording(false);
  }

  function confirmCombo() {
    if (pending) {
      onChange(pending);
      setPending(null);
    }
  }

  function cancelPending() {
    setPending(null);
  }

  return h('div', { class: 'wizard-field' },
    h('label', null, 'Hotkey Combination'),

    !pending && h('div', { class: 'hotkey-display' },
      h('span', { class: 'hotkey-keys' }, recording ? 'Listening...' : (currentKeys || 'Not set')),
      h('button', {
        class: `hotkey-record-btn ${recording ? 'recording' : ''}`,
        onClick: startRecording,
        disabled: recording,
      }, recording ? '...' : 'Record'),
    ),

    pending && h('div', { class: 'hotkey-pending' },
      h('span', { class: 'hotkey-pending-combo' }, pending),
      h('button', { class: 'hotkey-confirm-btn', onClick: confirmCombo }, 'Confirm'),
      h('button', { class: 'hotkey-retry-btn', onClick: startRecording }, 'Retry'),
      h('button', { class: 'hotkey-cancel-btn', onClick: cancelPending }, 'Cancel'),
    ),
  );
}

function WizardStyle({ widget, variant, onChange }: {
  widget: WidgetConfig;
  variant: WidgetVariant;
  onChange: (v: WidgetVariant) => void;
}) {
  const entries = WIDGET_VARIANTS.find(e => e.type === widget.type);
  if (!entries) return null;

  return h('div', { class: 'wizard-step-content' },
    h('h3', { class: 'wizard-step-heading' }, 'Style Variant'),
    h('p', { class: 'wizard-step-desc' }, 'Choose how this widget displays'),

    h('div', { class: 'variant-grid' },
      entries.variants.map(v =>
        h('button', {
          class: `variant-card ${variant === v.value ? 'selected' : ''}`,
          key: v.value,
          onClick: () => onChange(v.value),
        },
          h('div', { class: 'variant-card-preview' },
            h(VariantPreview, { type: widget.type, variant: v.value }),
          ),
          h('div', { class: 'variant-card-info' },
            h('div', { class: 'variant-card-label' }, v.label),
            h('div', { class: 'variant-card-desc' }, v.description),
          ),
        )
      )
    ),
  );
}

function VariantPreview({ type, variant }: { type: WidgetType; variant: WidgetVariant }) {
  switch (type) {
    case 'system-monitor':
      switch (variant) {
        case 'minimal': return h('div', { class: 'variant-preview sysmon-minimal' }, h('div', null, '42% CPU'), h('div', null, '56% RAM'));
        case 'compact': return h('div', { class: 'variant-preview sysmon-compact' }, h('div', { class: 'mini-bar' }, h('div', { class: 'mini-bar-fill', style: { width: '42%', background: '#4caf50' } })), h('div', { class: 'mini-bar' }, h('div', { class: 'mini-bar-fill', style: { width: '56%', background: '#2196f3' } })));
        case 'detailed': return h('div', { class: 'variant-preview sysmon-detailed' }, h('div', { class: 'mini-grid' }, h('div', null, '42%'), h('div', null, '56%'), h('div', null, '1.2'), h('div', null, '2d')));
      }
    case 'clock':
      switch (variant) {
        case 'simple': return h('div', { class: 'variant-preview clock-simple' }, '14:30');
        case 'digital': return h('div', { class: 'variant-preview clock-digital' }, '14:30', h('div', { class: 'mini-sec' }, '15'), h('div', { class: 'mini-date' }, 'Mon'));
        case 'detailed': return h('div', { class: 'variant-preview clock-detailed' }, '14:30:15', h('div', { class: 'mini-date' }, 'Monday, Jun 10'));
      }
    default:
      return h('div', { class: 'variant-preview simple-preview' },
        h('div', { class: variant === 'compact' ? 'preview-btn-sm' : 'preview-btn-lg' }, 'Action'),
      );
  }
}

function WizardConfirm({ widget, title, colSpan, settings, variant, onApply, onRemove }: {
  widget: WidgetConfig;
  title: string;
  colSpan: number;
  settings: Record<string, any>;
  variant: WidgetVariant;
  onApply: () => void;
  onRemove: () => void;
}) {
  return h('div', { class: 'wizard-step-content' },
    h('h3', { class: 'wizard-step-heading' }, 'Confirm Changes'),
    h('p', { class: 'wizard-step-desc' }, 'Review your widget configuration before saving'),

    h('div', { class: 'confirm-details' },
      h('div', { class: 'confirm-row' }, h('span', null, 'Title'), h('span', null, title)),
      h('div', { class: 'confirm-row' }, h('span', null, 'Span'), h('span', null, `${colSpan} column${colSpan > 1 ? 's' : ''}`)),
      h('div', { class: 'confirm-row' }, h('span', null, 'Variant'), h('span', null, variant)),
      ...Object.entries(settings).filter(([k]) => k !== 'variant').map(([k, v]) =>
        h('div', { class: 'confirm-row', key: k }, h('span', null, k), h('span', null, String(v).substring(0, 40)))
      ),
    ),

    h('div', { class: 'confirm-preview' },
      h('div', { class: 'wizard-step-heading', style: 'font-size:0.8rem;color:#888;margin-bottom:0.5rem' }, 'Preview'),
      h('div', { class: 'preview-frame' },
        h(WidgetContent, { widget: { ...widget, title, colSpan, settings: { ...settings, variant } } }),
      ),
    ),

    h('button', { class: 'wizard-remove-btn', onClick: onRemove }, 'Delete Widget'),
  );
}

function WidgetContent({ widget }: { widget: WidgetConfig }) {
  switch (widget.type) {
    case 'system-monitor': return h(SystemMonitorWidget, { settings: widget.settings });
    case 'clock': return h(ClockWidget, { settings: widget.settings });
    case 'quick-actions': return h(QuickActionsWidget, null);
    case 'send-hotkey': return h(SendHotkeyWidget, { settings: widget.settings });
    case 'open-url': return h(OpenUrlWidget, { settings: widget.settings });
    case 'type-text': return h(TypeTextWidget, { settings: widget.settings });
    default: return h('div', { class: 'widget-unknown' }, 'Unknown widget');
  }
}

function SystemMonitorWidget({ settings }: { settings: Record<string, any> }) {
  const [stats, setStats] = useState<SystemStats | null>(null);
  const variant = (settings.variant || 'compact') as WidgetVariant;

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

  if (variant === 'minimal') {
    return h('div', { class: 'sysmon-variant minimal' },
      h('div', { class: 'sysmon-big', style: { color: cpuColor } }, `${stats.cpu_usage.toFixed(0)}%`),
      h('div', { class: 'sysmon-big', style: { color: memColor } }, `${stats.memory_usage.toFixed(0)}%`),
    );
  }

  if (variant === 'compact') {
    return h('div', { class: 'sysmon-variant compact' },
      h('div', { class: 'sysmon-cpu-model' }, stats.cpu_model.substring(0, 30)),
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
          h('span', { class: 'sysmon-bar-label' }, 'RAM'),
          h('span', { class: 'sysmon-bar-value', style: { color: memColor } }, `${stats.memory_usage.toFixed(1)}%`),
        ),
        h('div', { class: 'sysmon-bar-track' },
          h('div', { class: 'sysmon-bar-fill', style: { width: `${stats.memory_usage}%`, background: memColor } })
        ),
      ),
      h('div', { class: 'sysmon-load-row' },
        h('span', null, `Load: ${stats.load_avg[0].toFixed(2)}`),
        h('span', null, `Up: ${fmtUptime(stats.uptime)}`),
      ),
    );
  }

  const swapPct = stats.swap_total > 0 ? (stats.swap_used / stats.swap_total * 100) : 0;
  const swapColor = swapPct < 50 ? '#9c27b0' : swapPct < 80 ? '#ff9800' : '#f44336';

  return h('div', { class: 'sysmon-variant detailed' },
    h('div', { class: 'sysmon-cpu-model' }, stats.cpu_model),
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

function ClockWidget({ settings }: { settings: Record<string, any> }) {
  const [time, setTime] = useState(() => new Date());
  const variant = (settings.variant || 'digital') as string;

  useEffect(() => {
    const interval = setInterval(() => setTime(new Date()), 1000);
    return () => clearInterval(interval);
  }, []);

  const hh = time.getHours().toString().padStart(2, '0');
  const mm = time.getMinutes().toString().padStart(2, '0');
  const ss = time.getSeconds().toString().padStart(2, '0');

  if (variant === 'simple') {
    return h('div', { class: 'clock-variant simple' },
      h('div', { class: 'clock-time' }, `${hh}:${mm}`),
    );
  }

  if (variant === 'detailed') {
    return h('div', { class: 'clock-variant detailed' },
      h('div', { class: 'clock-time' }, `${hh}:${mm}:${ss}`),
      h('div', { class: 'clock-date' },
        time.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric', year: 'numeric' })
      ),
    );
  }

  return h('div', { class: 'clock-variant digital' },
    h('div', { class: 'clock-time' }, `${hh}:${mm}`),
    h('div', { class: 'clock-seconds' }, ss),
    h('div', { class: 'clock-date' },
      time.toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric', year: 'numeric' })
    ),
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
  const variant = (settings.variant || 'compact') as string;

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

  if (variant === 'detailed') {
    return h('div', { class: 'action-single-widget detailed' },
      h('div', { class: 'action-single-keys' }, settings.keys || 'No key set'),
      h('div', { class: 'action-single-desc' }, 'Send keyboard hotkey'),
      result && h('div', { class: 'action-result' }, result),
      h('button', {
        class: `action-single-btn ${executing ? 'executing' : ''}`,
        onClick: handleExecute,
        disabled: executing,
      }, executing ? 'Sending...' : 'Send'),
    );
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
  const variant = (settings.variant || 'compact') as string;

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

  if (variant === 'detailed') {
    return h('div', { class: 'action-single-widget detailed' },
      h('div', { class: 'action-single-url' }, settings.url || 'No URL set'),
      h('div', { class: 'action-single-desc' }, 'Open URL in browser'),
      result && h('div', { class: 'action-result' }, result),
      h('button', {
        class: `action-single-btn ${executing ? 'executing' : ''}`,
        onClick: handleExecute,
        disabled: executing,
      }, executing ? 'Opening...' : 'Open'),
    );
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
  const variant = (settings.variant || 'compact') as string;

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

  if (variant === 'detailed') {
    return h('div', { class: 'action-single-widget detailed' },
      h('div', { class: 'action-single-text' }, settings.text || 'No text set'),
      h('div', { class: 'action-single-desc' }, 'Type text string'),
      result && h('div', { class: 'action-result' }, result),
      h('button', {
        class: `action-single-btn ${executing ? 'executing' : ''}`,
        onClick: handleExecute,
        disabled: executing,
      }, executing ? 'Typing...' : 'Type'),
    );
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
