import { h } from 'preact';
import { StreamEvent } from '../lib/types';
import { WidgetGrid } from '../components/WidgetGrid';

interface DashboardProps {
  events: StreamEvent[];
}

export function Dashboard({ events }: DashboardProps) {
  return h(WidgetGrid, { events });
}
