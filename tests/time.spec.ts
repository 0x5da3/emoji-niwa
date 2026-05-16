import { test, expect } from '@playwright/test';
import { gotoFresh, saveAndRead, trackErrors } from './helpers';

test.describe('time of day', () => {
  test('preset sets & persists timeOfDay; pause toggle persists dayPaused', async ({
    page,
  }) => {
    const errors = trackErrors(page);
    await gotoFresh(page);

    await page.click('#clock-display');
    await expect(page.locator('#time-popup')).toBeVisible();

    // 18:00 preset → timeOfDay = 1080/1440, and selecting a time pauses it.
    await page.click('.time-preset[data-min="1080"]');
    let s = await saveAndRead(page);
    expect(s!.timeOfDay).toBeCloseTo(0.75, 5);
    expect(s!.dayPaused).toBe(true);

    // Resume.
    await page.click('#time-toggle-pause');
    s = await saveAndRead(page);
    expect(s!.dayPaused).toBe(false);
    expect(s!.timeOfDay).toBeCloseTo(0.75, 5);

    expect(errors(), errors().join('\n')).toHaveLength(0);
  });
});
