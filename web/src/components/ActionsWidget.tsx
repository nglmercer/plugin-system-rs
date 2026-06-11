import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { fetchActions } from '../lib/api';

interface ActionsWidgetProps {
  settings: Record<string, any>;
}

export function ActionsWidget({ settings }: ActionsWidgetProps) {
  const [actions, setActions] = useState<string[]>([]);

  useEffect(() => {
    loadActions();
  }, []);

  async function loadActions() {
    try {
      const data = await fetchActions();
      setActions(data);
    } catch (e) {
      console.error('Failed to load actions:', e);
    }
  }

  function handleActionClick(action: string) {
    console.log('Action clicked:', action);
  }

  return h('div', { class: 'widget-actions' },
    actions.length === 0
      ? h('div', { class: 'actions-empty' }, 'No actions available')
      : h('div', { class: 'actions-list' },
          actions.map((action, i) =>
            h('button', {
              class: 'action-button',
              key: i,
              onClick: () => handleActionClick(action),
            }, action)
          )
        )
  );
}
