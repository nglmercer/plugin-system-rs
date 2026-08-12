import { h, render } from 'preact';
import { App } from './App';
import { TokenGate } from './components/TokenGate';
import { initI18n } from './lib/i18n';
import { initAuth } from './lib/auth';
import { initTheme } from './ui';
import { installExternalWidgetApi, registerBuiltinWidgets } from './widgets';
import './ui/tokens.css';
import './ui/themes/dark.css';
import './ui/themes/light.css';
import './ui/themes/accents.css';
import './styles/base.css';
import './styles/dashboard.css';
import './styles/deck.css';
import './styles/library.css';
import './styles/widgets.css';
import './styles/wizard.css';
import './styles/pages.css';

initTheme();

// Widgets register before anything renders, and the external door opens before
// any plugin script could run — a script that loaded first and found no
// `window.streamdeck` would simply give up.
registerBuiltinWidgets();
installExternalWidgetApi();

const root = document.getElementById('app')!;

function mountApp() {
  render(h(App, null), root);
}

// Auth first: every API call the app makes on mount needs the token attached,
// and `initAuth` is what installs the interceptor that does it.
Promise.all([initI18n(), initAuth()]).then(([, authenticated]) => {
  if (authenticated) {
    mountApp();
  } else {
    render(h(TokenGate, { onAuthenticated: mountApp }), root);
  }
});
