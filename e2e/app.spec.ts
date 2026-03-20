import { test, expect } from '@playwright/test';

// ---------------------------------------------------------------------------
// App Shell
// ---------------------------------------------------------------------------

test.describe('App Shell', () => {
  test('renders the main layout with sidebar navigation', async ({ page }) => {
    await page.goto('/');

    // The sidebar nav should be visible once React hydrates
    const sidebar = page.getByTestId('sidebar-nav');
    await expect(sidebar).toBeVisible();

    // Verify all five nav buttons exist
    for (const mode of ['overview', 'observability', 'replay', 'ecosystem', 'cloud']) {
      await expect(page.getByTestId(`nav-${mode}`)).toBeVisible();
    }
  });

  test('shows the Shepherd brand in the header', async ({ page }) => {
    await page.goto('/');

    // The header contains the app name
    const heading = page.getByRole('heading', { name: /shepherd/i });
    await expect(heading).toBeVisible();
  });

  test('shows disconnected status when no backend server', async ({ page }) => {
    await page.goto('/');

    // Without the Axum backend the WebSocket cannot connect. The header
    // renders a status label; it should eventually settle on Disconnected
    // (it may briefly show "Connecting..." first).
    await expect(page.getByText(/disconnected/i)).toBeVisible({ timeout: 10_000 });
  });

  test('renders without unexpected console errors', async ({ page }) => {
    const errors: string[] = [];

    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    await page.goto('/');

    // Wait for React to render and initial connection attempts to complete
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    // Filter out expected network errors (backend not running)
    const unexpectedErrors = errors.filter(
      (e) =>
        !e.includes('WebSocket') &&
        !e.includes('ws://') &&
        !e.includes('fetch') &&
        !e.includes('127.0.0.1') &&
        !e.includes('ERR_CONNECTION') &&
        !e.includes('Failed to fetch') &&
        !e.includes('NetworkError') &&
        !e.includes('net::') &&
        !e.includes('localhost') &&
        !e.includes('Failed to load resource') &&
        !e.includes('Range Not Satisfiable'),
    );

    expect(unexpectedErrors).toHaveLength(0);
  });

  test('has a page title', async ({ page }) => {
    await page.goto('/');
    const title = await page.title();
    expect(title).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

test.describe('Navigation', () => {
  test('starts on the kanban overview by default', async ({ page }) => {
    await page.goto('/');

    // The overview nav button should be active (indicated by bg-blue-600)
    const overviewBtn = page.getByTestId('nav-overview');
    await expect(overviewBtn).toBeVisible();

    // The kanban columns should be rendered (look for well-known column labels)
    await expect(page.getByRole('heading', { name: 'Queued' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Running' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Done' })).toBeVisible();
  });

  test('switches to Costs view when nav button clicked', async ({ page }) => {
    await page.goto('/');

    // Wait for initial render
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    // Click the observability (Costs) nav button
    await page.getByTestId('nav-observability').click();

    // Cost dashboard shows either spending data or the empty state
    await expect(page.getByTestId('no-spending').or(page.getByText('Cost Dashboard'))).toBeVisible();
  });

  test('switches to Cloud view when nav button clicked', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    await page.getByTestId('nav-cloud').click();

    // Cloud settings shows one of its states: loading, unavailable, or unauthenticated
    const cloudState = page.getByTestId('cloud-loading')
      .or(page.getByTestId('cloud-unavailable'))
      .or(page.getByTestId('cloud-unauthenticated'))
      .or(page.getByTestId('cloud-authenticated'));
    await expect(cloudState).toBeVisible({ timeout: 10_000 });
  });

  test('switches back to overview from another view', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    // Go to costs view
    await page.getByTestId('nav-observability').click();
    await expect(page.getByTestId('no-spending').or(page.getByText('Cost Dashboard'))).toBeVisible();

    // Switch back to overview
    await page.getByTestId('nav-overview').click();

    // Kanban columns should reappear
    await expect(page.getByRole('heading', { name: 'Queued' })).toBeVisible();
  });

  test('Cmd+K opens the command palette', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    // Press Meta+K to toggle the command palette
    await page.keyboard.press('Meta+k');

    // The command palette has a search input with placeholder "Search commands..."
    const searchInput = page.getByPlaceholder('Search commands...');
    await expect(searchInput).toBeVisible();
  });

  test('Escape closes the command palette', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    // Open command palette
    await page.keyboard.press('Meta+k');
    const searchInput = page.getByPlaceholder('Search commands...');
    await expect(searchInput).toBeVisible();

    // Close with Escape
    await page.keyboard.press('Escape');
    await expect(searchInput).not.toBeVisible();
  });

  test('Cmd+N opens the new task dialog', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    await page.keyboard.press('Meta+n');

    // The New Task dialog has a heading
    const dialogHeading = page.getByRole('heading', { name: 'New Task' });
    await expect(dialogHeading).toBeVisible();
  });

  test('Escape closes the new task dialog', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    // Open new task dialog
    await page.keyboard.press('Meta+n');
    const dialogHeading = page.getByRole('heading', { name: 'New Task' });
    await expect(dialogHeading).toBeVisible();

    // Close with Escape
    await page.keyboard.press('Escape');
    await expect(dialogHeading).not.toBeVisible();
  });

  test('+ New Task button opens the new task dialog', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    // Click the header button
    await page.getByRole('button', { name: /new task/i }).click();

    const dialogHeading = page.getByRole('heading', { name: 'New Task' });
    await expect(dialogHeading).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Kanban Board
// ---------------------------------------------------------------------------

test.describe('Kanban Board', () => {
  test('displays all expected column headers', async ({ page }) => {
    await page.goto('/');

    // The board renders five columns with these labels
    const columnLabels = ['Queued', 'Running', 'Needs Input', 'Review', 'Done'];
    for (const label of columnLabels) {
      await expect(page.getByRole('heading', { name: label })).toBeVisible();
    }
  });

  test('shows empty state in each column when no tasks', async ({ page }) => {
    await page.goto('/');

    // Each column shows "No tasks" when empty. Without a backend there are
    // no tasks, so every column should contain the empty placeholder.
    const emptyLabels = page.getByText('No tasks');
    // There should be at least 5 "No tasks" elements (one per column)
    await expect(emptyLabels.first()).toBeVisible();
    const count = await emptyLabels.count();
    expect(count).toBeGreaterThanOrEqual(5);
  });

  test('each column shows a count badge', async ({ page }) => {
    await page.goto('/');

    // Each column header has a count badge. With no tasks they should all read "0"
    // The count is in a span with specific classes - just verify the columns render
    await expect(page.getByRole('heading', { name: 'Queued' })).toBeVisible();

    // The count badges are sibling spans showing "0" next to each heading
    const zeroBadges = page.locator('span').filter({ hasText: /^0$/ });
    const badgeCount = await zeroBadges.count();
    expect(badgeCount).toBeGreaterThanOrEqual(5);
  });
});

// ---------------------------------------------------------------------------
// Command Palette
// ---------------------------------------------------------------------------

test.describe('Command Palette', () => {
  test('shows action categories when opened', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    await page.keyboard.press('Meta+k');

    // The palette organizes actions into categories
    await expect(page.getByText('Tasks')).toBeVisible();
    await expect(page.getByText('View')).toBeVisible();
    await expect(page.getByText('Lifecycle')).toBeVisible();
  });

  test('shows built-in actions like Toggle View and New Task', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    await page.keyboard.press('Meta+k');

    await expect(page.getByText('Toggle View')).toBeVisible();
    await expect(page.getByText('New Task')).toBeVisible();
    await expect(page.getByText('Approve All')).toBeVisible();
  });

  test('filters actions based on search input', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    await page.keyboard.press('Meta+k');

    const searchInput = page.getByPlaceholder('Search commands...');
    await expect(searchInput).toBeVisible();

    // Type a query that should narrow down the results
    await searchInput.fill('new');

    // "New Task" should still be visible
    await expect(page.getByText('New Task')).toBeVisible();

    // Unrelated actions should be filtered out
    // (Logo Generator shouldn't match "new")
    await expect(page.getByText('Logo Generator')).not.toBeVisible();
  });

  test('shows "No matching commands" for impossible search', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    await page.keyboard.press('Meta+k');

    const searchInput = page.getByPlaceholder('Search commands...');
    await searchInput.fill('zzzzxqwpj');

    await expect(page.getByText('No matching commands')).toBeVisible();
  });

  test('shows keyboard hint footer', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    await page.keyboard.press('Meta+k');

    await expect(page.getByText('navigate')).toBeVisible();
    await expect(page.getByText('select')).toBeVisible();
    await expect(page.getByText('close')).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// New Task Dialog
// ---------------------------------------------------------------------------

test.describe('New Task Dialog', () => {
  test('contains form fields for task creation', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    await page.keyboard.press('Meta+n');

    // Task Prompt textarea
    await expect(page.getByPlaceholder(/describe what you want/i)).toBeVisible();

    // Agent selector buttons
    await expect(page.getByRole('button', { name: 'Claude Code' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Codex CLI' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Aider' })).toBeVisible();

    // Isolation mode radio buttons
    await expect(page.getByRole('radio', { name: 'Worktree' })).toBeVisible();
    await expect(page.getByRole('radio', { name: 'Docker' })).toBeVisible();
    await expect(page.getByRole('radio', { name: 'Local' })).toBeVisible();

    // Submit and cancel buttons
    await expect(page.getByRole('button', { name: 'Create Task' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Cancel' })).toBeVisible();
  });

  test('Cancel button closes the dialog', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    await page.keyboard.press('Meta+n');
    const heading = page.getByRole('heading', { name: 'New Task' });
    await expect(heading).toBeVisible();

    await page.getByRole('button', { name: 'Cancel' }).click();
    await expect(heading).not.toBeVisible();
  });

  test('defaults to Claude Code agent and Worktree isolation', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    await page.keyboard.press('Meta+n');

    // The Claude Code button should have the active/selected styling
    // (border-blue-500 class indicates selection)
    const claudeBtn = page.getByRole('button', { name: 'Claude Code' });
    await expect(claudeBtn).toHaveClass(/border-blue-500/);

    // Worktree radio should be checked
    const worktreeRadio = page.getByRole('radio', { name: 'Worktree' });
    await expect(worktreeRadio).toBeChecked();
  });
});

// ---------------------------------------------------------------------------
// Responsive Layout
// ---------------------------------------------------------------------------

test.describe('Responsive Layout', () => {
  test('renders on a narrow viewport (768px)', async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 600 });
    await page.goto('/');

    // Core elements must still be visible
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();
    await expect(page.getByRole('heading', { name: /shepherd/i })).toBeVisible();

    // At least one kanban column should be visible (may need horizontal scroll)
    await expect(page.getByRole('heading', { name: 'Queued' })).toBeVisible();
  });

  test('renders on a wide viewport (1920px)', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto('/');

    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    // All kanban columns should be visible without scrolling on a wide viewport
    for (const label of ['Queued', 'Running', 'Needs Input', 'Review', 'Done']) {
      await expect(page.getByRole('heading', { name: label })).toBeVisible();
    }
  });

  test('renders on a mobile viewport (375px)', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');

    // The app should still load without crashing
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();
    await expect(page.getByRole('heading', { name: /shepherd/i })).toBeVisible();
  });
});
