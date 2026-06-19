import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { fetchPlugins, setPluginEnabled } from '../lib/api';
import type { PluginStatus } from '../lib/types';

export function Plugins() {
  const [plugins, setPlugins] = useState<PluginStatus[]>([]);
  const [loading, setLoading] = useState(false);
  const [toggling, setToggling] = useState<string | null>(null);

  useEffect(() => {
    loadPlugins();
  }, []);

  async function loadPlugins() {
    setLoading(true);
    try {
      const plugins = await fetchPlugins();
      setPlugins(plugins);
    } catch (error) {
      console.error('Failed to load plugins:', error);
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
      console.error('Failed to toggle plugin:', error);
    } finally {
      setToggling(null);
    }
  }

  return h('div', { class: 'plugins-page' },
    h('h2', null, 'Plugins'),
    h('button', {
      class: 'reload-btn',
      onClick: loadPlugins,
      disabled: loading || toggling !== null,
    }, loading ? 'Loading...' : 'Reload'),
    h('div', { class: 'plugin-list' },
      plugins.length === 0
        ? h('p', null, 'No plugins found. Place .so/.dll files in ./plugins directory.')
        : plugins.map(plugin =>
            h('div', { class: 'plugin-item', key: plugin.name },
              h('div', { class: 'plugin-header' },
                h('span', { class: 'plugin-name' }, plugin.name),
                plugin.version
                  ? h('span', { class: 'plugin-version' }, `v${plugin.version}`)
                  : null,
                h('label', { class: 'switch' },
                  h('input', {
                    type: 'checkbox',
                    checked: plugin.enabled,
                    disabled: toggling !== null,
                    onChange: () => handleToggle(plugin),
                  }),
                  h('span', { class: 'slider' })
                )
              ),
              h('div', { class: 'plugin-meta' },
                h('span', {
                  class: `plugin-status ${plugin.loaded ? 'loaded' : 'not-loaded'
                  }`,
                }, plugin.loaded ? 'Loaded' : 'Disabled'),
                h('span', { class: 'plugin-path' }, plugin.path.split(/[\\/]/).pop() ?? plugin.path)
              )
            )
          )
    )
  );
}
