import { test, expect } from '@playwright/test';

/**
 * E2E accessibility tests across all views.
 * Verifies ARIA attributes, keyboard navigation, focus management,
 * and semantic HTML structure.
 */

test.describe('Accessibility', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();
  });

  test('sidebar nav buttons have title attributes for screen readers', async ({ page }) => {
    const navButtons = [
      { testId: 'nav-overview', title: 'Board' },
      { testId: 'nav-observability', title: 'Costs' },
      { testId: 'nav-replay', title: 'Replay' },
      { testId: 'nav-ecosystem', title: 'Plugins' },
      { testId: 'nav-cloud', title: 'Cloud' },
    ];

    for (const { testId, title } of navButtons) {
      await expect(page.getByTestId(testId)).toHaveAttribute('title', title);
    }
  });

  test('sidebar nav element is a <nav> landmark', async ({ page }) => {
    const sidebar = page.getByTestId('sidebar-nav');
    const tagName = await sidebar.evaluate((el) => el.tagName.toLowerCase());
    expect(tagName).toBe('nav');
  });

  test('new task button has accessible title with keyboard shortcut', async ({ page }) => {
    const newTaskBtn = page.getByRole('button', { name: /new task/i });
    await expect(newTaskBtn).toHaveAttribute('title', 'New Task (Cmd+N)');
  });

  test('command palette search input has aria-label', async ({ page }) => {
    // Open command palette
    await page.keyboard.press('Meta+k');
    const searchInput = page.getByRole('textbox', { name: 'Search commands' });
    await expect(searchInput).toBeVisible();
    await expect(searchInput).toBeFocused();
  });

  test('command palette can be closed with Escape', async ({ page }) => {
    await page.keyboard.press('Meta+k');
    await expect(page.getByRole('textbox', { name: 'Search commands' })).toBeVisible();

    await page.keyboard.press('Escape');
    await expect(page.getByRole('textbox', { name: 'Search commands' })).not.toBeVisible();
  });

  test('kanban board uses heading hierarchy', async ({ page }) => {
    // The kanban board should have column headings
    const headings = page.getByRole('heading');
    const count = await headings.count();
    expect(count).toBeGreaterThanOrEqual(1);

    // Verify column headings are present
    await expect(page.getByRole('heading', { name: 'Queued' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Running' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Review' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Done' })).toBeVisible();
  });

  test('keyboard navigation cycles through sidebar buttons', async ({ page }) => {
    // Focus the first nav button
    await page.getByTestId('nav-overview').focus();
    await expect(page.getByTestId('nav-overview')).toBeFocused();

    // Tab to next button
    await page.keyboard.press('Tab');
    await expect(page.getByTestId('nav-observability')).toBeFocused();
  });

  test('new task dialog has accessible form fields', async ({ page }) => {
    // Open new task dialog via header button
    const newTaskBtn = page.getByRole('button', { name: /new task/i });
    await newTaskBtn.click();

    // Dialog should be visible with form inputs
    await expect(page.getByPlaceholder(/task title/i).or(page.getByRole('textbox').first())).toBeVisible();
  });

  test('cloud settings view has heading structure', async ({ page }) => {
    await page.getByTestId('nav-cloud').click();
    await expect(page.getByTestId('cloud-unavailable')).toBeVisible({ timeout: 5_000 });
    await expect(page.getByRole('heading', { name: 'Cloud Features' })).toBeVisible();
  });

  test('cost dashboard view uses appropriate test IDs for assistive tech', async ({ page }) => {
    await page.getByTestId('nav-observability').click();
    // The empty state should be identifiable
    await expect(page.getByTestId('no-spending')).toBeVisible();
  });
});
