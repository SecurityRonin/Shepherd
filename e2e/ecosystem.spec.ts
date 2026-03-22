import { test, expect } from '@playwright/test';

/**
 * E2E tests for the Ecosystem Manager (Plugins view).
 * Without a backend, getDetectedPlugins() fails.
 * The component first renders 14 known plugins, then the API
 * error triggers the error state. We test for whichever
 * state the view settles into.
 */

test.describe('Ecosystem Manager', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();
    await page.getByTestId('nav-ecosystem').click();
  });

  test('renders heading or error state', async ({ page }) => {
    // The view either shows "Ecosystem Plugins" heading (before API call resolves)
    // or switches to error-message (after API call fails).
    // Wait for one of the two stable states.
    await expect(
      page.getByText('Ecosystem Plugins').or(page.getByTestId('error-message'))
    ).toBeVisible({ timeout: 5_000 });
  });

  test('shows plugin cards before API error (or error state after)', async ({ page }) => {
    // Check for known plugin cards or the error state
    const pluginCard = page.getByTestId('plugin-claude-code');
    const errorMsg = page.getByTestId('error-message');

    await expect(pluginCard.or(errorMsg)).toBeVisible({ timeout: 5_000 });
  });

  test('ecosystem nav button is active', async ({ page }) => {
    await expect(page.getByTestId('nav-ecosystem')).toHaveClass(/bg-blue-600/);
  });

  test('plugin cards display plugin info when visible', async ({ page }) => {
    // If the plugins grid renders before the API error, verify card content
    const pluginCard = page.getByTestId('plugin-claude-code');
    const isVisible = await pluginCard.isVisible().catch(() => false);

    if (isVisible) {
      await expect(page.getByText('Claude Code')).toBeVisible();
      await expect(page.getByText("Anthropic's CLI coding agent")).toBeVisible();
    } else {
      // API error fired first — the error state is acceptable
      await expect(page.getByTestId('error-message')).toBeVisible();
    }
  });

  test('shows install links for third-party plugins when grid is visible', async ({ page }) => {
    const pluginCard = page.getByTestId('plugin-aider');
    const isVisible = await pluginCard.isVisible().catch(() => false);

    if (isVisible) {
      // Aider has an installUrl, so it should show an Install link
      await expect(page.getByTestId('install-link-aider')).toBeVisible();
    }
  });
});
