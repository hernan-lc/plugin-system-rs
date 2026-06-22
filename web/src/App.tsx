import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { Dashboard } from './routes/Dashboard';
import { Profiles } from './routes/Profiles';
import { Plugins } from './routes/Plugins';
import { QrButton } from './components/QrModal';
import { Icons } from './components/Icons';
import { t, getLocale, setLocale, getAvailableLocales } from './lib/i18n';

type Page = 'dashboard' | 'profiles' | 'plugins';

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
  const [theme, setTheme] = useState<'dark' | 'light'>(() => {
    return (localStorage.getItem('theme') as 'dark' | 'light') || 'dark';
  });
  const [menuOpen, setMenuOpen] = useState(false);
  const [locale, setLocaleState] = useState(getLocale());

  useEffect(() => {
    function handlePopState() {
      setCurrentPage(getPageFromURL());
    }
    window.addEventListener('popstate', handlePopState);
    return () => window.removeEventListener('popstate', handlePopState);
  }, []);

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('theme', theme);
  }, [theme]);

  useEffect(() => {
    if (menuOpen) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => { document.body.style.overflow = ''; };
  }, [menuOpen]);

  function toggleTheme() {
    setTheme(t => t === 'dark' ? 'light' : 'dark');
  }

  function navigateTo(page: Page) {
    setCurrentPage(page);
    setPageToURL(page);
    setMenuOpen(false);
  }

  function handleAddWidget() {
    setMenuOpen(false);
    window.dispatchEvent(new CustomEvent('sd:add-widget'));
  }

  async function handleLocaleChange(newLocale: string) {
    await setLocale(newLocale);
    setLocaleState(newLocale);
  }

  return h('div', { class: 'app' },
    h('main', { class: 'main' },
      currentPage === 'dashboard' && h(Dashboard, null),
      currentPage === 'profiles' && h(Profiles, null),
      currentPage === 'plugins' && h(Plugins, null),
    ),
    menuOpen && h('div', { class: 'fab-overlay', onClick: () => setMenuOpen(false) }),
    h('div', { class: `fab-menu ${menuOpen ? 'open' : ''}` },
      h('button', {
        class: currentPage === 'dashboard' ? 'active' : '',
        onClick: () => navigateTo('dashboard')
      }, h(Icons.dashboard, null), t('nav.dashboard')),
      h('button', {
        class: currentPage === 'profiles' ? 'active' : '',
        onClick: () => navigateTo('profiles')
      }, h(Icons.profiles, null), t('nav.profiles')),
      h('button', {
        class: currentPage === 'plugins' ? 'active' : '',
        onClick: () => navigateTo('plugins')
      }, h(Icons.plugins, null), t('nav.plugins')),
      h('div', { class: 'fab-divider' }),
      h('button', { class: 'fab-add-btn', onClick: handleAddWidget },
        h(Icons.plus, null), t('dashboard.addWidget')
      ),
      h('div', { class: 'fab-divider' }),
      h('div', { class: 'fab-bottom-row' },
        h('button', {
          class: 'fab-theme-btn',
          onClick: toggleTheme,
          title: t('theme.switchTo', { theme: theme === 'dark' ? t('theme.light') : t('theme.dark') }),
        }, theme === 'dark' ? h(Icons.sun, null) : h(Icons.moon, null)),
        h('div', { class: 'fab-lang-selector' },
          h('select', {
            class: 'lang-select',
            value: locale,
            onChange: (e: Event) => handleLocaleChange((e.target as HTMLSelectElement).value),
          },
            getAvailableLocales().map((loc) =>
              h('option', { key: loc, value: loc }, loc.toUpperCase())
            )
          )
        ),
        h(QrButton, null)
      )
    ),
    h('button', {
      class: `fab-burger ${menuOpen ? 'open' : ''}`,
      onClick: () => setMenuOpen(!menuOpen),
      'aria-label': 'Menu',
    },
      h('span', null),
      h('span', null),
      h('span', null),
    )
  );
}
