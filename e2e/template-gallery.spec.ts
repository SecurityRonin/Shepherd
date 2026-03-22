import { test, expect } from '@playwright/test';

/**
 * E2E tests for the Template Gallery.
 * The template gallery has no sidebar nav button — it's only
 * reachable via setViewMode("templates"). We use the dev-mode
 * store exposure (__STORE__) to navigate there.
 *
 * Without a backend, getTemplates() fails → loading → error state.
 */

test.describe('Template Gallery', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();

    // Navigate to templates view via the zustand store
    await page.evaluate(() => {
      (window as any).__STORE__?.setState({ viewMode: 'templates' });
    });
  });

  test('shows loading state initially', async ({ page }) => {
    // The templates view starts in loading state before the API call resolves
    // It may transition quickly to error, so we check for either
    const loading = page.getByTestId('templates-loading');
    const error = page.getByTestId('templates-error');

    await expect(loading.or(error)).toBeVisible({ timeout: 5_000 });
  });

  test('transitions to error state when API fails', async ({ page }) => {
    // Without backend, getTemplates() fails → error state
    await expect(page.getByTestId('templates-error')).toBeVisible({ timeout: 5_000 });
    await expect(page.getByText(/Failed to load templates/)).toBeVisible();
  });

  test('can navigate back to overview via sidebar nav', async ({ page }) => {
    // Wait for templates view to settle
    await expect(page.getByTestId('templates-error')).toBeVisible({ timeout: 5_000 });

    // Click overview nav button
    await page.getByTestId('nav-overview').click();
    await expect(page.getByRole('heading', { name: 'Queued' })).toBeVisible();
  });
});
