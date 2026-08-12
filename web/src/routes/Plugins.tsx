import { h } from 'preact';
import { useState, useEffect, useCallback } from 'preact/hooks';
import {
  fetchPlugins,
  refreshPlugins,
  setPluginEnabled,
  uploadPluginFile,
  updatePluginFile,
  uninstallPlugin,
  readPluginManifest,
} from '../lib/api';
import type { PluginManifestUpload } from '../lib/api';
import type { PluginStatus } from '../lib/types';
import { FormTabs, FormToggle } from '../components/FormComponents';
import './plugins.css';

/**
 * Ask before handing a plugin any host capability.
 *
 * The server refuses to grant a capability that was not acknowledged by name,
 * and this is the acknowledgement: the user is told exactly what the plugin is
 * asking for before anything is written to disk. Declining is the default.
 */
function confirmCapabilities(manifest: PluginManifestUpload | null): boolean {
  if (!manifest || manifest.capabilities.length === 0) return true;

  const described = manifest.capabilities
    .map(capability => `  • ${capability}${CAPABILITY_WARNINGS[capability] ?? ''}`)
    .join('\n');

  return confirm(
    `This plugin requests the following host capabilities:\n\n${described}\n\n` +
      'Only continue if you trust the source of this plugin. Grant them?',
  );
}

const CAPABILITY_WARNINGS: Record<string, string> = {
  input: ' — can press keys and see everything you type',
  audio: ' — can read and change system and per-app volume',
  websocket: ' — can open outbound network connections',
  'system-info': ' — can read CPU, memory and process counts',
};

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
  const [installManifest, setInstallManifest] = useState<PluginManifestUpload | null>(null);
  const [installEnabled, setInstallEnabled] = useState(true);
  const [updateFiles, setUpdateFiles] = useState<Record<string, File>>({});
  const [updateManifests, setUpdateManifests] = useState<Record<string, PluginManifestUpload>>({});
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

  async function handleInstallManifest(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) {
      setInstallManifest(null);
      return;
    }

    setError(null);
    try {
      setInstallManifest(await readPluginManifest(file));
    } catch (error) {
      setInstallManifest(null);
      setError(toErrorMessage(error));
    }
  }

  async function handleInstall() {
    if (!installFile) {
      return;
    }

    if (!confirmCapabilities(installManifest)) {
      return;
    }

    setInstalling(true);
    setMessage(null);
    setError(null);
    try {
      await uploadPluginFile(installFile, installEnabled, installManifest ?? undefined);
      setInstallFile(null);
      setInstallManifest(null);
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

  async function handleUpdateManifest(pluginName: string, event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) {
      return;
    }

    setError(null);
    try {
      const manifest = await readPluginManifest(file);
      setUpdateManifests(current => ({ ...current, [pluginName]: manifest }));
    } catch (error) {
      setError(toErrorMessage(error));
    }
  }

  async function handleUpdate(plugin: PluginStatus) {
    const file = updateFiles[plugin.name];
    if (!file) {
      return;
    }

    const manifest = updateManifests[plugin.name] ?? null;
    // An update is a new binary with new grants, so it is confirmed the same
    // way an install is — inheriting the old plugin's capabilities silently
    // would make "update" the way around this prompt.
    if (!confirmCapabilities(manifest)) {
      return;
    }

    setUpdating(plugin.name);
    setMessage(null);
    setError(null);
    try {
      await updatePluginFile(plugin.name, file, plugin.enabled, manifest ?? undefined);
      const nextFiles = { ...updateFiles };
      delete nextFiles[plugin.name];
      setUpdateFiles(nextFiles);
      const nextManifests = { ...updateManifests };
      delete nextManifests[plugin.name];
      setUpdateManifests(nextManifests);
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
        h('p', { class: 'page-help' }, 'Install, update, enable, or disable WebAssembly plugins.')
      ),
      h('button', {
        class: 'btn btn-primary',
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

    h(FormTabs, {
      tabs: [
        { value: 'installed', label: 'Installed' },
        { value: 'install', label: 'Install' },
      ],
      active: activeTab,
      onChange: (tab) => handleTabChange(tab as PluginsTab),
    }),

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
          class: 'btn btn-primary',
          onClick: handleInstall,
          disabled: installing || !installFile,
        }, installing ? 'Installing...' : 'Install')
      ),
      h('div', { class: 'plugin-upload-row' },
        h('label', { class: 'plugin-manifest-label' }, 'Manifest (optional)'),
        h('input', {
          type: 'file',
          accept: manifestAccept(),
          onChange: handleInstallManifest,
        })
      ),
      h('p', { class: 'page-help' },
        'The manifest declares the host capabilities the plugin may use. Without one, ' +
        'the plugin is installed with no capabilities at all.'
      ),
      installFile ? h('p', { class: 'plugin-file-name' }, installFile.name) : null,
      installManifest && installManifest.capabilities.length > 0
        ? h('p', { class: 'plugin-capabilities' },
            `Requests: ${installManifest.capabilities.join(', ')}`)
        : null
    ),

    activeTab === 'installed' && h('div', { class: 'plugin-list' },
      plugins.length === 0
        ? h('div', { class: 'plugin-empty' },
            h('p', { class: 'plugin-empty-title' }, 'No plugins installed'),
            h('p', { class: 'plugin-empty-hint' }, 'Install a .wasm plugin component or place one in the plugin directory.')
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
                  h(FormToggle, {
                    checked: plugin.enabled,
                    disabled: toggling !== null || installing || updating !== null,
                    onChange: () => handleToggle(plugin),
                  }),
                  h('input', {
                    id: `plugin-update-${plugin.name}`,
                    type: 'file',
                    accept: pluginAccept(),
                    style: { display: 'none' },
                    onChange: event => handleUpdateFile(plugin.name, event),
                  }),
                  h('input', {
                    id: `plugin-update-manifest-${plugin.name}`,
                    type: 'file',
                    accept: manifestAccept(),
                    style: { display: 'none' },
                    onChange: event => handleUpdateManifest(plugin.name, event),
                  }),
                  h('button', {
                    class: 'btn btn-ghost btn-sm',
                    onClick: () => {
                      const input = document.getElementById(
                        `plugin-update-manifest-${plugin.name}`,
                      );
                      input?.click();
                    },
                    disabled: installing || updating !== null || toggling !== null || uninstalling !== null,
                  }, updateManifests[plugin.name] ? 'Manifest ✓' : 'Manifest'),
                  h('button', {
                    class: 'btn btn-ghost btn-sm',
                    onClick: () => {
                      const input = document.getElementById(`plugin-update-${plugin.name}`);
                      input?.click();
                    },
                    disabled: installing || updating !== null || toggling !== null || uninstalling !== null,
                  }, updating === plugin.name ? 'Updating...' : 'Update'),
                  h('button', {
                    class: 'btn btn-danger btn-sm',
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
  // Native shared libraries are no longer loadable — a plugin is a WASI
  // component. Offering the old extensions filtered out the only file the
  // server accepts.
  return '.wasm';
}

function manifestAccept() {
  return '.json';
}

function toErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
