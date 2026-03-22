import { test, expect } from '@playwright/test';

/**
 * E2E tests for the Cost Dashboard (Observability view).
 * Without a backend, fetchMetrics() fails and the dashboard
 * renders its "No spending data" empty state.
 */

test.describe('Cost Dashboard', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();
    await page.getByTestId('nav-observability').click();
  });

  test('renders the empty spending state when no backend', async ({ page }) => {
    await expect(page.getByTestId('no-spending')).toBeVisible();
    await expect(page.getByText('No spending data')).toBeVisible();
  });

  test('observability nav button is active', async ({ page }) => {
    await expect(page.getByTestId('nav-observability')).toHaveClass(/bg-blue-600/);
    await expect(page.getByTestId('nav-overview')).not.toHaveClass(/bg-blue-600/);
  });

  test('can navigate back to overview from cost dashboard', async ({ page }) => {
    await page.getByTestId('nav-overview').click();
    await expect(page.getByTestId('nav-overview')).toHaveClass(/bg-blue-600/);
    await expect(page.getByRole('heading', { name: 'Queued' })).toBeVisible();
  });
});
