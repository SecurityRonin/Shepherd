import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { Iterm2Badge } from '../Iterm2Badge';
import { SessionPicker } from '../SessionPicker';
import { SetupPrompt } from '../SetupPrompt';

describe('Iterm2Badge', () => {
  it('renders iTerm2 pill', () => {
    render(<Iterm2Badge />);
    expect(screen.getByText(/iterm2/i)).toBeTruthy();
  });
});

describe('SessionPicker', () => {
  it('renders Resume and Start Fresh buttons', () => {
    render(
      <SessionPicker
        taskId={1}
        sessions={['session-abc', 'session-def']}
        onResume={vi.fn()}
        onFresh={vi.fn()}
      />
    );
    expect(screen.getByRole('button', { name: /resume/i })).toBeTruthy();
    expect(screen.getByRole('button', { name: /start fresh/i })).toBeTruthy();
  });

  it('shows session options in dropdown', () => {
    render(
      <SessionPicker
        taskId={1}
        sessions={['session-abc', 'session-def']}
        onResume={vi.fn()}
        onFresh={vi.fn()}
      />
    );
    expect(screen.getByText('session-abc')).toBeTruthy();
    expect(screen.getByText('session-def')).toBeTruthy();
  });

  it('renders "No sessions" when list is empty', () => {
    render(
      <SessionPicker
        taskId={1}
        sessions={[]}
        onResume={vi.fn()}
        onFresh={vi.fn()}
      />
    );
    expect(screen.getByText(/no sessions/i)).toBeTruthy();
  });

  it('disables Resume button when no sessions available', () => {
    render(
      <SessionPicker
        taskId={1}
        sessions={[]}
        onResume={vi.fn()}
        onFresh={vi.fn()}
      />
    );
    const resumeBtn = screen.getByRole('button', { name: /resume/i });
    expect(resumeBtn).toBeDisabled();
  });

  it('calls onResume with selected session id', () => {
    const onResume = vi.fn();
    render(
      <SessionPicker
        taskId={1}
        sessions={['session-abc']}
        onResume={onResume}
        onFresh={vi.fn()}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /resume/i }));
    expect(onResume).toHaveBeenCalledWith('session-abc');
  });

  it('calls onFresh when Start Fresh clicked', () => {
    const onFresh = vi.fn();
    render(
      <SessionPicker
        taskId={1}
        sessions={['session-abc']}
        onResume={vi.fn()}
        onFresh={onFresh}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /start fresh/i }));
    expect(onFresh).toHaveBeenCalled();
  });
});

describe('SetupPrompt', () => {
  it('renders setup instructions', () => {
    render(<SetupPrompt onDismiss={vi.fn()} />);
    expect(screen.getAllByText(/iterm2/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/shepherd-bridge\.py/i)).toBeTruthy();
  });

  it('calls onDismiss when dismissed', () => {
    const onDismiss = vi.fn();
    render(<SetupPrompt onDismiss={onDismiss} />);
    const btn = screen.getByRole('button', { name: /dismiss/i });
    fireEvent.click(btn);
    expect(onDismiss).toHaveBeenCalled();
  });

  it('renders the setup title', () => {
    render(<SetupPrompt onDismiss={vi.fn()} />);
    expect(screen.getByText('Enable iTerm2 Integration')).toBeTruthy();
  });

  it('renders three numbered steps', () => {
    render(<SetupPrompt onDismiss={vi.fn()} />);
    const listItems = screen.getAllByRole('listitem');
    expect(listItems.length).toBe(3);
  });

  it('mentions AutoLaunch directory in setup instructions', () => {
    render(<SetupPrompt onDismiss={vi.fn()} />);
    expect(screen.getByText(/AutoLaunch/)).toBeTruthy();
  });
});

// --- Additional Iterm2Badge tests ---

describe('Iterm2Badge - styling', () => {
  it('renders as an inline badge with correct styling classes', () => {
    const { container } = render(<Iterm2Badge />);
    const badge = container.firstElementChild;
    expect(badge?.className).toContain('rounded-full');
    expect(badge?.className).toContain('text-xs');
  });
});

// --- Additional SessionPicker tests ---

describe('SessionPicker - selection change', () => {
  it('changes selected session when dropdown value changes', () => {
    const onResume = vi.fn();
    render(
      <SessionPicker
        taskId={1}
        sessions={['session-abc', 'session-def']}
        onResume={onResume}
        onFresh={vi.fn()}
      />
    );
    // Change the select to second option
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'session-def' } });
    // Click resume - should call with the newly selected session
    fireEvent.click(screen.getByRole('button', { name: /resume/i }));
    expect(onResume).toHaveBeenCalledWith('session-def');
  });
});
