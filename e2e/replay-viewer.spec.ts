import { test, expect } from '@playwright/test';

/**
 * E2E tests for the Replay Viewer.
 * Without a backend and no focused task, the ReplayViewer
 * receives no taskId so the useEffect skips the API call,
 * leaving the events array empty → "No events" empty state.
 */

test.describe('Replay Viewer', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();
    await page.getByTestId('nav-replay').click();
  });

  test('renders the empty events state', async ({ page }) => {
    await expect(page.getByTestId('no-events')).toBeVisible();
    await expect(page.getByText('No events')).toBeVisible();
  });

  test('replay nav button is active', async ({ page }) => {
    await expect(page.getByTestId('nav-replay')).toHaveClass(/bg-blue-600/);
  });

  test('can switch directly to another view', async ({ page }) => {
    await page.getByTestId('nav-ecosystem').click();
    await expect(page.getByTestId('nav-ecosystem')).toHaveClass(/bg-blue-600/);
    await expect(page.getByTestId('nav-replay')).not.toHaveClass(/bg-blue-600/);
  });
});
