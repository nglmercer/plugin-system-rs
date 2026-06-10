import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { fetchProfiles, createProfile, deleteProfile } from '../lib/api';
export function Profiles() {
    const [profiles, setProfiles] = useState([]);
    const [newProfileName, setNewProfileName] = useState('');
    useEffect(() => {
        loadProfiles();
    }, []);
    async function loadProfiles() {
        const profiles = await fetchProfiles();
        setProfiles(profiles);
    }
    async function handleCreate() {
        if (newProfileName.trim()) {
            await createProfile(newProfileName.trim());
            setNewProfileName('');
            await loadProfiles();
        }
    }
    async function handleDelete(id) {
        await deleteProfile(id);
        await loadProfiles();
    }
    return h('div', { class: 'profiles-page' }, h('h2', null, 'Profiles'), h('div', { class: 'create-profile' }, h('input', {
        type: 'text',
        placeholder: 'New profile name',
        value: newProfileName,
        onInput: (e) => setNewProfileName(e.target.value),
    }), h('button', { onClick: handleCreate }, 'Create Profile')), h('div', { class: 'profile-list' }, profiles.map(profile => h('div', { class: 'profile-item', key: profile.id }, h('div', { class: 'profile-info' }, h('h3', null, profile.name), h('span', null, `${profile.pages.length} page(s), ${profile.pages[0]?.buttons.length || 0} buttons`)), h('button', {
        class: 'delete-btn',
        onClick: () => handleDelete(profile.id),
    }, 'Delete')))));
}
