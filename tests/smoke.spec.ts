import { test, expect } from '@playwright/test';
import { gotoFresh, trackErrors } from './helpers';

test.describe('smoke', () => {
  test('boots cleanly with the core UI and no console/page errors', async ({ page }) => {
    const errors = trackErrors(page);
    await gotoFresh(page);

    await expect(page.locator('#cv')).toBeVisible();
    await expect(page.locator('#topbar')).toBeVisible();
    await expect(page.locator('#modebar')).toBeVisible();
    await expect(page.locator('#mode-place')).toHaveClass(/active/);
    await expect(page.locator('#corner-btns')).toBeVisible();

    // Canvas actually rendered something (terrain/background), not blank.
    const blank = await page.locator('#cv').evaluate((cv: HTMLCanvasElement) => {
      const ctx = cv.getContext('2d');
      if (!ctx || !cv.width || !cv.height) return true;
      const { data } = ctx.getImageData(0, 0, cv.width, cv.height);
      for (let i = 3; i < data.length; i += 4) if (data[i] !== 0) return false;
      return true;
    });
    expect(blank).toBe(false);

    // Weather random is ON by default (#24) → 🎲 prefix on the weather button.
    await expect(page.locator('#weather-btn')).toContainText('🎲');

    expect(errors(), errors().join('\n')).toHaveLength(0);
  });

  test('online-only buttons exist and stay safe offline', async ({ page }) => {
    const errors = trackErrors(page);
    await gotoFresh(page);

    // Share / multiplayer entry points are present...
    await expect(page.locator('#btn-share')).toBeVisible();
    await expect(page.locator('#btn-room')).toBeVisible();

    // ...and with no backend configured, opening the room must not navigate
    // away or throw — it just shows an "unconfigured" toast.
    const urlBefore = page.url();
    page.on('dialog', (d) => d.dismiss());
    await page.click('#btn-room');
    await expect(page.locator('#toast')).toBeVisible();
    expect(page.url()).toBe(urlBefore);
    await expect(page.locator('#visiting-bar')).toBeHidden();

    expect(errors(), errors().join('\n')).toHaveLength(0);
  });
});
