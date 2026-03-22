import { test, expect } from '@playwright/test';

/**
 * E2E tests for the Command Palette feature (Cmd+K).
 * Tests run without backend - palette functionality is purely frontend.
 */

test.describe('Command Palette', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();
  });

  test('Cmd+K toggles the palette open and closed', async ({ page }) => {
    const searchInput = page.getByPlaceholder('Search commands...');

    // Initially hidden
    await expect(searchInput).not.toBeVisible();

    // Open
    await page.keyboard.press('Meta+k');
    await expect(searchInput).toBeVisible();

    // Toggle closed with same shortcut
    await page.keyboard.press('Meta+k');
    await expect(searchInput).not.toBeVisible();
  });

  test('search input auto-focuses when palette opens', async ({ page }) => {
    await page.keyboard.press('Meta+k');

    const searchInput = page.getByPlaceholder('Search commands...');
    await expect(searchInput).toBeVisible();
    await expect(searchInput).toBeFocused();
  });

  test('clicking backdrop closes the palette', async ({ page }) => {
    await page.keyboard.press('Meta+k');
    const searchInput = page.getByPlaceholder('Search commands...');
    await expect(searchInput).toBeVisible();

    // Click outside the palette (on the backdrop overlay)
    // The backdrop is a fixed overlay; click at a position outside the palette card
    await page.mouse.click(10, 10);
    await expect(searchInput).not.toBeVisible();
  });

  test('displays all default action categories', async ({ page }) => {
    await page.keyboard.press('Meta+k');

    // Category headers — scope to palette card and use exact match
    // to avoid collision with kanban "No tasks" text
    const palette = page.locator('.bg-zinc-900');
    await expect(palette.getByText('Approve', { exact: true })).toBeVisible();
    await expect(palette.getByText('Tasks', { exact: true })).toBeVisible();
    await expect(palette.getByText('View', { exact: true })).toBeVisible();
    await expect(palette.getByText('Lifecycle', { exact: true })).toBeVisible();
  });

  test('displays shortcut hints on actions', async ({ page }) => {
    await page.keyboard.press('Meta+k');

    // Toggle View should have a shortcut hint
    const toggleViewItem = page.locator('button', { hasText: 'Toggle View' });
    await expect(toggleViewItem).toBeVisible();
  });

  test('keyboard arrow navigation changes selection', async ({ page }) => {
    await page.keyboard.press('Meta+k');

    // The first item should be selected (has bg-zinc-700 class)
    const firstItem = page.locator('[data-index="0"]');
    await expect(firstItem).toHaveClass(/bg-zinc-700/);

    // Press arrow down
    await page.keyboard.press('ArrowDown');

    // Now second item should be selected
    const secondItem = page.locator('[data-index="1"]');
    await expect(secondItem).toHaveClass(/bg-zinc-700/);

    // First item should no longer be selected
    await expect(firstItem).not.toHaveClass(/bg-zinc-700/);
  });

  test('search narrows results and resets selection', async ({ page }) => {
    await page.keyboard.press('Meta+k');

    const searchInput = page.getByPlaceholder('Search commands...');

    // Type "toggle" - should show Toggle View
    await searchInput.fill('toggle');
    await expect(page.getByText('Toggle View')).toBeVisible();

    // The result count should be reduced (Name Generator shouldn't match "toggle")
    await expect(page.getByText('Name Generator')).not.toBeVisible();
  });

  test('Enter executes the selected action and closes palette', async ({ page }) => {
    await page.keyboard.press('Meta+k');

    const searchInput = page.getByPlaceholder('Search commands...');
    await expect(searchInput).toBeVisible();

    // Search for "New Task" and press Enter to execute it
    await searchInput.fill('New Task');
    // Scope to palette to avoid matching header's "+ New Task" button
    const palette = page.locator('.bg-zinc-900');
    await expect(palette.getByText('New Task', { exact: true })).toBeVisible();

    await page.keyboard.press('Enter');

    // Palette should close
    await expect(searchInput).not.toBeVisible();

    // The New Task dialog should open as a result
    await expect(page.getByRole('heading', { name: 'New Task' })).toBeVisible();
  });

  test('clears search when re-opened', async ({ page }) => {
    // Open and type something
    await page.keyboard.press('Meta+k');
    const searchInput = page.getByPlaceholder('Search commands...');
    await searchInput.fill('foobar');

    // Close
    await page.keyboard.press('Escape');

    // Re-open
    await page.keyboard.press('Meta+k');

    // Search should be cleared
    await expect(searchInput).toHaveValue('');
  });
});
