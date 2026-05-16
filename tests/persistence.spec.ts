import { test, expect } from '@playwright/test';
import {
  gotoFresh,
  readSave,
  readSaveRaw,
  saveAndRead,
  scatterAndSettle,
  storeItems,
  trackErrors,
} from './helpers';

test.describe('persistence', () => {
  test('a scattered world survives a reload', async ({ page }) => {
    const errors = trackErrors(page);
    await gotoFresh(page);

    const before = await scatterAndSettle(page);
    const beforeCount =
      Object.keys(before.items).length + Object.keys(before.animals).length;

    await page.reload();
    await expect(page.locator('#cv')).toBeVisible();

    const after = await saveAndRead(page);
    expect(after).not.toBeNull();
    expect(after!.grid).toBe(before.grid);
    expect(
      Object.keys(after!.items).length + Object.keys(after!.animals).length,
    ).toBe(beforeCount);
    expect(storeItems(after!)).toHaveLength(1);
    expect(storeItems(after!)[0][1].size).toBe(3);

    expect(errors(), errors().join('\n')).toHaveLength(0);
  });

  test('全リセット clears the saved world', async ({ page }) => {
    await gotoFresh(page);
    await scatterAndSettle(page);

    await page.click('#btn-menu');
    await expect(page.locator('#menu-popup')).toBeVisible();
    await page.click('#btn-clear-all');

    // clearSave() removes the key outright.
    expect(await readSaveRaw(page)).toBeNull();

    // A subsequent manual save writes an empty world.
    const s = await saveAndRead(page);
    expect(Object.keys(s!.items)).toHaveLength(0);
    expect(Object.keys(s!.animals)).toHaveLength(0);
  });

  test('新規マップ resizes the grid and empties the world', async ({ page }) => {
    await gotoFresh(page);
    await scatterAndSettle(page);

    await page.click('#btn-menu');
    await page.click('#btn-newmap');
    await expect(page.locator('#newmap-popup')).toBeVisible();
    await page.click('#nm-presets .nm-preset[data-size="15"]');
    await page.click('#nm-create');
    await expect(page.locator('#newmap-popup')).toBeHidden();

    const s = await readSave(page);
    expect(s!.grid).toBe(15);
    expect(Object.keys(s!.items)).toHaveLength(0);
    expect(Object.keys(s!.animals)).toHaveLength(0);
  });
});
