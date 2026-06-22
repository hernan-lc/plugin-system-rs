import { h } from 'preact';
import { useState, useEffect, useCallback } from 'preact/hooks';
import {
  fetchPlugins,
  refreshPlugins,
  setPluginEnabled,
  uploadPluginFile,
  updatePluginFile,
  uninstallPlugin,
} from '../lib/api';
import type { PluginStatus } from '../lib/types';

type PluginsTab = 'installed' | 'install';

function getTabFromURL(): PluginsTab {
  const params = new URLSearchParams(window.location.search);
  const tab = params.get('tab');
  if (tab === 'install') return 'install';
  return 'installed';
}

function setTabToURL(tab: PluginsTab) {
  const url = new URL(window.location.href);
  url.searchParams.set('tab', tab);
  window.history.replaceState({}, '', url.toString());
}

export function Plugins() {
  const [plugins, setPlugins] = useState<PluginStatus[]>([]);
  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<PluginsTab>(getTabFromURL);
  const [toggling, setToggling] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const [updating, setUpdating] = useState<string | null>(null);
  const [uninstalling, setUninstalling] = useState<string | null>(null);
  const [installFile, setInstallFile] = useState<File | null>(null);
  const [installEnabled, setInstallEnabled] = useState(true);
  const [updateFiles, setUpdateFiles] = useState<Record<string, File>>({});
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleTabChange = useCallback((tab: PluginsTab) => {
    setActiveTab(tab);
    setTabToURL(tab);
  }, []);

  useEffect(() => {
    function handlePopState() {
      setActiveTab(getTabFromURL());
    }
    window.addEventListener('popstate', handlePopState);
    return () => window.removeEventListener('popstate', handlePopState);
  }, []);

  useEffect(() => {
    loadPlugins();
  }, []);

  async function loadPlugins() {
    setLoading(true);
    setError(null);
    try {
      const plugins = await fetchPlugins();
      setPlugins(plugins);
    } catch (error) {
      setError(toErrorMessage(error));
    } finally {
      setLoading(false);
    }
  }

  async function handleRefresh() {
    setLoading(true);
    setMessage(null);
    setError(null);
    try {
      const plugins = await refreshPlugins();
      setPlugins(plugins);
      setMessage('Plugins refreshed');
    } catch (error) {
      setError(toErrorMessage(error));
    } finally {
      setLoading(false);
    }
  }

  async function handleToggle(plugin: PluginStatus) {
    setToggling(plugin.name);
    try {
      await setPluginEnabled(plugin.name, !plugin.enabled);
      await loadPlugins();
    } catch (error) {
      setError(toErrorMessage(error));
    } finally {
      setToggling(null);
    }
  }

  function handleInstallFile(event: Event) {
    const input = event.target as HTMLInputElement;
    setInstallFile(input.files?.[0] ?? null);
  }

  async function handleInstall() {
    if (!installFile) {
      return;
    }

    setInstalling(true);
    setMessage(null);
    setError(null);
    try {
      await uploadPluginFile(installFile, installEnabled);
      setInstallFile(null);
      setMessage('Plugin installed');
      await loadPlugins();
    } catch (error) {
      setError(toErrorMessage(error));
    } finally {
      setInstalling(false);
    }
  }

  function handleUpdateFile(pluginName: string, event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) {
      return;
    }

    setUpdateFiles(current => ({
      ...current,
      [pluginName]: file,
    }));
  }

  async function handleUpdate(plugin: PluginStatus) {
    const file = updateFiles[plugin.name];
    if (!file) {
      return;
    }

    setUpdating(plugin.name);
    setMessage(null);
    setError(null);
    try {
      await updatePluginFile(plugin.name, file, plugin.enabled);
      const nextFiles = { ...updateFiles };
      delete nextFiles[plugin.name];
      setUpdateFiles(nextFiles);
      setMessage('Plugin updated');
      await loadPlugins();
    } catch (error) {
      setError(toErrorMessage(error));
    } finally {
      setUpdating(null);
    }
  }

  async function handleUninstall(plugin: PluginStatus) {
    if (!confirm(`Are you sure you want to uninstall plugin "${plugin.name}"? This will delete the plugin file.`)) {
      return;
    }

    setUninstalling(plugin.name);
    setMessage(null);
    setError(null);
    try {
      await uninstallPlugin(plugin.name);
      setMessage('Plugin uninstalled');
      await loadPlugins();
    } catch (error) {
      setError(toErrorMessage(error));
    } finally {
      setUninstalling(null);
    }
  }

  function statusLabel(plugin: PluginStatus) {
    if (plugin.loaded) {
      return 'Loaded';
    }
    return plugin.enabled ? 'Not loaded' : 'Disabled';
  }

  function fileName(path: string) {
    return path.split(/[\\/]/).pop() ?? path;
  }

  const enabledCount = plugins.filter(p => p.enabled).length;
  const loadedCount = plugins.filter(p => p.loaded).length;

  return h('div', { class: 'plugins-page' },
    h('section', { class: 'plugin-actions' },
      h('div', null,
        h('h2', null, 'Plugins'),
        h('p', { class: 'page-help' }, 'Install, update, enable, or disable native plugins.')
      ),
      h('button', {
        class: 'reload-btn',
        onClick: handleRefresh,
        disabled: loading || installing || updating !== null || toggling !== null,
      }, loading ? 'Loading...' : 'Refresh')
    ),

    h('div', { class: 'plugin-stats' },
      h('div', { class: 'plugin-stat-card' },
        h('span', { class: 'plugin-stat-value' }, String(plugins.length)),
        h('span', { class: 'plugin-stat-label' }, 'Total')
      ),
      h('div', { class: 'plugin-stat-card' },
        h('span', { class: 'plugin-stat-value accent' }, String(enabledCount)),
        h('span', { class: 'plugin-stat-label' }, 'Enabled')
      ),
      h('div', { class: 'plugin-stat-card' },
        h('span', { class: 'plugin-stat-value success' }, String(loadedCount)),
        h('span', { class: 'plugin-stat-label' }, 'Loaded')
      )
    ),

    message ? h('div', { class: 'plugin-message' }, message) : null,
    error ? h('div', { class: 'plugin-error' }, error) : null,

    h('div', { class: 'plugin-tabs' },
      h('button', {
        class: `plugin-tab ${activeTab === 'installed' ? 'active' : ''}`,
        onClick: () => handleTabChange('installed'),
      }, 'Installed'),
      h('button', {
        class: `plugin-tab ${activeTab === 'install' ? 'active' : ''}`,
        onClick: () => handleTabChange('install'),
      }, 'Install')
    ),

    activeTab === 'install' && h('section', { class: 'plugin-upload-card' },
      h('h3', null, 'Install plugin'),
      h('div', { class: 'plugin-upload-row' },
        h('input', {
          type: 'file',
          accept: pluginAccept(),
          onChange: handleInstallFile,
        }),
        h('label', { class: 'inline-checkbox' },
          h('input', {
            type: 'checkbox',
            checked: installEnabled,
            onChange: () => setInstallEnabled(!installEnabled),
          }),
          'Enable after install'
        ),
        h('button', {
          class: 'primary-btn',
          onClick: handleInstall,
          disabled: installing || !installFile,
        }, installing ? 'Installing...' : 'Install')
      ),
      installFile ? h('p', { class: 'plugin-file-name' }, installFile.name) : null
    ),

    activeTab === 'installed' && h('div', { class: 'plugin-list' },
      plugins.length === 0
        ? h('div', { class: 'plugin-empty' },
            h('p', { class: 'plugin-empty-title' }, 'No plugins installed'),
            h('p', { class: 'plugin-empty-hint' }, 'Install a .so/.dll/.dylib plugin file or place one in the plugin directory.')
          )
        : plugins.map(plugin =>
            h('div', { class: `plugin-item ${plugin.loaded ? 'loaded' : ''} ${!plugin.enabled ? 'disabled' : ''}`, key: plugin.name },
              h('div', { class: 'plugin-header' },
                h('div', { class: 'plugin-title' },
                  h('span', { class: 'plugin-name' }, plugin.name),
                  plugin.version
                    ? h('span', { class: 'plugin-version' }, `v${plugin.version}`)
                    : null
                ),
                h('div', { class: 'plugin-controls' },
                  h('label', { class: 'switch' },
                    h('input', {
                      type: 'checkbox',
                      checked: plugin.enabled,
                      disabled: toggling !== null || installing || updating !== null,
                      onChange: () => handleToggle(plugin),
                    }),
                    h('span', { class: 'slider' })
                  ),
                  h('input', {
                    id: `plugin-update-${plugin.name}`,
                    type: 'file',
                    accept: pluginAccept(),
                    style: { display: 'none' },
                    onChange: event => handleUpdateFile(plugin.name, event),
                  }),
                  h('button', {
                    class: 'secondary-btn',
                    onClick: () => {
                      const input = document.getElementById(`plugin-update-${plugin.name}`);
                      input?.click();
                    },
                    disabled: installing || updating !== null || toggling !== null || uninstalling !== null,
                  }, updating === plugin.name ? 'Updating...' : 'Update'),
                  h('button', {
                    class: 'danger-btn',
                    onClick: () => handleUninstall(plugin),
                    disabled: installing || updating !== null || toggling !== null || uninstalling !== null,
                  }, uninstalling === plugin.name ? 'Removing...' : 'Remove')
                )
              ),
              h('div', { class: 'plugin-meta' },
                h('span', {
                  class: `plugin-badge ${plugin.loaded ? 'badge-loaded' : 'badge-error'}`,
                }, statusLabel(plugin)),
                h('span', { class: 'plugin-path' }, fileName(plugin.path)),
                updateFiles[plugin.name]
                  ? h('span', { class: 'plugin-file-name' }, updateFiles[plugin.name].name)
                  : null
              )
            )
          )
    )
  );
}

function pluginAccept() {
  return '.dll,.so,.dylib';
}

function toErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
