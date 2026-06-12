import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { Dashboard } from './routes/Dashboard';
import { Profiles } from './routes/Profiles';
import { Plugins } from './routes/Plugins';
import { QrButton } from './components/QrModal';
import { Icons } from './components/Icons';

export function App() {
  const [currentPage, setCurrentPage] = useState<'dashboard' | 'profiles' | 'plugins'>('dashboard');
  const [theme, setTheme] = useState<'dark' | 'light'>(() => {
    return (localStorage.getItem('theme') as 'dark' | 'light') || 'dark';
  });

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('theme', theme);
  }, [theme]);

  function toggleTheme() {
    setTheme(t => t === 'dark' ? 'light' : 'dark');
  }

  return h('div', { class: 'app' },
    h('nav', { class: 'nav' },
      h('h1', null, 'StreamDeck'),
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
        h('button', {
          class: 'nav-theme-btn',
          onClick: toggleTheme,
          title: `Switch to ${theme === 'dark' ? 'light' : 'dark'} theme`,
        }, theme === 'dark' ? h(Icons.sun, null) : h(Icons.moon, null)),
        h(QrButton, null)
      )
    ),
    h('main', { class: 'main' },
      currentPage === 'dashboard' && h(Dashboard, null),
      currentPage === 'profiles' && h(Profiles, null),
      currentPage === 'plugins' && h(Plugins, null),
    )
  );
}
