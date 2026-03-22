import { test, expect } from '@playwright/test';

/**
 * E2E tests for the Cloud Settings view.
 * Without a backend, getCloudStatus() fails and the component
 * transitions: loading → unavailable.
 */

test.describe('Cloud Settings', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();
    await page.getByTestId('nav-cloud').click();
  });

  test('transitions from loading to unavailable state', async ({ page }) => {
    // The initial state is "loading" — may flash briefly
    // Eventually settles on "unavailable" since getCloudStatus() fails
    await expect(page.getByTestId('cloud-unavailable')).toBeVisible({ timeout: 5_000 });
  });

  test('unavailable state shows Cloud Features heading', async ({ page }) => {
    await expect(page.getByTestId('cloud-unavailable')).toBeVisible({ timeout: 5_000 });
    await expect(page.getByRole('heading', { name: 'Cloud Features' })).toBeVisible();
  });

  test('unavailable state describes setup instructions', async ({ page }) => {
    await expect(page.getByTestId('cloud-unavailable')).toBeVisible({ timeout: 5_000 });
    await expect(page.getByText(/Set up your shepherd\.pro account/)).toBeVisible();
  });

  test('cloud nav button is active', async ({ page }) => {
    await expect(page.getByTestId('nav-cloud')).toHaveClass(/bg-blue-600/);
  });

  test('can navigate away and back', async ({ page }) => {
    await expect(page.getByTestId('cloud-unavailable')).toBeVisible({ timeout: 5_000 });

    // Navigate away
    await page.getByTestId('nav-overview').click();
    await expect(page.getByRole('heading', { name: 'Queued' })).toBeVisible();

    // Navigate back — should show unavailable again
    await page.getByTestId('nav-cloud').click();
    await expect(page.getByTestId('cloud-unavailable')).toBeVisible({ timeout: 5_000 });
  });
});
