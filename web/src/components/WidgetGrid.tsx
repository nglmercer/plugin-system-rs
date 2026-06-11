import { h } from 'preact';
import { useState } from 'preact/hooks';
import { WidgetConfig, StreamEvent } from '../lib/types';
import { loadLayout, saveLayout, createWidget } from '../lib/widgetStore';
import { ButtonGridWidget } from './ButtonGridWidget';
import { EventLogWidget } from './EventLogWidget';
import { SystemMonitorWidget } from './SystemMonitorWidget';
import { ClockWidget } from './ClockWidget';
import { ActionsWidget } from './ActionsWidget';

interface WidgetGridProps {
  events: StreamEvent[];
}

const WIDGET_TYPES: { type: WidgetConfig['type']; label: string; icon: string }[] = [
  { type: 'button-grid', label: 'Button Grid', icon: '[=]' },
  { type: 'event-log', label: 'Event Log', icon: '[#]' },
  { type: 'system-monitor', label: 'System Monitor', icon: '[%]' },
  { type: 'clock', label: 'Clock', icon: '[T]' },
  { type: 'actions', label: 'Actions', icon: '[*]' },
];

export function WidgetGrid({ events }: WidgetGridProps) {
  const [layout, setLayout] = useState(() => loadLayout());
  const [editingWidget, setEditingWidget] = useState<string | null>(null);
  const [showAddMenu, setShowAddMenu] = useState(false);
  const [contextMenu, setContextMenu] = useState<{ widgetId: string; x: number; y: number } | null>(null);

  function handleAddWidget(type: WidgetConfig['type']) {
    const typeInfo = WIDGET_TYPES.find(t => t.type === type);
    const widget = createWidget(type, typeInfo?.label || type);
    const newLayout = {
      ...layout,
      widgets: [...layout.widgets, widget],
    };
    setLayout(newLayout);
    saveLayout(newLayout);
    setShowAddMenu(false);
  }

  function handleRemoveWidget(widgetId: string) {
    const newLayout = {
      ...layout,
      widgets: layout.widgets.filter(w => w.id !== widgetId),
    };
    setLayout(newLayout);
    saveLayout(newLayout);
    setContextMenu(null);
  }

  function handleEditWidget(widgetId: string) {
    setEditingWidget(editingWidget === widgetId ? null : widgetId);
    setContextMenu(null);
  }

  function handleUpdateWidget(widgetId: string, updates: Partial<WidgetConfig>) {
    const newLayout = {
      ...layout,
      widgets: layout.widgets.map(w =>
        w.id === widgetId ? { ...w, ...updates } : w
      ),
    };
    setLayout(newLayout);
    saveLayout(newLayout);
  }

  function handleContextMenu(e: MouseEvent, widgetId: string) {
    e.preventDefault();
    setContextMenu({ widgetId, x: e.clientX, y: e.clientY });
  }

  function renderWidget(widget: WidgetConfig) {
    switch (widget.type) {
      case 'button-grid':
        return h(ButtonGridWidget, { settings: widget.settings });
      case 'event-log':
        return h(EventLogWidget, { events, settings: widget.settings });
      case 'system-monitor':
        return h(SystemMonitorWidget, { settings: widget.settings });
      case 'clock':
        return h(ClockWidget, { settings: widget.settings });
      case 'actions':
        return h(ActionsWidget, { settings: widget.settings });
      default:
        return h('div', { class: 'widget-unknown' }, 'Unknown widget type');
    }
  }

  return h('div', { class: 'widget-grid-container' },
    h('div', { class: 'widget-grid-header' },
      h('h2', null, 'Dashboard'),
      h('div', { class: 'widget-grid-actions' },
        h('button', {
          class: 'add-widget-btn',
          onClick: () => setShowAddMenu(!showAddMenu),
        }, showAddMenu ? 'Cancel' : '+ Add Widget')
      )
    ),

    showAddMenu && h('div', { class: 'widget-picker' },
      h('div', { class: 'widget-picker-title' }, 'Add Widget'),
      h('div', { class: 'widget-picker-grid' },
        WIDGET_TYPES.map(wt =>
          h('button', {
            class: 'widget-picker-item',
            key: wt.type,
            onClick: () => handleAddWidget(wt.type),
          },
            h('span', { class: 'widget-picker-icon' }, wt.icon),
            h('span', { class: 'widget-picker-label' }, wt.label)
          )
        )
      )
    ),

    h('div', {
      class: 'widget-grid',
      style: { gridTemplateColumns: `repeat(${layout.columns}, 1fr)` },
    },
      layout.widgets.map(widget =>
        h('div', {
          class: `widget-cell ${editingWidget === widget.id ? 'editing' : ''}`,
          key: widget.id,
          style: {
            gridColumn: `span ${widget.colSpan}`,
            gridRow: `span ${widget.rowSpan}`,
          },
          onContextMenu: (e: MouseEvent) => handleContextMenu(e, widget.id),
        },
          h('div', { class: 'widget-header' },
            h('span', { class: 'widget-title' }, widget.title),
            h('div', { class: 'widget-controls' },
              h('button', {
                class: 'widget-control-btn edit',
                onClick: () => handleEditWidget(widget.id),
                title: 'Edit',
              }, '[E]'),
              h('button', {
                class: 'widget-control-btn remove',
                onClick: () => handleRemoveWidget(widget.id),
                title: 'Remove',
              }, '[X]')
            )
          ),
          editingWidget === widget.id && h('div', { class: 'widget-editor' },
            h('div', { class: 'editor-field' },
              h('label', null, 'Title'),
              h('input', {
                type: 'text',
                value: widget.title,
                onInput: (e: Event) => handleUpdateWidget(widget.id, { title: (e.target as HTMLInputElement).value }),
              })
            ),
            h('div', { class: 'editor-field' },
              h('label', null, 'Width (columns)'),
              h('input', {
                type: 'number',
                min: '1',
                max: `${layout.columns}`,
                value: widget.colSpan.toString(),
                onInput: (e: Event) => handleUpdateWidget(widget.id, { colSpan: parseInt((e.target as HTMLInputElement).value) || 1 }),
              })
            ),
            h('div', { class: 'editor-field' },
              h('label', null, 'Height (rows)'),
              h('input', {
                type: 'number',
                min: '1',
                max: '4',
                value: widget.rowSpan.toString(),
                onInput: (e: Event) => handleUpdateWidget(widget.id, { rowSpan: parseInt((e.target as HTMLInputElement).value) || 1 }),
              })
            )
          ),
          h('div', { class: 'widget-content' },
            renderWidget(widget)
          )
        )
      )
    ),

    contextMenu && h('div', {
      class: 'context-menu',
      style: { left: `${contextMenu.x}px`, top: `${contextMenu.y}px` },
      onClick: () => setContextMenu(null),
    },
      h('button', {
        class: 'context-menu-item',
        onClick: () => handleEditWidget(contextMenu.widgetId),
      }, 'Edit Widget'),
      h('button', {
        class: 'context-menu-item delete',
        onClick: () => handleRemoveWidget(contextMenu.widgetId),
      }, 'Remove Widget')
    )
  );
}
