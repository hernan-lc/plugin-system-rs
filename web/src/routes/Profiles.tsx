import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { fetchProfiles, createProfile, deleteProfile } from '../lib/api';
import { Profile } from '../lib/types';

export function Profiles() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
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

  async function handleDelete(id: string, name: string) {
    if (!confirm(`Delete profile "${name}"? This cannot be undone.`)) return;
    await deleteProfile(id);
    await loadProfiles();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') handleCreate();
  }

  return h('div', { class: 'profiles-page' },
    h('section', { class: 'profiles-header' },
      h('div', null,
        h('h2', null, 'Profiles'),
        h('p', { class: 'page-help' }, 'Manage your Stream Deck profiles')
      ),
    ),

    h('div', { class: 'create-profile' },
      h('input', {
        type: 'text',
        placeholder: 'New profile name...',
        value: newProfileName,
        onInput: (e: any) => setNewProfileName(e.target.value),
        onKeyDown: handleKeyDown,
      }),
      h('button', {
        class: 'primary-btn',
        onClick: handleCreate,
        disabled: !newProfileName.trim(),
      }, 'Create Profile')
    ),

    profiles.length === 0
      ? h('div', { class: 'profiles-empty' },
          h('p', { class: 'profiles-empty-title' }, 'No profiles yet'),
          h('p', { class: 'profiles-empty-hint' }, 'Create your first profile to get started')
        )
      : h('div', { class: 'profile-list' },
          profiles.map(profile =>
            h('div', { class: 'profile-item', key: profile.id },
              h('div', { class: 'profile-info' },
                h('h3', { class: 'profile-name' }, profile.name),
                h('div', { class: 'profile-stats' },
                  h('span', { class: 'profile-stat' },
                    h('span', { class: 'profile-stat-value' }, String(profile.pages.length)),
                    h('span', { class: 'profile-stat-label' }, `page${profile.pages.length !== 1 ? 's' : ''}`)
                  ),
                  h('span', { class: 'profile-stat' },
                    h('span', { class: 'profile-stat-value' }, String(profile.pages[0]?.buttons.length || 0)),
                    h('span', { class: 'profile-stat-label' }, 'buttons')
                  )
                )
              ),
              h('button', {
                class: 'danger-btn',
                onClick: () => handleDelete(profile.id, profile.name),
              }, 'Delete')
            )
          )
        )
  );
}
