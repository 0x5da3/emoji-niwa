import { test, expect } from '@playwright/test';
import { gotoFresh, readSave, trackErrors } from './helpers';

test.describe('i18n', () => {
  test('language switch updates the UI, keeps 🎲, and persists', async ({ page }) => {
    const errors = trackErrors(page);
    await gotoFresh(page);

    const jaText = await page.locator('#mode-place').textContent();

    await page.click('#btn-menu');
    await expect(page.locator('#menu-popup')).toBeVisible();
    await page.click('#lang-opts .lang-opt[data-lang="en"]');

    await expect(page.locator('html')).toHaveAttribute('lang', 'en');
    await expect(page.locator('#lang-opts .lang-opt[data-lang="en"]')).toHaveClass(/active/);
    const enText = await page.locator('#mode-place').textContent();
    expect(enText).not.toBe(jaText);
    // Weather-random badge survives the relabel (applyLang → refreshWeatherBtn).
    await expect(page.locator('#weather-btn')).toContainText('🎲');

    // setLang() persists the choice synchronously.
    expect((await readSave(page))?.lang).toBe('en');

    // Survives a reload.
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('lang', 'en');

    // Switch back to Japanese.
    await page.click('#btn-menu');
    await page.click('#lang-opts .lang-opt[data-lang="ja"]');
    await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
    expect(await page.locator('#mode-place').textContent()).toBe(jaText);
    expect((await readSave(page))?.lang).toBe('ja');

    expect(errors(), errors().join('\n')).toHaveLength(0);
  });
});
