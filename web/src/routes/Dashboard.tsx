import { h } from 'preact';
import { WidgetGrid } from '../components/deck/WidgetGrid';

export function Dashboard({
  arranging,
  onToggleArrange,
}: {
  arranging: boolean;
  onToggleArrange: () => void;
}) {
  return h(WidgetGrid, { arranging, onToggleArrange });
}
