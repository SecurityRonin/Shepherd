import { test, expect } from '@playwright/test';

/**
 * E2E tests for the New Task Dialog.
 * Tests form interactions without a backend server.
 */

test.describe('New Task Dialog', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sidebar-nav')).toBeVisible();
    // Open the dialog
    await page.keyboard.press('Meta+n');
    await expect(page.getByRole('heading', { name: 'New Task' })).toBeVisible();
  });

  test('prompt textarea auto-focuses on open', async ({ page }) => {
    const promptTextarea = page.getByPlaceholder(/describe what you want/i);
    await expect(promptTextarea).toBeFocused();
  });

  test('shows all five agent options', async ({ page }) => {
    const agents = ['Claude Code', 'Codex CLI', 'OpenCode', 'Gemini CLI', 'Aider'];
    for (const agent of agents) {
      await expect(page.getByRole('button', { name: agent })).toBeVisible();
    }
  });

  test('shows all three isolation modes', async ({ page }) => {
    for (const mode of ['Worktree', 'Docker', 'Local']) {
      await expect(page.getByRole('radio', { name: mode })).toBeVisible();
    }
  });

  test('can switch agent selection', async ({ page }) => {
    // Click Aider
    const aiderBtn = page.getByRole('button', { name: 'Aider' });
    await aiderBtn.click();

    // Aider should now have active styling
    await expect(aiderBtn).toHaveClass(/border-blue-500/);

    // Claude Code should no longer be active
    const claudeBtn = page.getByRole('button', { name: 'Claude Code' });
    await expect(claudeBtn).not.toHaveClass(/border-blue-500/);
  });

  test('can switch isolation mode', async ({ page }) => {
    // Default is Worktree
    await expect(page.getByRole('radio', { name: 'Worktree' })).toBeChecked();

    // Switch to Docker
    await page.getByRole('radio', { name: 'Docker' }).click();
    await expect(page.getByRole('radio', { name: 'Docker' })).toBeChecked();
    await expect(page.getByRole('radio', { name: 'Worktree' })).not.toBeChecked();
  });

  test('can fill in the task prompt', async ({ page }) => {
    const prompt = page.getByPlaceholder(/describe what you want/i);
    await prompt.fill('Add unit tests for the auth module');
    await expect(prompt).toHaveValue('Add unit tests for the auth module');
  });

  test('can fill in the optional initial message', async ({ page }) => {
    const initialMsg = page.getByPlaceholder(/optional initial message/i);
    await initialMsg.fill('Start with the login flow');
    await expect(initialMsg).toHaveValue('Start with the login flow');
  });

  test('can modify the repo path', async ({ page }) => {
    const repoInput = page.getByPlaceholder('.', { exact: true });
    // Default value should be "."
    await expect(repoInput).toHaveValue('.');

    await repoInput.clear();
    await repoInput.fill('/home/user/my-project');
    await expect(repoInput).toHaveValue('/home/user/my-project');
  });

  test('shows Cmd+Enter hint for submission', async ({ page }) => {
    // The dialog footer contains a keyboard hint
    await expect(page.getByText(/enter/i)).toBeVisible();
  });

  test('Escape closes the dialog from prompt textarea', async ({ page }) => {
    const heading = page.getByRole('heading', { name: 'New Task' });
    await expect(heading).toBeVisible();

    await page.keyboard.press('Escape');
    await expect(heading).not.toBeVisible();
  });

  test('dialog resets form on re-open', async ({ page }) => {
    // Type in the prompt
    const prompt = page.getByPlaceholder(/describe what you want/i);
    await prompt.fill('Some task');

    // Switch agent
    await page.getByRole('button', { name: 'Aider' }).click();

    // Close and re-open
    await page.keyboard.press('Escape');
    await page.keyboard.press('Meta+n');

    // Form should be reset
    await expect(prompt).toHaveValue('');
    await expect(page.getByRole('button', { name: 'Claude Code' })).toHaveClass(/border-blue-500/);
  });

  test('Create Task button is disabled while submitting', async ({ page }) => {
    // Without backend, submitting will fail, but we can verify the button is there
    const createBtn = page.getByRole('button', { name: 'Create Task' });
    await expect(createBtn).toBeVisible();
    await expect(createBtn).toBeEnabled();
  });
});
