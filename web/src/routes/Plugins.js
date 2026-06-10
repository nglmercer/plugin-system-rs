import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { fetchPlugins, reloadPlugins } from '../lib/api';
export function Plugins() {
    const [plugins, setPlugins] = useState([]);
    const [loading, setLoading] = useState(false);
    useEffect(() => {
        loadPlugins();
    }, []);
    async function loadPlugins() {
        const plugins = await fetchPlugins();
        setPlugins(plugins);
    }
    async function handleReload() {
        setLoading(true);
        await reloadPlugins();
        await loadPlugins();
        setLoading(false);
    }
    return h('div', { class: 'plugins-page' }, h('h2', null, 'Plugins'), h('button', {
        class: 'reload-btn',
        onClick: handleReload,
        disabled: loading,
    }, loading ? 'Reloading...' : 'Reload Plugins'), h('div', { class: 'plugin-list' }, plugins.length === 0
        ? h('p', null, 'No plugins loaded. Place .so files in ./plugins directory.')
        : plugins.map(plugin => h('div', { class: 'plugin-item', key: plugin }, h('span', null, plugin)))));
}
