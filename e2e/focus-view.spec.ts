import { test, expect } from '@playwright/test';

/**
 * E2E tests for the Focus View.
 * The focus view has no sidebar nav button — it's reached via
 * enterFocus(taskId). We use the dev-mode store exposure
 * (__STORE__) to set viewMode programmatically.
 *
 * Note: Both the Header and SessionSidebar render an "Overview"
 * back button in focus mode, so we scope locators to avoid
 * strict mode violations.
 */

test.describe('Focus View', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    // Switch to focus view via the zustand store
    await page.evaluate(() => {
      (window as any).__STORE__?.setState({ viewMode: 'focus' });
    });
  });

  test('shows "No task selected" when no task is focused', async ({ page }) => {
    await expect(page.getByText('No task selected')).toBeVisible();
  });

  test('focus view renders SessionSidebar with Sessions heading', async ({ page }) => {
    // "SESSIONS" is rendered via CSS uppercase; text content is "Sessions"
    const main = page.getByRole('main');
    await expect(main.getByText('Sessions', { exact: true })).toBeVisible();
  });

  test('focus view shows Overview back button in sidebar', async ({ page }) => {
    // Scope to main to avoid matching the header's Overview button
    const main = page.getByRole('main');
    await expect(main.getByRole('button', { name: /Overview/ })).toBeVisible();
  });

  test('clicking sidebar Overview button exits focus and returns to kanban', async ({ page }) => {
    await expect(page.getByText('No task selected')).toBeVisible();

    // Click the sidebar's Overview back button (scoped to main)
    const main = page.getByRole('main');
    await main.getByRole('button', { name: /Overview/ }).click();

    // Should return to overview mode with kanban columns
    await expect(page.getByRole('heading', { name: 'Queued' })).toBeVisible();
  });

  test('session sidebar shows 0 sessions when no tasks exist', async ({ page }) => {
    await expect(page.getByText('0 sessions')).toBeVisible();
  });
});
