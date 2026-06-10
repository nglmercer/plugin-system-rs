import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { Dashboard } from './routes/Dashboard';
import { Profiles } from './routes/Profiles';
import { Plugins } from './routes/Plugins';
import { wsClient } from './lib/websocket';
export function App() {
    const [currentPage, setCurrentPage] = useState('dashboard');
    const [events, setEvents] = useState([]);
    useEffect(() => {
        wsClient.connect();
        const unsubscribe = wsClient.onEvent((event) => {
            setEvents(prev => [event, ...prev].slice(0, 50));
            console.log('Event:', event);
        });
        return () => {
            unsubscribe();
            wsClient.disconnect();
        };
    }, []);
    return h('div', { class: 'app' }, h('nav', { class: 'nav' }, h('h1', null, 'StreamDeck Core'), h('div', { class: 'nav-links' }, h('button', {
        class: currentPage === 'dashboard' ? 'active' : '',
        onClick: () => setCurrentPage('dashboard')
    }, 'Dashboard'), h('button', {
        class: currentPage === 'profiles' ? 'active' : '',
        onClick: () => setCurrentPage('profiles')
    }, 'Profiles'), h('button', {
        class: currentPage === 'plugins' ? 'active' : '',
        onClick: () => setCurrentPage('plugins')
    }, 'Plugins'))), h('main', { class: 'main' }, currentPage === 'dashboard' && h(Dashboard, { events }), currentPage === 'profiles' && h(Profiles, null), currentPage === 'plugins' && h(Plugins, null)));
}
