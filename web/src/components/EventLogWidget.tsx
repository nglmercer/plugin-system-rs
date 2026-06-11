import { h } from 'preact';
import { StreamEvent } from '../lib/types';

interface EventLogProps {
  events: StreamEvent[];
  settings: Record<string, any>;
}

export function EventLogWidget({ events, settings }: EventLogProps) {
  const maxEvents = settings.maxEvents || 20;

  function formatEventTime(timestamp?: number): string {
    if (!timestamp) return '';
    const date = new Date(timestamp * 1000);
    return date.toLocaleTimeString();
  }

  function getEventIcon(type: string): string {
    switch (type) {
      case 'button_pressed': return '[>]';
      case 'button_released': return '[ ]';
      case 'profile_changed': return '[P]';
      case 'action_executed': return '[A]';
      case 'plugin_loaded': return '[+]';
      case 'plugin_unloaded': return '[-]';
      case 'device_connected': return '[D]';
      case 'device_disconnected': return '[d]';
      default: return '[?]';
    }
  }

  return h('div', { class: 'widget-event-log' },
    h('div', { class: 'events-list' },
      events.length === 0
        ? h('div', { class: 'events-empty' }, 'No events yet')
        : events.slice(0, maxEvents).map((event, i) =>
            h('div', { class: 'event-item', key: i },
              h('div', { class: 'event-header' },
                h('span', { class: 'event-icon' }, getEventIcon(event.type)),
                h('span', { class: 'event-type' }, event.type || 'unknown'),
                h('span', { class: 'event-time' }, formatEventTime(event.timestamp))
              ),
              h('pre', { class: 'event-data' }, JSON.stringify(event, null, 2))
            )
          )
    )
  );
}
