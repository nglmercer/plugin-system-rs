import { h, render } from 'preact';
import { App } from './App';
import './styles/theme.css';
import './styles/base.css';
import './styles/dashboard.css';
import './styles/library.css';
import './styles/widgets.css';
import './styles/wizard.css';
import './styles/pages.css';

render(h(App, null), document.getElementById('app')!);
