import { h, render } from 'preact';
import { App } from './App';
import { initI18n } from './lib/i18n';
import './styles/theme.css';
import './styles/base.css';
import './styles/dashboard.css';
import './styles/library.css';
import './styles/widgets.css';
import './styles/wizard.css';
import './styles/pages.css';

initI18n().then(() => {
  render(h(App, null), document.getElementById('app')!);
});
