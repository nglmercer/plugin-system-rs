import { h } from 'preact';
import { useState } from 'preact/hooks';
import { Dashboard } from './routes/Dashboard';
import { Profiles } from './routes/Profiles';
import { Plugins } from './routes/Plugins';

export function App() {
  const [currentPage, setCurrentPage] = useState<'dashboard' | 'profiles' | 'plugins'>('dashboard');

  return h('div', { class: 'app' },
    h('nav', { class: 'nav' },
      h('h1', null, 'StreamDeck Core'),
      h('div', { class: 'nav-links' },
        h('button', {
          class: currentPage === 'dashboard' ? 'active' : '',
          onClick: () => setCurrentPage('dashboard')
        }, 'Dashboard'),
        h('button', {
          class: currentPage === 'profiles' ? 'active' : '',
          onClick: () => setCurrentPage('profiles')
        }, 'Profiles'),
        h('button', {
          class: currentPage === 'plugins' ? 'active' : '',
          onClick: () => setCurrentPage('plugins')
        }, 'Plugins'),
      )
    ),
    h('main', { class: 'main' },
      currentPage === 'dashboard' && h(Dashboard, null),
      currentPage === 'profiles' && h(Profiles, null),
      currentPage === 'plugins' && h(Plugins, null),
    )
  );
}
