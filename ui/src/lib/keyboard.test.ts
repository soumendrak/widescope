import { beforeEach, describe, expect, it, vi } from 'vitest';
import { installKeyboardRouter, type KeyboardActions } from './keyboard';

function actions(overrides: Partial<KeyboardActions> = {}): KeyboardActions {
  return {
    toggleHelp: vi.fn(),
    closeHelp: vi.fn(),
    isHelpOpen: () => false,
    openFile: vi.fn(),
    focusSearch: vi.fn(),
    submitEditor: vi.fn(),
    pasteFromClipboard: vi.fn(),
    selectView: vi.fn(),
    viewCount: 5,
    toggleFullscreen: vi.fn(),
    exitFullscreen: vi.fn(),
    isFullscreen: () => false,
    ...overrides,
  };
}

function press(key: string, init: KeyboardEventInit = {}, target: EventTarget = document): void {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(event);
}

let teardown: () => void;
beforeEach(() => {
  document.body.innerHTML = '';
});

describe('keyboard router', () => {
  it('opens the help dialog on ? and closes it on Escape', () => {
    let open = false;
    const a = actions({
      toggleHelp: vi.fn(() => { open = true; }),
      closeHelp: vi.fn(() => { open = false; }),
      isHelpOpen: () => open,
    });
    teardown = installKeyboardRouter(a);

    press('?');
    expect(a.toggleHelp).toHaveBeenCalled();

    press('Escape');
    expect(a.closeHelp).toHaveBeenCalled();
    teardown();
  });

  it('lets the help dialog swallow every other key while it is open', () => {
    const a = actions({ isHelpOpen: () => true });
    teardown = installKeyboardRouter(a);

    press('1');
    press('o', { metaKey: true });
    expect(a.selectView).not.toHaveBeenCalled();
    expect(a.openFile).not.toHaveBeenCalled();
    teardown();
  });

  it('routes the modified shortcuts', () => {
    const a = actions();
    teardown = installKeyboardRouter(a);

    press('o', { metaKey: true });
    press('k', { ctrlKey: true });
    press('Enter', { metaKey: true });
    press('v', { metaKey: true });

    expect(a.openFile).toHaveBeenCalledOnce();
    expect(a.focusSearch).toHaveBeenCalledOnce();
    expect(a.submitEditor).toHaveBeenCalledOnce();
    expect(a.pasteFromClipboard).toHaveBeenCalledOnce();
    teardown();
  });

  it('maps the number keys onto the view order, zero-indexed', () => {
    const a = actions();
    teardown = installKeyboardRouter(a);

    press('1');
    press('5');
    expect(a.selectView).toHaveBeenNthCalledWith(1, 0);
    expect(a.selectView).toHaveBeenNthCalledWith(2, 4);

    // Beyond viewCount there is no lens to switch to.
    press('6');
    expect(a.selectView).toHaveBeenCalledTimes(2);
    teardown();
  });

  it('reaches the overflow lenses when more views are available', () => {
    const a = actions({ viewCount: 9 });
    teardown = installKeyboardRouter(a);
    press('9');
    expect(a.selectView).toHaveBeenCalledWith(8);
    teardown();
  });

  it('never fires while the user is typing', () => {
    const a = actions();
    teardown = installKeyboardRouter(a);

    for (const tag of ['input', 'textarea', 'select']) {
      const el = document.createElement(tag);
      document.body.appendChild(el);
      press('1', {}, el);
      press('o', { metaKey: true }, el);
    }
    const editable = document.createElement('div');
    editable.contentEditable = 'true';
    Object.defineProperty(editable, 'isContentEditable', { value: true });
    document.body.appendChild(editable);
    press('1', {}, editable);

    expect(a.selectView).not.toHaveBeenCalled();
    expect(a.openFile).not.toHaveBeenCalled();
    teardown();
  });

  it('toggles fullscreen on Shift+F and exits on Escape only while fullscreen', () => {
    let full = false;
    const a = actions({
      toggleFullscreen: vi.fn(() => { full = true; }),
      exitFullscreen: vi.fn(() => { full = false; }),
      isFullscreen: () => full,
    });
    teardown = installKeyboardRouter(a);

    press('Escape');
    expect(a.exitFullscreen).not.toHaveBeenCalled();

    press('F', { shiftKey: true });
    expect(a.toggleFullscreen).toHaveBeenCalled();

    press('Escape');
    expect(a.exitFullscreen).toHaveBeenCalled();
    teardown();
  });

  it('stops routing once torn down', () => {
    const a = actions();
    installKeyboardRouter(a)();
    press('1');
    expect(a.selectView).not.toHaveBeenCalled();
  });
});
