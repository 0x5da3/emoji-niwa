import { test, expect } from '@playwright/test';
import { gotoFresh, saveAndRead, scatterAndSettle, trackErrors } from './helpers';

test.describe('modes & undo', () => {
  test('mode switch toggles palette / terrain-palette', async ({ page }) => {
    await gotoFresh(page);

    await expect(page.locator('#mode-place')).toHaveClass(/active/);
    await expect(page.locator('#palette')).toBeVisible();
    await expect(page.locator('#terrain-palette')).toBeHidden();

    await page.click('#mode-terrain');
    await expect(page.locator('#mode-terrain')).toHaveClass(/active/);
    await expect(page.locator('#mode-place')).not.toHaveClass(/active/);
    await expect(page.locator('#terrain-palette')).toBeVisible();
    await expect(page.locator('#palette')).toBeHidden();

    await page.click('#mode-erase');
    await expect(page.locator('#mode-erase')).toHaveClass(/active/);
    await expect(page.locator('#terrain-palette')).toBeHidden();
    await expect(page.locator('#palette')).toBeVisible();

    await page.click('#mode-place');
    await expect(page.locator('#mode-place')).toHaveClass(/active/);
    await expect(page.locator('#palette')).toBeVisible();
  });

  test('undo reverts the last scatter-spawned animal, or is a safe no-op', async ({
    page,
  }) => {
    const errors = trackErrors(page);
    await gotoFresh(page);

    const before = await scatterAndSettle(page);
    const histBefore = (before.history || []).length;
    const animalsBefore = Object.keys(before.animals || {}).length;
    const itemsBefore = Object.keys(before.items || {}).length;

    await page.click('#btn-undo');
    const after = await saveAndRead(page);

    if (histBefore > 0) {
      // Scatter only pushes 'animal' history entries.
      expect((after!.history || []).length).toBe(histBefore - 1);
      expect(Object.keys(after!.animals).length).toBe(animalsBefore - 1);
    } else {
      // Empty history → undo must change nothing and must not throw.
      expect((after!.history || []).length).toBe(0);
      expect(Object.keys(after!.items).length).toBe(itemsBefore);
    }

    expect(errors(), errors().join('\n')).toHaveLength(0);
  });
});
