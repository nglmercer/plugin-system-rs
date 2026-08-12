import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { Dashboard } from './routes/Dashboard';
import { Profiles } from './routes/Profiles';
import { Plugins } from './routes/Plugins';
import { FabMenu } from './components/FabMenu';
import type { Page } from './components/FabMenu';

/**
 * App shell: the page under test and the floating menu above it.
 *
 * Navigation state lives here because the URL and the FAB menu both touch it;
 * arrange state lives here for the same reason — the FAB menu toggles it and
 * the dashboard consumes it, and neither should know about the other.
 */

function getPageFromURL(): Page {
  const params = new URLSearchParams(window.location.search);
  const page = params.get('page');
  if (page === 'dashboard' || page === 'profiles' || page === 'plugins') {
    return page;
  }
  return 'dashboard';
}

function setPageToURL(page: Page) {
  const url = new URL(window.location.href);
  url.searchParams.set('page', page);
  window.history.replaceState({}, '', url.toString());
}

export function App() {
  const [currentPage, setCurrentPage] = useState<Page>(getPageFromURL);
  const [arranging, setArranging] = useState(false);

  useEffect(() => {
    function handlePopState() {
      setCurrentPage(getPageFromURL());
    }
    window.addEventListener('popstate', handlePopState);
    return () => window.removeEventListener('popstate', handlePopState);
  }, []);

  function navigateTo(page: Page) {
    setCurrentPage(page);
    setPageToURL(page);
    // Arrangement belongs to the dashboard; leaving cancels it.
    if (page !== 'dashboard') setArranging(false);
  }

  function handleAddWidget() {
    window.dispatchEvent(new CustomEvent('sd:add-widget'));
  }

  return h('div', { class: 'app' },
    h('main', { class: 'main' },
      currentPage === 'dashboard' && h(Dashboard, {
        arranging,
        onToggleArrange: () => setArranging((v) => !v),
      }),
      currentPage === 'profiles' && h(Profiles, null),
      currentPage === 'plugins' && h(Plugins, null),
    ),
    h(FabMenu, {
      page: currentPage,
      onNavigate: navigateTo,
      onAddWidget: handleAddWidget,
      arranging,
      onToggleArrange: () => setArranging((v) => !v),
    }),
  );
}
