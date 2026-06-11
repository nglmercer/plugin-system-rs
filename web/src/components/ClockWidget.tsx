import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';

interface ClockProps {
  settings: Record<string, any>;
}

export function ClockWidget({ settings }: ClockProps) {
  const [time, setTime] = useState(new Date());

  useEffect(() => {
    const interval = setInterval(() => setTime(new Date()), 1000);
    return () => clearInterval(interval);
  }, []);

  function formatTime(date: Date): string {
    const hours = date.getHours().toString().padStart(2, '0');
    const minutes = date.getMinutes().toString().padStart(2, '0');
    const seconds = date.getSeconds().toString().padStart(2, '0');
    return `${hours}:${minutes}:${seconds}`;
  }

  function formatDate(date: Date): string {
    const options: Intl.DateTimeFormatOptions = {
      weekday: 'short',
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    };
    return date.toLocaleDateString(undefined, options);
  }

  return h('div', { class: 'widget-clock' },
    h('div', { class: 'clock-time' }, formatTime(time)),
    h('div', { class: 'clock-date' }, formatDate(time))
  );
}
