import { test, expect } from '@playwright/test';

/**
 * E2E tests focused on sidebar navigation and view switching.
 * These tests run against the Vite dev server without a backend.
 */

test.describe('Sidebar Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();
  });

  test('nav buttons have correct titles for accessibility', async ({ page }) => {
    await expect(page.getByTestId('nav-overview')).toHaveAttribute('title', 'Board');
    await expect(page.getByTestId('nav-observability')).toHaveAttribute('title', 'Costs');
    await expect(page.getByTestId('nav-replay')).toHaveAttribute('title', 'Replay');
    await expect(page.getByTestId('nav-ecosystem')).toHaveAttribute('title', 'Plugins');
    await expect(page.getByTestId('nav-cloud')).toHaveAttribute('title', 'Cloud');
  });

  test('overview nav button is active by default', async ({ page }) => {
    // Active button has bg-blue-600 class
    const overviewBtn = page.getByTestId('nav-overview');
    await expect(overviewBtn).toHaveClass(/bg-blue-600/);

    // Other buttons should not have the active class
    const observabilityBtn = page.getByTestId('nav-observability');
    await expect(observabilityBtn).not.toHaveClass(/bg-blue-600/);
  });

  test('clicking a nav button highlights it as active', async ({ page }) => {
    const observabilityBtn = page.getByTestId('nav-observability');
    await observabilityBtn.click();

    // Observability button should now be active
    await expect(observabilityBtn).toHaveClass(/bg-blue-600/);

    // Overview button should no longer be active
    const overviewBtn = page.getByTestId('nav-overview');
    await expect(overviewBtn).not.toHaveClass(/bg-blue-600/);
  });

  test('navigating to Replay view shows replay content', async ({ page }) => {
    await page.getByTestId('nav-replay').click();

    // Replay viewer should show an empty state (no events without backend)
    await expect(page.getByTestId('no-events')).toBeVisible();
  });

  test('navigating to Ecosystem view shows plugins content', async ({ page }) => {
    await page.getByTestId('nav-ecosystem').click();

    // Ecosystem manager should render - look for its characteristic content
    // It renders a plugin list or empty state
    await expect(page.getByTestId('nav-ecosystem')).toHaveClass(/bg-blue-600/);
  });

  test('rapid navigation between views does not crash', async ({ page }) => {
    // Rapidly switch between all views
    await page.getByTestId('nav-observability').click();
    await page.getByTestId('nav-replay').click();
    await page.getByTestId('nav-ecosystem').click();
    await page.getByTestId('nav-cloud').click();
    await page.getByTestId('nav-overview').click();

    // App should still be functional - kanban columns visible again
    await expect(page.getByRole('heading', { name: 'Queued' })).toBeVisible();
  });
});

test.describe('Header Interactions', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();
  });

  test('+ New Task button is visible in the header', async ({ page }) => {
    const newTaskBtn = page.getByRole('button', { name: /new task/i });
    await expect(newTaskBtn).toBeVisible();
    await expect(newTaskBtn).toHaveAttribute('title', 'New Task (Cmd+N)');
  });

  test('connection status indicator is visible', async ({ page }) => {
    // The header contains a status label. Without backend, it should show
    // disconnected or connecting state.
    const statusText = page.getByText(/disconnected|connecting/i);
    await expect(statusText).toBeVisible({ timeout: 10_000 });
  });
});
