import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import AccountCard from './AccountCard.svelte';
import { i18n } from '../i18n.svelte.js';

function buildItem(overrides = {}) {
  return {
    id: 1,
    title: 'GitHub',
    username: 'octocat',
    password: 's3cret',
    website: 'github.com',
    icon: 'GI',
    color: '#6750A4',
    folder_id: null,
    folder_icon: 'folder',
    is_favorite: false,
    is_deleted: false,
    icon_data: null,
    has_totp: false,
    totp: null,
    ...overrides
  };
}

function buildProps(item, overrides = {}) {
  return {
    item,
    userColor: '#6750A4',
    expanded: true,
    onToggleExpanded: () => {},
    isFavorite: item.is_favorite,
    isDeleted: item.is_deleted,
    totpProgress: 100,
    totpSeconds: 30,
    onToggleFavorite: () => {},
    onTogglePassword: () => {},
    onRestore: () => {},
    onEdit: () => {},
    onDelete: () => {},
    isVisible: true,
    iconMap: {},
    loadedIcons: {},
    onIconLoad: () => {},
    onIconError: () => {},
    getIconUrl: () => null,
    ...overrides
  };
}

describe('AccountCard', () => {
  let target;

  beforeEach(() => {
    i18n.lang = 'es';
    target = document.createElement('div');
    document.body.appendChild(target);
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
      configurable: true
    });
  });

  it('renders the title and username', () => {
    const item = buildItem();
    const component = mount(AccountCard, { target, props: buildProps(item) });
    flushSync();

    expect(target.textContent).toContain('GitHub');
    expect(target.textContent).toContain('octocat');

    unmount(component);
    target.remove();
  });

  it('invokes onEdit and onDelete with the item', () => {
    const item = buildItem();
    const onEdit = vi.fn();
    const onDelete = vi.fn();
    const component = mount(AccountCard, {
      target,
      props: buildProps(item, { onEdit, onDelete })
    });
    flushSync();

    target.querySelector('[aria-label="Editar entrada"]').click();
    target.querySelector('[aria-label="Eliminar entrada"]').click();
    flushSync();

    expect(onEdit).toHaveBeenCalledWith(item);
    expect(onDelete).toHaveBeenCalledWith(1);

    unmount(component);
    target.remove();
  });

  it('copies the password to the clipboard', async () => {
    const item = buildItem();
    const component = mount(AccountCard, { target, props: buildProps(item) });
    flushSync();

    target.querySelector('[aria-label="Contraseña"]').click();
    await Promise.resolve();

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('s3cret');

    unmount(component);
    target.remove();
  });
});
