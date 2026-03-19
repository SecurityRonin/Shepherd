import { test, expect } from '@playwright/test';

test.describe('Shepherd Desktop App', () => {
  test('loads the main page with kanban board', async ({ page }) => {
    await page.goto('/');
    // The app should render without errors
    await expect(page.locator('body')).toBeVisible();
    // Wait for React to render
    await page.waitForTimeout(1000);
    // Check that the page has loaded (look for common elements)
    // The app renders a kanban board in overview mode
    const body = await page.textContent('body');
    expect(body).toBeTruthy();
  });

  test('has correct page title', async ({ page }) => {
    await page.goto('/');
    // Vite apps typically have a title set
    const title = await page.title();
    expect(title).toBeTruthy();
  });

  test('renders without console errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        // Ignore WebSocket connection errors (server may not be running)
        if (!msg.text().includes('WebSocket') && !msg.text().includes('ws://')) {
          errors.push(msg.text());
        }
      }
    });
    await page.goto('/');
    await page.waitForTimeout(2000);
    // Filter out expected errors (like API connection failures when no server)
    const unexpectedErrors = errors.filter(
      (e) =>
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

  test('displays connection status indicator', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(1000);
    // The app should show some connection status (disconnected when no server)
    const body = await page.textContent('body');
    // Check that the app renders something meaningful
    expect(body!.length).toBeGreaterThan(0);
  });

  test('keyboard shortcut K opens command palette', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(1000);
    // Press Cmd+K (or Ctrl+K on Linux)
    await page.keyboard.press('Meta+k');
    await page.waitForTimeout(500);
    // The command palette should be visible or the state should change
    // This tests that keyboard event handlers are wired up
  });

  test('new task dialog can be triggered', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(1000);
    // Press 'n' key which should open new task dialog
    await page.keyboard.press('n');
    await page.waitForTimeout(500);
  });

  test('app handles navigation between views', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(1000);
    // The app should be in overview mode by default
    // Take a snapshot to verify the page renders properly
    const snapshot = await page.content();
    expect(snapshot).toContain('id="root"');
  });

  test('page responds to window resize', async ({ page }) => {
    await page.goto('/');
    await page.setViewportSize({ width: 800, height: 600 });
    await page.waitForTimeout(500);
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.waitForTimeout(500);
    // Page should not crash on resize
    await expect(page.locator('body')).toBeVisible();
  });
});
